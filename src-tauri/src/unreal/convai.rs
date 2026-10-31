// =============================================================================
// DBV Unreal AI — Convai en el proyecto: lo que se ve en los ficheros
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-55
//
// El plugin ConvAI (4.0.0-beta.27, instalado desde Fab en `Engine/Plugins/Marketplace/`) registra sus toolsets en el
// MCP de Epic y se autentica solo: al iniciar sesión en su ventana guarda `API_Key` y `AuthToken` en
// `Config/DefaultEngine.ini` del proyecto. Aquí se comprueba con el editor cerrado si está citado, instalado, si el
// motor sirve y si hay sesión. De la clave solo se mira si está VACÍA: el valor no sale de `signed_in` (R-26). Lo que
// necesita el editor (toolsets, guías) lo comprueba la vista Proyecto en vivo (D27).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use super::discovery::{EngineInstall, UProject};
use super::ini;

/// Nombre del plugin en el `.uproject` y de su `.uplugin`.
pub const PLUGIN: &str = "ConvAI";
/// Sección de sus ajustes en `DefaultEngine.ini`.
const SETTINGS: &str = "/Script/Convai.ConvaiSettings";

/// Estado de Convai en un proyecto.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvaiStatus {
    /// El `.uproject` cita el plugin.
    pub cited: bool,
    /// Y lo tiene activo.
    pub enabled: bool,
    /// Versión del plugin instalado (en el proyecto o en el motor).
    pub version: Option<String>,
    /// El motor es 5.8 o posterior (las herramientas para agentes no existen antes).
    pub engine_ok: bool,
    /// Hay sesión iniciada en el plugin (`API_Key` o `AuthToken` no vacíos).
    pub signed_in: bool,
    /// `Config/DefaultEngine.ini`, con la clave, está versionado en git.
    pub key_in_git: bool,
}

/// `ConvAI.uplugin` en `Plugins/*/` del proyecto o en `Engine/Plugins/Marketplace/*/` del motor.
fn find_uplugin(project_dir: &Path, engine: Option<&EngineInstall>) -> Option<PathBuf> {
    let mut folders = vec![project_dir.join("Plugins")];
    if let Some(engine) = engine {
        folders.push(Path::new(&engine.root).join("Engine").join("Plugins").join("Marketplace"));
    }
    folders
        .iter()
        .filter_map(|f| std::fs::read_dir(f).ok())
        .flat_map(|entries| entries.flatten().map(|e| e.path().join(format!("{PLUGIN}.uplugin"))).collect::<Vec<_>>())
        .find(|p| p.is_file())
}

/// ¿Hay sesión iniciada? Solo se mira si los campos están vacíos; el valor no se guarda ni se devuelve. @spec REQ-55
pub fn signed_in(default_engine_ini: &str) -> bool {
    ["API_Key", "AuthToken"].iter().any(|key| ini::get(default_engine_ini, SETTINGS, key).is_some_and(|value| !value.trim().is_empty()))
}

/// ¿Está `Config/DefaultEngine.ini` versionado en un repositorio git? Sin git o fuera de un repositorio, no.
fn tracked_in_git(project_dir: &Path) -> bool {
    let inside = project_dir.ancestors().any(|dir| dir.join(".git").exists());
    let mut command = Command::new("git");
    command.arg("-C").arg(project_dir).args(["ls-files", "--error-unmatch", "Config/DefaultEngine.ini"]);
    // En Windows, sin esto cada comprobación abre y cierra una ventana de consola (como en `ai/detect.rs`).
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    inside && command.output().is_ok_and(|out| out.status.success())
}

/// Estado de Convai en el proyecto, sin el editor. @spec REQ-55
pub fn status(project_dir: &Path, project: &UProject, engine: Option<&EngineInstall>, engine_ok: bool) -> ConvaiStatus {
    let entry = project.plugins.iter().find(|p| p.name.eq_ignore_ascii_case(PLUGIN));
    let uplugin = find_uplugin(project_dir, engine);
    let ini_text = std::fs::read_to_string(project_dir.join("Config").join("DefaultEngine.ini")).unwrap_or_default();
    let signed = signed_in(&ini_text);
    ConvaiStatus {
        cited: entry.is_some(),
        enabled: entry.is_some_and(|p| p.enabled),
        version: uplugin.as_deref().and_then(super::prepare::plugin_version),
        engine_ok,
        signed_in: signed,
        key_in_git: signed && tracked_in_git(project_dir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unreal::discovery::PluginEntry;

    const SECRET: &str = "cvai_SECRETO_1234567890";

    fn project(plugins: Vec<PluginEntry>) -> UProject {
        UProject { path: String::new(), name: "Demo".into(), engine_association: "5.8".into(), modules: vec![], plugins, has_cpp: false }
    }

    /// Plugin en el motor (Fab), sesión iniciada y la clave sin salir de la comprobación. @spec REQ-55
    #[test]
    fn detecta_el_plugin_del_motor_y_la_sesion_sin_exponer_la_clave() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("Demo");
        let engine_root = dir.path().join("UE_5.8");
        let fab = engine_root.join("Engine/Plugins/Marketplace/AIforNPC123");
        std::fs::create_dir_all(&fab).unwrap();
        std::fs::write(fab.join("ConvAI.uplugin"), r#"{ "VersionName": "4.0.0-beta.27", "EngineVersion": "5.8.0" }"#).unwrap();
        std::fs::create_dir_all(proj.join("Config")).unwrap();
        std::fs::write(proj.join("Config/DefaultEngine.ini"), format!("[/Script/Engine.Engine]\nX=1\n\n[/Script/Convai.ConvaiSettings]\nAPI_Key={SECRET}\nAuthToken=\nCachedUsername=\n")).unwrap();
        let engine = EngineInstall { version: "5.8.4".into(), major: 5, minor: 8, patch: 4, root: engine_root.to_string_lossy().to_string(), editor: String::new(), editor_exists: true };
        let found = status(&proj, &project(vec![PluginEntry { name: "ConvAI".into(), enabled: true }]), Some(&engine), true);
        assert_eq!(found, ConvaiStatus { cited: true, enabled: true, version: Some("4.0.0-beta.27".into()), engine_ok: true, signed_in: true, key_in_git: false });
        assert!(!serde_json::to_string(&found).unwrap().contains("SECRETO"), "la clave no sale del diagnóstico");
        assert!(!signed_in("[/Script/Convai.ConvaiSettings]\nAPI_Key=\nAuthToken=  \n"));
        let missing = status(&dir.path().join("Otro"), &project(vec![]), None, true);
        assert_eq!((missing.cited, missing.version, missing.signed_in), (false, None, false));
    }
}
