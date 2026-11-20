// =============================================================================
// DBV Unreal AI — Probar en una sesión de UEFN y leer lo que imprime Verse
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-128
//
// Lo medido en el spike P-14 (2026-11-20, fixtures `call_tool.SessionToolset.*`):
//   · `StartSession` sube el proyecto y lanza el cliente: ~2 min, y no responde hasta que acaba. Por eso tiene su propio
//     tiempo (5 min) y un candado por servidor: mientras arranca, otra llamada de sesión se rechaza con un mensaje
//     claro, salvo «parar la sesión», que espera su turno.
//   · `PushChanges(bVerseOnly)` tarda ~15 s y vuelve a ejecutar `OnBegin`; lo que imprime llega unos segundos después,
//     así que se relee `GetVerseOutput` hasta 10 s buscando la etiqueta que se pida.
//   · UEFN contesta solo los diálogos durante una llamada MCP: tras cada llamada que cambia algo se leen las líneas
//     nuevas del log del editor y se cuenta qué diálogo salió y qué contestó.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::server::{answered_dialogs, read_new, LogCursor};
use crate::unreal::compact;
use crate::unreal::mcp_client::{McpSession, DEFAULT_TIMEOUT};

const SESSION: &str = "ValkyrieFortniteToolset.SessionToolset";
pub const START_TIMEOUT: Duration = Duration::from_secs(300);
const PUSH_TIMEOUT: Duration = Duration::from_secs(180);
/// Cuánto se relee la salida de Verse buscando la etiqueta tras enviar o empezar.
const OUTPUT_WAIT: Duration = Duration::from_secs(10);

pub const START: &str = "uefn_session_start";
pub const PUSH: &str = "uefn_session_push";
pub const GAME: &str = "uefn_game";
pub const STATUS: &str = "uefn_session_status";
pub const OUTPUT: &str = "uefn_verse_output";
pub const CLIENT_LOG: &str = "uefn_client_log";
pub const STOP: &str = "uefn_session_stop";

/// Las que no cambian nada: sin aprobación.
pub const READ_ONLY: [&str; 3] = [STATUS, OUTPUT, CLIENT_LOG];

/// Definiciones, las mismas para la conversación y el `--mcp`.
pub fn tool_specs() -> Vec<Value> {
    let tool = |name: &str, description: &str, properties: Value, read_only: bool| json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": properties, "required": [] }, "annotations": { "readOnlyHint": read_only } });
    let wait_for = json!({ "type": "string", "description": "Text your Verse prints when it starts (e.g. \"ZZTEST v2\"): the output is re-read for up to 10 s until it appears." });
    vec![
        tool(START, "Start a UEFN play-test session: UEFN saves the level, uploads the project and launches the Fortnite client (about 2 minutes; the call waits). Needs the user approval. If the level has zz_ test entities, it stops first and tells you: ask the user, then call again with allow_test_entities=true.", json!({ "allow_test_entities": { "type": "boolean" } }), false),
        tool(PUSH, "Push your Verse changes to the running session (verse_only=true, ~15 s; it re-runs OnBegin). Then the Verse output is re-read until wait_for appears. Use it after BuildAll compiles.", json!({ "verse_only": { "type": "boolean" }, "wait_for": wait_for }), false),
        tool(GAME, "Start or stop the in-game match on the connected session (action: start | stop).", json!({ "action": { "type": "string", "enum": ["start", "stop"] }, "wait_for": wait_for }), false),
        tool(STATUS, "Session connection status and in-game match state. Read-only.", json!({}), true),
        tool(OUTPUT, "Read what Verse printed in the session (Print, Verse log and runtime errors with file and line). Filter with pattern (case-insensitive regex), e.g. \"ZZTEST\". Read-only.", json!({ "pattern": { "type": "string" }, "max_results": { "type": "integer" } }), true),
        tool(CLIENT_LOG, "Search the Fortnite client log of the session (engine warnings and errors), e.g. pattern \"Error|Warning\". Read-only.", json!({ "pattern": { "type": "string" }, "max_results": { "type": "integer" } }), true),
        tool(STOP, "Stop and tear down the play-test session.", json!({}), false),
    ]
}

/// Candado por servidor: una llamada de sesión cada vez.
fn lock_for(url: &str) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    let map = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let lock = map.lock().map(|mut m| m.entry(url.to_string()).or_default().clone()).unwrap_or_default();
    lock
}

