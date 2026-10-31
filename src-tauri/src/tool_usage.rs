// =============================================================================
// DBV Unreal AI — Cuánto devuelven las herramientas a la IA
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-51
//
// El `--mcp` de un agente es otro proceso: lo que devuelve no lo ve la ventana. Como con los trabajos de generación
// (estado compartido = ficheros, lección del 2026-10-24), cada llamada deja una línea en `<datos>/tool-usage.jsonl`
// y la ventana suma las de su proyecto entre el principio y el final del turno para enseñar «Herramientas: N
// llamadas · X KB». Solo se anota el tamaño, nunca el contenido. El fichero rota a los 512 KB (D23).

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Fichero de uso dentro de la carpeta de datos de la app.
pub const FILE: &str = "tool-usage.jsonl";
/// La copia anterior tras rotar.
const OLD: &str = "tool-usage.old.jsonl";
/// Tamaño a partir del cual se rota.
const MAX_BYTES: u64 = 512 * 1024;

/// Una llamada a una herramienta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRecord {
    /// Milisegundos desde 1970.
    pub at: u64,
    pub pid: u32,
    /// Carpeta del proyecto, normalizada (`project_key`).
    pub project: String,
    pub tool: String,
    /// Caracteres de texto devueltos.
    pub text: usize,
    /// Imágenes devueltas.
    #[serde(default)]
    pub images: usize,
    /// El resultado se recortó al tope.
    #[serde(default)]
    pub clipped: bool,
}

/// Suma de las llamadas de un turno.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub calls: usize,
    pub text: usize,
    pub images: usize,
    pub clipped: usize,
}

/// Ahora en milisegundos.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Clave de un proyecto: su carpeta (aunque llegue la ruta del `.uproject`), con `/` y en minúsculas.
pub fn project_key(project: &str) -> String {
    let path = Path::new(project.trim());
    let dir = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("uproject")) { path.parent().unwrap_or(path) } else { path };
    dir.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// Anota una llamada. Un fallo de disco no debe romper la herramienta: se ignora y se devuelve `false`.
pub fn record(data_dir: &Path, entry: &UsageRecord) -> bool {
    let path = data_dir.join(FILE);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::rename(&path, data_dir.join(OLD));
    }
    let line = serde_json::to_string(entry).unwrap_or_default();
    let written = std::fs::create_dir_all(data_dir).is_ok()
        && std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut file| writeln!(file, "{line}")).is_ok();
    written
}

/// Llamadas del proyecto desde `since` (ms), en el fichero actual y en el rotado.
pub fn since(data_dir: &Path, project: &str, since: u64) -> UsageSummary {
    let key = project_key(project);
    let files: [PathBuf; 2] = [data_dir.join(OLD), data_dir.join(FILE)];
    let mut summary = UsageSummary::default();
    for record in files.iter().filter_map(|f| std::fs::read_to_string(f).ok()).flat_map(|text| text.lines().filter_map(|l| serde_json::from_str::<UsageRecord>(l).ok()).collect::<Vec<_>>()) {
        if record.at >= since && record.project == key {
            summary.calls += 1;
            summary.text += record.text;
            summary.images += record.images;
            summary.clipped += usize::from(record.clipped);
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(at: u64, project: &str, text: usize) -> UsageRecord {
        UsageRecord { at, pid: 1, project: project_key(project), tool: "unreal_describe_toolset".into(), text, images: 0, clipped: false }
    }

    /// La ventana suma lo del `--mcp` de su proyecto en el turno, no lo de otro proyecto ni lo de antes. @spec REQ-51
    #[test]
    fn suma_las_llamadas_del_proyecto_desde_el_inicio_del_turno() {
        let dir = tempfile::tempdir().unwrap();
        assert!(record(dir.path(), &entry(10, "D:/P/Demo/Demo.uproject", 500)));
        record(dir.path(), &entry(20, "d:\\P\\Demo", 300));
        record(dir.path(), &UsageRecord { images: 1, clipped: true, ..entry(30, "D:/P/Demo", 100) });
        record(dir.path(), &entry(25, "D:/P/Other", 9999));
        let summary = since(dir.path(), "D:/P/Demo/Demo.uproject", 15);
        assert_eq!(summary, UsageSummary { calls: 2, text: 400, images: 1, clipped: 1 });
        assert_eq!(since(dir.path(), "D:/P/Nada", 0).calls, 0);
    }

    /// Rota sin perder lo del turno en curso. @spec REQ-51
    #[test]
    fn rota_al_pasar_del_tamano_y_sigue_sumando() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE), "x".repeat(MAX_BYTES as usize + 1)).unwrap();
        record(dir.path(), &entry(5, "D:/P/Demo", 10));
        assert!(dir.path().join(OLD).exists());
        assert!(std::fs::metadata(dir.path().join(FILE)).unwrap().len() < 1024);
        assert_eq!(since(dir.path(), "D:/P/Demo", 0).calls, 1);
    }
}
