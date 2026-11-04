// =============================================================================
// DBV Unreal AI — Comandos Tauri de la conexión con Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Frontera con el frontend. Reglas:
//   · Toda llamada que pueda esperar al editor (red) corre en `spawn_blocking`:
//     el hilo de la interfaz nunca espera a Unreal (RNF-03).
//   · Solo se habla con `127.0.0.1` (RNF-06): el frontend da puerto y ruta, no un host.
//   · Una sesión MCP caducada (el editor se reinició) se reabre una vez sola.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use super::discovery::{self, EngineInstall, McpSettings, RunningEditor, UProject};
use super::mcp_client::{self, McpSession, ToolResult, ToolsetInfo, ToolsetSummary};
use super::prepare::{self, PreparePlan, Readiness};
use super::{io_error, UnrealError};

/// Sesiones MCP abiertas, por URL.
#[derive(Default)]
pub struct UnrealState {
    pub sessions: Mutex<HashMap<String, McpSession>>,
}

/// Un editor abierto con lo que sabemos de su proyecto.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorStatus {
    pub editor: RunningEditor,
    pub project: Option<UProject>,
    pub mcp: Option<McpSettings>,
    /// Hay algo escuchando en el puerto MCP del proyecto.
    pub listening: bool,
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, UnrealError> + Send + 'static) -> Result<T, UnrealError> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|error| UnrealError::Io(format!("tarea interrumpida: {error}")))?
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, UnrealError> {
    app.path().app_config_dir().map_err(|error| UnrealError::Io(error.to_string()))
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, UnrealError> {
    app.path().app_data_dir().map_err(|error| UnrealError::Io(error.to_string()))
}

/// Instalaciones del motor. @spec REQ-01
#[tauri::command]
pub async fn unreal_engines() -> Result<Vec<EngineInstall>, UnrealError> {
    blocking(|| Ok(discovery::find_engines())).await
}

/// Por qué la IA no puede proponer cambiar ese fichero del proyecto, o `None` si puede. @spec REQ-22
#[tauri::command]
pub fn unreal_code_path_problem(relative: String) -> Option<String> {
    super::code_policy::code_path_problem(&relative)
}

/// Lee un `.uproject` elegido por el usuario. @spec REQ-03
#[tauri::command]
pub fn unreal_read_project(path: String) -> Result<UProject, UnrealError> {
    discovery::read_uproject(Path::new(&path))
}

/// Estado de un editor a partir de su proceso. @spec REQ-01
pub fn editor_status(editor: RunningEditor) -> EditorStatus {
    let project = editor.project.as_deref().and_then(|p| discovery::read_uproject(Path::new(p)).ok());
    let mcp = project.as_ref().and_then(|p| Path::new(&p.path).parent().map(discovery::read_mcp_settings));
    let listening = mcp.as_ref().is_some_and(|m| discovery::port_listening(m.port));
    EditorStatus { editor, project, mcp, listening }
}

/// Editores abiertos, con su proyecto y si su servidor MCP escucha. @spec REQ-01
#[tauri::command]
pub async fn unreal_editors() -> Result<Vec<EditorStatus>, UnrealError> {
    blocking(|| Ok(discovery::running_editors().into_iter().map(editor_status).collect())).await
}

const RECENTS_FILE: &str = "recent-projects.json";
const MAX_RECENTS: usize = 12;

/// Proyectos recientes de la app (rutas de `.uproject`, el más reciente primero). @spec REQ-03
pub fn load_recents(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join(RECENTS_FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Añade `path` al principio de los recientes (sin duplicados, como mucho `MAX_RECENTS`). @spec REQ-03
pub fn remember_recent(dir: &Path, path: &str) -> Result<Vec<String>, UnrealError> {
    let mut recents: Vec<String> = load_recents(dir).into_iter().filter(|p| !p.eq_ignore_ascii_case(path)).collect();
    recents.insert(0, path.to_string());
    recents.truncate(MAX_RECENTS);
    std::fs::create_dir_all(dir).map_err(|error| io_error(dir, &error))?;
    let file = dir.join(RECENTS_FILE);
    let text = serde_json::to_string_pretty(&recents).map_err(|error| UnrealError::Format(error.to_string()))?;
    std::fs::write(&file, text).map_err(|error| io_error(&file, &error))?;
    Ok(recents)
}

#[tauri::command]
pub fn unreal_recent_projects(app: AppHandle) -> Result<Vec<String>, UnrealError> {
    Ok(load_recents(&config_dir(&app)?))
}

#[tauri::command]
pub fn unreal_remember_project(app: AppHandle, path: String) -> Result<Vec<String>, UnrealError> {
    remember_recent(&config_dir(&app)?, &path)
}

/// Abre el proyecto en el editor de su versión. @spec REQ-03
#[tauri::command]
pub async fn unreal_open_project(path: String) -> Result<u32, UnrealError> {
    blocking(move || {
        let project = discovery::read_uproject(Path::new(&path))?;
        let engines = discovery::find_engines();
        let engine = discovery::engine_for(&engines, &project.engine_association)
            .ok_or_else(|| UnrealError::NotFound(format!("no hay una instalación de Unreal {} en este equipo", project.engine_association)))?;
        if !engine.editor_exists {
            return Err(UnrealError::NotFound(format!("la instalación {} no tiene el editor: {}", engine.version, engine.editor)));
        }
        spawn_outside_job(&engine.editor, &project.path)
    })
    .await
}

/// Línea de órdenes del editor para WMI (las rutas de Windows no pueden llevar comillas).
pub fn editor_command_line(editor: &str, project: &str) -> String {
    format!("\"{editor}\" \"{project}\"")
}

/// Lanza el editor fuera del Job Object de quien abrió la app: con `cargo run` (y `tauri dev`) la app vive en un job
/// que mata a todos sus hijos al reiniciarse, y el editor del usuario se cerraba sin aviso. Primero pide salir del job;
/// si el job no lo permite, lo crea WMI, que no está en él. Fuera de un job, el primer intento basta. @spec REQ-03
#[cfg(windows)]
fn spawn_outside_job(editor: &str, project: &str) -> Result<u32, UnrealError> {
    use std::os::windows::process::CommandExt;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const WMI_CREATE: &str = "$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = $env:DBV_EDITOR_COMMAND }; if ($r.ReturnValue -ne 0) { exit [int]$r.ReturnValue }; $r.ProcessId";
    let launched = match std::process::Command::new(editor).arg(project).creation_flags(CREATE_BREAKAWAY_FROM_JOB).spawn() {
        Ok(child) => Ok(child.id()),
        Err(_) => std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", WMI_CREATE])
            .env("DBV_EDITOR_COMMAND", editor_command_line(editor, project))
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|error| io_error(Path::new(editor), &error))
            .and_then(|out| {
                String::from_utf8_lossy(&out.stdout)
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|_| out.status.success())
                    .ok_or_else(|| UnrealError::Io(format!("no se pudo abrir el editor {editor} (WMI devolvió {:?})", out.status.code())))
            }),
    };
    launched
}

#[cfg(not(windows))]
fn spawn_outside_job(editor: &str, project: &str) -> Result<u32, UnrealError> {
    let child = std::process::Command::new(editor).arg(project).spawn().map_err(|error| io_error(Path::new(editor), &error))?;
    Ok(child.id())
}

// ─── MCP ─────────────────────────────────────────────────────────────

/// Sesión de `url`, abriéndola si no existe; `fresh` fuerza abrir otra.
fn session_for(state: &UnrealState, url: &str, fresh: bool) -> Result<McpSession, UnrealError> {
    let cached = if fresh { None } else { state.sessions.lock().ok().and_then(|s| s.get(url).cloned()) };
    let session = match cached {
        Some(session) => session,
        None => McpSession::connect(url)?,
    };
    if let Ok(mut sessions) = state.sessions.lock() {
        sessions.insert(url.to_string(), session.clone());
    }
    Ok(session)
}

/// Ejecuta código Python FIJO de la app (comprobaciones de tareas, zonas) con el `run_python` del toolset DBV y
/// devuelve su informe. No pasa por `validate_python`: no lo escribió la IA. @spec REQ-37, REQ-57
pub fn run_fixed_python(state: &UnrealState, url: &str, code: &str) -> Result<serde_json::Value, UnrealError> {
    let ran = with_session(state, url, |session| {
        let toolsets = session.list_toolsets()?;
        let dbv = toolsets.iter().find(|t| t.name.contains("DBVUnrealAI")).map(|t| t.name.clone()).ok_or_else(|| UnrealError::NotFound("the DBV Unreal AI toolset is not installed: prepare the project".into()))?;
        session.call_toolset_tool(&dbv, "run_python", serde_json::json!({ "code": code }), std::time::Duration::from_secs(120))
    })?;
    ran.value.ok_or_else(|| UnrealError::Protocol(format!("unexpected answer from the editor: {}", ran.text)))
}

/// Ejecuta `work` con la sesión de `url`; si la sesión caducó, la reabre y repite una vez.
pub fn with_session<T>(state: &UnrealState, url: &str, work: impl Fn(&McpSession) -> Result<T, UnrealError>) -> Result<T, UnrealError> {
    let first = work(&session_for(state, url, false)?);
    match first {
        Err(UnrealError::Expired(_)) => work(&session_for(state, url, true)?),
        other => other,
    }
}

/// Dónde está el servidor MCP de un editor. Solo puerto y ruta: el host es siempre `127.0.0.1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub port: u16,
    #[serde(default)]
    pub path: Option<String>,
}