/// Una entrada de `GetVerseOutput` en una línea: `[Error] zz_test.verse:8 mensaje`.
fn entry_line(entry: &Value) -> String {
    let text = |k: &str| entry.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let line = entry.get("sourceLine").and_then(Value::as_u64).map(|l| format!(":{l}")).unwrap_or_default();
    let severity = text("severity");
    let tag = if severity.is_empty() || severity == "Normal" { String::new() } else { format!("[{severity}] ") };
    format!("{tag}{}{line} {}", text("sourceFile"), text("message"))
}

/// Entradas de `GetVerseOutput` (`returnValue.entries`).
pub fn output_entries(value: &Value) -> Vec<Value> {
    value.get("entries").and_then(Value::as_array).cloned().unwrap_or_default()
}

/// Lo que se cuenta de los diálogos contestados solos (vacío si no hubo).
pub fn dialogs_note(log_text: &str) -> String {
    let dialogs = answered_dialogs(log_text);
    let note = if dialogs.is_empty() {
        String::new()
    } else {
        let list = dialogs.iter().map(|d| format!("«{}» → {}", d.title, d.result)).collect::<Vec<_>>().join("; ");
        format!("\nUEFN answered these dialogs by itself during the call: {list}. Tell the user.")
    };
    note
}

struct Call<'a> {
    session: &'a McpSession,
    log: Option<&'a Path>,
}

impl Call<'_> {
    fn tool(&self, tool: &str, args: Value, timeout: Duration) -> Result<Value, String> {
        let called = self.session.call_toolset_tool(SESSION, tool, args, timeout).map_err(|e| e.to_string())?;
        let result = if called.is_error { Err(format!("{tool}: {}", called.text)) } else { Ok(called.value.unwrap_or(Value::Null)) };
        result
    }

    /// Llama a una herramienta que cambia algo y añade los diálogos que UEFN contestó mientras tanto.
    fn changing(&self, tool: &str, args: Value, timeout: Duration) -> Result<(Value, String), String> {
        let cursor = self.log.map(LogCursor::at_end);
        let value = self.tool(tool, args, timeout)?;
        let note = match (self.log, cursor) {
            (Some(log), Some(cursor)) => read_new(log, cursor).map(|(text, _)| dialogs_note(&text)).unwrap_or_default(),
            _ => String::new(),
        };
        Ok((value, note))
    }

    fn output(&self, pattern: &str, max: u64) -> Result<Vec<Value>, String> {
        let value = self.tool("GetVerseOutput", json!({ "pattern": pattern, "maxResults": max.clamp(1, 200), "cursor": 0 }), DEFAULT_TIMEOUT)?;
        Ok(output_entries(&value))
    }

    /// Relee la salida hasta que haya más entradas con `wait_for` que antes (como mucho `OUTPUT_WAIT`).
    fn wait_output(&self, wait_for: &str, before: usize, sleep: &dyn Fn(Duration)) -> String {
        let started = Instant::now();
        let mut found: Vec<Value> = Vec::new();
        while found.len() <= before && started.elapsed() < OUTPUT_WAIT {
            sleep(Duration::from_millis(1000));
            found = self.output(wait_for, 200).unwrap_or_default();
        }
        let fresh: Vec<String> = found.iter().skip(before).map(entry_line).collect();
        let text = if fresh.is_empty() { format!("\n\"{wait_for}\" did not appear in the Verse output within 10 s. Check uefn_verse_output, and that the code prints it when it starts.") } else { format!("\nNew Verse output:\n{}", fresh.join("\n")) };
        text
    }
}

/// ¿Hay entidades o actores de prueba `zz_` en el nivel? (LevelToolset.FindActors por nombre.)
fn test_actors(session: &McpSession) -> Vec<String> {
    let found = session
        .call_toolset_tool("EditorToolset.LevelToolset", "FindActors", json!({ "filter": { "name": "zz_" } }), DEFAULT_TIMEOUT)
        .ok()
        .and_then(|r| r.value)
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    found.iter().filter_map(|a| a.get("refPath").and_then(Value::as_str).or_else(|| a.as_str()).map(|s| s.rsplit('.').next().unwrap_or(s).to_string())).collect()
}

