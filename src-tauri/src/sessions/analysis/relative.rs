// =============================================================================
// DBV Unreal AI — Caja negra: relación entre dos señales (`follow`)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-93

use serde::Serialize;

use super::{round, Dataset};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Follow {
    pub samples: usize,
    pub used: Vec<String>,
    /// `true` si A se expresó en el sistema de B (se usó el giro de B); `false` si se usó la diferencia en el mundo.
    pub in_b_frame: bool,
    /// Posición media de A en el sistema de B (cm).
    pub mean_cm: [f64; 3],
    /// Desviación típica por eje (cm).
    pub std_cm: [f64; 3],
    /// Mayor distancia de una muestra a la posición media (cm): lo que A «se despega» de B.
    pub max_deviation_cm: f64,
    /// Pendiente por eje (cm/s) de la posición relativa: una deriva constante.
    pub drift_cm_s: [f64; 3],
    /// Lo que A se habrá alejado de su sitio por la deriva a lo largo de toda la sesión (cm).
    pub drift_total_cm: f64,
    /// A sigue a B: la desviación máxima y la deriva total caben en `tolerance_cm`.
    pub follows: bool,
    pub tolerance_cm: f64,
}

/// ¿Sigue A a B? Pone A en el sistema de B (si hay `b.yaw`, grados) y mide su media, desviación máxima y deriva.
pub fn follow(ds: &Dataset, a: &str, b: &str, tolerance_cm: f64) -> Option<Follow> {
    let pa = ds.position(a)?;
    let pb = ds.position(b)?;
    let yaw = ds.col(&format!("{b}.yaw"));
    let mut cols: Vec<&[f32]> = pa.iter().chain(pb.iter()).copied().collect();
    if let Some(y) = yaw {
        cols.push(y);
    }

    let mut t = Vec::new();
    let mut rel: Vec<[f64; 3]> = Vec::new();
    for i in 0..ds.len() {
        let ok = ds.valid[i] && cols.iter().all(|c| c[i].is_finite());
        if !ok {
            continue;
        }
        let d = [0, 1, 2].map(|k| f64::from(pa[k][i] - pb[k][i]));
        let local = match yaw {
            Some(y) => {
                let (s, c) = f64::from(y[i]).to_radians().sin_cos();
                [d[0] * c + d[1] * s, -d[0] * s + d[1] * c, d[2]]
            }
            None => d,
        };
        t.push(ds.times[i]);
        rel.push(local);
    }
    let n = rel.len();
    if n < 3 {
        return None;
    }

    let mean = [0, 1, 2].map(|k| rel.iter().map(|r| r[k]).sum::<f64>() / n as f64);
    let std = [0, 1, 2].map(|k| (rel.iter().map(|r| (r[k] - mean[k]).powi(2)).sum::<f64>() / n as f64).sqrt());
    let max_dev = rel
        .iter()
        .map(|r| ((r[0] - mean[0]).powi(2) + (r[1] - mean[1]).powi(2) + (r[2] - mean[2]).powi(2)).sqrt())
        .fold(0.0_f64, f64::max);
    let t_mean = t.iter().sum::<f64>() / n as f64;
    let var_t: f64 = t.iter().map(|x| (x - t_mean).powi(2)).sum();
    let drift = [0, 1, 2].map(|k| {
        let cov: f64 = t.iter().zip(&rel).map(|(x, r)| (x - t_mean) * (r[k] - mean[k])).sum();
        if var_t > 0.0 { cov / var_t } else { 0.0 }
    });
    let span = t[n - 1] - t[0];
    let drift_total = drift.iter().map(|d| (d * span).powi(2)).sum::<f64>().sqrt();

    let out = Some(Follow {
        samples: n,
        used: [a, b].iter().map(|s| format!("{s}.x/y/z")).chain(yaw.map(|_| format!("{b}.yaw"))).collect(),
        in_b_frame: yaw.is_some(),
        mean_cm: mean.map(|v| round(v, 3)),
        std_cm: std.map(|v| round(v, 3)),
        max_deviation_cm: round(max_dev, 3),
        drift_cm_s: drift.map(|v| round(v, 4)),
        drift_total_cm: round(drift_total, 3),
        follows: max_dev <= tolerance_cm && drift_total <= tolerance_cm,
        tolerance_cm,
    });
    out
}