impl Endpoint {
    pub fn url(&self) -> String {
        mcp_client::endpoint_url(self.port, self.path.as_deref().unwrap_or(discovery::DEFAULT_MCP_PATH))
    }
}

/// Respuesta de conexión con los toolsets y el endpoint activo detectado. @spec REQ-75
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResponse {
    pub toolsets: Vec<ToolsetSummary>,
    pub endpoint: Endpoint,
}

/// Ajustes persistentes del proyecto (autoría, etc.). @spec REQ-74
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    #[serde(default)]
    pub copyright_holder: Option<String>,
}

/// Lee los ajustes del proyecto desde el directorio de configuración. @spec REQ-74
#[tauri::command]
pub fn unreal_get_project_settings(app: AppHandle) -> Result<ProjectSettings, UnrealError> {
    let path = config_dir(&app)?.join("project-settings.json");
    if path.is_file() {
        let content = std::fs::read_to_string(&path).map_err(|e| io_error(&path, &e))?;
        serde_json::from_str(&content).map_err(|e| UnrealError::Format(e.to_string()))
    } else {
        Ok(ProjectSettings::default())
    }
}

/// Guarda los ajustes del proyecto en el directorio de configuración. @spec REQ-74
#[tauri::command]
pub fn unreal_set_project_settings(app: AppHandle, settings: ProjectSettings) -> Result<(), UnrealError> {
    let dir = config_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| io_error(&dir, &e))?;
    let path = dir.join("project-settings.json");
    let content = serde_json::to_string_pretty(&settings).map_err(|e| UnrealError::Format(e.to_string()))?;
    std::fs::write(&path, content).map_err(|e| io_error(&path, &e))?;
    Ok(())
}

