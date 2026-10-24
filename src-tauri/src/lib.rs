// =============================================================================
// DBV Unreal AI — Punto de entrada de la aplicación Tauri
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
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

    builder
        .invoke_handler(tauri::generate_handler![
            #[cfg(desktop)]
            app_mcp_command,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
