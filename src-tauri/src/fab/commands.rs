// =============================================================================
// DBV Unreal AI — Comandos de Tauri de Fab (ventana)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-105, REQ-106, REQ-117
//
// Las herramientas `fab_*` para el asistente integrado y la vista Fab, con el editor conectado de la ventana.

use serde_json::Value;

use super::tools::{self, FabEditor, FabToolResult, FAB_TOOLSET};
use crate::unreal::commands::{with_session, Endpoint, UnrealState};
use crate::unreal::mcp_client::DEFAULT_TIMEOUT;
use crate::unreal::UnrealError;

struct AppFab<'a> {
    state: &'a UnrealState,
    url: String,
}

impl FabEditor for AppFab<'_> {
    fn call_cpp(&mut self, tool: &str, args: Value) -> Result<Value, String> {
        let out = with_session(self.state, &self.url, |s| s.call_toolset_tool(FAB_TOOLSET, tool, args.clone(), DEFAULT_TIMEOUT))
            .map_err(|e| e.to_string())
            .map(|r| r.value.unwrap_or(Value::String(r.text)));
        out
    }
}

/// Definiciones de las herramientas de Fab (las mismas que el `--mcp`).
#[tauri::command]
pub fn fab_tool_specs() -> Vec<Value> {
    tools::specs()
}

/// Ejecuta una herramienta `fab_*`. `endpoint` es el editor conectado (falta si no lo hay: la búsqueda funciona igual);
/// `engine_version`, la del proyecto, para filtrar sin editor.
#[tauri::command]
pub async fn fab_run_tool(
    app: tauri::AppHandle,
    endpoint: Option<Endpoint>,
    engine_version: Option<String>,
    name: String,
    args: Value,
) -> Result<FabToolResult, UnrealError> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<UnrealState>();
        let mut editor = endpoint.map(|e| AppFab { state: &state, url: e.url() });
        let editor_ref = editor.as_mut().map(|e| e as &mut dyn FabEditor);
        tools::run_tool(&name, &args, editor_ref, engine_version.as_deref().unwrap_or_default())
    })
    .await
    .map_err(|error| UnrealError::Io(error.to_string()))?
    .map_err(UnrealError::Protocol)
}