#[cfg(test)]
mod tests {
    use super::super::testing::synth;
    use super::*;

    fn two_actors(b_yaw: bool, a_extra: impl Fn(f64) -> [f32; 3] + 'static) -> Dataset {
        // B se desplaza a 100 cm/s en x; A va 30 cm por delante (en el sistema de B) y 10 a su derecha.
        let extra = std::rc::Rc::new(a_extra);
        let yaw = move |t: f64| if b_yaw { (t * 30.0) as f32 } else { 0.0 };
        let a_pos = std::rc::Rc::new(move |t: f64, k: usize| -> f32 {
            let (s, c) = f64::from(yaw(t)).to_radians().sin_cos();
            let (lx, ly) = (30.0_f64, 10.0_f64);
            let world = [(t * 100.0) as f32 + (lx * c - ly * s) as f32, (lx * s + ly * c) as f32, 5.0];
            world[k] + extra(t)[k]
        });
        let col = |k: usize| -> Box<dyn Fn(usize, f64) -> f32> {
            let f = std::rc::Rc::clone(&a_pos);
            Box::new(move |_, t| f(t, k))
        };
        synth(
            72.0,
            720,
            &[
                ("a.x", col(0)),
                ("a.y", col(1)),
                ("a.z", col(2)),
                ("b.x", Box::new(|_, t| (t * 100.0) as f32)),
                ("b.y", Box::new(|_, _| 0.0)),
                ("b.z", Box::new(|_, _| 0.0)),
                ("b.yaw", Box::new(move |_, t| yaw(t))),
            ],
        )
    }

    #[test]
    fn a_rigid_follower_in_a_turning_frame_has_no_deviation() {
        let f = follow(&two_actors(true, |_| [0.0; 3]), "a", "b", 1.0).unwrap();
        assert!(f.in_b_frame && f.follows, "{f:?}");
        assert!(f.max_deviation_cm < 0.5, "{}", f.max_deviation_cm); // solo el redondeo a f32
        assert!((f.mean_cm[0] - 30.0).abs() < 0.5 && (f.mean_cm[1] - 10.0).abs() < 0.5 && (f.mean_cm[2] - 5.0).abs() < 0.5);
    }

    #[test]
    fn a_seeded_drift_is_measured_and_breaks_following() {
        // A se va desplazando 0,5 cm/s hacia delante respecto a B (que no gira): en 10 s son 5 cm.
        let f = follow(&two_actors(false, |t| [(t * 0.5) as f32, 0.0, 0.0]), "a", "b", 1.0).unwrap();
        assert!(!f.follows);
        assert!((f.drift_cm_s[0] - 0.5).abs() < 0.05, "{:?}", f.drift_cm_s);
        assert!((f.drift_total_cm - 5.0).abs() < 0.5, "{}", f.drift_total_cm);
    }

    #[test]
    fn without_yaw_the_world_difference_is_used() {
        let mut ds = two_actors(false, |_| [0.0; 3]);
        let yaw = ds.names.iter().position(|n| n == "b.yaw").unwrap();
        ds.names.remove(yaw);
        ds.cols.remove(yaw);
        let f = follow(&ds, "a", "b", 1.0).unwrap();
        assert!(!f.in_b_frame && f.follows);
    }

    #[test]
    fn a_trembling_follower_reports_its_maximum_deviation() {
        let f = follow(&two_actors(false, |t| [if ((t * 72.0) as i64) % 2 == 0 { 0.6 } else { 0.0 }, 0.0, 0.0]), "a", "b", 1.0).unwrap();
        assert!((f.max_deviation_cm - 0.3).abs() < 0.1, "{}", f.max_deviation_cm); // ±0,3 respecto a la media
        assert!(f.follows);
    }

    #[test]
    fn unknown_signals_or_too_little_data_give_none() {
        let ds = two_actors(false, |_| [0.0; 3]);
        assert!(follow(&ds, "a", "nada", 1.0).is_none());
        let mut short = ds.clone();
        for v in short.valid.iter_mut().skip(2) {
            *v = false;
        }
        assert!(follow(&short, "a", "b", 1.0).is_none());
    }
}
