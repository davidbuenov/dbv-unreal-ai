// =============================================================================
// DBV Unreal AI — Caja negra: eventos de la sesión (`transitions`, `timeline`, pausas)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-93
//
// Formato de `events.jsonl` (una línea JSON por evento, `t` en segundos desde el inicio):
//   {"t":12.4,"type":"mark","label":"se atasca"}
//   {"t":30.1,"type":"movement_mode","from":"Walking","to":"Falling"}
//   {"t":30.1,"type":"movement_base","from":"Cesta","to":"None"}
//   {"t":41.0,"type":"pause","paused":true}      …     {"t":43.5,"type":"pause","paused":false}

use serde::Serialize;
use serde_json::Value;

use super::{median, round, Dataset};

/// Tipos que cuentan como cambio de estado en `transitions`.
const STATE_KINDS: [&str; 3] = ["movement_mode", "movement_base", "pause"];
/// Máximo de transiciones que se listan; el resto solo se cuenta (las respuestas son siempre compactas).
const MAX_LISTED: usize = 50;

fn time_of(ev: &Value) -> Option<f64> {
    let out = ev.get("t").and_then(Value::as_f64).filter(|t| t.is_finite());
    out
}

fn kind_of(ev: &Value) -> &str {
    let out = ev.get("type").and_then(Value::as_str).unwrap_or("");
    out
}

fn text_of(ev: &Value, key: &str) -> String {
    let out = match ev.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    };
    out
}

