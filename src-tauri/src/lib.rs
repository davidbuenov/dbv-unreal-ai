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

// Gaussian Splats: lectura y malla de vista previa para Unreal (REQ-41, spike P-10).
#[cfg(desktop)]
pub mod splats;

// Memoria y tareas del proyecto en `dbv-unreal-ai/`, junto al `.uproject` (REQ-34..REQ-38).
#[cfg(desktop)]
pub mod project_memory;

// Servidor MCP propio por stdio (`--mcp`) y las carpetas de la app que usa sin Tauri.
#[cfg(desktop)]
pub mod mcp_server;
#[cfg(desktop)]
pub mod paths;
// Personajes de Convai registrados en Ajustes (REQ-58).
#[cfg(desktop)]
pub mod convai_characters;
// Zonas del nivel para los personajes (Convai), una sola definición para las dos superficies (REQ-57).
#[cfg(desktop)]
pub mod zones;
// Log local rotativo de la ventana y del `--mcp`, sin claves (RNF-08).
#[cfg(desktop)]
pub mod logging;
// Cuánto devuelven las herramientas a la IA, compartido entre la ventana y el `--mcp` (REQ-51).
#[cfg(desktop)]
pub mod tool_usage;

/// Lo que devolvieron las herramientas al proyecto desde `since` (ms): la ventana lo pide al acabar un turno para
/// enseñar el contador, también con las llamadas que hizo el `--mcp` de un agente. @spec REQ-51
#[cfg(desktop)]
#[tauri::command]
fn tool_usage_since(app: tauri::AppHandle, project: String, since: u64) -> Result<tool_usage::UsageSummary, String> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().map_err(|error| error.to_string())?;
    Ok(tool_usage::since(&dir, &project, since))
}

/// Resuelve la ruta al ejecutable para el comando MCP.
///
/// En Windows, si la aplicación está instalada desde Microsoft Store / MSIX dentro de `\WindowsApps\`,
/// la ejecución directa por un proceso externo Win32 (como el runtime de Go de un agente ACP) falla
/// con `ERROR_ACCESS_DENIED` (error 5) debido a las restricciones de AppModel y permisos de WindowsApps.
/// En ese caso, si existe el alias de ejecución en `%LOCALAPPDATA%\Microsoft\WindowsApps\dbv-unreal-ai.exe`,
/// se devuelve la ruta al alias (usando `symlink_metadata` para detectar el reparse point AppExecLink).
#[cfg(desktop)]
fn resolve_mcp_executable(current_exe: std::path::PathBuf) -> std::path::PathBuf {
    #[cfg(windows)]
    {
        let exe_str = current_exe.to_string_lossy();
        if exe_str.contains(r"\WindowsApps\") || exe_str.contains(r"/WindowsApps/") {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                let alias_path = std::path::Path::new(&local_app_data)
                    .join("Microsoft")
                    .join("WindowsApps")
                    .join("dbv-unreal-ai.exe");
                if std::fs::symlink_metadata(&alias_path).is_ok() {
                    return alias_path;
                }
            }
        }
    }
    current_exe
}

/// Orden con la que un agente ACP lanza el servidor MCP propio de la app (REQ-11, REQ-20):
/// este mismo ejecutable con `--mcp`, el puerto del editor conectado y, si lo hay, el `.uproject` (sin él el agente
/// no ve la memoria del proyecto, REQ-38).
#[cfg(desktop)]
#[tauri::command]
fn app_mcp_command(port: u16, project: Option<String>) -> Result<serde_json::Value, String> {
    let raw_exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let exe = resolve_mcp_executable(raw_exe);
    let mut args = vec!["--mcp".to_string(), "--port".to_string(), port.to_string()];
    if let Some(project) = project.filter(|p| !p.trim().is_empty()) {
        args.extend(["--project".to_string(), project]);
    }
    Ok(serde_json::json!({ "command": exe.to_string_lossy(), "args": args }))
}

/// Diagnóstico para una incidencia (RNF-08): versión, sistema, instalaciones de Unreal y editores
/// abiertos. Nunca lleva claves, prompts ni contenido de proyectos.
#[cfg(desktop)]
pub fn diagnostics(
    engines: &[unreal::discovery::EngineInstall],
    editors: &[unreal::commands::EditorStatus],
) -> serde_json::Value {
    serde_json::json!({
        "app": "DBV Unreal AI",
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "engines": engines.iter().map(|e| serde_json::json!({ "version": e.version, "root": e.root, "editorExists": e.editor_exists })).collect::<Vec<_>>(),
        "editors": editors.iter().map(|e| serde_json::json!({ "project": e.editor.project, "port": e.mcp.as_ref().map(|m| m.port), "autoStart": e.mcp.as_ref().and_then(|m| m.auto_start), "listening": e.listening })).collect::<Vec<_>>(),
    })
}

/// Rutas reales de la app y si existen, y el final del log (RNF-08, REQ-53): lo que faltó para ver que la biblioteca
/// de la Store estaba en otra carpeta.
#[cfg(desktop)]
pub fn paths_report(data_dir: &std::path::Path, library_dir: &std::path::Path) -> serde_json::Value {
    serde_json::json!({
        "packaged": logging::is_packaged(),
        "dataDir": data_dir,
        "dataDirExists": data_dir.is_dir(),
        "libraryDir": library_dir,
        "libraryDirExists": library_dir.is_dir(),
        "log": logging::tail(data_dir, 50),
    })
}

