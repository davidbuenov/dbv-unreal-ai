// =============================================================================
// DBV Unreal AI — Servidor MCP de UEFN: identificarlo, diagnóstico y log del editor
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-123
//
// Lo que se vio en el spike P-14 (2026-11-20, fixtures en `tests/fixtures/unreal-mcp/uefn-43.10/`):
//   · UEFN habla el MCP de UE 5.8 (metaherramientas, `describe_toolset` en singular), así que el cliente MCP no cambia.
//   · `serverInfo` llega vacío: lo que distingue a UEFN de un editor de UE en el mismo puerto son sus toolsets
//     (`ValkyrieFortniteToolset.SessionToolset`, `VerseToolset.VerseToolset`). Las `instructions` no sirven: el usuario
//     puede escribir las suyas.
//   · Si otro editor tenía el puerto al arrancar UEFN, el log dice `HttpListener unable to bind to 127.0.0.1:8000` y el
//     panel sigue marcando «Running».
//   · Durante una llamada MCP, UEFN contesta solo los diálogos con la respuesta por defecto. Solo queda la línea
//     `Message dialog closed, result: <botón>, title: <título>, text: <texto>` en el log.
//   · Python del proyecto: `.uefnproject` → `dataSets.experimental.pythonExperimental.bEnablePythonForProject`, y por
//     usuario `EnablePythonLocallyPerProject=((<projectId>, True))` en el `.ini`. El MCP funciona sin Python.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::unreal::discovery::{McpSettings, DEFAULT_MCP_PATH, DEFAULT_MCP_PORT, MCP_SECTION, MCP_SECTION_59};
use crate::unreal::mcp_client::ToolsetSummary;
use crate::unreal::{ini, io_error, UnrealError};

/// Toolsets que solo tiene UEFN y que hacen falta para la v0.7.0 (compilar y probar).
pub const UEFN_TOOLSETS: [&str; 2] = ["ValkyrieFortniteToolset.SessionToolset", "VerseToolset.VerseToolset"];

/// Lo más que se lee del log de una vez, para no cargar varios MB tras una mañana de trabajo.
const MAX_LOG_READ: u64 = 2 * 1024 * 1024;

/// ¿Es UEFN el editor que responde? Por sus toolsets, no por `serverInfo` (vacío) ni por `instructions`.
pub fn is_uefn(toolsets: &[ToolsetSummary]) -> bool {
    UEFN_TOOLSETS.iter().all(|wanted| toolsets.iter().any(|t| t.name == *wanted))
}

/// Puerto y ruta del MCP de UEFN, desde el `.ini` del usuario (solo aparecen si se cambiaron: por defecto, 8000 y `/mcp`).
pub fn mcp_settings(ini_text: &str) -> McpSettings {
    let mut settings = McpSettings { port: DEFAULT_MCP_PORT, path: DEFAULT_MCP_PATH.to_string(), auto_start: None };
    for section in [MCP_SECTION, MCP_SECTION_59] {
        if let Some(port) = ini::get(ini_text, section, "ServerPortNumber").and_then(|v| v.parse().ok()) {
            settings.port = port;
        }
        if let Some(path) = ini::get(ini_text, section, "ServerUrlPath").filter(|v| v.starts_with('/')) {
            settings.path = path;
        }
        if let Some(flag) = ini::get(ini_text, section, "bAutoStartServer") {
            settings.auto_start = Some(flag.eq_ignore_ascii_case("true"));
        }
    }
    settings
}

// ─── Python ─────────────────────────────────────────────────────────

/// Python en un proyecto UEFN: el ajuste del proyecto y el permiso local del usuario.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonState {
    pub project: bool,
    pub local: bool,
}

impl PythonState {
    pub fn enabled(&self) -> bool {
        self.project && self.local
    }
}

