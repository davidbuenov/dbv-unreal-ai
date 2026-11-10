// =============================================================================
// DBV Unreal AI — Caja negra: herramientas que gobiernan la grabación en el editor
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-92
//
// `session_plan`, `session_start`, `session_status`, `session_mark` y `session_stop`. A diferencia de las de
// `tools.rs`, estas hablan con el editor: ejecutan `dbv_unreal_ai.session.live` por Python y, para arrancar y parar
// el juego, usan la misma ruta que `unreal_playtest` (REQ-82, D71: StartPIE/StopPIE con parada garantizada). Cada
// superficie (MCP propio y asistente) implementa `Editor`; aquí no se sabe cuál es, y los tests usan uno simulado.

use std::path::Path;

use serde_json::{json, Value};

use super::store::prune_sessions;

/// Lo que estas herramientas necesitan del editor. Lo implementa cada superficie.
pub trait Editor {
    /// Ejecuta Python en el editor y devuelve la respuesta JSON de `run_python` (`success`, `stdout`, `error`…).
    fn run_python(&mut self, code: &str) -> Result<String, String>;
    /// Empieza Play en el visor (la misma ruta que `unreal_playtest`).
    fn start_pie(&mut self) -> Result<(), String>;
    /// Para Play. Un error de «no hay partida» no es grave.
    fn stop_pie(&mut self) -> Result<(), String>;
}

/// Argumentos de `StartPIE` según el esquema real de UE 5.8 (`options` es obligatorio): PIE en el visor y sin espera
/// extra. Lo usan `unreal_playtest` y la caja negra, en las dos superficies. @spec REQ-82, REQ-92
pub fn start_pie_args() -> Value {
    json!({ "options": { "bSimulate": false, "playMode": "PlayMode_InViewPort", "warmupSeconds": 0 } })
}

/// Modos de `session_start`. `vr_preview` y `simulate` llegan con la comprobación con visor (S112).
pub const MODES: [&str; 2] = ["pie", "attach"];

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": properties, "required": required } })
}

/// Definiciones de las herramientas que gobiernan la grabación.
pub fn specs() -> Vec<Value> {
    vec![
        tool(
            "session_plan",
            "Propose WHAT to record and WHAT to ask the user to do, from the symptom they describe (\"it trembles\", \"it stutters\", \"it jumps\", \"it does not follow\", \"it gets stuck\"). Free and read-only: nothing is recorded. Show the plan to the user and let them edit it before session_start. Pass `targets` (actors involved) when you know them; without actors it proposes the generic player/performance/events presets.",
            json!({
                "symptom": { "type": "string", "description": "What the user noticed, in their words." },
                "targets": {
                    "type": "array",
                    "items": { "type": "object", "properties": {
                        "actor_label": { "type": "string" }, "actor_class": { "type": "string" }, "socket": { "type": "string" }
                    } }
                },
                "max_seconds": { "type": "number", "description": "Recording limit in seconds (default 120)." }
            }),
            &["symptom"],
        ),
        tool(
            "session_start",
            "Start a black-box recording with a plan the user accepted. It only READS the game (no changes to the project). mode \"pie\" (default) starts Play in the editor and records it; \"attach\" records a Play session that is already running. Only one recording at a time, and not while unreal_playtest is running. The user plays while it records. The chat is BUSY during your turn, so the user cannot type: the app shows a recording panel with «Mark moment» and «Finish» buttons (tell them to use it, never to type \"now\" or \"done\"). Tell the user: click ONCE inside the game viewport so the game gets the keyboard; press F9 INSIDE the game to mark a moment without losing focus; clicking the app panel takes the keyboard away from the game (use the panel mainly for «Finish»). Keep calling session_wait: it returns on every mark and when the recording finishes (the user pressed Finish or closed Play); then call session_stop if it is still recording. Never use per-frame callbacks of your own instead of this.",
            json!({
                "plan": { "type": "object", "description": "The accepted plan: {presets:[…], signals:[…], limits:{…}} (from session_plan)." },
                "mode": { "type": "string", "enum": MODES },
                "name": { "type": "string", "description": "Short label added to the session id (optional)." }
            }),
            &["plan"],
        ),
        tool(
            "session_status",
            "Status of the recording in progress (or the last one): rows recorded, disabled signals, warnings and the measured cost per frame. Free and read-only.",
            json!({}),
            &[],
        ),
        tool(
            "session_mark",
            "Add a mark (a labelled moment) to the recording in progress, for example when the user says \"now\" or \"it just happened\". Analyses can then look around it.",
            json!({ "label": { "type": "string" } }),
            &["label"],
        ),
        tool(
            "session_stop",
            "Stop the recording and (in mode pie) stop Play. Always call it when you are done, even after an error: it closes the recording safely. Returns the final status; analyse it with session_analyze.",
            json!({ "keep_playing": { "type": "boolean", "description": "Do not stop Play (default false)." } }),
            &[],
        ),
    ]
}

