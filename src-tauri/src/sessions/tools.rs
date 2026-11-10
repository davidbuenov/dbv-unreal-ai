// =============================================================================
// DBV Unreal AI — Caja negra: herramientas `session_*` para la IA (una sola definición)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-92
//
// Como las de Tripo y la memoria (D10, D69): se definen AQUÍ y las publican el MCP propio y el asistente integrado.
// Esta parte es la que trabaja sobre los ficheros de la sesión (lista, análisis, consulta, espera y borrado): no
// necesita el editor, así que sirve con el editor cerrado o colgado. Las respuestas son siempre compactas: quien
// quiera más detalle lo pide con un filtro.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use super::analysis::{events, frame, motion, relative, Dataset};
use super::store::{delete_session, list_sessions, read_events, read_meta};

/// Dónde guarda el plugin las sesiones de un proyecto (`dir` es la carpeta del `.uproject`).
pub fn sessions_root(project_dir: &Path) -> PathBuf {
    let out = project_dir.join("Saved").join("DBVUnrealAI").join("Sessions");
    out
}

/// Filas como mucho en una consulta, por mucho que se pidan (REQ-92: límite estricto).
pub const MAX_QUERY_ROWS: usize = 200;
const DEFAULT_QUERY_ROWS: usize = 60;
/// Elementos como mucho en cada lista de un análisis; el resto se resume en «omitted».
const MAX_LIST: usize = 8;
/// Espera máxima de `session_wait`: la llamada no debe colgar un cliente.
pub const MAX_WAIT_S: f64 = 120.0;

/// Operaciones que entiende `session_analyze`.
pub const ANALYSES: [&str; 8] =
    ["gaps", "frame_budget", "stillness", "oscillation", "jumps", "follow", "transitions", "timeline"];

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": properties, "required": required } })
}

/// Definiciones de las herramientas que trabajan sobre los ficheros.
pub fn specs() -> Vec<Value> {
    let session = json!({ "type": "string", "description": "Session id from session_list, or \"latest\" (default)." });
    vec![
        tool(
            "session_list",
            "List the black-box recordings of this project (newest first): id, status, level, duration, rows, columns and warnings. Free and read-only; it reads files, so it works with the editor closed. A session marked incomplete was never closed by the editor and is still readable.",
            json!({ "limit": { "type": "integer", "description": "How many sessions (default 10)." } }),
            &[],
        ),
        tool(
            "session_analyze",
            "Run analyses on a recorded session and get a compact result (never the raw samples). Always includes `gaps`, the quality of the measurement itself: if it says reliable=false (low FPS, hitches, pauses) do NOT present the other results as certain. Ops: gaps; frame_budget (median/p99/spikes); stillness {signal} (jitter at rest); oscillation {signal} (two-value loops: period and amplitude); jumps {signal} (sudden steps grouped into episodes); follow {a, b} (does a follow b: mean, max deviation, drift); transitions (movement mode/base/pause changes); timeline {every_s, signal}. A `signal` is the prefix of columns like player.x/.y/.z. Correlation is not cause: confirm with an experiment or by reading the code. Read-only.",
            json!({
                "session": session,
                "ops": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "op": { "type": "string", "enum": ANALYSES },
                            "signal": { "type": "string" },
                            "a": { "type": "string" },
                            "b": { "type": "string" },
                            "window_s": { "type": "number" },
                            "min_path_cm": { "type": "number" },
                            "min_amplitude_cm": { "type": "number" },
                            "threshold_cm": { "type": "number" },
                            "tolerance_cm": { "type": "number" },
                            "every_s": { "type": "number" },
                            "budget_ms": { "type": "number" }
                        },
                        "required": ["op"]
                    }
                }
            }),
            &["ops"],
        ),
        tool(
            "session_query",
            "Read a slice of the raw samples of a session, with a strict row limit (max 200). Use it to look at the moments an analysis pointed to, not to read the whole recording. Filter by time range and columns.",
            json!({
                "session": session,
                "columns": { "type": "array", "items": { "type": "string" }, "description": "Column names or signal prefixes (player = player.x/.y/.z…). Default: all." },
                "from_s": { "type": "number" },
                "to_s": { "type": "number" },
                "max_rows": { "type": "integer", "description": "Default 60, hard limit 200. If the range has more rows they are sampled evenly." }
            }),
            &[],
        ),
        tool(
            "session_wait",
            "Wait until the recording finishes or the user adds a mark, up to timeout_s (max 120). It watches the session files, so it never blocks the editor. Returns the status, the new marks and why it returned.",
            json!({
                "session": session,
                "timeout_s": { "type": "number", "description": "Default 30, max 120." },
                "after_marks": { "type": "integer", "description": "Return when there are more marks than this (default: the number of marks now)." }
            }),
            &[],
        ),
        tool(
            "session_delete",
            "Delete a recorded session from disk. Asks for confirmation: call it with confirm=false first to see what would be deleted, then confirm=true only after the user agrees. Refuses a session that is being recorded.",
            json!({ "session": { "type": "string" }, "confirm": { "type": "boolean" } }),
            &["session"],
        ),
    ]
}

