// =============================================================================
// DBV Unreal AI — Punto de entrada de la aplicación Tauri
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

// Un `panic` cierra toda la app: fuera de los tests, nada de `unwrap`/`expect` (MASTER_PROMPT.md §2).
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
// El estándar del proyecto guarda el resultado en una variable y la devuelve en la última línea (un solo
// return, MASTER_PROMPT.md §1): es justo lo que este lint de estilo desaconseja.
#![allow(clippy::let_and_return)]

// IA integrada de tres niveles (local, nube, agentes ACP). Solo escritorio: en móvil no hay `keyring`
// ni procesos hijos. El frontend la carga de forma perezosa al abrir la Conversación (RNF-IA.1).
#[cfg(desktop)]
pub mod ai;

// Conexión con Unreal Engine: descubrimiento, cliente MCP del editor y preparación de proyectos.
#[cfg(desktop)]
pub mod unreal;

// Generación 3D (Tripo) y biblioteca local de assets generados.
#[cfg(desktop)]
pub mod generation;
#[cfg(desktop)]
pub mod library;

// Servidor MCP propio por stdio (`--mcp`) y las carpetas de la app que usa sin Tauri.
#[cfg(desktop)]
pub mod mcp_server;
#[cfg(desktop)]
pub mod paths;

/// Orden con la que un agente ACP lanza el servidor MCP propio de la app (REQ-11, REQ-20):
/// este mismo ejecutable con `--mcp` y el puerto del editor conectado.
#[cfg(desktop)]
#[tauri::command]
fn app_mcp_command(port: u16) -> Result<serde_json::Value, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    Ok(serde_json::json!({ "command": exe.to_string_lossy(), "args": ["--mcp", "--port", port.to_string()] }))
}

/// Diagnóstico para una incidencia (RNF-08): versión, sistema, instalaciones de Unreal y editores
/// abiertos. Nunca lleva claves, prompts ni contenido de proyectos.
#[cfg(desktop)]
pub fn diagnostics(engines: &[unreal::discovery::EngineInstall], editors: &[unreal::commands::EditorStatus]) -> serde_json::Value {
    serde_json::json!({
        "app": "DBV Unreal AI",
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "engines": engines.iter().map(|e| serde_json::json!({ "version": e.version, "root": e.root, "editorExists": e.editor_exists })).collect::<Vec<_>>(),
        "editors": editors.iter().map(|e| serde_json::json!({ "project": e.editor.project, "port": e.mcp.as_ref().map(|m| m.port), "autoStart": e.mcp.as_ref().and_then(|m| m.auto_start), "listening": e.listening })).collect::<Vec<_>>(),
    })
}

/// @spec RNF-08
#[cfg(desktop)]
#[tauri::command]
async fn app_diagnostics() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let editors: Vec<unreal::commands::EditorStatus> = unreal::discovery::running_editors().into_iter().map(unreal::commands::editor_status).collect();
        diagnostics(&unreal::discovery::find_engines(), &editors)
    })
    .await
    .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
// Arranque: si Tauri no puede abrir la ventana no hay nada que recuperar (excepción permitida por el estándar).
#[allow(clippy::expect_used)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init()).plugin(tauri_plugin_dialog::init());

    #[cfg(desktop)]
    {
        builder = builder
            .manage(ai::AiState::default())
            .manage(ai::acp::AcpState::default())
            .manage(unreal::commands::UnrealState::default())
            .manage(generation::commands::GenerationState::default());
    }

    #[cfg(desktop)]
    {
        // La sesión de generación empieza con la ventana: el gasto cuenta desde aquí (RNF-04).
        builder = builder.setup(|app| {
            generation::commands::begin_session(app.handle());
            Ok(())
        });
    }

    builder
        .invoke_handler(tauri::generate_handler![
            #[cfg(desktop)]
            app_mcp_command,
            #[cfg(desktop)]
            app_diagnostics,
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
            #[cfg(desktop)]
            unreal::commands::unreal_engines,
            #[cfg(desktop)]
            unreal::commands::unreal_read_project,
            #[cfg(desktop)]
            unreal::commands::unreal_editors,
            #[cfg(desktop)]
            unreal::commands::unreal_recent_projects,
            #[cfg(desktop)]
            unreal::commands::unreal_remember_project,
            #[cfg(desktop)]
            unreal::commands::unreal_open_project,
            #[cfg(desktop)]
            unreal::commands::unreal_connect,
            #[cfg(desktop)]
            unreal::commands::unreal_list_toolsets,
            #[cfg(desktop)]
            unreal::commands::unreal_describe_toolset,
            #[cfg(desktop)]
            unreal::commands::unreal_call_tool,
            #[cfg(desktop)]
            unreal::commands::unreal_readiness,
            #[cfg(desktop)]
            unreal::commands::unreal_prepare_plan,
            #[cfg(desktop)]
            unreal::commands::unreal_prepare_apply,
            #[cfg(desktop)]
            unreal::commands::unreal_prepare_revert,
            #[cfg(desktop)]
            generation::commands::generation_status,
            #[cfg(desktop)]
            generation::commands::generation_set_key,
            #[cfg(desktop)]
            generation::commands::generation_delete_key,
            #[cfg(desktop)]
            generation::commands::generation_set_budget,
            #[cfg(desktop)]
            generation::commands::generation_tool_specs,
            #[cfg(desktop)]
            generation::commands::generation_prepare,
            #[cfg(desktop)]
            generation::commands::generation_launch,
            #[cfg(desktop)]
            generation::commands::generation_operations,
            #[cfg(desktop)]
            generation::commands::generation_balance,
            #[cfg(desktop)]
            generation::commands::generation_suggest_name,
            #[cfg(desktop)]
            generation::commands::generation_presets,
            #[cfg(desktop)]
            generation::commands::generation_cancel,
            #[cfg(desktop)]
            generation::commands::generation_jobs,
            #[cfg(desktop)]
            generation::commands::library_list,
            #[cfg(desktop)]
            generation::commands::library_remove,
            #[cfg(desktop)]
            generation::commands::library_file_path,
            #[cfg(desktop)]
            generation::commands::library_record_sent,
            #[cfg(desktop)]
            generation::commands::library_location,
            #[cfg(desktop)]
            generation::commands::library_set_location,
            #[cfg(desktop)]
            generation::commands::library_import,
            #[cfg(desktop)]
            generation::commands::library_rename,
            #[cfg(desktop)]
            generation::commands::library_entry_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;
    use unreal::commands::EditorStatus;
    use unreal::discovery::{EngineInstall, McpSettings, RunningEditor};

    /// @spec RNF-08
    #[test]
    fn el_diagnostico_no_lleva_secretos_y_describe_el_entorno() {
        let engines = vec![EngineInstall { version: "5.8.2".into(), major: 5, minor: 8, patch: 2, root: "D:/UE_5.8".into(), editor: "D:/UE_5.8/x.exe".into(), editor_exists: true }];
        let editors = vec![EditorStatus { editor: RunningEditor { pid: 1, executable: "x".into(), project: Some("D:/P/P.uproject".into()) }, project: None, mcp: Some(McpSettings { port: 8000, path: "/mcp".into(), auto_start: Some(true) }), listening: true }];
        let report = diagnostics(&engines, &editors);
        assert_eq!(report["engines"][0]["version"], "5.8.2");
        assert_eq!(report["editors"][0]["port"], 8000);
        assert_eq!(report["os"], std::env::consts::OS);
        let text = report.to_string().to_lowercase();
        for secret in ["key", "token", "prompt", "password"] {
            assert!(!text.contains(secret), "{secret} en el diagnóstico");
        }
    }
}
