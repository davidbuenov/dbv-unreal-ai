// =============================================================================
// DBV Unreal AI — Punto de entrada de la aplicación Tauri
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

// IA integrada y OPCIONAL (ver docs/IA.md). Solo escritorio: en móvil no hay `keyring`, ni procesos
// hijos para los agentes ACP. Sin ninguna IA conectada no se carga nada (cero huella).
#[cfg(desktop)]
pub mod ai;

use std::path::{Path, PathBuf};

use tauri::Manager;

// Ejemplo mínimo pero real de comando Tauri: Guard Clause + flujo de salida único
// (ver dbv-specs-ops/docs/MASTER_PROMPT.md, Estándares de Codificación §1). Devuelve datos, no una
// frase ya construida — la presentación (con i18n) vive en el frontend, no aquí.
#[tauri::command]
fn get_greeting_name(name: &str) -> String {
    name.trim().to_string()
}

/// Texto de la nota de ejemplo que se crea la primera vez (en español, como la demo).
const DEMO_NOTE: &str = "# Notas de la demo\n\nEste es el documento de ejemplo de DBV Unreal AI. \
Conecta una IA (botón ✨) y pídele, por ejemplo, «corrige la ortografía» o «resume este texto».\n\n\
Hay un erorr de ortografia a proposito para probarlo.\n";

/// Crea (si no existe) la carpeta de trabajo de la demo y su nota de ejemplo, y la devuelve.
/// Sustituye esto por la carpeta o el documento real de tu aplicación.
fn ensure_demo_workspace(dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("no se pudo crear la carpeta de la demo: {e}"))?;
    let note = dir.join("notas.md");
    if !note.exists() {
        std::fs::write(&note, DEMO_NOTE).map_err(|e| format!("no se pudo crear notas.md: {e}"))?;
    }
    Ok(dir.to_path_buf())
}

/// Carpeta de trabajo de la demo (en los datos de la aplicación, nunca junto al ejecutable). El
/// frontend la usa como «carpeta del documento» de la IA: ahí es donde la IA puede leer y proponer.
#[tauri::command]
fn demo_workspace(app: tauri::AppHandle) -> Result<String, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    ensure_demo_workspace(&base.join("demo-workspace")).map(|dir| dir.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(desktop)]
    {
        builder = builder.manage(ai::AiState::default()).manage(ai::acp::AcpState::default());
    }

    builder
        .invoke_handler(tauri::generate_handler![
            get_greeting_name,
            demo_workspace,
            #[cfg(desktop)]
            ai::commands::ai_connections,
            #[cfg(desktop)]
            ai::commands::ai_save_connection,
            #[cfg(desktop)]
            ai::commands::ai_delete_connection,
            #[cfg(desktop)]
            ai::commands::ai_set_preferences,
            #[cfg(desktop)]
            ai::commands::ai_providers,
            #[cfg(desktop)]
            ai::commands::ai_detect,
            #[cfg(desktop)]
            ai::commands::ai_list_models,
            #[cfg(desktop)]
            ai::commands::ai_model_info,
            #[cfg(desktop)]
            ai::commands::ai_chat,
            #[cfg(desktop)]
            ai::commands::ai_cancel,
            #[cfg(desktop)]
            ai::commands::ai_check_proposal,
            #[cfg(desktop)]
            ai::commands::ai_project_state_load,
            #[cfg(desktop)]
            ai::commands::ai_project_state_save,
            #[cfg(desktop)]
            ai::files::ai_list_files,
            #[cfg(desktop)]
            ai::files::ai_read_file,
            #[cfg(desktop)]
            ai::files::ai_search_files,
            #[cfg(desktop)]
            ai::files::ai_write_file,
            #[cfg(desktop)]
            ai::files::ai_delete_created,
            #[cfg(desktop)]
            ai::acp::acp_start,
            #[cfg(desktop)]
            ai::acp::acp_request,
            #[cfg(desktop)]
            ai::acp::acp_notify,
            #[cfg(desktop)]
            ai::acp::acp_respond,
            #[cfg(desktop)]
            ai::acp::acp_stop,
            #[cfg(desktop)]
            ai::acp::acp_snapshot,
            #[cfg(desktop)]
            ai::acp::acp_changes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(get_greeting_name("  David  "), "David");
    }

    #[test]
    fn empty_input_returns_empty_string() {
        assert_eq!(get_greeting_name("   "), "");
    }

    #[test]
    fn demo_workspace_creates_the_sample_note_once() {
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join("demo-workspace");
        ensure_demo_workspace(&dir).unwrap();
        assert!(std::fs::read_to_string(dir.join("notas.md")).unwrap().contains("Notas de la demo"));
        // Una segunda llamada no pisa lo que el usuario haya escrito.
        std::fs::write(dir.join("notas.md"), "mío").unwrap();
        ensure_demo_workspace(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("notas.md")).unwrap(), "mío");
    }
}
