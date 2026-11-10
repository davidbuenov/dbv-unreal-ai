// =============================================================================
// DBV Unreal AI — Caja negra: calidad de la medición (`gaps`) y rendimiento (`frame_budget`)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-93

use serde::Serialize;

use super::{median, percentile, round, Dataset};

/// Por debajo de esto la medición no es fiable: el editor va tan lento que cada fotograma salta mucho.
pub const LOW_FPS: f64 = 20.0;
/// Un fotograma de más de 100 ms cuenta como lento (el mismo umbral que usa la grabadora).
pub const SLOW_DT_S: f64 = 0.1;
/// Si más de esta fracción de los fotogramas es lenta, se avisa.
pub const SLOW_FRACTION: f64 = 0.2;
/// Un tramo seguido de fotogramas lentos que dure esto (s) o más invalida la medición, aunque sean pocos fotogramas:
/// 10 s a 3 FPS son unos 30 fotogramas entre miles (visto el 2026-11-10 con el editor sin foco), pero 10 s sin medir.
pub const STRETCH_MIN_S: f64 = 1.0;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Gaps {
    pub samples: usize,
    pub fps_median: f64,
    pub slow_frames: usize,
    pub slow_fraction: f64,
    pub longest_gap_s: f64,
    /// Fracción del TIEMPO (no de los fotogramas) que se fue en fotogramas lentos.
    pub slow_time_fraction: f64,
    /// Tramos seguidos de fotogramas lentos de `STRETCH_MIN_S` o más (los 5 más largos).
    pub stretches: Vec<Stretch>,
    /// Fotogramas excluidos del análisis (PIE en pausa).
    pub excluded_frames: usize,
    pub issues: Vec<String>,
    /// `false` si hay algún problema: el resto de resultados no deben presentarse como fiables.
    pub reliable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stretch {
    pub t_start: f64,
    pub t_end: f64,
    pub fps: f64,
}

/// Tramos de fotogramas lentos consecutivos (válidos) que duran `STRETCH_MIN_S` o más, del más largo al más corto.
fn slow_stretches(ds: &Dataset) -> Vec<Stretch> {
    let mut found: Vec<Stretch> = Vec::new();
    let mut run: Option<(usize, usize)> = None; // (primer fotograma anterior, último fotograma)
    let close = |run: &mut Option<(usize, usize)>, found: &mut Vec<Stretch>| {
        if let Some((from, to)) = run.take() {
            let span = ds.times[to] - ds.times[from];
            if span >= STRETCH_MIN_S {
                found.push(Stretch { t_start: round(ds.times[from], 3), t_end: round(ds.times[to], 3), fps: round((to - from) as f64 / span, 1) });
            }
        }
    };
    for i in 1..ds.len() {
        let slow = ds.valid[i] && ds.valid[i - 1] && ds.times[i] - ds.times[i - 1] > SLOW_DT_S;
        match (slow, run) {
            (true, None) => run = Some((i - 1, i)),
            (true, Some((from, _))) => run = Some((from, i)),
            (false, _) => close(&mut run, &mut found),
        }
    }
    close(&mut run, &mut found);
    found.sort_by(|a, b| (b.t_end - b.t_start).total_cmp(&(a.t_end - a.t_start)));
    found.truncate(5);
    found
}

/// Intervalos entre fotogramas contiguos y válidos, en segundos.
fn deltas(ds: &Dataset) -> Vec<f64> {
    let out = (1..ds.len())
        .filter(|&i| ds.valid[i] && ds.valid[i - 1])
        .map(|i| ds.times[i] - ds.times[i - 1])
        .filter(|dt| *dt > 0.0)
        .collect();
    out
}

/// Calidad de la propia medición: fotogramas perdidos, FPS bajos y pausas. Sin datos suficientes no afirma nada.
pub fn gaps(ds: &Dataset) -> Gaps {
    let dts = deltas(ds);
    let slow = dts.iter().filter(|dt| **dt > SLOW_DT_S).count();
    let longest = dts.iter().copied().fold(0.0_f64, f64::max);
    let mut sorted = dts.clone();
    let fps = median(&mut sorted).map_or(0.0, |m| 1.0 / m);
    let fraction = if dts.is_empty() { 0.0 } else { slow as f64 / dts.len() as f64 };
    let excluded = ds.valid.iter().filter(|v| !**v).count();
    let total_time: f64 = dts.iter().sum();
    let slow_time: f64 = dts.iter().filter(|dt| **dt > SLOW_DT_S).sum();
    let slow_time_fraction = if total_time > 0.0 { slow_time / total_time } else { 0.0 };
    let stretches = slow_stretches(ds);

    let mut issues = Vec::new();
    if dts.len() < 2 {
        issues.push("hay menos de 2 intervalos: no se puede medir la calidad".to_string());
    }
    if !dts.is_empty() && fps < LOW_FPS {
        issues.push(format!("FPS medianos bajos ({fps:.1} < {LOW_FPS}): los incrementos por fotograma son enormes"));
    }
    if fraction > SLOW_FRACTION {
        issues.push(format!("{:.0} % de los fotogramas duró más de 100 ms", fraction * 100.0));
    }
    if let Some(worst) = stretches.first() {
        issues.push(format!(
            "{} tramo(s) de FPS bajos de más de {STRETCH_MIN_S:.0} s (el peor, de {:.1} s a {:.1} s, a {} FPS): ahí no se midió; suele ser el editor sin foco",
            stretches.len(),
            worst.t_start,
            worst.t_end,
            worst.fps
        ));
    }
    if excluded > 0 {
        issues.push(format!("{excluded} fotogramas excluidos (PIE en pausa)"));
    }
    let out = Gaps {
        samples: dts.len(),
        fps_median: round(fps, 1),
        slow_frames: slow,
        slow_fraction: round(fraction, 3),
        longest_gap_s: round(longest, 3),
        slow_time_fraction: round(slow_time_fraction, 3),
        stretches,
        excluded_frames: excluded,
        reliable: issues.is_empty(),
        issues,
    };
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spike {
    pub t: f64,
    pub ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrameBudget {
    pub samples: usize,
    /// De dónde salen los tiempos: la columna `frame.dt_ms` o la resta de tiempos de fotograma.
    pub source: String,
    pub median_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub budget_ms: f64,
    /// Fracción de fotogramas por encima del presupuesto.
    pub over_budget: f64,
    /// Fotogramas de más del doble de la mediana (los 5 peores).
    pub spikes: usize,
    pub worst: Vec<Spike>,
}

/// Rendimiento: mediana, percentil 99 y picos. `budget_ms` es el objetivo (16,7 ms = 60 FPS).
pub fn frame_budget(ds: &Dataset, budget_ms: f64) -> FrameBudget {
    let (source, ms, at): (&str, Vec<f64>, Vec<f64>) = match ds.col("frame.dt_ms") {
        Some(col) => {
            let idx: Vec<usize> = (0..ds.len()).filter(|&i| ds.valid[i] && col[i].is_finite()).collect();
            ("frame.dt_ms", idx.iter().map(|&i| f64::from(col[i])).collect(), idx.iter().map(|&i| ds.times[i]).collect())
        }
        None => {
            let idx: Vec<usize> = (1..ds.len()).filter(|&i| ds.valid[i] && ds.valid[i - 1]).collect();
            (
                "diferencia de tiempos",
                idx.iter().map(|&i| (ds.times[i] - ds.times[i - 1]) * 1000.0).collect(),
                idx.iter().map(|&i| ds.times[i]).collect(),
            )
        }
    };
    let mut sorted = ms.clone();
    let med = median(&mut sorted).unwrap_or(0.0);
    let p99 = percentile(&mut sorted, 99.0).unwrap_or(0.0);
    let max = ms.iter().copied().fold(0.0_f64, f64::max);
    let over = if ms.is_empty() { 0.0 } else { ms.iter().filter(|v| **v > budget_ms).count() as f64 / ms.len() as f64 };
    let mut spikes: Vec<Spike> = ms
        .iter()
        .zip(&at)
        .filter(|(v, _)| med > 0.0 && **v > 2.0 * med)
        .map(|(v, t)| Spike { t: round(*t, 3), ms: round(*v, 2) })
        .collect();
    let count = spikes.len();
    spikes.sort_unstable_by(|a, b| b.ms.total_cmp(&a.ms));
    spikes.truncate(5);
    let out = FrameBudget {
        samples: ms.len(),
        source: source.to_string(),
        median_ms: round(med, 2),
        p99_ms: round(p99, 2),
        max_ms: round(max, 2),
        budget_ms,
        over_budget: round(over, 3),
        spikes: count,
        worst: spikes,
    };
    out
}

#[cfg(test)]
mod tests {
    use super::super::testing::synth;
    use super::*;

    #[test]
    fn a_clean_72_fps_session_is_reliable() {
        let ds = synth(72.0, 720, &[("p.x", Box::new(|_, _| 0.0))]);
        let g = gaps(&ds);
        assert!(g.reliable, "{:?}", g.issues);
        assert_eq!((g.fps_median, g.slow_frames, g.excluded_frames), (72.0, 0, 0));
    }

    #[test]
    fn low_fps_are_flagged_like_the_3_fps_case() {
        let ds = synth(3.0, 40, &[("p.x", Box::new(|_, _| 0.0))]);
        let g = gaps(&ds);
        assert!(!g.reliable);
        assert!(g.issues.iter().any(|m| m.contains("FPS medianos bajos")));
        assert!(g.issues.iter().any(|m| m.contains("más de 100 ms")));
    }

    #[test]
    fn a_long_hitch_is_reported_as_the_longest_gap() {
        let mut ds = synth(72.0, 200, &[("p.x", Box::new(|_, _| 0.0))]);
        for t in ds.times.iter_mut().skip(100) {
            *t += 0.5; // un tirón de medio segundo en el fotograma 100
        }
        let g = gaps(&ds);
        assert!((g.longest_gap_s - 0.514).abs() < 0.001, "{}", g.longest_gap_s);
        assert_eq!(g.slow_frames, 1);
        assert!(g.reliable); // un tirón aislado no invalida la medición
    }

    #[test]
    fn ten_seconds_at_3_fps_inside_a_fast_session_make_it_unreliable() {
        // 55 s a ~90 FPS con un tramo de 10 s (de 20 a 30) a 3 FPS: pocos fotogramas, mucho tiempo sin medir.
        let mut times = Vec::new();
        let mut t = 0.0;
        while t < 55.0 {
            times.push(t);
            t += if (20.0..30.0).contains(&t) { 1.0 / 3.0 } else { 1.0 / 90.0 };
        }
        let n = times.len();
        let ds = Dataset::new(vec!["p.x".into()], times, vec![vec![0.0; n]]);
        let g = gaps(&ds);
        assert!(g.slow_fraction < 0.02, "por fotogramas casi no se nota: {}", g.slow_fraction); // lo que engañaba antes
        assert!(g.slow_time_fraction > 0.15, "{}", g.slow_time_fraction);
        assert!(!g.reliable, "{:?}", g.issues);
        assert_eq!(g.stretches.len(), 1);
        assert!((g.stretches[0].t_start - 20.0).abs() < 0.1 && (g.stretches[0].fps - 3.0).abs() < 0.2, "{:?}", g.stretches);
        assert!(g.issues.iter().any(|m| m.contains("tramo(s) de FPS bajos")));
    }

    #[test]
    fn paused_frames_are_reported_and_make_it_unreliable() {
        let mut ds = synth(72.0, 100, &[("p.x", Box::new(|_, _| 0.0))]);
        for v in ds.valid.iter_mut().skip(40).take(10) {
            *v = false;
        }
        let g = gaps(&ds);
        assert_eq!(g.excluded_frames, 10);
        assert!(!g.reliable);
    }

    #[test]
    fn too_little_data_makes_no_claim() {
        let ds = synth(72.0, 1, &[("p.x", Box::new(|_, _| 0.0))]);
        let g = gaps(&ds);
        assert!(!g.reliable);
        assert_eq!(g.samples, 0);
    }

    #[test]
    fn frame_budget_uses_the_recorded_column_with_median_p99_and_spikes() {
        let ds = synth(
            72.0,
            200,
            &[("frame.dt_ms", Box::new(|i, _| if i == 50 { 40.0 } else { 13.9 }))],
        );
        let b = frame_budget(&ds, 16.7);
        assert_eq!(b.source, "frame.dt_ms");
        assert_eq!((b.median_ms, b.max_ms, b.spikes), (13.9, 40.0, 1));
        assert_eq!(b.worst[0].ms, 40.0);
        assert!((b.over_budget - 0.005).abs() < 1e-9);
    }

    #[test]
    fn frame_budget_falls_back_to_the_time_differences() {
        let ds = synth(60.0, 120, &[("p.x", Box::new(|_, _| 0.0))]);
        let b = frame_budget(&ds, 16.7);
        assert_eq!(b.source, "diferencia de tiempos");
        assert_eq!(b.median_ms, 16.67);
        assert_eq!(b.spikes, 0);
    }
}