fn num(args: &Value, key: &str) -> Option<f64> {
    let out = args.get(key).and_then(Value::as_f64);
    out
}

fn text<'a>(args: &'a Value, key: &str) -> &'a str {
    let out = args.get(key).and_then(Value::as_str).unwrap_or("").trim();
    out
}

/// El id pedido; «latest» o vacío es la sesión más reciente.
fn resolve(root: &Path, args: &Value) -> Result<String, String> {
    let wanted = text(args, "session");
    let all = list_sessions(root);
    let out = if wanted.is_empty() || wanted == "latest" {
        all.first().map(|s| s.id.clone()).ok_or_else(|| "there are no recorded sessions in this project".to_string())
    } else if all.iter().any(|s| s.id == wanted) {
        Ok(wanted.to_string())
    } else {
        Err(format!("no session «{wanted}»; session_list shows the ones that exist"))
    };
    out
}

/// `session_list`.
pub fn list_text(root: &Path, args: &Value) -> Result<String, String> {
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(10).clamp(1, 50) as usize;
    let all = list_sessions(root);
    let shown: Vec<_> = all.iter().take(limit).collect();
    let out = serde_json::to_string(&json!({ "total": all.len(), "sessions": shown })).map_err(|e| e.to_string());
    out
}

// ------------------------------------------------------------------------------------------------- session_analyze

/// Recorta cada lista a `MAX_LIST` elementos y dice cuántos se omitieron: la respuesta cabe siempre en pocos KB.
fn limit_lists(value: &mut Value) {
    match value {
        Value::Array(items) => {
            items.truncate(MAX_LIST);
            items.iter_mut().for_each(limit_lists);
        }
        Value::Object(map) => {
            let mut omitted = Map::new();
            for (key, v) in map.iter_mut() {
                if let Value::Array(items) = v {
                    if items.len() > MAX_LIST && !items.iter().all(Value::is_number) {
                        omitted.insert(format!("{key}_omitted"), json!(items.len() - MAX_LIST));
                    }
                }
                limit_lists(v);
            }
            map.extend(omitted);
        }
        _ => {}
    }
}

fn run_op(ds: &Dataset, evs: &[Value], op: &Value) -> Result<Value, String> {
    let name = text(op, "op");
    let signal = text(op, "signal");
    let need_signal = || if signal.is_empty() { Err(format!("«{name}» needs `signal`")) } else { Ok(signal) };
    let to_value = |v: Result<Value, serde_json::Error>| v.map_err(|e| e.to_string());
    let out = match name {
        "gaps" => to_value(serde_json::to_value(frame::gaps(ds))),
        "frame_budget" => to_value(serde_json::to_value(frame::frame_budget(ds, num(op, "budget_ms").unwrap_or(1000.0 / 60.0)))),
        "stillness" => to_value(serde_json::to_value(motion::stillness(
            ds,
            need_signal()?,
            num(op, "window_s").unwrap_or(0.5),
            num(op, "min_path_cm").unwrap_or(0.05),
        ))),
        "oscillation" => to_value(serde_json::to_value(motion::oscillation(
            ds,
            need_signal()?,
            num(op, "min_amplitude_cm").unwrap_or(0.05),
        ))),
        "jumps" => to_value(serde_json::to_value(motion::jumps(ds, need_signal()?, num(op, "threshold_cm").unwrap_or(10.0)))),
        "follow" => {
            let (a, b) = (text(op, "a"), text(op, "b"));
            match relative::follow(ds, a, b, num(op, "tolerance_cm").unwrap_or(1.0)) {
                Some(f) => to_value(serde_json::to_value(f)),
                None => Err(format!("follow needs two signals with x/y/z columns and data (a=«{a}», b=«{b}»)")),
            }
        }
        "transitions" => to_value(serde_json::to_value(events::transitions(evs))),
        "timeline" => to_value(serde_json::to_value(events::timeline(
            ds,
            evs,
            num(op, "every_s").unwrap_or(10.0),
            if signal.is_empty() { None } else { Some(signal) },
        ))),
        other => Err(format!("unknown op «{other}»; valid: {}", ANALYSES.join(", "))),
    };
    out
}