/// Lee los dos ajustes de Python (P-14). La app no los escribe: «Preparar» usa la herramienta del MCP.
pub fn python_state(uefnproject_text: &str, ini_text: &str, project_id: &str) -> PythonState {
    let project = serde_json::from_str::<Value>(uefnproject_text.trim_start_matches('\u{feff}'))
        .ok()
        .and_then(|v| v.pointer("/dataSets/experimental/pythonExperimental/bEnablePythonForProject").and_then(Value::as_bool))
        .unwrap_or(false);
    let wanted = format!("({}, true)", project_id.to_lowercase());
    let local = !project_id.is_empty()
        && ini_text.lines().any(|line| {
            let compact = line.trim().to_lowercase();
            compact.starts_with("enablepythonlocallyperproject=") && compact.contains(&wanted)
        });
    PythonState { project, local }
}

// ─── Log del editor ─────────────────────────────────────────────────

/// Posición ya leída del log del editor. Se lee solo lo que ha crecido; si el fichero es más pequeño que la posición,
/// UEFN lo ha rotado y se empieza por el principio del nuevo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct LogCursor {
    pub offset: u64,
}

impl LogCursor {
    /// Cursor al final del log de ahora: lo de antes no interesa.
    pub fn at_end(path: &Path) -> Self {
        let offset = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        Self { offset }
    }
}

/// Texto añadido al log desde `cursor` (como mucho `MAX_LOG_READ`, los bytes más recientes) y el cursor nuevo.
pub fn read_new(path: &Path, cursor: LogCursor) -> Result<(String, LogCursor), UnrealError> {
    let mut file = std::fs::File::open(path).map_err(|error| io_error(path, &error))?;
    let len = file.metadata().map_err(|error| io_error(path, &error))?.len();
    let from = if len < cursor.offset { 0 } else { cursor.offset };
    let start = from.max(len.saturating_sub(MAX_LOG_READ));
    file.seek(SeekFrom::Start(start)).map_err(|error| io_error(path, &error))?;
    let mut bytes = Vec::new();
    file.take(len - start).read_to_end(&mut bytes).map_err(|error| io_error(path, &error))?;
    let result = (String::from_utf8_lossy(&bytes).into_owned(), LogCursor { offset: len });
    Ok(result)
}

/// Un diálogo que UEFN contestó solo mientras atendía una llamada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnsweredDialog {
    pub result: String,
    pub title: String,
    pub text: String,
}

/// Diálogos contestados en un trozo del log (`Message dialog closed, result: Ok, title: …, text: …`).
pub fn answered_dialogs(log: &str) -> Vec<AnsweredDialog> {
    const MARK: &str = "Message dialog closed, result: ";
    let dialogs = log
        .lines()
        .filter_map(|line| line.split_once(MARK).map(|(_, rest)| rest))
        .map(|rest| {
            let (result, rest) = rest.split_once(", title: ").unwrap_or((rest, ""));
            let (title, text) = rest.split_once(", text: ").unwrap_or((rest, ""));
            AnsweredDialog { result: result.trim().to_string(), title: title.trim().to_string(), text: text.trim().to_string() }
        })
        .collect();
    dialogs
}

/// ¿Falló UEFN al abrir el puerto en su última ejecución? Solo cuenta lo que hay tras el último `LogInit: Build:` (el
/// arranque más reciente) y si después no hubo un arranque bueno del servidor.
pub fn bind_failed(log: &str, port: u16) -> bool {
    let last_run = log.rfind("LogInit: Build:").map(|i| &log[i..]).unwrap_or(log);
    let failure = format!("unable to bind to 127.0.0.1:{port}");
    let failed_at = last_run.rfind(&failure);
    let started_ok_after = failed_at.is_some_and(|at| last_run[at..].lines().skip(1).any(|l| l.contains("LogHttpListener") && l.contains("Created new HttpListener") && l.contains(&format!(":{port}"))));
    failed_at.is_some() && !started_ok_after
}

// ─── Diagnóstico ────────────────────────────────────────────────────

/// Lo que se sabe del servidor del puerto de UEFN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerProbe {
    pub responding: bool,
    pub uefn: bool,
    pub toolsets: usize,
}

/// Datos de entrada del diagnóstico, ya leídos (así se prueba sin UEFN).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagnosticInput {
    pub version: Option<String>,
    pub running: bool,
    pub open_is_selected: bool,
    pub open_title: Option<String>,
    pub python: PythonState,
    pub port: u16,
    pub server: ServerProbe,
    pub bind_failed: bool,
}

