// =============================================================================
// DBV Unreal AI — Carpetas de la app sin AppHandle (modo --mcp)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// El servidor MCP propio (`--mcp`) no arranca Tauri, pero tiene que leer la MISMA biblioteca y
// los mismos ajustes que la ventana. Tauri usa `<dataDir>/<identifier>` y `<configDir>/<identifier>`;
// aquí se calculan igual a partir de las variables del sistema.

use std::path::{Path, PathBuf};

/// Identificador del bundle (`tauri.conf.json`).
pub const IDENTIFIER: &str = "com.davidbuenov.dbv-unreal-ai";

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)
}

/// Carpeta de datos (biblioteca, copias de seguridad, conversaciones).
pub fn data_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".local").join("share")))
    };
    base.map(|b| b.join(IDENTIFIER))
}

/// Biblioteca por defecto: `<Documentos>/DBV Unreal AI/Library` (REQ-53). Una app de la Store escribe lo que crea en
/// `%APPDATA%` en una copia privada que no ven el Explorador, Unreal ni los agentes (lección del 2026-10-31);
/// Documentos no se virtualiza. El nombre es fijo para que la ventana y el `--mcp` lleguen a la misma ruta.
pub fn documents_library() -> Option<PathBuf> {
    library_folder(dirs::document_dir(), dirs::home_dir())
}

/// Dónde va la biblioteca: en Documentos, salvo que OneDrive la sincronice (D31): entonces en la carpeta del usuario,
/// que tampoco se virtualiza, para no subir a la nube gigas de modelos y splats (visto el 2026-10-31: 1,9 GB).
pub fn library_folder(documents: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    let synced = documents.as_ref().is_some_and(|d| d.components().any(|c| c.as_os_str().to_string_lossy().to_lowercase().starts_with("onedrive")));
    let base = if synced { home.or(documents) } else { documents.or(home) };
    base.map(|b| b.join("DBV Unreal AI").join("Library"))
}

/// Carpeta de configuración (conexiones, ajustes de generación).
pub fn config_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".config")))
    };
    base.map(|b| b.join(IDENTIFIER))
}

/// Sustituye `path` por `content` sin dejarlo a medias: escribe el temporal `tmp` (en la misma carpeta) y lo renombra.
/// Si el renombrado falla, escribe directamente sobre `path`. Pasa en dos casos vistos: en la versión de la Microsoft
/// Store (MSIX), `%APPDATA%` está virtualizado y renombrar entre la carpeta virtual y la real da `os error 17`
/// («no se puede mover el archivo a otra unidad»; un usuario no podía conectar Claude, 2026-11-14); y en Windows,
/// si otro programa tiene el destino abierto sin permitir borrarlo (un antivirus, un editor). @spec REQ-90
pub fn replace_file(tmp: &Path, path: &Path, content: &[u8]) -> std::io::Result<()> {
    std::fs::write(tmp, content)?;
    let replaced = match std::fs::rename(tmp, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            let _ = std::fs::remove_file(tmp);
            std::fs::write(path, content)
        }
    };
    replaced
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Si el renombrado no se puede hacer (aquí, el destino abierto por otro programa sin permitir borrarlo; en la
    /// Store, `os error 17`), el fichero se escribe igual y no queda el temporal. @spec REQ-90
    #[cfg(windows)]
    #[test]
    fn si_no_se_puede_renombrar_escribe_directamente() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai-connections.json");
        let tmp = dir.path().join("ai-connections.json.tmp");
        std::fs::write(&path, "viejo").unwrap();
        // FILE_SHARE_READ | FILE_SHARE_WRITE, sin FILE_SHARE_DELETE: el renombrado encima falla, la escritura no.
        let _held = std::fs::OpenOptions::new().read(true).share_mode(0x1 | 0x2).open(&path).unwrap();
        assert!(std::fs::rename(&path, dir.path().join("x")).is_err(), "el bloqueo impide renombrar");
        replace_file(&tmp, &path, b"nuevo").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "nuevo");
        assert!(!tmp.exists(), "no deja el temporal");
    }

    /// El caso normal: temporal y renombrado. @spec REQ-90
    #[test]
    fn sustituye_el_fichero_por_completo() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.json");
        std::fs::write(&path, "un contenido largo de antes").unwrap();
        replace_file(&dir.path().join("a.json.tmp"), &path, b"{}").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{}");
        assert!(!dir.path().join("a.json.tmp").exists());
    }

    /// Documentos salvo que la sincronice OneDrive; entonces la carpeta del usuario. @spec REQ-53
    #[test]
    fn la_biblioteca_no_va_a_una_carpeta_de_onedrive() {
        let home = PathBuf::from("C:/Users/u");
        assert_eq!(library_folder(Some(PathBuf::from("C:/Users/u/Documents")), Some(home.clone())), Some(PathBuf::from("C:/Users/u/Documents/DBV Unreal AI/Library")));
        assert_eq!(library_folder(Some(PathBuf::from("C:/Users/u/OneDrive/Documentos2")), Some(home.clone())), Some(home.join("DBV Unreal AI").join("Library")));
        assert_eq!(library_folder(Some(PathBuf::from("C:/Users/u/OneDrive - Empresa/Documents")), Some(home.clone())), Some(home.join("DBV Unreal AI").join("Library")));
        assert_eq!(library_folder(None, None), None);
    }

    #[test]
    fn las_carpetas_acaban_en_el_identificador() {
        if let Some(dir) = data_dir() {
            assert!(dir.ends_with(IDENTIFIER));
        }
        if let Some(dir) = config_dir() {
            assert!(dir.ends_with(IDENTIFIER));
        }
    }
}
