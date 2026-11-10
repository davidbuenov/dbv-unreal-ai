// =============================================================================
// DBV Unreal AI — Caja negra: movimiento (`stillness`, `oscillation`, `jumps`)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-93
//
// Los tres trabajan sobre la posición (cm) de una señal. «Parado» se deduce de la propia señal, no de las entradas:
// así no se repite el error de la variable sin grabar (REQ-93, reglas para interpretar).

use serde::Serialize;

use super::{round, Dataset};

/// Recorrido por debajo del cual una ventana cuenta como quieta (cm): ruido numérico, no movimiento.
pub const STILL_EPS_CM: f64 = 0.01;

/// Las columnas de posición de una señal: `x`, `y`, `z` si están las tres; si no, la columna con ese nombre.
fn axes<'a>(ds: &'a Dataset, signal: &str) -> Vec<(String, &'a [f32])> {
    let out = match ds.position(signal) {
        Some([x, y, z]) => vec![("x".to_string(), x), ("y".to_string(), y), ("z".to_string(), z)],
        None => ds.col(signal).map(|c| vec![(signal.to_string(), c)]).unwrap_or_default(),
    };
    out
}

/// Distancia entre el fotograma `i - 1` e `i` en las columnas dadas; `None` si el incremento no es fiable.
fn step_len(ds: &Dataset, cols: &[&[f32]], i: usize) -> Option<f64> {
    let out = if ds.step_ok(i, cols) {
        let sq: f64 = cols.iter().map(|c| f64::from(c[i] - c[i - 1]).powi(2)).sum();
        Some(sq.sqrt())
    } else {
        None
    };
    out
}