fn text<'a>(args: &'a Value, key: &str) -> &'a str {
    let out = args.get(key).and_then(Value::as_str).unwrap_or("").trim();
    out
}

/// Un texto como literal de Python: un literal de cadena JSON es también un literal válido de Python.
fn py_str(s: &str) -> String {
    let out = serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string());
    out
}

/// Ejecuta `code` en el editor y devuelve el JSON de la última línea que imprimió.
fn run_json(editor: &mut dyn Editor, code: &str) -> Result<Value, String> {
    let raw = editor.run_python(code)?;
    let envelope: Value = serde_json::from_str(&raw).map_err(|_| format!("unexpected answer from the editor: {raw}"))?;
    if envelope.get("success").and_then(Value::as_bool) == Some(false) {
        let why = envelope.get("error").and_then(Value::as_str).unwrap_or("unknown error");
        return Err(if why.contains("No module named") {
            format!("the DBV plugin in this project has no recorder (session module missing): prepare the project again in DBV Unreal AI ({why})")
        } else {
            why.to_string()
        });
    }
    let stdout = envelope.get("stdout").and_then(Value::as_str).unwrap_or("");
    let last = stdout.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
    let out = serde_json::from_str(last).map_err(|_| format!("the recorder answered something unexpected: {last}"));
    out
}

// ------------------------------------------------------------------------------------------------------ session_plan

/// Síntoma → presets y qué pedirle al usuario. Palabras en español e inglés.
const RULES: [(&[&str], &[&str], &str); 5] = [
    (
        &["tiembla", "temblor", "vibra", "jitter", "shake", "tremble", "jiggle"],
        &["player", "performance"],
        "Stay still for ~10 s looking at the thing that trembles, then move and stop again.",
    ),
    (
        &["tirones", "lag", "lento", "slow", "fps", "stutter", "hitch", "rendimiento", "performance"],
        &["performance", "gameplay_events"],
        "Play normally for ~30 s and press F9 in the game when it stutters.",
    ),
    (
        &["salta", "teletransporta", "glitch", "pop", "jump", "teleport", "snap"],
        &["player", "performance"],
        "Repeat what makes it jump and press F9 in the game right when you see it.",
    ),
    (
        &["no sigue", "sigue", "follow", "lag behind", "se despega", "detached"],
        &["player", "performance"],
        "Move around (turn, walk, stop) so the follower has to keep up, and mark when it falls behind.",
    ),
    (
        &["atasca", "stuck", "cae", "fall", "suelo", "floor", "atraviesa"],
        &["player", "gameplay_events", "performance"],
        "Walk to the place where it happens and press F9 in the game.",
    ),
];