/// @spec RNF-08
#[cfg(desktop)]
#[tauri::command]
async fn app_diagnostics(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    use tauri::Manager;
    let data = app.path().app_data_dir().map_err(|error| error.to_string())?;
    let config = app.path().app_config_dir().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let editors: Vec<unreal::commands::EditorStatus> = unreal::discovery::running_editors()
            .into_iter()
            .map(unreal::commands::editor_status)
            .collect();
        let mut report = diagnostics(&unreal::discovery::find_engines(), &editors);
        let library = generation::commands::GenerationSettings::load(&config).library_root(&data);
        if let (Some(map), serde_json::Value::Object(paths)) = (report.as_object_mut(), paths_report(&data, &library)) {
            map.extend(paths);
        }
        report
    })
    .await
    .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
// Arranque: si Tauri no puede abrir la ventana no hay nada que recuperar (excepción permitida por el estándar).
#[allow(clippy::expect_used)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

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
            use tauri::Manager;
            if let Ok(data) = app.path().app_data_dir() {
                logging::init(&data, "window");
                logging::info(&format!("start v{} packaged={} data={}", env!("CARGO_PKG_VERSION"), logging::is_packaged(), data.display()));
            }
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
            ai::commands::ai_list_agent_models,
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
            ai::attachments::ai_read_attachment,
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
            unreal::commands::unreal_code_path_problem,
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
            generation::commands::generation_worldlabs_balance,
            #[cfg(desktop)]
            generation::commands::generation_suggest_name,
            #[cfg(desktop)]
            generation::commands::generation_presets,
            #[cfg(desktop)]
            generation::commands::generation_allow_reference,
            #[cfg(desktop)]
            generation::commands::library_save_thumbnail,
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
            generation::commands::library_world_place_code,
            #[cfg(desktop)]
            generation::commands::library_location,
            #[cfg(desktop)]
            generation::commands::library_set_location,
            #[cfg(desktop)]
            generation::commands::library_import,
            #[cfg(desktop)]
            generation::commands::library_import_url,
            #[cfg(desktop)]
            generation::commands::library_rename,
            #[cfg(desktop)]
            generation::commands::library_entry_dir,
            #[cfg(desktop)]
            project_memory::commands::memory_status,
            #[cfg(desktop)]
            project_memory::commands::memory_activate,
            #[cfg(desktop)]
            project_memory::commands::memory_apply,
            #[cfg(desktop)]
            project_memory::commands::memory_check_entries,
            #[cfg(desktop)]
            tool_usage_since,
            #[cfg(desktop)]
            convai_characters::convai_characters_list,
            #[cfg(desktop)]
            convai_characters::convai_characters_save,
            #[cfg(desktop)]
            convai_characters::convai_characters_spec,
            #[cfg(desktop)]
            convai_characters::convai_characters_text,
            #[cfg(desktop)]
            zones::zones_tool_specs,
            #[cfg(desktop)]
            zones::zones_preview,
            #[cfg(desktop)]
            zones::zones_apply,
            #[cfg(desktop)]
            zones::zones_list,
            #[cfg(desktop)]
            project_memory::commands::memory_tool_specs,
            #[cfg(desktop)]
            project_memory::commands::memory_plan_preview,
            #[cfg(desktop)]
            project_memory::commands::memory_task_update,
            #[cfg(desktop)]
            unreal::commands::unreal_capture_viewport,
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
        let engines = vec![EngineInstall {
            version: "5.8.2".into(),
            major: 5,
            minor: 8,
            patch: 2,
            root: "D:/UE_5.8".into(),
            editor: "D:/UE_5.8/x.exe".into(),
            editor_exists: true,
        }];
        let editors = vec![EditorStatus {
            editor: RunningEditor {
                pid: 1,
                executable: "x".into(),
                project: Some("D:/P/P.uproject".into()),
            },
            project: None,
            mcp: Some(McpSettings {
                port: 8000,
                path: "/mcp".into(),
                auto_start: Some(true),
            }),
            listening: true,
        }];
        let report = diagnostics(&engines, &editors);
        assert_eq!(report["engines"][0]["version"], "5.8.2");
        assert_eq!(report["editors"][0]["port"], 8000);
        assert_eq!(report["os"], std::env::consts::OS);
        let text = report.to_string().to_lowercase();
        for secret in ["key", "token", "prompt", "password"] {
            assert!(!text.contains(secret), "{secret} en el diagnóstico");
        }
    }

    /// @spec REQ-20
    #[test]
    fn resolve_mcp_executable_preserva_rutas_normales_y_fallback_seguro() {
        let normal = std::path::PathBuf::from(r"C:\Program Files\DBV Unreal AI\dbv-unreal-ai.exe");
        assert_eq!(resolve_mcp_executable(normal.clone()), normal);

        let store_ficticia = std::path::PathBuf::from(
            r"C:\Program Files\WindowsApps\davidbuenov.DBVUnrealAI_ficticio\dbv-unreal-ai.exe",
        );
        let res = resolve_mcp_executable(store_ficticia.clone());
        assert!(!res.as_os_str().is_empty());
    }

    /// @spec REQ-20
    #[test]
    fn app_mcp_command_genera_argumentos_correctos() {
        let res = app_mcp_command(8123, Some("D:/Project/Test.uproject".into()))
            .expect("app_mcp_command con proyecto");
        assert_eq!(
            res["args"],
            serde_json::json!([
                "--mcp",
                "--port",
                "8123",
                "--project",
                "D:/Project/Test.uproject"
            ])
        );

        let sin_proyecto = app_mcp_command(8000, None).expect("app_mcp_command sin proyecto");
        assert_eq!(
            sin_proyecto["args"],
            serde_json::json!(["--mcp", "--port", "8000"])
        );
    }
}