/// `session_analyze`: `gaps` primero y siempre; después, lo pedido. Una operación que falla no tumba las demás.
pub fn analyze_text(root: &Path, args: &Value) -> Result<String, String> {
    let id = resolve(root, args)?;
    let folder = root.join(&id);
    let mut ds = Dataset::from_session(&folder)?;
    let evs = read_events(&folder);
    let pauses = events::apply_pauses(&mut ds, &evs);
    let ops = args.get("ops").and_then(Value::as_array).cloned().unwrap_or_default();
    if ops.is_empty() {
        return Err("`ops` is empty; ask for at least one analysis (see the tool description)".to_string());
    }
    let mut gaps = serde_json::to_value(frame::gaps(&ds)).map_err(|e| e.to_string())?;
    limit_lists(&mut gaps);
    let results: Vec<Value> = ops
        .iter()
        .filter(|op| text(op, "op") != "gaps")
        .map(|op| match run_op(&ds, &evs, op) {
            Ok(mut v) => {
                limit_lists(&mut v);
                json!({ "op": text(op, "op"), "result": v })
            }
            Err(e) => json!({ "op": text(op, "op"), "error": e }),
        })
        .collect();
    let reliable = gaps.get("reliable").and_then(Value::as_bool).unwrap_or(false);
    let out = serde_json::to_string(&json!({
        "session": id,
        "reliable": reliable,
        "note": if reliable { "measurement quality is fine" } else { "measurement is NOT reliable (see gaps.issues): do not present these results as certain" },
        "paused_spans": pauses.len(),
        "gaps": gaps,
        "results": results,
    }))
    .map_err(|e| e.to_string());
    out
}

// ---------------------------------------------------------------------------------------------------- session_query

/// `session_query`: un tramo de muestras, con un tope estricto de filas.
pub fn query_text(root: &Path, args: &Value) -> Result<String, String> {
    let id = resolve(root, args)?;
    let ds = Dataset::from_session(&root.join(&id))?;
    let wanted: Vec<String> = args
        .get("columns")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let picked: Vec<usize> = (0..ds.names.len())
        .filter(|&c| {
            wanted.is_empty()
                || wanted.iter().any(|w| ds.names[c] == *w || ds.names[c].starts_with(&format!("{w}.")))
        })
        .collect();
    if picked.is_empty() {
        return Err(format!("no column matches; the session has: {}", ds.names.join(", ")));
    }
    let (from, to) = (num(args, "from_s").unwrap_or(f64::MIN), num(args, "to_s").unwrap_or(f64::MAX));
    let rows: Vec<usize> = (0..ds.len()).filter(|&i| ds.times[i] >= from && ds.times[i] <= to).collect();
    let cap = args.get("max_rows").and_then(Value::as_u64).map_or(DEFAULT_QUERY_ROWS, |n| n as usize).clamp(1, MAX_QUERY_ROWS);
    let step = rows.len().div_ceil(cap).max(1);
    let sampled: Vec<Value> = rows
        .iter()
        .step_by(step)
        .take(cap)
        .map(|&i| {
            let mut row = vec![json!(super::analysis::round(ds.times[i], 3))];
            row.extend(picked.iter().map(|&c| {
                let v = ds.cols[c][i];
                if v.is_finite() { json!(super::analysis::round(f64::from(v), 3)) } else { Value::Null }
            }));
            Value::Array(row)
        })
        .collect();
    let mut names = vec!["t".to_string()];
    names.extend(picked.iter().map(|&c| ds.names[c].clone()));
    let out = serde_json::to_string(&json!({
        "session": id,
        "columns": names,
        "rows": sampled,
        "rows_in_range": rows.len(),
        "sampled_every": step,
        "paused_frames_included": ds.valid.iter().filter(|v| !**v).count(),
    }))
    .map_err(|e| e.to_string());
    out
}

// ----------------------------------------------------------------------------------------------------- session_wait

fn marks_of(folder: &Path) -> Vec<Value> {
    let out = read_events(folder).into_iter().filter(|e| e.get("type").and_then(Value::as_str) == Some("mark")).collect();
    out
}

