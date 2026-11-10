// =============================================================================
// DBV Unreal AI — Caja negra: comandos Tauri para el asistente integrado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-92
//
// La segunda superficie de las herramientas `session_*` (D69): la misma tabla y el mismo despacho que el MCP propio;
// aquí el `Editor` habla con el editor conectado desde la app (sesión MCP compartida) en vez de abrir la suya.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::control::{start_pie_args, Editor};
use crate::unreal::commands::{run_fixed_python, with_session, Endpoint, UnrealState};
use crate::unreal::mcp_client::DEFAULT_TIMEOUT;
use crate::unreal::UnrealError;

/// Carpeta del proyecto a partir de lo que guarda la app (la ruta del `.uproject` o ya la carpeta).
pub fn project_dir(project: &str) -> PathBuf {
    let path = Path::new(project);
    let is_uproject = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("uproject"));
    let out = if is_uproject { path.parent().map_or_else(|| path.to_path_buf(), Path::to_path_buf) } else { path.to_path_buf() };
    out
}

/// El editor conectado desde la app. Sin editor, solo fallan las herramientas que lo necesitan.
struct AppEditor<'a> {
    state: &'a UnrealState,
    url: Option<String>,
}

impl AppEditor<'_> {
    fn url(&self) -> Result<&str, String> {
        let out = self.url.as_deref().ok_or_else(|| "no editor is connected: connect it in the Project view".to_string());
        out
    }

    fn play(&self, tool: &str, args: Value) -> Result<(), String> {
        let url = self.url()?;
        let out = with_session(self.state, url, |s| {
            s.call_toolset_tool("EditorToolset.EditorAppToolset", tool, args.clone(), DEFAULT_TIMEOUT)
        })
        .map(|_| ())
        .map_err(|e| e.to_string());
        out
    }
}

impl Editor for AppEditor<'_> {
    fn run_python(&mut self, code: &str) -> Result<String, String> {
        let url = self.url()?.to_string();
        let out = run_fixed_python(self.state, &url, code).map(|v| v.to_string()).map_err(|e| e.to_string());
        out
    }

    fn start_pie(&mut self) -> Result<(), String> {
        self.play("StartPIE", start_pie_args())
    }

    fn stop_pie(&mut self) -> Result<(), String> {
        self.play("StopPIE", serde_json::json!({}))
    }
}

/// Definiciones de las herramientas, las mismas que publica el MCP propio (D69). @spec REQ-92
#[tauri::command]
pub fn sessions_tool_specs() -> Vec<Value> {
    super::specs()
}

/// ¿Hay una grabación en marcha en el proyecto? Solo lee ficheros: el panel de la app lo pregunta cada par de segundos
/// y funciona también cuando la grabación la lanzó un agente por el `--mcp`. @spec REQ-94
#[tauri::command]
pub fn sessions_active(project: Option<String>) -> Option<super::ActiveSession> {
    let root = super::tools::sessions_root(&project_dir(&project?));
    let out = super::active_session(&root);
    out
}

/// Una herramienta `session_*` desde el asistente integrado. `project` es el proyecto abierto en la app y `endpoint`
/// el editor conectado (puede faltar: las que solo leen ficheros no lo necesitan). @spec REQ-92
#[tauri::command]
pub async fn sessions_run_tool(
    app: tauri::AppHandle,
    project: Option<String>,
    endpoint: Option<Endpoint>,
    name: String,
    args: Value,
) -> Result<String, UnrealError> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let project = project.ok_or_else(|| "no project is open in the app".to_string())?;
        let root = super::tools::sessions_root(&project_dir(&project));
        let state = app.state::<UnrealState>();
        let mut editor = AppEditor { state: &state, url: endpoint.map(|e| e.url()) };
        super::run_tool(&name, &args, &root, &mut editor)
    })
    .await
    .map_err(|error| UnrealError::Io(error.to_string()))?
    .map_err(UnrealError::Format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_project_dir_comes_from_a_uproject_path_or_a_folder() {
        assert_eq!(project_dir("D:/P/Demo/Demo.uproject"), PathBuf::from("D:/P/Demo"));
        assert_eq!(project_dir("D:/P/Demo"), PathBuf::from("D:/P/Demo"));
    }

    #[test]
    fn the_specs_command_returns_the_ten_tools() {
        assert_eq!(sessions_tool_specs().len(), 10);
    }
}
