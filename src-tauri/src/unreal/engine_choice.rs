// =============================================================================
// DBV Unreal AI — Motor de Unreal elegido a mano para un proyecto
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-119
//
// Si la app no encuentra el motor de un proyecto (o el usuario quiere otro), lo elige con el diálogo de carpetas.
// Se guarda `proyecto → carpeta del motor` en `Documentos/DBV Unreal AI/engine-overrides.json`, al lado de la
// Biblioteca: en la versión de la Store, lo que se escribe en %APPDATA% va a una copia privada que el `--mcp` (lanzado
// por un agente) no ve, y Documentos no se virtualiza (lección del 2026-10-31). Nunca se toca el `.uproject` ni el
// registro.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::discovery::{engines_from, EngineInstall};
use super::prepare::MIN_ENGINE;
use super::{io_error, UnrealError};

pub const FILE_NAME: &str = "engine-overrides.json";

/// `Documentos/DBV Unreal AI/engine-overrides.json` (la misma ruta para la ventana y el `--mcp`).
pub fn overrides_file() -> Option<PathBuf> {
    let file = crate::paths::documents_library().and_then(|library| library.parent().map(|p| p.join(FILE_NAME)));
    file
}

/// Clave de un proyecto: su ruta en minúsculas y con `/`, para que `D:\P\Demo.uproject` y `d:/p/demo.uproject` coincidan.
pub fn project_key(project: &Path) -> String {
    project.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn load(file: &Path) -> BTreeMap<String, String> {
    let map = std::fs::read_to_string(file).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
    map
}

fn save(file: &Path, map: &BTreeMap<String, String>) -> Result<(), UnrealError> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|error| io_error(parent, &error))?;
    }
    let text = serde_json::to_string_pretty(map).map_err(|error| UnrealError::Format(error.to_string()))?;
    let result = std::fs::write(file, text).map_err(|error| io_error(file, &error));
    result
}

/// La carpeta del motor elegida para `project`, si la hay.
pub fn chosen_root(file: &Path, project: &Path) -> Option<PathBuf> {
    let root = load(file).get(&project_key(project)).map(PathBuf::from);
    root
}

/// La raíz del motor desde la propia raíz o cualquier carpeta de dentro: la primera, subiendo, que tenga
/// `Engine/Build/Build.version`.
pub fn engine_root_of(folder: &Path) -> Option<PathBuf> {
    let root = folder.ancestors().find(|a| a.join("Engine").join("Build").join("Build.version").is_file()).map(Path::to_path_buf);
    root
}

/// Comprueba que `folder` es (o está dentro de) un motor válido: 5.8 o posterior y con el editor. @spec REQ-119
pub fn validate(folder: &Path) -> Result<EngineInstall, UnrealError> {
    let root = engine_root_of(folder).ok_or_else(|| UnrealError::NotFound(format!("{} no es una carpeta de Unreal Engine: no tiene Engine/Build/Build.version. Elige la carpeta del motor (por ejemplo «UE_5.8», la que contiene «Engine»)", folder.display())))?;
    let engine = engines_from(vec![root.clone()]).into_iter().next().ok_or_else(|| UnrealError::Format(format!("no se puede leer la versión de {}", root.display())))?;
    let result = if (engine.major, engine.minor) < MIN_ENGINE {
        Err(UnrealError::Format(format!("esa carpeta es Unreal {}; la app necesita Unreal {}.{} o posterior", engine.version, MIN_ENGINE.0, MIN_ENGINE.1)))
    } else if !engine.editor_exists {
        Err(UnrealError::NotFound(format!("esa instalación de Unreal {} no tiene el editor ({})", engine.version, engine.editor)))
    } else {
        Ok(engine)
    };
    result
}

/// Guarda la elección (ya validada) para `project`.
pub fn set(file: &Path, project: &Path, engine_root: &Path) -> Result<(), UnrealError> {
    let mut map = load(file);
    map.insert(project_key(project), engine_root.to_string_lossy().into_owned());
    save(file, &map)
}

/// Vuelve a la detección automática para `project`.
pub fn clear(file: &Path, project: &Path) -> Result<(), UnrealError> {
    let mut map = load(file);
    let result = if map.remove(&project_key(project)).is_some() { save(file, &map) } else { Ok(()) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_engine(dir: &Path, version: (u32, u32, u32), editor: bool) -> PathBuf {
        let build = dir.join("Engine").join("Build");
        std::fs::create_dir_all(&build).unwrap();
        std::fs::write(build.join("Build.version"), format!("{{\"MajorVersion\": {}, \"MinorVersion\": {}, \"PatchVersion\": {}}}", version.0, version.1, version.2)).unwrap();
        if editor {
            let exe = super::super::discovery::editor_executable(dir);
            std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
            std::fs::write(exe, b"").unwrap();
        }
        dir.to_path_buf()
    }

    /// @spec REQ-119 Vale la raíz o cualquier carpeta de dentro; se rechazan lo que no es un motor, una 5.7 y uno sin editor.
    #[test]
    fn valida_la_carpeta_del_motor() {
        let base = tempfile::tempdir().unwrap();
        let ue58 = fake_engine(&base.path().join("UE_5.8"), (5, 8, 4), true);
        assert_eq!(validate(&ue58).unwrap().version, "5.8.4");
        let inside = ue58.join("Engine").join("Binaries");
        assert_eq!(PathBuf::from(validate(&inside).unwrap().root), ue58, "desde una carpeta de dentro se sube hasta la raíz");
        assert!(validate(&base.path().join("nada")).unwrap_err().to_string().contains("no es una carpeta de Unreal"));
        let ue57 = fake_engine(&base.path().join("UE_5.7"), (5, 7, 4), true);
        assert!(validate(&ue57).unwrap_err().to_string().contains("5.8 o posterior"));
        let bare = fake_engine(&base.path().join("UE_5.9"), (5, 9, 0), false);
        assert!(validate(&bare).unwrap_err().to_string().contains("no tiene el editor"));
    }

    /// @spec REQ-119 La elección se guarda por proyecto, sin distinguir mayúsculas ni barras, y se puede quitar.
    #[test]
    fn guarda_y_quita_la_eleccion_por_proyecto() {
        let base = tempfile::tempdir().unwrap();
        let file = base.path().join("DBV Unreal AI").join(FILE_NAME);
        let project = Path::new(r"D:\Proyectos\Demo\Demo.uproject");
        assert_eq!(chosen_root(&file, project), None);
        set(&file, project, Path::new(r"E:\Motores\UE_5.8")).unwrap();
        assert_eq!(chosen_root(&file, Path::new("d:/proyectos/demo/demo.uproject")), Some(PathBuf::from(r"E:\Motores\UE_5.8")));
        clear(&file, project).unwrap();
        assert_eq!(chosen_root(&file, project), None);
        clear(&file, project).unwrap();
    }
}