/// `session_plan`: no graba nada.
pub fn plan_text(args: &Value) -> Result<String, String> {
    let symptom = text(args, "symptom");
    if symptom.is_empty() {
        return Err("describe the symptom the user noticed".to_string());
    }
    let lower = symptom.to_lowercase();
    let mut presets: Vec<&str> = Vec::new();
    let mut asks: Vec<&str> = Vec::new();
    for (words, wanted, ask) in RULES {
        if words.iter().any(|w| lower.contains(w)) {
            asks.push(ask);
            for p in wanted {
                if !presets.contains(p) {
                    presets.push(p);
                }
            }
        }
    }
    let generic = presets.is_empty();
    if generic {
        presets.extend(["player", "performance", "gameplay_events"]);
        asks.push("Play the way that triggers it and press F9 in the game when you notice it.");
    }
    let signals: Vec<Value> = args
        .get("targets")
        .and_then(Value::as_array)
        .map(|ts| ts.iter().filter(|t| t.is_object()).map(target_signal).collect())
        .unwrap_or_default();
    let seconds = args.get("max_seconds").and_then(Value::as_f64).unwrap_or(120.0).clamp(5.0, 600.0);
    let out = serde_json::to_string(&json!({
        "plan": { "presets": presets, "signals": signals, "limits": { "max_seconds": seconds } },
        "ask_the_user": asks,
        "marks": "While recording, the chat is busy with your turn, so the user cannot type: press F9 INSIDE the game to mark a moment without losing the keyboard (the app panel also has «Mark moment» and «Finish», but clicking it takes the focus away from the game). Click once inside the game viewport first so it gets the keyboard. Marks let the analysis look around each moment.",
        "generic": generic,
        "note": if signals.is_empty() { "No actors given: only presets. Add `targets` (actor_label/actor_class, optional socket) to record the objects involved." } else { "Show this plan to the user and let them edit it; session_start counts as their approval to record and analyse (nothing in the project is changed)." },
    }))
    .map_err(|e| e.to_string());
    out
}

fn target_signal(t: &Value) -> Value {
    let mut target = serde_json::Map::new();
    for key in ["actor_label", "actor_class"] {
        if let Some(v) = t.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()) {
            target.insert(key.to_string(), json!(v));
        }
    }
    let mut sig = json!({ "kind": "transform", "target": target });
    if let Some(socket) = t.get("socket").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        sig["socket"] = json!(socket);
    }
    sig
}

// ----------------------------------------------------------------------------------------------- start / status / mark / stop

/// `session_start`.
pub fn start_text(editor: &mut dyn Editor, args: &Value) -> Result<String, String> {
    let mode = if text(args, "mode").is_empty() { "pie" } else { text(args, "mode") };
    if !MODES.contains(&mode) {
        return Err(format!("mode «{mode}» is not available yet; use one of: {}", MODES.join(", ")));
    }
    let plan = args.get("plan").filter(|p| p.is_object()).ok_or("`plan` must be an object (see session_plan)")?;
    let code = format!(
        "from dbv_unreal_ai.session import live\nprint(live.start_recording({}, {}))",
        py_str(&plan.to_string()),
        py_str(text(args, "name"))
    );
    let started = run_json(editor, &code)?;
    if started.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(started.get("error").and_then(Value::as_str).unwrap_or("could not start the recording").to_string());
    }
    if mode == "pie" {
        if let Err(e) = editor.start_pie() {
            let _ = run_json(editor, "from dbv_unreal_ai.session import live\nprint(live.stop_recording())");
            return Err(format!("the recording was armed but Play could not start, so it was closed: {e}"));
        }
    }
    let out = serde_json::to_string(&json!({
        "started": true,
        "mode": mode,
        "session": started.get("id"),
        "columns": started.get("columns"),
        "plan_errors": started.get("errors"),
        "plan_warnings": started.get("warnings"),
        "next": "The user plays now: tell them to click once inside the game, press F9 in the game to mark moments, and use the app panel (Finish) when done. Keep calling session_wait until it returns finished, then session_stop if needed and session_analyze.",
        "mark_key": started.get("mark_key"),
    }))
    .map_err(|e| e.to_string());
    out
}

/// `session_status`.
pub fn status_text(editor: &mut dyn Editor) -> Result<String, String> {
    let v = run_json(editor, "from dbv_unreal_ai.session import live\nprint(live.status())")?;
    Ok(v.to_string())
}

/// `session_mark`.
pub fn mark_text(editor: &mut dyn Editor, args: &Value) -> Result<String, String> {
    let label = text(args, "label");
    if label.is_empty() {
        return Err("a mark needs a label".to_string());
    }
    let code = format!("from dbv_unreal_ai.session import live\nprint(live.mark({}))", py_str(label));
    let v = run_json(editor, &code)?;
    if v.get("success").and_then(Value::as_bool) == Some(true) {
        Ok(format!("Marked «{label}»."))
    } else {
        Err(v.get("error").and_then(Value::as_str).unwrap_or("could not add the mark").to_string())
    }
}

