// =============================================================================
// DBV Unreal AI — Caja negra: análisis de una sesión (conjunto de datos común)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-93
//
// Rust puro sobre los ficheros de la sesión (D70): determinista y comprobable sin el motor. Cada análisis es una
// función con su resultado tipado, que dice qué columnas usó y con cuántas muestras (para poder dudar de él) y trata
// igual los fotogramas sin dato (`NaN`) o excluidos (pausas): un incremento solo cuenta si sus dos extremos valen.

pub mod events;
pub mod frame;
pub mod motion;
pub mod relative;

use std::path::Path;

use super::store::{read_frames, read_meta};

/// Señales por fotograma de una sesión, con el nombre de cada columna y los fotogramas que no cuentan.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dataset {
    pub times: Vec<f64>,
    pub names: Vec<String>,
    /// `cols[c][i]`: la señal `c` en el fotograma `i`. `NaN` si no hubo dato.
    pub cols: Vec<Vec<f32>>,
    /// `false` en los fotogramas que se excluyen (PIE en pausa): ningún incremento los cruza.
    pub valid: Vec<bool>,
}

impl Dataset {
    pub fn new(names: Vec<String>, times: Vec<f64>, cols: Vec<Vec<f32>>) -> Self {
        let valid = vec![true; times.len()];
        Self { times, names, cols, valid }
    }

    /// Lee la sesión de una carpeta: `frames.bin` y los nombres de columna de `meta.json`.
    pub fn from_session(folder: &Path) -> Result<Self, String> {
        let read = read_frames(folder)?;
        let meta = read_meta(folder);
        let names: Vec<String> = meta
            .get("columns")
            .and_then(serde_json::Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let out = if names.len() == read.frames.cols.len() {
            Ok(Self::new(names, read.frames.times, read.frames.cols))
        } else {
            Err(format!(
                "meta.json lista {} columnas y frames.bin trae {}",
                names.len(),
                read.frames.cols.len()
            ))
        };
        out
    }

    pub fn len(&self) -> usize {
        self.times.len()
    }

    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    pub fn col(&self, name: &str) -> Option<&[f32]> {
        let out = self.names.iter().position(|n| n == name).map(|i| self.cols[i].as_slice());
        out
    }

    /// Las columnas de una señal: `jugador` → `jugador.x`, `jugador.y`… ; o la propia columna si existe tal cual.
    pub fn signal(&self, signal: &str) -> Vec<(&str, &[f32])> {
        let prefix = format!("{signal}.");
        let out = self
            .names
            .iter()
            .zip(&self.cols)
            .filter(|(n, _)| n.as_str() == signal || n.starts_with(&prefix))
            .map(|(n, c)| (n.as_str(), c.as_slice()))
            .collect();
        out
    }

    /// Las columnas de posición `x`, `y`, `z` de una señal (en cm), si existen las tres.
    pub fn position(&self, signal: &str) -> Option<[&[f32]; 3]> {
        let out = match (
            self.col(&format!("{signal}.x")),
            self.col(&format!("{signal}.y")),
            self.col(&format!("{signal}.z")),
        ) {
            (Some(x), Some(y), Some(z)) => Some([x, y, z]),
            _ => None,
        };
        out
    }

    /// `true` si el incremento entre `i - 1` e `i` es fiable: ambos fotogramas cuentan y valen en todas las columnas dadas.
    pub fn step_ok(&self, i: usize, cols: &[&[f32]]) -> bool {
        let out = i > 0
            && self.valid.get(i).copied().unwrap_or(false)
            && self.valid.get(i - 1).copied().unwrap_or(false)
            && cols.iter().all(|c| c[i].is_finite() && c[i - 1].is_finite());
        out
    }
}

/// Mediana de una lista (la ordena). `None` si está vacía.
pub fn median(values: &mut [f64]) -> Option<f64> {
    values.sort_unstable_by(f64::total_cmp);
    let n = values.len();
    let out = if n == 0 {
        None
    } else if n % 2 == 1 {
        Some(values[n / 2])
    } else {
        Some((values[n / 2 - 1] + values[n / 2]) / 2.0)
    };
    out
}

/// Percentil `p` (0..=100) por el método del rango más cercano. `None` si está vacía.
pub fn percentile(values: &mut [f64], p: f64) -> Option<f64> {
    values.sort_unstable_by(f64::total_cmp);
    let out = if values.is_empty() {
        None
    } else {
        let rank = ((p / 100.0) * values.len() as f64).ceil() as usize;
        Some(values[rank.clamp(1, values.len()) - 1])
    };
    out
}

/// Redondeo para los informes (los números salen compactos).
pub fn round(value: f64, decimals: i32) -> f64 {
    let k = 10f64.powi(decimals);
    let out = (value * k).round() / k;
    out
}

#[cfg(test)]
pub mod testing {
    use super::*;

    /// Generador de una columna: `f(fotograma, tiempo)`.
    pub type Gen = Box<dyn Fn(usize, f64) -> f32>;

    /// Sesión sintética a `fps` fotogramas por segundo. Cada columna se genera con su `Gen`.
    pub fn synth(fps: f64, n: usize, cols: &[(&str, Gen)]) -> Dataset {
        let times: Vec<f64> = (0..n).map(|i| i as f64 / fps).collect();
        let names = cols.iter().map(|(name, _)| (*name).to_string()).collect();
        let data = cols.iter().map(|(_, f)| times.iter().enumerate().map(|(i, t)| f(i, *t)).collect()).collect();
        Dataset::new(names, times, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_and_percentile() {
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&mut [4.0, 1.0, 2.0, 3.0]), Some(2.5));
        assert_eq!(median(&mut []), None);
        let mut v: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&mut v, 99.0), Some(99.0));
        assert_eq!(percentile(&mut v, 50.0), Some(50.0));
    }

    #[test]
    fn signal_groups_columns_by_prefix() {
        let ds = Dataset::new(
            vec!["a.x".into(), "a.y".into(), "ab.x".into(), "speed".into()],
            vec![0.0],
            vec![vec![0.0]; 4],
        );
        assert_eq!(ds.signal("a").len(), 2); // «ab.x» no es de «a»
        assert_eq!(ds.signal("speed").len(), 1);
        assert!(ds.position("a").is_none());
    }

    #[test]
    fn step_ok_skips_missing_data_and_excluded_frames() {
        let mut ds = Dataset::new(vec!["s".into()], vec![0.0, 1.0, 2.0, 3.0], vec![vec![1.0, f32::NAN, 3.0, 4.0]]);
        ds.valid[3] = false;
        let c = ds.cols[0].clone();
        let cols = [c.as_slice()];
        assert!(!ds.step_ok(0, &cols)); // no hay fotograma anterior
        assert!(!ds.step_ok(1, &cols)); // NaN
        assert!(!ds.step_ok(2, &cols)); // el anterior es NaN
        assert!(!ds.step_ok(3, &cols)); // excluido (pausa)
    }

    #[test]
    fn from_session_rejects_a_column_mismatch() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/session_sample");
        let ds = Dataset::from_session(&dir).unwrap();
        assert_eq!((ds.len(), ds.names.len()), (5, 3));
        assert_eq!(ds.col("b").unwrap(), &[0.0, 2.0, 4.0, 6.0, 8.0]);
        let tmp = tempfile::tempdir().unwrap();
        std::fs::copy(dir.join("frames.bin"), tmp.path().join("frames.bin")).unwrap();
        std::fs::write(tmp.path().join("meta.json"), r#"{"columns":["solo_una"]}"#).unwrap();
        assert!(Dataset::from_session(tmp.path()).is_err());
    }
}