/// `session_wait`: espera mirando los ficheros hasta que la sesión termina o hay marcas nuevas. `poll` es el
/// intervalo de mirada (los tests lo acortan).
pub fn wait_text(root: &Path, args: &Value, poll: Duration) -> Result<String, String> {
    let id = resolve(root, args)?;
    let folder = root.join(&id);
    let timeout = num(args, "timeout_s").unwrap_or(30.0).clamp(0.0, MAX_WAIT_S);
    let after = args.get("after_marks").and_then(Value::as_u64).map_or_else(|| marks_of(&folder).len(), |n| n as usize);
    let started = Instant::now();
    let (reason, marks) = loop {
        let status = read_meta(&folder).get("status").and_then(Value::as_str).unwrap_or("").to_string();
        let marks = marks_of(&folder);
        if status == "finished" {
            break ("finished", marks);
        }
        if marks.len() > after {
            break ("new_mark", marks);
        }
        if started.elapsed().as_secs_f64() >= timeout {
            break ("timeout", marks);
        }
        std::thread::sleep(poll);
    };
    let meta = read_meta(&folder);
    let out = serde_json::to_string(&json!({
        "session": id,
        "returned_because": reason,
        "status": meta.get("status"),
        "reason": meta.get("reason"),
        "duration_s": meta.get("duration_s"),
        "new_marks": marks.iter().skip(after).collect::<Vec<_>>(),
        "marks_total": marks.len(),
    }))
    .map_err(|e| e.to_string());
    out
}

// --------------------------------------------------------------------------------------------------- session_delete