/// `session_stop`: cierra la grabación y, salvo `keep_playing`, para Play. Pasa lo que pase en un paso, intenta el otro.
pub fn stop_text(editor: &mut dyn Editor, root: &Path, args: &Value) -> Result<String, String> {
    let keep_playing = args.get("keep_playing").and_then(Value::as_bool).unwrap_or(false);
    let stopped = run_json(editor, "from dbv_unreal_ai.session import live\nprint(live.stop_recording())");
    let pie = if keep_playing { Ok(()) } else { editor.stop_pie() };
    let pruned = prune_sessions(root, super::DEFAULT_KEEP_SESSIONS);
    let status = stopped?;
    let out = serde_json::to_string(&json!({
        "stopped": true,
        "status": status,
        "play_stopped": !keep_playing && pie.is_ok(),
        "play_stop_note": pie.err(),
        "pruned_old_sessions": pruned,
        "next": "Analyse it with session_analyze (gaps first).",
    }))
    .map_err(|e| e.to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeEditor {
        python: Vec<String>,
        replies: Vec<Result<String, String>>,
        pie_started: bool,
        pie_stopped: bool,
        start_error: Option<String>,
    }

    fn envelope(inner: &Value) -> Result<String, String> {
        Ok(json!({"success": true, "stdout": format!("{inner}\n"), "stderr": ""}).to_string())
    }

    impl Editor for FakeEditor {
        fn run_python(&mut self, code: &str) -> Result<String, String> {
            self.python.push(code.to_string());
            if self.replies.is_empty() { envelope(&json!({"success": true})) } else { self.replies.remove(0) }
        }
        fn start_pie(&mut self) -> Result<(), String> {
            self.pie_started = self.start_error.is_none();
            self.start_error.clone().map_or(Ok(()), Err)
        }
        fn stop_pie(&mut self) -> Result<(), String> {
            self.pie_stopped = true;
            Ok(())
        }
    }

    #[test]
    fn specs_list_the_five_editor_tools() {
        let names: Vec<String> = specs().iter().map(|s| s["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names, ["session_plan", "session_start", "session_status", "session_mark", "session_stop"]);
    }

    #[test]
    fn plan_picks_presets_from_the_symptom_in_spanish_and_english() {
        let v: Value = serde_json::from_str(&plan_text(&json!({"symptom": "La consola tiembla y a veces va a tirones"})).unwrap()).unwrap();
        assert_eq!(v["plan"]["presets"], json!(["player", "performance", "gameplay_events"]));
        assert_eq!(v["generic"], false);
        assert_eq!(v["ask_the_user"].as_array().unwrap().len(), 2);
        let e: Value = serde_json::from_str(&plan_text(&json!({"symptom": "it keeps falling through the floor"})).unwrap()).unwrap();
        assert!(e["plan"]["presets"].as_array().unwrap().contains(&json!("gameplay_events")));
    }

    #[test]
    fn plan_without_a_known_symptom_is_generic_and_targets_become_signals() {
        let args = json!({"symptom": "algo raro", "targets": [{"actor_label": "Grua", "socket": "cesta"}, "no es un objeto"], "max_seconds": 99999});
        let v: Value = serde_json::from_str(&plan_text(&args).unwrap()).unwrap();
        assert_eq!(v["generic"], true);
        assert_eq!(v["plan"]["signals"].as_array().unwrap().len(), 1);
        assert_eq!(v["plan"]["signals"][0]["target"]["actor_label"], "Grua");
        assert_eq!(v["plan"]["signals"][0]["socket"], "cesta");
        assert_eq!(v["plan"]["limits"]["max_seconds"], 600.0);
        assert!(plan_text(&json!({"symptom": "  "})).is_err());
    }

    #[test]
    fn start_arms_the_recorder_then_starts_play_and_passes_the_plan_safely() {
        let mut ed = FakeEditor::default();
        ed.replies.push(envelope(&json!({"success": true, "id": "20261110-1", "columns": ["player.x"], "errors": [], "warnings": []})));
        let args = json!({"plan": {"presets": ["player"], "signals": [{"kind": "variable", "target": {"actor_label": "A\"; import os #"}, "name": "x"}]}, "name": "prueba"});
        let v: Value = serde_json::from_str(&start_text(&mut ed, &args).unwrap()).unwrap();
        assert_eq!((v["started"].as_bool(), v["mode"].as_str(), v["session"].as_str()), (Some(true), Some("pie"), Some("20261110-1")));
        assert!(ed.pie_started);
        // El plan viaja como literal de cadena JSON: las comillas del actor no rompen el código.
        let code = &ed.python[0];
        assert!(code.contains("live.start_recording(\"{") && code.contains("\\\\\\\"; import os #"), "{code}");
    }

    #[test]
    fn attach_does_not_start_play_and_unavailable_modes_are_refused() {
        let mut ed = FakeEditor::default();
        ed.replies.push(envelope(&json!({"success": true, "id": "s"})));
        start_text(&mut ed, &json!({"plan": {}, "mode": "attach"})).unwrap();
        assert!(!ed.pie_started);
        assert!(start_text(&mut ed, &json!({"plan": {}, "mode": "vr_preview"})).unwrap_err().contains("not available yet"));
        assert!(start_text(&mut ed, &json!({"mode": "pie"})).unwrap_err().contains("`plan` must be an object"));
    }

    #[test]
    fn if_play_cannot_start_the_armed_recording_is_closed() {
        let mut ed = FakeEditor { start_error: Some("ya hay una partida".into()), ..Default::default() };
        ed.replies.push(envelope(&json!({"success": true, "id": "s"})));
        let err = start_text(&mut ed, &json!({"plan": {}})).unwrap_err();
        assert!(err.contains("was closed"));
        assert!(ed.python.last().unwrap().contains("stop_recording"));
    }

    #[test]
    fn a_busy_recorder_and_a_missing_module_give_clear_errors() {
        let mut ed = FakeEditor::default();
        ed.replies.push(envelope(&json!({"success": false, "error": "hay un playtest en marcha"})));
        assert!(start_text(&mut ed, &json!({"plan": {}})).unwrap_err().contains("playtest"));
        let mut ed2 = FakeEditor::default();
        ed2.replies.push(Ok(json!({"success": false, "error": "ModuleNotFoundError: No module named 'dbv_unreal_ai.session'"}).to_string()));
        assert!(status_text(&mut ed2).unwrap_err().contains("prepare the project again"));
    }

    #[test]
    fn mark_needs_a_label_and_quotes_it_safely() {
        let mut ed = FakeEditor::default();
        assert!(mark_text(&mut ed, &json!({"label": ""})).is_err());
        assert_eq!(mark_text(&mut ed, &json!({"label": "se \"atasca\""})).unwrap(), "Marked «se \"atasca\"».");
        assert!(ed.python[0].contains("live.mark(\"se \\\"atasca\\\"\")"));
    }

    #[test]
    fn stop_closes_the_recording_stops_play_and_prunes_old_sessions() {
        let root = tempfile::tempdir().unwrap();
        for i in 0..23 {
            let d = root.path().join(format!("20261110-1000{i:02}"));
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("meta.json"), r#"{"status":"finished"}"#).unwrap();
        }
        let mut ed = FakeEditor::default();
        ed.replies.push(envelope(&json!({"active": false, "status": "finished", "reason": "stopped"})));
        let v: Value = serde_json::from_str(&stop_text(&mut ed, root.path(), &json!({})).unwrap()).unwrap();
        assert!(ed.pie_stopped);
        assert_eq!((v["stopped"].as_bool(), v["play_stopped"].as_bool()), (Some(true), Some(true)));
        assert_eq!(v["pruned_old_sessions"].as_array().unwrap().len(), 3); // quedan las últimas 20
    }

    #[test]
    fn stop_still_stops_play_when_closing_the_recording_fails_and_keep_playing_is_respected() {
        let root = tempfile::tempdir().unwrap();
        let mut ed = FakeEditor::default();
        ed.replies.push(Err("sin conexión".into()));
        assert!(stop_text(&mut ed, root.path(), &json!({})).is_err());
        assert!(ed.pie_stopped); // se intenta parar Play aunque falle lo otro
        let mut keep = FakeEditor::default();
        stop_text(&mut keep, root.path(), &json!({"keep_playing": true})).unwrap();
        assert!(!keep.pie_stopped);
    }
}