/// Una fila del diagnóstico de UEFN (las 6 primeras de REQ-123; la 7, la skill, la añade REQ-124). `key` es la clave
/// de traducción de la fila y `fix` la de su solución; `detail` lleva los datos (versión, puerto, número de toolsets).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRow {
    pub step: u8,
    pub key: String,
    pub ok: bool,
    /// Una fila opcional que falla no impide trabajar (Python).
    pub optional: bool,
    pub detail: String,
    pub fix: Option<String>,
}

fn row(step: u8, key: &str, ok: bool, optional: bool, detail: String, fix: &str) -> DiagnosticRow {
    DiagnosticRow { step, key: format!("uefn.diag.{key}"), ok, optional, detail, fix: (!ok).then(|| format!("uefn.fix.{fix}")) }
}

/// Filas del diagnóstico de UEFN, en el orden de REQ-123. Sin filas de UE (PythonScriptPlugin, plugin DBV, Fab).
pub fn diagnostic_rows(input: &DiagnosticInput) -> Vec<DiagnosticRow> {
    let server_fix = if input.bind_failed {
        "portBusy"
    } else if input.server.responding && !input.server.uefn {
        "otherEditor"
    } else {
        "startServer"
    };
    let rows = vec![
        row(1, "installed", input.version.is_some(), false, input.version.clone().unwrap_or_default(), "install"),
        row(2, "editorOpen", input.running, false, String::new(), "openEditor"),
        row(3, "projectOpen", input.running && input.open_is_selected, false, input.open_title.clone().unwrap_or_default(), "openProject"),
        row(4, "python", input.python.enabled(), true, String::new(), "enablePython"),
        row(5, "server", input.server.responding && input.server.uefn, false, input.port.to_string(), server_fix),
        row(6, "toolsets", input.server.uefn && input.server.toolsets > 0, false, input.server.toolsets.to_string(), "toolsets"),
    ];
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/../tests/fixtures/unreal-mcp/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// Toolsets de un `list_toolsets` grabado: líneas `- <nombre>: <descripción>`.
    fn toolsets_of(name: &str) -> Vec<ToolsetSummary> {
        let value = fixture(name);
        let text = value.pointer("/result/content/0/text").and_then(Value::as_str).unwrap().to_string();
        text.lines().filter_map(|l| l.strip_prefix("- ")).filter_map(|l| l.split_once(':')).map(|(n, d)| ToolsetSummary { name: n.trim().to_string(), description: d.trim().to_string() }).collect()
    }

    #[test]
    fn reconoce_uefn_por_sus_toolsets_y_no_confunde_un_editor_de_ue() {
        // @spec REQ-123
        let uefn = toolsets_of("uefn-43.10/list_toolsets.response.json");
        assert!(uefn.len() > 50, "{}", uefn.len());
        assert!(is_uefn(&uefn));
        let ue = toolsets_of("ue-5.8.4/list_toolsets.response.json");
        assert!(!ue.is_empty());
        assert!(!is_uefn(&ue));
    }

    #[test]
    fn el_initialize_de_uefn_no_trae_server_info() {
        // @spec REQ-123 — por eso la identificación va por toolsets.
        let init = fixture("uefn-43.10/initialize.response.json");
        assert_eq!(init.pointer("/result/serverInfo/name").and_then(Value::as_str), Some(""));
        assert_eq!(init.pointer("/result/serverInfo/version").and_then(Value::as_str), Some(""));
    }

    #[test]
    fn lee_el_puerto_del_ini_o_usa_el_8000() {
        // @spec REQ-123
        assert_eq!(mcp_settings("").port, 8000);
        let ini = "[/Script/ModelContextProtocolEngine.ModelContextProtocolSettings]\nServerPortNumber=8010\n";
        assert_eq!(mcp_settings(ini).port, 8010);
    }

    #[test]
    fn python_necesita_el_ajuste_del_proyecto_y_el_permiso_local() {
        // @spec REQ-123
        let project = r#"{"dataSets": {"experimental": {"pythonExperimental": {"bEnablePythonForProject": true}}}}"#;
        let ini = "[X]\nEnablePythonLocallyPerProject=((34c4d41b-355e-4572-a641-504313d5e904, True))\n";
        let state = python_state(project, ini, "34c4d41b-355e-4572-a641-504313d5e904");
        assert!(state.enabled());
        assert!(!python_state(project, ini, "otro-id").enabled());
        assert!(!python_state("{}", ini, "34c4d41b-355e-4572-a641-504313d5e904").enabled());
    }

    #[test]
    fn lee_los_dialogos_contestados_solos() {
        // @spec REQ-123, REQ-128 — líneas reales del log del spike.
        let log = "[2026.11.20-07.02.24:982][789]Message dialog closed, result: Ok, title: Unable to Check Out From Revision Control!, text: The following assets could not be successfully checked out from revision control:\nLogVerse: otra cosa\n[2026.11.20-07.07.18:112][920]Message dialog closed, result: No, title: DBV spike, text: Prueba de dialogo modal: pulsa Aceptar\n";
        let dialogs = answered_dialogs(log);
        assert_eq!(dialogs.len(), 2);
        assert_eq!(dialogs[0].result, "Ok");
        assert_eq!(dialogs[0].title, "Unable to Check Out From Revision Control!");
        assert_eq!(dialogs[1], AnsweredDialog { result: "No".into(), title: "DBV spike".into(), text: "Prueba de dialogo modal: pulsa Aceptar".into() });
    }

    #[test]
    fn detecta_el_puerto_ocupado_solo_en_el_ultimo_arranque() {
        // @spec REQ-123
        let failed = "LogInit: Build: ++Fortnite+Release-43.12-CL-1\nLogHttpListener: Error: HttpListener unable to bind to 127.0.0.1:8000\nLogModelContextProtocol: Starting MCP server on port 8000\n";
        assert!(bind_failed(failed, 8000));
        assert!(!bind_failed(failed, 8010));
        let old_failure_then_restart = format!("{failed}LogInit: Build: ++Fortnite+Release-43.12-CL-1\nLogModelContextProtocol: Starting MCP server on port 8000\n");
        assert!(!bind_failed(&old_failure_then_restart, 8000));
        let retried = format!("{failed}LogHttpListener: Created new HttpListener on 127.0.0.1:8000\n");
        assert!(!bind_failed(&retried, 8000));
    }

    #[test]
    fn lee_solo_lo_nuevo_del_log_y_vuelve_al_principio_si_se_rota() {
        // @spec REQ-123
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("UnrealEditorFortnite.log");
        std::fs::write(&log, "antes\n").unwrap();
        let cursor = LogCursor::at_end(&log);
        std::fs::write(&log, "antes\nnuevo\n").unwrap();
        let (text, cursor) = read_new(&log, cursor).unwrap();
        assert_eq!(text, "nuevo\n");
        std::fs::write(&log, "rotado\n").unwrap();
        let (text, _) = read_new(&log, cursor).unwrap();
        assert_eq!(text, "rotado\n");
    }

    #[test]
    fn diagnostico_con_uefn_listo_y_con_el_puerto_ocupado() {
        // @spec REQ-123
        let ready = DiagnosticInput {
            version: Some("43.12".into()),
            running: true,
            open_is_selected: true,
            open_title: Some("DBVUnrealAITest".into()),
            python: PythonState { project: true, local: true },
            port: 8000,
            server: ServerProbe { responding: true, uefn: true, toolsets: 63 },
            bind_failed: false,
        };
        let rows = diagnostic_rows(&ready);
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|r| r.ok && r.fix.is_none()));
        assert!(rows.iter().all(|r| !r.key.contains("Plugin") && !r.key.contains("fab")));
        let busy = DiagnosticInput { bind_failed: true, server: ServerProbe::default(), ..ready.clone() };
        let rows = diagnostic_rows(&busy);
        assert_eq!(rows[4].fix.as_deref(), Some("uefn.fix.portBusy"));
        let other = DiagnosticInput { server: ServerProbe { responding: true, uefn: false, toolsets: 40 }, ..ready };
        let rows = diagnostic_rows(&other);
        assert_eq!(rows[4].fix.as_deref(), Some("uefn.fix.otherEditor"));
        assert!(!rows[5].ok);
    }
}