// ---------------------------------------------------------------------------------------------------- stillness

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Window {
    pub t_start: f64,
    pub t_end: f64,
    pub path_cm: f64,
    pub net_cm: f64,
    pub reversals: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stillness {
    pub samples: usize,
    pub used: Vec<String>,
    pub window_s: f64,
    pub windows: usize,
    /// Ventanas sin movimiento (recorrido bajo `STILL_EPS_CM`).
    pub still_windows: usize,
    /// Ventanas con temblor: recorrido apreciable, desplazamiento neto casi nulo y cambios de sentido.
    pub jitter_windows: usize,
    pub moving_windows: usize,
    /// Mayor recorrido dentro de una ventana quieta: debería ser ~0.
    pub max_still_path_cm: f64,
    pub worst_jitter: Option<Window>,
}

/// Temblor en reposo. Por ventanas de `window_s`: recorrido total frente a desplazamiento neto y cambios de sentido.
/// `min_path_cm` es el recorrido mínimo para hablar de temblor.
pub fn stillness(ds: &Dataset, signal: &str, window_s: f64, min_path_cm: f64) -> Stillness {
    let ax = axes(ds, signal);
    let cols: Vec<&[f32]> = ax.iter().map(|(_, c)| *c).collect();
    let mut out = Stillness {
        samples: ds.len(),
        used: ax.iter().map(|(n, _)| format!("{signal}.{n}")).collect(),
        window_s,
        windows: 0,
        still_windows: 0,
        jitter_windows: 0,
        moving_windows: 0,
        max_still_path_cm: 0.0,
        worst_jitter: None,
    };
    if cols.is_empty() || ds.is_empty() || window_s <= 0.0 {
        return out;
    }
    let mut start = 0usize;
    while start < ds.len() {
        let t0 = ds.times[start];
        let mut end = start;
        while end + 1 < ds.len() && ds.times[end + 1] - t0 < window_s {
            end += 1;
        }
        if let Some(w) = window_stats(ds, &cols, start, end) {
            out.windows += 1;
            if w.path_cm <= STILL_EPS_CM {
                out.still_windows += 1;
                out.max_still_path_cm = out.max_still_path_cm.max(w.path_cm);
            } else if w.path_cm >= min_path_cm && w.net_cm <= 0.3 * w.path_cm && w.reversals >= 2 {
                out.jitter_windows += 1;
                if out.worst_jitter.as_ref().is_none_or(|b| w.path_cm > b.path_cm) {
                    out.worst_jitter = Some(w);
                }
            } else {
                out.moving_windows += 1;
            }
        }
        start = end + 1;
    }
    out.max_still_path_cm = round(out.max_still_path_cm, 4);
    out
}

fn window_stats(ds: &Dataset, cols: &[&[f32]], start: usize, end: usize) -> Option<Window> {
    let mut path = 0.0;
    let mut steps = 0usize;
    let mut first: Option<usize> = None;
    let mut last = start;
    for i in (start + 1)..=end {
        if let Some(len) = step_len(ds, cols, i) {
            path += len;
            steps += 1;
            first.get_or_insert(i - 1);
            last = i;
        }
    }
    let out = match (steps >= 3, first) {
        (true, Some(f)) => {
            let net: f64 = cols.iter().map(|c| f64::from(c[last] - c[f]).powi(2)).sum::<f64>().sqrt();
            let reversals = cols.iter().map(|c| sign_changes(ds, c, f, last)).sum();
            Some(Window {
                t_start: round(ds.times[start], 3),
                t_end: round(ds.times[end], 3),
                path_cm: round(path, 4),
                net_cm: round(net, 4),
                reversals,
            })
        }
        _ => None,
    };
    out
}

/// Cambios de sentido del incremento de una columna entre dos fotogramas (ignora incrementos nulos).
fn sign_changes(ds: &Dataset, col: &[f32], from: usize, to: usize) -> usize {
    let mut prev = 0.0_f32;
    let mut count = 0usize;
    for i in (from + 1)..=to {
        if !ds.step_ok(i, &[col]) {
            continue;
        }
        let d = col[i] - col[i - 1];
        if d != 0.0 {
            if prev != 0.0 && (d > 0.0) != (prev > 0.0) {
                count += 1;
            }
            prev = d;
        }
    }
    count
}

// ------------------------------------------------------------------------------------------------- oscillation

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OscillationEpisode {
    pub axis: String,
    pub t_start: f64,
    pub t_end: f64,
    pub steps: usize,
    /// Media del salto por fotograma (cm): la amplitud del vaivén.
    pub amplitude_cm: f64,
    /// Fotogramas que dura un ciclo completo (2 = sube y baja en fotogramas consecutivos).
    pub period_frames: u32,
    pub period_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Oscillation {
    pub samples: usize,
    pub used: Vec<String>,
    pub min_amplitude_cm: f64,
    pub episodes: Vec<OscillationEpisode>,
    pub oscillating_s: f64,
}

/// Mínimo de pasos seguidos que alternan de signo para hablar de bucle.
const MIN_RUN: usize = 6;

/// Bucles de dos valores: el incremento cambia de signo en fotogramas consecutivos con un salto de al menos
/// `min_amplitude_cm`. Mide período y amplitud. (Los vaivenes lentos, de varios fotogramas por semiciclo, no se
/// buscan aquí: son movimiento, no un bucle entre dos valores.)
pub fn oscillation(ds: &Dataset, signal: &str, min_amplitude_cm: f64) -> Oscillation {
    let ax = axes(ds, signal);
    let mut out = Oscillation {
        samples: ds.len(),
        used: ax.iter().map(|(n, _)| format!("{signal}.{n}")).collect(),
        min_amplitude_cm,
        episodes: Vec::new(),
        oscillating_s: 0.0,
    };
    for (name, col) in &ax {
        out.episodes.extend(episodes_of(ds, name, col, min_amplitude_cm));
    }
    out.oscillating_s = round(out.episodes.iter().map(|e| e.t_end - e.t_start).sum::<f64>() + 0.0, 3); // «+ 0.0»: la suma vacía es -0.0
    out
}

fn episodes_of(ds: &Dataset, axis: &str, col: &[f32], min_amp: f64) -> Vec<OscillationEpisode> {
    let mut found = Vec::new();
    let mut run: Vec<usize> = Vec::new(); // fotogramas cuyo incremento alterna con el anterior
    let mut prev: Option<f32> = None;
    let flush = |run: &mut Vec<usize>, found: &mut Vec<OscillationEpisode>| {
        if run.len() >= MIN_RUN {
            found.push(episode(ds, axis, col, run));
        }
        run.clear();
    };
    for i in 1..ds.len() {
        let d = if ds.step_ok(i, &[col]) { Some(col[i] - col[i - 1]) } else { None };
        match d {
            Some(d) if f64::from(d.abs()) >= min_amp => {
                let alternates = prev.is_some_and(|p| (p > 0.0) != (d > 0.0));
                if alternates {
                    if run.is_empty() {
                        run.push(i - 1);
                    }
                    run.push(i);
                } else {
                    flush(&mut run, &mut found);
                }
                prev = Some(d);
            }
            _ => {
                flush(&mut run, &mut found);
                prev = None;
            }
        }
    }
    flush(&mut run, &mut found);
    found
}

fn episode(ds: &Dataset, axis: &str, col: &[f32], run: &[usize]) -> OscillationEpisode {
    let steps = run.len();
    let amplitude = run.iter().map(|&i| f64::from((col[i] - col[i - 1]).abs())).sum::<f64>() / steps as f64;
    let (t0, t1) = (ds.times[run[0]], ds.times[run[steps - 1]]);
    let dt = if steps > 1 { (t1 - t0) / (steps - 1) as f64 } else { 0.0 };
    OscillationEpisode {
        axis: axis.to_string(),
        t_start: round(t0, 3),
        t_end: round(t1, 3),
        steps,
        amplitude_cm: round(amplitude, 3),
        period_frames: 2,
        period_s: round(2.0 * dt, 4),
    }
}

// ------------------------------------------------------------------------------------------------------- jumps

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JumpEpisode {
    pub t_start: f64,
    pub t_end: f64,
    pub duration_s: f64,
    /// Saltos de más de `threshold_cm` entre fotogramas que forman el episodio.
    pub jumps: usize,
    pub max_step_cm: f64,
    /// Desplazamiento entre antes del primer salto y después del último.
    pub net_cm: f64,
    /// El valor vuelve a donde estaba (desplazamiento neto bajo): un saltito y no un teletransporte.
    pub returned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Jumps {
    pub samples: usize,
    pub used: Vec<String>,
    pub threshold_cm: f64,
    pub episodes: Vec<JumpEpisode>,
}

/// Dos saltos separados menos de esto (s) son el mismo episodio.
const MERGE_S: f64 = 0.25;

/// Saltos puntuales: el cambio entre fotogramas supera `threshold_cm`; los cercanos se agrupan en episodios con duración.
pub fn jumps(ds: &Dataset, signal: &str, threshold_cm: f64) -> Jumps {
    let ax = axes(ds, signal);
    let cols: Vec<&[f32]> = ax.iter().map(|(_, c)| *c).collect();
    let mut out = Jumps {
        samples: ds.len(),
        used: ax.iter().map(|(n, _)| format!("{signal}.{n}")).collect(),
        threshold_cm,
        episodes: Vec::new(),
    };
    let events: Vec<(usize, f64)> = (1..ds.len())
        .filter_map(|i| step_len(ds, &cols, i).filter(|len| *len > threshold_cm).map(|len| (i, len)))
        .collect();
    let mut group: Vec<(usize, f64)> = Vec::new();
    for ev in events {
        if group.last().is_some_and(|(prev, _)| ds.times[ev.0] - ds.times[*prev] > MERGE_S) {
            out.episodes.push(jump_episode(ds, &cols, &group));
            group.clear();
        }
        group.push(ev);
    }
    if !group.is_empty() {
        out.episodes.push(jump_episode(ds, &cols, &group));
    }
    out
}

fn jump_episode(ds: &Dataset, cols: &[&[f32]], group: &[(usize, f64)]) -> JumpEpisode {
    let (first, last) = (group[0].0, group[group.len() - 1].0);
    let max = group.iter().map(|(_, len)| *len).fold(0.0_f64, f64::max);
    let net: f64 = cols.iter().map(|c| f64::from(c[last] - c[first - 1]).powi(2)).sum::<f64>().sqrt();
    JumpEpisode {
        t_start: round(ds.times[first], 3),
        t_end: round(ds.times[last], 3),
        duration_s: round(ds.times[last] - ds.times[first], 3),
        jumps: group.len(),
        max_step_cm: round(max, 2),
        net_cm: round(net, 2),
        returned: net < 0.25 * max,
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::synth;
    use super::*;

    use super::super::testing::Gen;

    fn pos(x: Gen, y: Gen, z: Gen) -> Dataset {
        synth(72.0, 720, &[("p.x", x), ("p.y", y), ("p.z", z)])
    }

    fn zero() -> Gen {
        Box::new(|_, _| 0.0)
    }

    // ---------------------------------------------------------------- stillness
    #[test]
    fn a_perfectly_still_signal_has_only_still_windows_with_zero_path() {
        let ds = pos(zero(), zero(), Box::new(|_, _| 91.65));
        let s = stillness(&ds, "p", 0.5, 0.05);
        assert_eq!(s.still_windows, s.windows);
        assert!(s.windows >= 18);
        assert_eq!((s.jitter_windows, s.moving_windows, s.max_still_path_cm), (0, 0, 0.0));
        assert!(s.worst_jitter.is_none());
    }

    #[test]
    fn a_trembling_signal_is_found_as_jitter_with_its_window() {
        // 0,6 cm de temblor entre fotogramas en x durante los segundos 3..6; el resto quieto.
        let ds = pos(
            Box::new(|i, t| if (3.0..6.0).contains(&t) { if i % 2 == 0 { 0.0 } else { 0.6 } } else { 0.0 }),
            zero(),
            zero(),
        );
        let s = stillness(&ds, "p", 0.5, 0.05);
        assert!(s.jitter_windows >= 5, "{s:?}");
        assert!(s.still_windows >= 10);
        let w = s.worst_jitter.unwrap();
        assert!(w.t_start >= 2.9 && w.t_end <= 6.1, "{w:?}");
        assert!(w.path_cm > 10.0 && w.net_cm < 1.0 && w.reversals > 10);
    }

    #[test]
    fn steady_movement_is_moving_not_jitter() {
        let ds = pos(Box::new(|_, t| (t * 100.0) as f32), zero(), zero());
        let s = stillness(&ds, "p", 0.5, 0.05);
        assert_eq!((s.still_windows, s.jitter_windows), (0, 0));
        assert!(s.moving_windows > 0);
    }

    #[test]
    fn stillness_ignores_nan_and_paused_frames() {
        let mut ds = pos(Box::new(|i, _| if i == 100 { f32::NAN } else { 5.0 }), zero(), zero());
        for v in ds.valid.iter_mut().skip(200).take(50) {
            *v = false;
        }
        let s = stillness(&ds, "p", 0.5, 0.05);
        assert_eq!((s.jitter_windows, s.moving_windows), (0, 0));
    }

    #[test]
    fn unknown_signal_returns_an_empty_result() {
        let ds = pos(zero(), zero(), zero());
        let s = stillness(&ds, "nada", 0.5, 0.05);
        assert_eq!((s.windows, s.used.len()), (0, 0));
    }

    // -------------------------------------------------------------- oscillation
    #[test]
    fn a_two_value_loop_is_found_with_period_two_and_its_amplitude() {
        // Consola: 1,7 cm de vaivén entre dos valores durante 2 s.
        let ds = pos(
            Box::new(|i, t| if (4.0..6.0).contains(&t) && i % 2 == 0 { 1.7 } else { 0.0 }),
            zero(),
            zero(),
        );
        let o = oscillation(&ds, "p", 0.05);
        assert_eq!(o.episodes.len(), 1, "{o:?}");
        let e = &o.episodes[0];
        assert_eq!((e.axis.as_str(), e.period_frames), ("x", 2));
        assert!((e.amplitude_cm - 1.7).abs() < 0.01 && (e.period_s - 2.0 / 72.0).abs() < 1e-3);
        assert!(e.t_start >= 3.9 && e.t_end <= 6.1);
        assert!((o.oscillating_s - 2.0).abs() < 0.1);
    }

    #[test]
    fn a_clean_session_has_no_oscillation() {
        let ds = pos(Box::new(|_, t| (t * 3.0) as f32), zero(), Box::new(|_, t| (t * 0.5).sin() as f32 * 40.0));
        assert!(oscillation(&ds, "p", 0.05).episodes.is_empty());
    }

    #[test]
    fn oscillation_below_the_minimum_amplitude_is_ignored() {
        let ds = pos(Box::new(|i, _| if i % 2 == 0 { 0.0 } else { 0.01 }), zero(), zero());
        assert!(oscillation(&ds, "p", 0.05).episodes.is_empty());
    }

    // -------------------------------------------------------------------- jumps
    #[test]
    fn a_31_cm_blip_of_0_18_s_is_one_episode_that_returns() {
        // +31 cm en el fotograma 200 y de vuelta 13 fotogramas (0,18 s) después.
        let ds = pos(Box::new(|i, _| if (200..213).contains(&i) { 31.0 } else { 0.0 }), zero(), zero());
        let j = jumps(&ds, "p", 10.0);
        assert_eq!(j.episodes.len(), 1, "{j:?}");
        let e = &j.episodes[0];
        assert_eq!((e.jumps, e.max_step_cm), (2, 31.0));
        assert!((e.duration_s - 0.18).abs() < 0.01, "{}", e.duration_s);
        assert!(e.returned && e.net_cm < 0.01);
    }

    #[test]
    fn a_teleport_does_not_return() {
        let ds = pos(Box::new(|i, _| if i >= 300 { 500.0 } else { 0.0 }), zero(), zero());
        let e = &jumps(&ds, "p", 10.0).episodes[0];
        assert_eq!((e.jumps, e.returned), (1, false));
        assert_eq!(e.net_cm, 500.0);
    }

    #[test]
    fn distant_jumps_are_separate_episodes_and_smooth_movement_is_not_a_jump() {
        let ds = pos(Box::new(|i, _| if i == 100 || i == 400 { 50.0 } else { 0.0 }), zero(), zero());
        assert_eq!(jumps(&ds, "p", 10.0).episodes.len(), 2); // cada pico: salida y regreso, dentro de 0,25 s
        let smooth = pos(Box::new(|_, t| (t * 20.0) as f32), zero(), zero());
        assert!(jumps(&smooth, "p", 10.0).episodes.is_empty());
    }

    #[test]
    fn jumps_do_not_count_a_step_across_a_pause() {
        let mut ds = pos(Box::new(|i, _| if i >= 300 { 80.0 } else { 0.0 }), zero(), zero());
        ds.valid[300] = false; // la pausa esconde el salto: no se inventa uno
        assert!(jumps(&ds, "p", 10.0).episodes.is_empty());
    }
}