/// Conecta y devuelve los toolsets disponibles, con sondeo de resiliencia en puertos 8000..8005. @spec REQ-02, REQ-75
#[tauri::command]
pub async fn unreal_connect(app: AppHandle, endpoint: Endpoint) -> Result<ConnectResponse, UnrealError> {
    let initial_endpoint = endpoint.clone();
    blocking(move || {
        let state = app.state::<UnrealState>();
        let initial_url = initial_endpoint.url();
        let initial_res = session_for(&state, &initial_url, true).and_then(|s| s.list_toolsets().map(|t| (t, initial_endpoint.clone())));

        let (toolsets, active_endpoint) = match initial_res {
            Ok(pair) => pair,
            Err(first_err) => {
                crate::logging::info(&format!("connect failed on {}: {}; probing ports 8000..=8005", initial_url, first_err));
                let mut found = None;
                for p in 8000..=8005 {
                    if p == initial_endpoint.port {
                        continue;
                    }
                    let probe_endpoint = Endpoint { port: p, path: initial_endpoint.path.clone() };
                    let probe_url = probe_endpoint.url();
                    if let Ok(sess) = McpSession::connect(&probe_url) {
                        if let Ok(toolsets) = sess.list_toolsets() {
                            if let Ok(mut sessions) = state.sessions.lock() {
                                sessions.insert(probe_url.clone(), sess);
                            }
                            crate::logging::info(&format!("auto-detected active MCP server on port {p} ({probe_url})"));
                            found = Some((toolsets, probe_endpoint));
                            break;
                        }
                    }
                }
                found.ok_or(first_err)?
            }
        };

        crate::logging::info(&format!("connect {}: {} toolsets", active_endpoint.url(), toolsets.len()));
        Ok(ConnectResponse { toolsets, endpoint: active_endpoint })
    })
    .await
}

