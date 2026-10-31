// =============================================================================
// DBV Unreal AI — Log local rotativo
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec RNF-08
//
// El fallo del `.ply` «importado pero que no está» (2026-10-31) no se pudo diagnosticar sin acceso al equipo: no
// había nada escrito. Aquí, un log sin crates nuevos (D26) en `<datos>/logs/dbv-unreal-ai.log`, 1 MB × 3 ficheros,
// que escriben la ventana y el `--mcp` (cada línea dice cuál). Se anotan rutas, tamaños y resultados; NUNCA claves,
// prompts ni código: antes de escribir, todo lo que tenga forma de clave se tapa.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Nombre del fichero dentro de `<datos>/logs`.
pub const FILE: &str = "dbv-unreal-ai.log";
/// Tamaño a partir del cual se rota.
const MAX_BYTES: u64 = 1024 * 1024;
/// Ficheros que se conservan: el actual, `.1` y `.2`.
const KEEP: usize = 3;

/// Carpeta del log y proceso que escribe (`window` o `mcp`), fijados al arrancar.
static TARGET: OnceLock<(PathBuf, &'static str)> = OnceLock::new();

/// Carpeta de los logs dentro de la de datos de la app.
pub fn log_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("logs")
}

/// Fija dónde se escribe y quién escribe. Solo vale la primera llamada.
pub fn init(data_dir: &Path, process: &'static str) {
    let _ = TARGET.set((log_dir(data_dir), process));
}

/// Claves con estas palabras se tapan aunque su valor no tenga una forma conocida.
const SECRET_KEYS: [&str; 6] = ["api_key", "apikey", "authtoken", "token", "password", "secret"];

/// El texto con todo lo que parezca una clave tapado: prefijos conocidos (`sk-`, `tsk_`…), lo que va tras
/// `Bearer`/`Authorization:` y el valor de `clave=valor` cuando la clave suena a secreto. @spec RNF-08
pub fn scrub(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut hide_next = false;
    for word in text.split(' ') {
        let lower = word.to_lowercase();
        let keyed = word.split_once(['=', ':']).filter(|(k, v)| !v.is_empty() && SECRET_KEYS.iter().any(|s| k.to_lowercase().trim_matches(['"', '\'']).ends_with(s)));
        let shown = if hide_next {
            "[redacted]".to_string()
        } else if let Some((key, _)) = keyed {
            format!("{key}=[redacted]")
        } else if crate::project_memory::find_secret(word).is_some() {
            "[redacted]".to_string()
        } else {
            word.to_string()
        };
        hide_next = matches!(lower.as_str(), "bearer" | "authorization:");
        out.push(shown);
    }
    out.join(" ")
}

/// Fecha UTC `AAAA-MM-DDTHH:MM:SSZ` sin crates (algoritmo de días civiles de Howard Hinnant).
fn timestamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// `dbv-unreal-ai.log` → `.1` → `.2`; el más viejo se pierde.
fn rotate(dir: &Path) {
    for n in (1..KEEP).rev() {
        let from = if n == 1 { dir.join(FILE) } else { dir.join(format!("{FILE}.{}", n - 1)) };
        let _ = std::fs::rename(from, dir.join(format!("{FILE}.{n}")));
    }
}

/// Escribe una línea en `dir`. Un fallo de disco no debe romper nada: devuelve si se escribió.
pub fn write_line(dir: &Path, process: &str, message: &str) -> bool {
    let path = dir.join(FILE);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        rotate(dir);
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let line = format!("{} [{process}] {}", timestamp(secs), scrub(&message.replace(['\r', '\n'], " ")));
    std::fs::create_dir_all(dir).is_ok() && std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| writeln!(f, "{line}")).is_ok()
}

/// Anota un hecho en el log del proceso (si se inicializó).
pub fn info(message: &str) {
    if let Some((dir, process)) = TARGET.get() {
        write_line(dir, process, message);
    }
}

/// Las últimas `n` líneas del log, para «Copiar diagnóstico».
pub fn tail(data_dir: &Path, n: usize) -> Vec<String> {
    let text = std::fs::read_to_string(log_dir(data_dir).join(FILE)).unwrap_or_default();
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    lines[lines.len().saturating_sub(n)..].to_vec()
}

/// ¿Corre la app empaquetada (MSIX de la Store)? Cambia dónde escribe en `AppData` (REQ-53).
pub fn is_packaged() -> bool {
    std::env::current_exe().map(|exe| exe.to_string_lossy().contains("WindowsApps")).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ninguna clave llega al log, tenga la forma que tenga. @spec RNF-08
    #[test]
    fn una_clave_nunca_llega_al_log() {
        let dir = tempfile::tempdir().unwrap();
        let line = "import ok tsk_abcdefgh12345678xyz Authorization: Bearer abc.def.ghi API_Key=cvai_1234567890abcdef password=hunter22 sk-ant-0123456789abcdefghij";
        assert!(write_line(dir.path(), "window", line));
        let written = std::fs::read_to_string(dir.path().join(FILE)).unwrap();
        for secret in ["tsk_abcdefgh", "abc.def.ghi", "cvai_1234567890", "hunter22", "sk-ant-"] {
            assert!(!written.contains(secret), "{secret} en {written}");
        }
        assert!(written.contains("[window] import ok") && written.contains("API_Key=[redacted]"), "{written}");
    }

    /// Rota a los 1 MB y conserva tres ficheros. @spec RNF-08
    #[test]
    fn rota_y_conserva_tres_ficheros() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..4 {
            std::fs::write(dir.path().join(FILE), "x".repeat(MAX_BYTES as usize + 1)).unwrap();
            write_line(dir.path(), "mcp", &format!("línea {n}"));
        }
        assert!(dir.path().join(format!("{FILE}.1")).exists() && dir.path().join(format!("{FILE}.2")).exists());
        assert!(!dir.path().join(format!("{FILE}.3")).exists());
        assert!(std::fs::read_to_string(dir.path().join(FILE)).unwrap().contains("[mcp] línea 3"));
    }

    /// @spec RNF-08
    #[test]
    fn fecha_utc_y_cola_del_log() {
        assert_eq!(timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(timestamp(1_793_430_000), "2026-10-31T07:00:00Z");
        let data = tempfile::tempdir().unwrap();
        for n in 0..60 {
            write_line(&log_dir(data.path()), "window", &format!("l{n}"));
        }
        let last = tail(data.path(), 50);
        assert_eq!(last.len(), 50);
        assert!(last[49].ends_with("l59"));
    }
}