/// Ejecuta una herramienta de sesión en el UEFN de `url`. `log` es el log del editor (para los diálogos contestados).
pub fn run(name: &str, args: &Value, url: &str, log: Option<&Path>, sleep: &dyn Fn(Duration)) -> Result<String, String> {
    let lock = lock_for(url);
    // «Parar la sesión» espera a que acabe la llamada en curso; las demás no esperan: se explica y se reintenta.
    let guard = if name == STOP { lock.lock().ok() } else { lock.try_lock().ok() };
    let Some(_guard) = guard else { return Err("another UEFN session call is still running (starting a session takes about 2 minutes). Wait for it to finish, then try again.".into()) };
    let session = McpSession::connect(url).map_err(|e| e.to_string())?;
    let call = Call { session: &session, log };
    let text = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let result = match name {
        START => {
            let zz = if args.get("allow_test_entities").and_then(Value::as_bool) == Some(true) { Vec::new() } else { test_actors(&session) };
            if zz.is_empty() {
                call.changing("StartSession", json!({}), START_TIMEOUT).map(|(v, note)| format!("Session: {}{note}", v.as_str().unwrap_or("started")))
            } else {
                Ok(format!("Not started: the level has test entities ({}). Tell the user; call again with allow_test_entities=true if they agree, or remove them first.", zz.join(", ")))
            }
        }
        PUSH => {
            let wait_for = text("wait_for");
            let before = if wait_for.is_empty() { 0 } else { call.output(&wait_for, 200).map(|e| e.len()).unwrap_or(0) };
            let verse_only = args.get("verse_only").and_then(Value::as_bool).unwrap_or(true);
            let pushed = call.changing("PushChanges", json!({ "bVerseOnly": verse_only }), PUSH_TIMEOUT).or_else(|error| {
                // Si UEFN no admite enviar solo Verse ahora, se envía todo (tarda más).
                if verse_only { call.changing("PushChanges", json!({ "bVerseOnly": false }), PUSH_TIMEOUT).map(|(v, n)| (v, format!("{n}\nVerse-only push was not available ({error}); the whole project was pushed."))) } else { Err(error) }
            });
            pushed.map(|(v, note)| format!("Push: {}{note}{}", v.as_str().unwrap_or("done"), if wait_for.is_empty() { String::new() } else { call.wait_output(&wait_for, before, sleep) }))
        }
        GAME => {
            let action = if text("action") == "stop" { "StopGame" } else { "StartGame" };
            let wait_for = text("wait_for");
            let before = if wait_for.is_empty() { 0 } else { call.output(&wait_for, 200).map(|e| e.len()).unwrap_or(0) };
            call.changing(action, json!({}), PUSH_TIMEOUT).map(|(v, note)| format!("{action}: {}{note}{}", v.as_str().unwrap_or("done"), if wait_for.is_empty() || action == "StopGame" { String::new() } else { call.wait_output(&wait_for, before, sleep) }))
        }
        STATUS => {
            let session_state = call.tool("GetSessionStatus", json!({}), DEFAULT_TIMEOUT)?;
            let game = call.tool("GetGameState", json!({}), DEFAULT_TIMEOUT).unwrap_or(Value::Null);
            Ok(format!("Session: {} · Game: {}", session_state.as_str().unwrap_or("?"), game.as_str().unwrap_or("?")))
        }
        OUTPUT => {
            let entries = call.output(&text("pattern"), args.get("max_results").and_then(Value::as_u64).unwrap_or(100))?;
            Ok(if entries.is_empty() { "No Verse output matches.".to_string() } else { entries.iter().map(entry_line).collect::<Vec<_>>().join("\n") })
        }
        CLIENT_LOG => {
            let max = args.get("max_results").and_then(Value::as_u64).unwrap_or(50).clamp(1, 200);
            let value = call.tool("GetClientLogEntries", json!({ "pattern": text("pattern"), "maxResults": max, "startLine": 0 }), DEFAULT_TIMEOUT)?;
            let lines: Vec<String> = value.get("entries").and_then(Value::as_array).map(|e| e.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
            Ok(if lines.is_empty() { "No client log line matches.".to_string() } else { lines.join("\n") })
        }
        STOP => call.changing("StopSession", json!({}), DEFAULT_TIMEOUT).map(|(_, note)| format!("Session stopped.{note}")),
        other => Err(format!("unknown UEFN session tool: {other}")),
    };
    result.map(|t| compact::clip(&t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn recorded(file: &str) -> Value {
        let text = std::fs::read_to_string(format!("{}/../tests/fixtures/unreal-mcp/uefn-43.10/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    const CRLF: &str = "\r\n";
    const CRLF2: &str = "\r\n\r\n";

    /// UEFN simulado para la sesión: responde con lo grabado en P-14. `slow_start` hace que StartSession tarde.
    /// La salida de Verse crece en cada lectura tras un envío (como en el spike, llega con retraso).
    fn fake_session(slow_start: Duration, zz: bool) -> (String, Arc<Mutex<Vec<String>>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let seen = calls.clone();
        let reads = Arc::new(AtomicUsize::new(0));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut socket) = stream else { break };
                let seen = seen.clone();
                let reads = reads.clone();
                std::thread::spawn(move || {
                    let mut received = Vec::new();
                    let mut buffer = [0u8; 8192];
                    loop {
                        let read = socket.read(&mut buffer).unwrap_or(0);
                        received.extend_from_slice(&buffer[..read]);
                        let text = String::from_utf8_lossy(&received).to_string();
                        let done = text.find(CRLF2).is_some_and(|end| {
                            let length = text[..end].lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                            received.len() >= end + 4 + length
                        });
                        if done || read == 0 {
                            break;
                        }
                    }
                    let request = String::from_utf8_lossy(&received).to_string();
                    let body: Value = request.split_once(CRLF2).and_then(|(_, b)| serde_json::from_str(b).ok()).unwrap_or(Value::Null);
                    let id = body.get("id").cloned().unwrap_or(Value::Null);
                    let tool = body["params"]["arguments"]["tool_name"].as_str().unwrap_or("").to_string();
                    if !tool.is_empty() {
                        seen.lock().unwrap().push(tool.clone());
                    }
                    let reply = |text: String| json!({ "jsonrpc": "2.0", "id": id, "result": { "content": [{ "type": "text", "text": text }] } }).to_string();
                    let payload = match (body["method"].as_str().unwrap_or(""), tool.as_str()) {
                        ("initialize", _) => json!({ "jsonrpc": "2.0", "id": id, "result": { "protocolVersion": "2025-06-18", "capabilities": {} } }).to_string(),
                        ("notifications/initialized", _) => String::new(),
                        (_, "StartSession") => {
                            std::thread::sleep(slow_start);
                            let mut r = recorded("call_tool.SessionToolset.StartSession.response.json");
                            r["id"] = id.clone();
                            r.to_string()
                        }
                        (_, "PushChanges") => {
                            let mut r = recorded("call_tool.SessionToolset.PushChanges.verseOnly.response.json");
                            r["id"] = id.clone();
                            r.to_string()
                        }
                        (_, "GetVerseOutput") => {
                            // Antes del envío, 1 línea con la etiqueta; dos lecturas después aparece la nueva.
                            let n = reads.fetch_add(1, Ordering::SeqCst);
                            let mut entries = vec![json!({ "sequence": 1, "severity": "Normal", "message": "ZZTEST v1", "sourceFile": "zz_test.verse", "sourceLine": 8 })];
                            if n >= 2 {
                                entries.push(json!({ "sequence": 2, "severity": "Normal", "message": "ZZTEST v2", "sourceFile": "zz_test.verse", "sourceLine": 8 }));
                            }
                            reply(json!({ "returnValue": { "entries": entries, "nextCursor": entries.len() } }).to_string())
                        }
                        (_, "GetSessionStatus") => reply("{\"returnValue\":\"Connected\"}".into()),
                        (_, "GetGameState") => reply("{\"returnValue\":\"Running\"}".into()),
                        (_, "FindActors") => reply(if zz { "{\"returnValue\":[{\"refPath\":\"/m/L.L:PersistentLevel.zz_cubo\"}]}".into() } else { "{\"returnValue\":[]}".into() }),
                        _ => reply("{\"returnValue\":null}".into()),
                    };
                    let head = format!("HTTP/1.1 200 OK{CRLF}Content-Type: application/json{CRLF}Mcp-Session-Id: s{CRLF}Content-Length: {}{CRLF}Connection: close{CRLF}{CRLF}", payload.len());
                    let _ = socket.write_all(head.as_bytes());
                    let _ = socket.write_all(payload.as_bytes());
                });
            }
        });
        (url, calls)
    }

    fn no_sleep(_: Duration) {}

    #[test]
    fn enviar_verse_relee_la_salida_hasta_ver_la_etiqueta() {
        // @spec REQ-128
        let (url, calls) = fake_session(Duration::ZERO, false);
        let text = run(PUSH, &json!({ "wait_for": "ZZTEST" }), &url, None, &no_sleep).unwrap();
        assert!(text.starts_with("Push: Completed"), "{text}");
        assert!(text.contains("zz_test.verse:8 ZZTEST v2"), "{text}");
        assert!(!text.contains("ZZTEST v1"), "solo lo nuevo: {text}");
        assert!(calls.lock().unwrap().contains(&"PushChanges".to_string()));
    }

    #[test]
    fn arrancar_avisa_de_entidades_zz_antes_de_empezar() {
        // @spec REQ-128
        let (url, calls) = fake_session(Duration::ZERO, true);
        let text = run(START, &json!({}), &url, None, &no_sleep).unwrap();
        assert!(text.starts_with("Not started") && text.contains("zz_cubo"), "{text}");
        assert!(!calls.lock().unwrap().contains(&"StartSession".to_string()));
        let started = run(START, &json!({ "allow_test_entities": true }), &url, None, &no_sleep).unwrap();
        assert!(started.contains("Session: Completed"), "{started}");
    }

    #[test]
    fn mientras_arranca_otra_llamada_se_rechaza_y_parar_espera_su_turno() {
        // @spec REQ-128 — el candado por servidor.
        let (url, calls) = fake_session(Duration::from_millis(800), false);
        let starting = {
            let url = url.clone();
            std::thread::spawn(move || run(START, &json!({}), &url, None, &no_sleep))
        };
        std::thread::sleep(Duration::from_millis(300));
        let busy = run(STATUS, &json!({}), &url, None, &no_sleep).unwrap_err();
        assert!(busy.contains("still running"), "{busy}");
        let stopped = run(STOP, &json!({}), &url, None, &no_sleep).unwrap();
        assert!(stopped.starts_with("Session stopped"));
        assert!(starting.join().unwrap().is_ok());
        let order = calls.lock().unwrap().clone();
        let start = order.iter().position(|t| t == "StartSession").unwrap();
        let stop = order.iter().position(|t| t == "StopSession").unwrap();
        assert!(start < stop, "parar va después de arrancar: {order:?}");
    }

    #[test]
    fn un_arranque_que_no_responde_se_corta_con_tiempo_agotado() {
        // @spec REQ-128 — con un tiempo corto para el test (el real es START_TIMEOUT).
        let (url, _) = fake_session(Duration::from_secs(3), false);
        let session = McpSession::connect(&url).unwrap();
        let error = session.call_toolset_tool(SESSION, "StartSession", json!({}), Duration::from_millis(500)).unwrap_err();
        assert!(matches!(error, crate::unreal::UnrealError::Timeout(_) | crate::unreal::UnrealError::Unreachable(_)), "{error:?}");
        assert!(START_TIMEOUT >= Duration::from_secs(300));
    }

    #[test]
    fn estado_y_salida_de_verse() {
        // @spec REQ-128
        let (url, _) = fake_session(Duration::ZERO, false);
        assert_eq!(run(STATUS, &json!({}), &url, None, &no_sleep).unwrap(), "Session: Connected · Game: Running");
        assert!(run(OUTPUT, &json!({ "pattern": "ZZTEST" }), &url, None, &no_sleep).unwrap().contains("zz_test.verse:8 ZZTEST v1"));
    }

    #[test]
    fn la_salida_grabada_se_lee_con_fichero_y_linea() {
        // @spec REQ-128 — `GetVerseOutput` real del spike.
        let rpc = recorded("call_tool.SessionToolset.GetVerseOutput.after-restart.response.json");
        let inner: Value = serde_json::from_str(rpc.pointer("/result/content/0/text").and_then(Value::as_str).unwrap()).unwrap();
        let entries = output_entries(&inner["returnValue"]);
        assert_eq!(entries.len(), 4);
        assert_eq!(entry_line(&entries[2]), "dbv_spike_device.verse:8 DBV spike: version 2 tras PushChanges");
    }

    #[test]
    fn cuenta_los_dialogos_que_uefn_contesto_solo() {
        // @spec REQ-128
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("UnrealEditorFortnite.log");
        std::fs::write(&log, "antes\n").unwrap();
        let cursor = LogCursor::at_end(&log);
        std::fs::write(&log, "antes\n[2026.11.20-07.02.24:982][789]Message dialog closed, result: Ok, title: Unable to Check Out From Revision Control!, text: x\n").unwrap();
        let (text, _) = read_new(&log, cursor).unwrap();
        let note = dialogs_note(&text);
        assert!(note.contains("«Unable to Check Out From Revision Control!» → Ok") && note.contains("Tell the user"));
        assert_eq!(dialogs_note("nada"), "");
    }

    #[test]
    fn las_de_lectura_no_piden_aprobacion() {
        // @spec REQ-128
        for spec in tool_specs() {
            let name = spec["name"].as_str().unwrap();
            assert_eq!(spec["annotations"]["readOnlyHint"].as_bool().unwrap(), READ_ONLY.contains(&name), "{name}");
            assert!(super::super::server::available_in_uefn(name), "{name} se ofrece con UEFN");
        }
    }
}