#[tauri::command]
pub async fn unreal_list_toolsets(app: AppHandle, endpoint: Endpoint) -> Result<Vec<ToolsetSummary>, UnrealError> {
    let url = endpoint.url();
    blocking(move || with_session(&app.state::<UnrealState>(), &url, McpSession::list_toolsets)).await
}

#[tauri::command]
pub async fn unreal_describe_toolset(app: AppHandle, endpoint: Endpoint, toolset: String) -> Result<ToolsetInfo, UnrealError> {
    let url = endpoint.url();
    blocking(move || with_session(&app.state::<UnrealState>(), &url, |s| s.describe_toolset(&toolset))).await
}

/// Llama a una herramienta de un toolset (o a una de primer nivel si `toolset` está vacío).
/// La decisión de si hace falta permiso se toma ANTES, en el frontend (REQ-12). @spec REQ-02, RNF-03
#[tauri::command]
pub async fn unreal_call_tool(app: AppHandle, endpoint: Endpoint, toolset: String, tool: String, arguments: Value, timeout_secs: Option<u64>) -> Result<ToolResult, UnrealError> {
    let url = endpoint.url();
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(mcp_client::DEFAULT_TIMEOUT.as_secs()).clamp(1, 900));
    blocking(move || {
        with_session(&app.state::<UnrealState>(), &url, |session| {
            if toolset.trim().is_empty() {
                session.call(&tool, arguments.clone(), timeout)
            } else {
                session.call_toolset_tool(&toolset, &tool, arguments.clone(), timeout)
            }
        })
    })
    .await
}

// ─── Compilar el código del proyecto (REQ-22) ─────────────────────────

/// Cómo se compilan esos ficheros cambiados (Live Coding, editor cerrado o nada). @spec REQ-22
#[tauri::command]
pub fn unreal_compile_plan(paths: Vec<String>) -> super::compile::CompilePlan {
    super::compile::compile_plan(&paths)
}

/// Compila con Live Coding en el editor conectado. @spec REQ-22
#[tauri::command]
pub async fn unreal_compile_live(app: AppHandle, endpoint: Endpoint) -> Result<super::compile::CompileOutcome, UnrealError> {
    let url = endpoint.url();
    blocking(move || with_session(&app.state::<UnrealState>(), &url, super::compile::live_coding)).await
}

/// Compila el editor del proyecto con UBT; el editor tiene que estar cerrado. @spec REQ-22
#[tauri::command]
pub async fn unreal_build_project(path: String) -> Result<super::compile::CompileOutcome, UnrealError> {
    blocking(move || {
        let project = Path::new(&path);
        super::compile::build_project(&super::compile::engine_root_for(project)?, project)
    })
    .await
}

/// Captura del viewport reducida a JPEG para un modelo con visión. @spec REQ-13
#[tauri::command]
pub async fn unreal_capture_viewport(app: AppHandle, endpoint: Endpoint, annotate: bool, detail: Option<String>) -> Result<super::capture::Capture, UnrealError> {
    let high = detail.as_deref() == Some("high");
    let url = endpoint.url();
    blocking(move || with_session(&app.state::<UnrealState>(), &url, |session| super::capture::capture(session, annotate, high))).await
}

// ─── Preparación ─────────────────────────────────────────────────────

/// Diagnóstico de un proyecto. @spec REQ-04
#[tauri::command]
pub async fn unreal_readiness(path: String) -> Result<Readiness, UnrealError> {
    blocking(move || prepare::readiness(Path::new(&path), &prepare::toolset_source()?)).await
}

/// Cambios que haría «Preparar» (con su diff), sin escribir nada. @spec REQ-05
#[tauri::command]
pub async fn unreal_prepare_plan(path: String, variant: Option<String>, port: Option<u16>) -> Result<PreparePlan, UnrealError> {
    blocking(move || prepare::plan(Path::new(&path), &prepare::toolset_source()?, prepare::Variant::parse(variant.as_deref()), port)).await
}

/// ¿Es `a` la misma ruta que `b`? (sin distinguir mayúsculas ni el tipo de barra, como en Windows).
pub fn same_path(a: &str, b: &str) -> bool {
    let normal = |p: &str| p.trim().trim_matches('"').replace('\\', "/").to_lowercase();
    normal(a) == normal(b)
}