/// `session_delete`: sin `confirm` solo cuenta qué se borraría.
pub fn delete_text(root: &Path, args: &Value) -> Result<String, String> {
    let id = text(args, "session");
    let info = list_sessions(root).into_iter().find(|s| s.id == id).ok_or_else(|| format!("no session «{id}»"))?;
    let confirmed = args.get("confirm").and_then(Value::as_bool).unwrap_or(false);
    let out = if confirmed {
        delete_session(root, id, true).map(|()| format!("Deleted session «{id}» ({} rows, {} bytes).", info.rows, info.bytes))
    } else {
        Ok(format!(
            "Not deleted yet: «{id}» has {} rows, {} bytes, {:.1} s. Ask the user, then call again with confirm=true.",
            info.rows, info.bytes, info.duration_s
        ))
    };
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Sesión sintética en disco con el formato real: 10 s a 20 FPS; se mueve en x desde 5 s y hay un saltito de 40 cm.
    fn write_session(root: &Path, id: &str, status: &str, marks: &[(f64, &str)]) -> PathBuf {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        let n = 200usize;
        let times: Vec<f64> = (0..n).map(|i| i as f64 / 20.0).collect();
        let x: Vec<f32> = times.iter().enumerate().map(|(i, t)| (if *t >= 5.0 { (t - 5.0) * 10.0 } else { 0.0 }) as f32 + if i == 120 { 40.0 } else { 0.0 }).collect();
        let zero = vec![0.0_f32; n];
        let dt = vec![50.0_f32; n];
        let mut body = Vec::new();
        for t in &times {
            body.extend_from_slice(&t.to_le_bytes());
        }
        for col in [&x, &zero, &zero, &dt] {
            for v in col {
                body.extend_from_slice(&v.to_le_bytes());
            }
        }
        let mut bytes = Vec::from(&b"DBVB"[..]);
        bytes.extend_from_slice(&(n as u32).to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&super::super::store::crc32(&body).to_le_bytes());
        bytes.extend_from_slice(&body);
        fs::write(dir.join("frames.bin"), bytes).unwrap();
        fs::write(
            dir.join("meta.json"),
            json!({"status": status, "reason": if status == "finished" { "pie_ended" } else { "" }, "columns": ["p.x", "p.y", "p.z", "frame.dt_ms"], "duration_s": 9.95, "level": "/Game/M"}).to_string(),
        )
        .unwrap();
        let lines: Vec<String> = marks.iter().map(|(t, l)| json!({"t": t, "type": "mark", "label": l}).to_string()).collect();
        fs::write(dir.join("events.jsonl"), lines.join("\n") + "\n").unwrap();
        dir
    }

    fn root_with_one() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        write_session(root.path(), "20261110-100000", "finished", &[(2.0, "tiembla")]);
        root
    }

    #[test]
    fn specs_list_the_five_file_tools_with_valid_schemas() {
        let names: Vec<String> = specs().iter().map(|s| s["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names, ["session_list", "session_analyze", "session_query", "session_wait", "session_delete"]);
        for s in specs() {
            assert_eq!(s["inputSchema"]["type"], "object");
            assert!(s["description"].as_str().unwrap().len() > 60);
        }
    }

    #[test]
    fn list_shows_sessions_newest_first() {
        let root = root_with_one();
        write_session(root.path(), "20261110-110000", "finished", &[]);
        let v: Value = serde_json::from_str(&list_text(root.path(), &json!({})).unwrap()).unwrap();
        assert_eq!(v["total"], 2);
        assert_eq!(v["sessions"][0]["id"], "20261110-110000");
        assert_eq!(v["sessions"][0]["rows"], 200);
    }

    #[test]
    fn analyze_puts_gaps_first_and_finds_the_seeded_jump() {
        let root = root_with_one();
        let args = json!({"ops": [{"op": "jumps", "signal": "p", "threshold_cm": 10}, {"op": "gaps"}, {"op": "frame_budget"}]});
        let text = analyze_text(root.path(), &args).unwrap();
        assert!(text.len() < 2500, "{} bytes", text.len()); // «un análisis completo ocupa menos de unos 2 KB»
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["session"], "20261110-100000");
        assert_eq!(v["reliable"], true);
        assert_eq!(v["results"].as_array().unwrap().len(), 2); // gaps no se repite
        let jumps = &v["results"][0]["result"]["episodes"];
        assert_eq!(jumps.as_array().unwrap().len(), 1);
        let step = jumps[0]["max_step_cm"].as_f64().unwrap();
        assert!((40.0..41.0).contains(&step), "{step}"); // 40 cm del salto más lo que ya avanzaba por fotograma
        assert_eq!(v["results"][1]["result"]["median_ms"], 50.0);
    }

    #[test]
    fn a_bad_op_does_not_sink_the_others_and_unreliable_measurement_is_said() {
        let root = tempfile::tempdir().unwrap();
        let dir = write_session(root.path(), "20261110-100000", "finished", &[]);
        let meta = json!({"status": "finished", "columns": ["p.x", "p.y", "p.z", "frame.dt_ms"]});
        fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
        // Pausa que cubre medio segundo: gaps lo marca como no fiable.
        fs::write(dir.join("events.jsonl"), "{\"t\":2.0,\"type\":\"pause\",\"paused\":true}\n{\"t\":2.5,\"type\":\"pause\",\"paused\":false}\n").unwrap();
        let args = json!({"ops": [{"op": "nada"}, {"op": "stillness"}, {"op": "transitions"}]});
        let v: Value = serde_json::from_str(&analyze_text(root.path(), &args).unwrap()).unwrap();
        assert_eq!(v["reliable"], false);
        assert!(v["note"].as_str().unwrap().contains("NOT reliable"));
        assert_eq!(v["paused_spans"], 1);
        assert!(v["results"][0]["error"].as_str().unwrap().contains("unknown op"));
        assert!(v["results"][1]["error"].as_str().unwrap().contains("needs `signal`"));
        assert_eq!(v["results"][2]["result"]["total"], 2);
    }

    #[test]
    fn analyze_without_ops_or_sessions_explains_what_to_do() {
        let empty = tempfile::tempdir().unwrap();
        assert!(analyze_text(empty.path(), &json!({"ops": [{"op": "gaps"}]})).unwrap_err().contains("no recorded sessions"));
        let root = root_with_one();
        assert!(analyze_text(root.path(), &json!({"ops": []})).unwrap_err().contains("`ops` is empty"));
        assert!(analyze_text(root.path(), &json!({"session": "otra", "ops": [{"op": "gaps"}]})).unwrap_err().contains("session_list"));
    }

    #[test]
    fn lists_in_results_are_capped_with_an_omitted_count() {
        let mut v = json!({"episodes": (0..20).map(|i| json!({"t": i})).collect::<Vec<_>>(), "plain": [1, 2, 3]});
        limit_lists(&mut v);
        assert_eq!(v["episodes"].as_array().unwrap().len(), MAX_LIST);
        assert_eq!(v["episodes_omitted"], 12);
        assert_eq!(v["plain"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn query_respects_the_hard_row_limit_columns_and_range() {
        let root = root_with_one();
        let v: Value = serde_json::from_str(&query_text(root.path(), &json!({"columns": ["p"], "max_rows": 100000})).unwrap()).unwrap();
        assert_eq!(v["rows"].as_array().unwrap().len(), 200); // 200 filas en total: caben justo
        assert_eq!(v["columns"], json!(["t", "p.x", "p.y", "p.z"])); // frame.dt_ms no coincide con «p»
        let r: Value = serde_json::from_str(&query_text(root.path(), &json!({"from_s": 5.0, "to_s": 6.0, "max_rows": 5, "columns": ["p.x"]})).unwrap()).unwrap();
        assert_eq!(r["rows"].as_array().unwrap().len(), 5);
        assert!(r["rows_in_range"].as_u64().unwrap() > 5 && r["sampled_every"].as_u64().unwrap() > 1);
        assert!(query_text(root.path(), &json!({"columns": ["nada"]})).unwrap_err().contains("no column matches"));
    }

    #[test]
    fn wait_returns_at_once_for_a_finished_session_and_times_out_otherwise() {
        let root = root_with_one();
        let done: Value = serde_json::from_str(&wait_text(root.path(), &json!({"timeout_s": 5}), Duration::from_millis(5)).unwrap()).unwrap();
        assert_eq!(done["returned_because"], "finished");
        write_session(root.path(), "20261110-120000", "recording", &[]);
        let t: Value = serde_json::from_str(&wait_text(root.path(), &json!({"timeout_s": 0.05}), Duration::from_millis(5)).unwrap()).unwrap();
        assert_eq!(t["returned_because"], "timeout");
    }

    #[test]
    fn wait_returns_when_the_user_adds_a_mark() {
        let root = tempfile::tempdir().unwrap();
        let dir = write_session(root.path(), "20261110-120000", "recording", &[]);
        let writer = std::thread::spawn({
            let dir = dir.clone();
            move || {
                std::thread::sleep(Duration::from_millis(60));
                fs::write(dir.join("events.jsonl"), "{\"t\":3.0,\"type\":\"mark\",\"label\":\"ahora se atasca\"}\n").unwrap();
            }
        });
        let v: Value = serde_json::from_str(&wait_text(root.path(), &json!({"timeout_s": 5}), Duration::from_millis(10)).unwrap()).unwrap();
        writer.join().unwrap();
        assert_eq!(v["returned_because"], "new_mark");
        assert_eq!(v["new_marks"][0]["label"], "ahora se atasca");
    }

    #[test]
    fn delete_asks_first_and_only_deletes_when_confirmed() {
        let root = root_with_one();
        let msg = delete_text(root.path(), &json!({"session": "20261110-100000"})).unwrap();
        assert!(msg.contains("Not deleted yet") && root.path().join("20261110-100000").exists());
        assert!(delete_text(root.path(), &json!({"session": "20261110-100000", "confirm": true})).unwrap().starts_with("Deleted"));
        assert!(!root.path().join("20261110-100000").exists());
        assert!(delete_text(root.path(), &json!({"session": "../x", "confirm": true})).is_err());
    }
}

#[cfg(test)]
mod real_session {
    use super::*;

    /// Prueba manual con una sesión grabada de verdad por el plugin: `DBV_SESSIONS_ROOT=<…/Saved/DBVUnrealAI/Sessions>
    /// cargo test --lib real_session -- --ignored --nocapture`. Comprueba que lector, análisis y herramientas
    /// entienden lo que escribe Unreal (no solo lo que escriben los tests).
    #[test]
    #[ignore = "necesita una sesión real: variable DBV_SESSIONS_ROOT"]
    fn the_tools_read_a_session_recorded_by_unreal() {
        let root = PathBuf::from(std::env::var("DBV_SESSIONS_ROOT").expect("falta DBV_SESSIONS_ROOT"));
        println!("{}", list_text(&root, &json!({"limit": 3})).unwrap());
        let ops = json!({"ops": [
            {"op": "frame_budget"},
            {"op": "stillness", "signal": "player"},
            {"op": "jumps", "signal": "player"},
            {"op": "oscillation", "signal": "player"},
            {"op": "transitions"},
            {"op": "timeline", "every_s": 5, "signal": "player"}
        ]});
        let text = analyze_text(&root, &ops).unwrap();
        println!("{} bytes\n{text}", text.len());
        let wait = wait_text(&root, &json!({"timeout_s": 1}), Duration::from_millis(50)).unwrap();
        println!("{wait}");
        println!("{}", query_text(&root, &json!({"columns": ["player.z", "frame.dt_ms"], "max_rows": 6})).unwrap());
    }
}