/// Excluye del análisis los fotogramas con el PIE en pausa. Una pausa sin reanudar llega hasta el final.
/// Devuelve los intervalos `(desde, hasta)` en segundos.
pub fn apply_pauses(ds: &mut Dataset, events: &[Value]) -> Vec<(f64, f64)> {
    let mut spans: Vec<(f64, f64)> = Vec::new();
    let mut open: Option<f64> = None;
    for ev in events.iter().filter(|e| kind_of(e) == "pause") {
        let (Some(t), Some(paused)) = (time_of(ev), ev.get("paused").and_then(Value::as_bool)) else {
            continue;
        };
        match (paused, open) {
            (true, None) => open = Some(t),
            (false, Some(from)) => {
                spans.push((from, t));
                open = None;
            }
            _ => {}
        }
    }
    if let Some(from) = open {
        spans.push((from, f64::INFINITY));
    }
    for (valid, t) in ds.valid.iter_mut().zip(&ds.times) {
        if spans.iter().any(|(from, to)| t >= from && t < to) {
            *valid = false;
        }
    }
    spans
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Transition {
    pub t: f64,
    pub kind: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Transitions {
    pub events: usize,
    pub total: usize,
    /// Cuántas hay de cada tipo.
    pub counts: std::collections::BTreeMap<String, usize>,
    pub listed: Vec<Transition>,
    pub truncated: bool,
}

/// Cambios de estado (modo de movimiento, base y pausa), en orden de tiempo.
pub fn transitions(events: &[Value]) -> Transitions {
    let mut all: Vec<Transition> = events
        .iter()
        .filter(|e| STATE_KINDS.contains(&kind_of(e)))
        .filter_map(|e| {
            let t = time_of(e)?;
            let kind = kind_of(e).to_string();
            let (from, to) = if kind == "pause" {
                let paused = e.get("paused").and_then(Value::as_bool).unwrap_or(false);
                (if paused { "running" } else { "paused" }.to_string(), if paused { "paused" } else { "running" }.to_string())
            } else {
                (text_of(e, "from"), text_of(e, "to"))
            };
            Some(Transition { t: round(t, 3), kind, from, to })
        })
        .collect();
    all.sort_by(|a, b| a.t.total_cmp(&b.t));
    let mut counts = std::collections::BTreeMap::new();
    for tr in &all {
        *counts.entry(tr.kind.clone()).or_insert(0) += 1;
    }
    let total = all.len();
    let truncated = total > MAX_LISTED;
    all.truncate(MAX_LISTED);
    let out = Transitions { events: events.len(), total, counts, listed: all, truncated };
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimelineRow {
    pub t_start: f64,
    pub t_end: f64,
    pub frames: usize,
    pub fps: f64,
    /// Velocidad media de la señal pedida (cm/s) en la ventana; `None` si no se pidió o no hay datos.
    pub speed_cm_s: Option<f64>,
    pub transitions: usize,
    pub marks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Timeline {
    pub every_s: f64,
    pub duration_s: f64,
    pub rows: Vec<TimelineRow>,
}

/// Un resumen cada `every_s` segundos: fotogramas, FPS, velocidad (si se da una señal con `x`, `y`, `z`), transiciones
/// y marcas. Es la visión general que se mira antes de pedir un tramo concreto.
pub fn timeline(ds: &Dataset, events: &[Value], every_s: f64, signal: Option<&str>) -> Timeline {
    let end = ds.times.last().copied().unwrap_or(0.0);
    let mut out = Timeline { every_s, duration_s: round(end, 3), rows: Vec::new() };
    if ds.is_empty() || every_s <= 0.0 {
        return out;
    }
    let pos = signal.and_then(|s| ds.position(s));
    let n_windows = (end / every_s).floor() as usize + 1;
    for w in 0..n_windows {
        let (from, to) = (w as f64 * every_s, (w + 1) as f64 * every_s);
        let idx: Vec<usize> = (0..ds.len()).filter(|&i| ds.times[i] >= from && ds.times[i] < to).collect();
        let mut dts: Vec<f64> = idx
            .iter()
            .filter(|&&i| i > 0 && ds.valid[i] && ds.valid[i - 1])
            .map(|&i| ds.times[i] - ds.times[i - 1])
            .filter(|dt| *dt > 0.0)
            .collect();
        let fps = median(&mut dts).map_or(0.0, |m| 1.0 / m);
        let speed = pos.and_then(|p| window_speed(ds, &p, &idx));
        let in_window = |e: &&Value| time_of(e).is_some_and(|t| t >= from && t < to);
        out.rows.push(TimelineRow {
            t_start: round(from, 3),
            t_end: round(to.min(end), 3),
            frames: idx.len(),
            fps: round(fps, 1),
            speed_cm_s: speed,
            transitions: events.iter().filter(in_window).filter(|e| STATE_KINDS.contains(&kind_of(e))).count(),
            marks: events.iter().filter(in_window).filter(|e| kind_of(e) == "mark").map(|e| text_of(e, "label")).collect(),
        });
    }
    out
}

fn window_speed(ds: &Dataset, pos: &[&[f32]; 3], idx: &[usize]) -> Option<f64> {
    let mut path = 0.0;
    let mut time = 0.0;
    for &i in idx.iter().filter(|&&i| ds.step_ok(i, pos)) {
        let sq: f64 = pos.iter().map(|c| f64::from(c[i] - c[i - 1]).powi(2)).sum();
        path += sq.sqrt();
        time += ds.times[i] - ds.times[i - 1];
    }
    let out = if time > 0.0 { Some(round(path / time, 2)) } else { None };
    out
}

#[cfg(test)]
mod tests {
    use super::super::testing::synth;
    use super::*;
    use serde_json::json;

    #[test]
    fn pauses_exclude_their_frames_and_an_open_pause_runs_to_the_end() {
        let mut ds = synth(10.0, 100, &[("p.x", Box::new(|_, _| 0.0))]);
        let events = vec![
            json!({"t": 2.0, "type": "pause", "paused": true}),
            json!({"t": 3.0, "type": "pause", "paused": false}),
            json!({"t": 8.0, "type": "pause", "paused": true}),
        ];
        let spans = apply_pauses(&mut ds, &events);
        assert_eq!(spans.len(), 2);
        assert_eq!(ds.valid.iter().filter(|v| !**v).count(), 10 + 20); // [2,3) y [8,10)
        assert!(!ds.valid[25] && ds.valid[35]);
    }

    #[test]
    fn malformed_pause_events_are_ignored() {
        let mut ds = synth(10.0, 20, &[("p.x", Box::new(|_, _| 0.0))]);
        let events = vec![json!({"type": "pause", "paused": true}), json!({"t": 1.0, "type": "pause"}), json!(42)];
        assert!(apply_pauses(&mut ds, &events).is_empty());
        assert!(ds.valid.iter().all(|v| *v));
    }

    #[test]
    fn transitions_list_state_changes_in_time_order_with_counts() {
        let events = vec![
            json!({"t": 112.0, "type": "movement_base", "from": "Cesta", "to": null}),
            json!({"t": 30.1, "type": "movement_mode", "from": "Walking", "to": "Falling"}),
            json!({"t": 40.0, "type": "mark", "label": "no cuenta"}),
            json!({"t": 41.0, "type": "pause", "paused": true}),
        ];
        let t = transitions(&events);
        assert_eq!((t.total, t.events, t.truncated), (3, 4, false));
        assert_eq!(t.listed[0].kind, "movement_mode");
        assert_eq!((t.listed[2].t, t.listed[2].to.as_str()), (112.0, "")); // «a None» llega como valor nulo
        assert_eq!((t.listed[1].from.as_str(), t.listed[1].to.as_str()), ("running", "paused"));
        assert_eq!(t.counts["movement_mode"], 1);
    }

    #[test]
    fn transitions_are_capped_but_still_counted() {
        let events: Vec<Value> = (0..80)
            .map(|i| json!({"t": f64::from(i), "type": "movement_mode", "from": "A", "to": "B"}))
            .collect();
        let t = transitions(&events);
        assert_eq!((t.total, t.listed.len(), t.truncated), (80, MAX_LISTED, true));
    }

    #[test]
    fn timeline_summarises_each_window_with_speed_marks_and_transitions() {
        // 20 s a 10 FPS; se mueve a 50 cm/s en x solo de 10 s en adelante.
        let ds = synth(
            10.0,
            200,
            &[
                ("p.x", Box::new(|_, t| if t >= 10.0 { ((t - 10.0) * 50.0) as f32 } else { 0.0 })),
                ("p.y", Box::new(|_, _| 0.0)),
                ("p.z", Box::new(|_, _| 0.0)),
            ],
        );
        let events = vec![
            json!({"t": 12.0, "type": "mark", "label": "empieza a temblar"}),
            json!({"t": 15.0, "type": "movement_mode", "from": "Walking", "to": "Falling"}),
        ];
        let tl = timeline(&ds, &events, 10.0, Some("p"));
        assert_eq!(tl.rows.len(), 2);
        assert_eq!((tl.rows[0].fps, tl.rows[0].speed_cm_s), (10.0, Some(0.0)));
        assert!((tl.rows[1].speed_cm_s.unwrap() - 50.0).abs() < 1.0);
        assert_eq!((tl.rows[0].transitions, tl.rows[1].transitions), (0, 1));
        assert_eq!(tl.rows[1].marks, ["empieza a temblar"]);
    }

    #[test]
    fn timeline_without_a_signal_has_no_speed_and_an_empty_session_has_no_rows() {
        let ds = synth(10.0, 30, &[("p.x", Box::new(|_, _| 0.0))]);
        assert_eq!(timeline(&ds, &[], 1.0, None).rows[0].speed_cm_s, None);
        assert!(timeline(&Dataset::default(), &[], 1.0, None).rows.is_empty());
    }
}