/// Error si el proyecto está abierto en un editor: al cerrarse, el editor guarda sus ajustes y
/// pisaría los que escribe la preparación (Architect Review de `/plan` v0.1).
fn ensure_closed(path: &str) -> Result<(), UnrealError> {
    let open = discovery::running_editors().into_iter().any(|e| e.project.as_deref().is_some_and(|p| same_path(p, path)));
    if open {
        return Err(UnrealError::Protocol("guarda y cierra el editor de este proyecto antes de prepararlo o revertirlo".into()));
    }
    Ok(())
}

/// Aplica el plan con copia de seguridad previa; devuelve el id de la copia. @spec REQ-05
#[tauri::command]
pub async fn unreal_prepare_apply(app: AppHandle, path: String, variant: Option<String>, port: Option<u16>) -> Result<String, UnrealError> {
    let backups = data_dir(&app)?.join("backups");
    blocking(move || {
        ensure_closed(&path)?;
        let project = Path::new(&path);
        let plan = prepare::plan(project, &prepare::toolset_source()?, prepare::Variant::parse(variant.as_deref()), port)?;
        prepare::apply(project, &plan, &backups)
    })
    .await
}

/// Deshace una preparación con su copia de seguridad. @spec REQ-05
#[tauri::command]
pub async fn unreal_prepare_revert(app: AppHandle, path: String, backup: String) -> Result<(), UnrealError> {
    let backups = data_dir(&app)?.join("backups");
    blocking(move || {
        ensure_closed(&path)?;
        prepare::revert(Path::new(&path), &backups, &backup)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec REQ-03
    #[test]
    fn la_orden_del_editor_para_wmi_entrecomilla_rutas_con_espacios() {
        let line = editor_command_line(r"D:\Program Files\Epic Games\UE_5.8\Engine\Binaries\Win64\UnrealEditor.exe", r"D:\Mis Proyectos\Juego.uproject");
        assert_eq!(line, "\"D:\\Program Files\\Epic Games\\UE_5.8\\Engine\\Binaries\\Win64\\UnrealEditor.exe\" \"D:\\Mis Proyectos\\Juego.uproject\"");
    }

    /// @spec REQ-03
    #[test]
    fn los_recientes_van_primero_sin_duplicados_y_con_tope() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..15 {
            remember_recent(dir.path(), &format!("C:/P{i}/P{i}.uproject")).unwrap();
        }
        let recents = remember_recent(dir.path(), "c:/p3/P3.uproject").unwrap();
        assert_eq!(recents.len(), MAX_RECENTS);
        assert_eq!(recents[0], "c:/p3/P3.uproject");
        assert_eq!(recents.iter().filter(|p| p.eq_ignore_ascii_case("C:/P3/P3.uproject")).count(), 1);
        assert_eq!(load_recents(dir.path()), recents);
    }

    /// @spec REQ-05
    #[test]
    fn compara_rutas_de_proyecto_como_windows() {
        assert!(same_path(r"D:\Programacion\Unreal\DBVMCPFirst\DBVMCPFirst.uproject", "d:/programacion/unreal/dbvmcpfirst/DBVMCPFirst.uproject"));
        assert!(!same_path("D:/A/A.uproject", "D:/B/A.uproject"));
    }

    /// La app solo habla con 127.0.0.1: el frontend da puerto y ruta, nunca un host.
    /// @spec REQ-02, RNF-06
    #[test]
    fn el_endpoint_siempre_apunta_a_localhost() {
        let endpoint = Endpoint { port: 8100, path: None };
        assert_eq!(endpoint.url(), "http://127.0.0.1:8100/mcp");
        assert_eq!(Endpoint { port: 1, path: Some("x".into()) }.url(), "http://127.0.0.1:1/x");
    }

    /// Guarda y lee ajustes de proyecto de forma persistente. @spec REQ-74
    #[test]
    fn guarda_y_recupera_ajustes_de_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project-settings.json");
        let initial = ProjectSettings { copyright_holder: Some("David Bueno Vallejo".into()) };
        let content = serde_json::to_string_pretty(&initial).unwrap();
        std::fs::write(&path, content).unwrap();

        let read_back: ProjectSettings = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(read_back.copyright_holder.as_deref(), Some("David Bueno Vallejo"));
    }
}
