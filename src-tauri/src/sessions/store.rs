// =============================================================================
// DBV Unreal AI — Caja negra: carpeta de sesiones, lector tolerante y límites
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-91

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

const MAGIC: &[u8; 4] = b"DBVB";
const HEADER_LEN: usize = 20;

/// Señales por fotograma en orden de columnas (el nombre de cada una está en `meta.json`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Frames {
    /// Segundos desde el inicio de la grabación.
    pub times: Vec<f64>,
    /// `cols[c][i]` es la señal `c` en el fotograma `i`; `NaN` si no hubo dato.
    pub cols: Vec<Vec<f32>>,
}

/// Resultado de leer `frames.bin`: lo válido y si había un resto cortado o corrupto que se descartó.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FramesRead {
    pub frames: Frames,
    pub blocks: usize,
    pub truncated: bool,
}

/// Una fila del historial de sesiones.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub status: String,
    pub reason: String,
    pub started_at: String,
    pub level: String,
    pub rows: u64,
    pub duration_s: f64,
    pub bytes: u64,
    pub columns: Vec<String>,
    pub warnings: usize,
    /// El editor se cerró sin cerrar la sesión (`meta.json` seguía en «recording»): se lee igual.
    pub incomplete: bool,
}

/// CRC-32 (IEEE, el de `zlib.crc32` de Python) sin dependencia nueva.
pub(crate) fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    let out = !crc;
    out
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let chunk: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    let out = Some(u32::from_le_bytes(chunk));
    out
}

/// Lee los bloques válidos de `frames.bin`. Se detiene en el primer bloque cortado, corrupto o con cabecera rara.
pub fn parse_frames(bytes: &[u8]) -> FramesRead {
    let mut out = FramesRead::default();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let Some(block) = parse_block(bytes, pos) else {
            out.truncated = true;
            break;
        };
        let (times, cols, next) = block;
        if out.blocks == 0 {
            out.frames.cols = vec![Vec::new(); cols.len()];
        }
        if cols.len() != out.frames.cols.len() {
            out.truncated = true;
            break;
        }
        out.frames.times.extend(times);
        for (dst, src) in out.frames.cols.iter_mut().zip(cols) {
            dst.extend(src);
        }
        out.blocks += 1;
        pos = next;
    }
    out
}

type Block = (Vec<f64>, Vec<Vec<f32>>, usize);

fn parse_block(bytes: &[u8], pos: usize) -> Option<Block> {
    let head = bytes.get(pos..pos + HEADER_LEN)?;
    if &head[0..4] != MAGIC {
        return None;
    }
    let rows = u32_at(bytes, pos + 4)? as usize;
    let n_cols = u32_at(bytes, pos + 8)? as usize;
    let size = u32_at(bytes, pos + 12)? as usize;
    let crc = u32_at(bytes, pos + 16)?;
    let expected = rows.checked_mul(8)?.checked_add(n_cols.checked_mul(rows)?.checked_mul(4)?)?;
    if size != expected {
        return None;
    }
    let body = bytes.get(pos + HEADER_LEN..pos + HEADER_LEN + size)?;
    if crc32(body) != crc {
        return None;
    }
    let times: Vec<f64> = body[..rows * 8].as_chunks::<8>().0.iter().map(|c| f64::from_le_bytes(*c)).collect();
    let mut cols = Vec::with_capacity(n_cols);
    for c in 0..n_cols {
        let start = rows * 8 + c * rows * 4;
        let col: Vec<f32> =
            body[start..start + rows * 4].as_chunks::<4>().0.iter().map(|v| f32::from_le_bytes(*v)).collect();
        cols.push(col);
    }
    let out = Some((times, cols, pos + HEADER_LEN + size));
    out
}

/// Lee `frames.bin` de una carpeta de sesión.
pub fn read_frames(session: &Path) -> Result<FramesRead, String> {
    let out = fs::read(session.join("frames.bin"))
        .map(|bytes| parse_frames(&bytes))
        .map_err(|e| format!("no se pudo leer frames.bin: {e}"));
    out
}

/// Filas de `frames.bin` sin leer las cargas (solo las cabeceras): barato para el historial.
fn count_rows(bytes_len: u64, session: &Path) -> u64 {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = fs::File::open(session.join("frames.bin")) else {
        return 0;
    };
    let (mut pos, mut rows) = (0u64, 0u64);
    let mut head = [0u8; HEADER_LEN];
    while pos + HEADER_LEN as u64 <= bytes_len {
        if file.seek(SeekFrom::Start(pos)).is_err() || file.read_exact(&mut head).is_err() || &head[0..4] != MAGIC {
            break;
        }
        let n = u64::from(u32::from_le_bytes([head[4], head[5], head[6], head[7]]));
        let size = u64::from(u32::from_le_bytes([head[12], head[13], head[14], head[15]]));
        if pos + HEADER_LEN as u64 + size > bytes_len {
            break; // bloque cortado
        }
        rows += n;
        pos += HEADER_LEN as u64 + size;
    }
    rows
}

/// Lee `meta.json`. Si falta o está roto devuelve un objeto vacío: la sesión se lista igual.
pub fn read_meta(session: &Path) -> Value {
    let out = fs::read_to_string(session.join("meta.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    out
}

/// Lee `events.jsonl`; una línea cortada o ilegible se salta.
pub fn read_events(session: &Path) -> Vec<Value> {
    let out = fs::read_to_string(session.join("events.jsonl"))
        .map(|text| text.lines().filter_map(|l| serde_json::from_str::<Value>(l).ok()).collect())
        .unwrap_or_default();
    out
}

fn dir_bytes(folder: &Path) -> u64 {
    let out = fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|e| e.metadata().ok())
                .filter(|m| m.is_file())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0);
    out
}

fn str_of(meta: &Value, key: &str) -> String {
    let out = meta.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    out
}

fn info_of(folder: &Path, id: &str) -> SessionInfo {
    let meta = read_meta(folder);
    let frames_len = fs::metadata(folder.join("frames.bin")).map(|m| m.len()).unwrap_or(0);
    let status = str_of(&meta, "status");
    let out = SessionInfo {
        id: id.to_string(),
        incomplete: status != "finished",
        status,
        reason: str_of(&meta, "reason"),
        started_at: str_of(&meta, "started_at"),
        level: str_of(&meta, "level"),
        rows: count_rows(frames_len, folder),
        duration_s: meta.get("duration_s").and_then(Value::as_f64).unwrap_or(0.0),
        bytes: dir_bytes(folder),
        columns: meta
            .get("columns")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        warnings: meta.get("warnings").and_then(Value::as_array).map_or(0, Vec::len),
    };
    out
}

/// Sesiones de `root`, la más reciente primero. Los ids empiezan por `AAAAMMDD-HHMMSS`, así que ordenar por nombre
/// es ordenar por fecha. Una carpeta sin `frames.bin` ni `meta.json` no es una sesión y se ignora.
pub fn list_sessions(root: &Path) -> Vec<SessionInfo> {
    let mut ids: Vec<String> = fs::read_dir(root)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().is_dir())
                .filter(|e| e.path().join("meta.json").is_file() || e.path().join("frames.bin").is_file())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    ids.sort_unstable_by(|a, b| b.cmp(a));
    let out = ids.iter().map(|id| info_of(&root.join(id), id)).collect();
    out
}

/// Un id válido es un solo nombre de carpeta: sin separadores ni `..` (no se sale de `Sessions/`).
fn session_path(root: &Path, id: &str) -> Result<PathBuf, String> {
    let valid = !id.is_empty()
        && id != "."
        && id != ".."
        && !id.contains(['/', '\\', ':'])
        && Path::new(id).components().count() == 1;
    let out = if valid { Ok(root.join(id)) } else { Err(format!("id de sesión no válido: «{id}»")) };
    out
}

/// Borra una sesión. Exige `confirmed` (REQ-92: `session_delete` pide confirmación) y no toca una grabación activa.
pub fn delete_session(root: &Path, id: &str, confirmed: bool) -> Result<(), String> {
    if !confirmed {
        return Err("borrar una sesión pide confirmación".to_string());
    }
    let folder = session_path(root, id)?;
    if !folder.is_dir() {
        return Err(format!("no existe la sesión «{id}»"));
    }
    if str_of(&read_meta(&folder), "status") == "recording" && is_recent(&folder) {
        return Err("la sesión parece estar grabándose ahora mismo".to_string());
    }
    let out = fs::remove_dir_all(&folder).map_err(|e| format!("no se pudo borrar «{id}»: {e}"));
    out
}

/// `meta.json` o `frames.bin` tocados hace menos de 10 s: la grabación sigue viva.
fn is_recent(folder: &Path) -> bool {
    let now = std::time::SystemTime::now();
    let out = ["frames.bin", "events.jsonl", "meta.json"].iter().any(|name| {
        fs::metadata(folder.join(name))
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age.as_secs() < 10)
    });
    out
}

/// Una grabación en marcha ahora mismo, para el panel de la app.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSession {
    pub id: String,
    pub started_at: String,
    pub marks: usize,
    /// Tecla que marca un momento dentro del juego (sin quitarle el teclado).
    pub mark_key: String,
}

/// La grabación que se está escribiendo ahora, si hay: la sesión más reciente con estado «recording» y ficheros tocados
/// en los últimos 10 s. Solo lee ficheros (barato: la app lo pregunta cada par de segundos y no molesta al editor).
pub fn active_session(root: &Path) -> Option<ActiveSession> {
    let newest = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .max()?;
    let folder = root.join(&newest);
    let meta = read_meta(&folder);
    let recording = str_of(&meta, "status") == "recording" && is_recent(&folder);
    let out = recording.then(|| ActiveSession {
        id: newest,
        started_at: str_of(&meta, "started_at"),
        mark_key: Some(str_of(&meta, "mark_key")).filter(|k| !k.is_empty()).unwrap_or_else(|| "F10".to_string()),
        marks: read_events(&folder).iter().filter(|e| e.get("type").and_then(Value::as_str) == Some("mark")).count(),
    });
    out
}

/// Deja como mucho `keep` sesiones: borra las más antiguas (y nunca una que se esté grabando). Devuelve los ids borrados.
pub fn prune_sessions(root: &Path, keep: usize) -> Vec<String> {
    let all = list_sessions(root);
    let mut deleted = Vec::new();
    for info in all.iter().skip(keep) {
        if delete_session(root, &info.id, true).is_ok() {
            deleted.push(info.id.clone());
        }
    }
    deleted
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Codifica un bloque como lo hace `session/frames.py`.
    fn block(times: &[f64], cols: &[Vec<f32>]) -> Vec<u8> {
        let mut body = Vec::new();
        for t in times {
            body.extend_from_slice(&t.to_le_bytes());
        }
        for col in cols {
            for v in col {
                body.extend_from_slice(&v.to_le_bytes());
            }
        }
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(times.len() as u32).to_le_bytes());
        out.extend_from_slice(&(cols.len() as u32).to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&crc32(&body).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn two_blocks() -> Vec<u8> {
        let mut bytes = block(&[0.0, 0.5], &[vec![1.0, 2.0], vec![f32::NAN, 4.0]]);
        bytes.extend(block(&[1.0], &[vec![3.0], vec![5.0]]));
        bytes
    }

    fn write_session(root: &Path, id: &str, meta: &str, frames: &[u8]) {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("meta.json"), meta).unwrap();
        fs::write(dir.join("frames.bin"), frames).unwrap();
    }

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn reads_all_blocks_in_column_order() {
        let r = parse_frames(&two_blocks());
        assert_eq!((r.blocks, r.truncated), (2, false));
        assert_eq!(r.frames.times, vec![0.0, 0.5, 1.0]);
        assert_eq!(r.frames.cols[0], vec![1.0, 2.0, 3.0]);
        assert!(r.frames.cols[1][0].is_nan());
        assert_eq!(&r.frames.cols[1][1..], &[4.0, 5.0]);
    }

    #[test]
    fn a_cut_last_block_is_dropped_and_the_rest_is_kept() {
        let bytes = two_blocks();
        let r = parse_frames(&bytes[..bytes.len() - 5]);
        assert_eq!((r.blocks, r.truncated, r.frames.times.len()), (1, true, 2));
    }

    #[test]
    fn a_corrupt_block_stops_reading() {
        let mut bytes = two_blocks();
        bytes[HEADER_LEN + 3] ^= 0xFF;
        let r = parse_frames(&bytes);
        assert_eq!((r.blocks, r.truncated), (0, true));
    }

    #[test]
    fn garbage_and_absurd_sizes_do_not_panic() {
        assert!(parse_frames(b"no es un fichero de frames").truncated);
        let mut head = Vec::from(&MAGIC[..]);
        head.extend_from_slice(&u32::MAX.to_le_bytes());
        head.extend_from_slice(&u32::MAX.to_le_bytes());
        head.extend_from_slice(&0u32.to_le_bytes());
        head.extend_from_slice(&0u32.to_le_bytes());
        assert!(parse_frames(&head).truncated);
        assert_eq!(parse_frames(&[]), FramesRead::default());
    }

    #[test]
    fn reads_a_file_written_by_the_python_recorder() {
        // Contrato entre lenguajes: `frames.bin` generado con `session/frames.py` (3 columnas, 2 bloques de 3 y 2 filas).
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/session_sample");
        let r = read_frames(&dir).unwrap();
        assert_eq!((r.blocks, r.truncated, r.frames.times.len()), (2, false, 5));
        assert_eq!(r.frames.cols.len(), 3);
        assert_eq!(r.frames.cols[1], vec![0.0, 2.0, 4.0, 6.0, 8.0]);
        assert!(r.frames.cols[2][1].is_nan());
        let meta = read_meta(&dir);
        assert_eq!(meta["columns"][1], "b");
        let events = read_events(&dir);
        assert_eq!(events[0]["label"], "marca");
    }

    #[test]
    fn lists_sessions_newest_first_with_rows_and_status() {
        let root = tempfile::tempdir().unwrap();
        write_session(root.path(), "20261110-100000", r#"{"status":"finished","reason":"pie_ended","duration_s":3.5,"columns":["a","b"],"warnings":["x"],"level":"/Game/M"}"#, &two_blocks());
        write_session(root.path(), "20261110-110000", r#"{"status":"recording"}"#, &two_blocks()[..30]);
        fs::create_dir_all(root.path().join("otra-carpeta")).unwrap();
        let list = list_sessions(root.path());
        assert_eq!(list.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["20261110-110000", "20261110-100000"]);
        assert!(list[0].incomplete);
        assert_eq!(list[0].rows, 0); // el único bloque está cortado
        assert_eq!((list[1].rows, list[1].incomplete, list[1].warnings), (3, false, 1));
        assert_eq!(list[1].columns, ["a", "b"]);
    }

    #[test]
    fn a_session_with_a_broken_meta_is_still_listed() {
        let root = tempfile::tempdir().unwrap();
        write_session(root.path(), "20261110-100000", "{ esto no es json", &two_blocks());
        let list = list_sessions(root.path());
        assert_eq!((list.len(), list[0].rows, list[0].incomplete), (1, 3, true));
    }

    #[test]
    fn events_skip_a_cut_line() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("events.jsonl"), "{\"t\":1.0,\"type\":\"mark\"}\n{\"t\":2.0,\"ty").unwrap();
        assert_eq!(read_events(root.path()).len(), 1);
    }

    #[test]
    fn delete_needs_confirmation_and_a_plain_id() {
        let root = tempfile::tempdir().unwrap();
        write_session(root.path(), "20261110-100000", r#"{"status":"finished"}"#, &[]);
        assert!(delete_session(root.path(), "20261110-100000", false).is_err());
        for bad in ["", "..", ".", "../x", "a/b", "a\\b", "C:"] {
            assert!(delete_session(root.path(), bad, true).is_err(), "{bad}");
        }
        assert!(root.path().join("20261110-100000").exists());
        delete_session(root.path(), "20261110-100000", true).unwrap();
        assert!(!root.path().join("20261110-100000").exists());
    }

    #[test]
    fn delete_refuses_a_session_that_is_being_recorded_right_now() {
        let root = tempfile::tempdir().unwrap();
        write_session(root.path(), "20261110-100000", r#"{"status":"recording"}"#, &two_blocks());
        assert!(delete_session(root.path(), "20261110-100000", true).is_err());
    }

    #[test]
    fn the_active_session_is_the_newest_one_being_written_with_its_marks() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(active_session(root.path()), None);
        write_session(root.path(), "20261110-100000", r#"{"status":"finished"}"#, &[]);
        assert_eq!(active_session(root.path()), None); // la última ya terminó
        write_session(root.path(), "20261110-110000", r#"{"status":"recording","started_at":"2026-11-10T11:00:00"}"#, &two_blocks());
        fs::write(root.path().join("20261110-110000/events.jsonl"), "{\"t\":1.0,\"type\":\"mark\",\"label\":\"a\"}\n{\"t\":2.0,\"type\":\"pause\",\"paused\":true}\n{\"t\":3.0,\"type\":\"mark\",\"label\":\"b\"}\n").unwrap();
        let active = active_session(root.path()).unwrap();
        assert_eq!((active.id.as_str(), active.marks, active.started_at.as_str()), ("20261110-110000", 2, "2026-11-10T11:00:00"));
        assert_eq!(active.mark_key, "F10"); // sin `mark_key` en meta.json (sesiones antiguas), la de siempre
        // El panel lee `startedAt` y `markKey`: el JSON sale en camelCase.
        let json = serde_json::to_value(&active).unwrap();
        assert!(json.get("startedAt").is_some() && json.get("markKey").is_some() && json.get("started_at").is_none());
    }

    #[test]
    fn a_recording_session_nobody_touches_any_more_is_not_active() {
        let root = tempfile::tempdir().unwrap();
        write_session(root.path(), "20261110-110000", r#"{"status":"recording"}"#, &two_blocks());
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        for name in ["meta.json", "frames.bin"] {
            let file = fs::OpenOptions::new().write(true).open(root.path().join("20261110-110000").join(name)).unwrap();
            file.set_modified(old).unwrap();
        }
        assert_eq!(active_session(root.path()), None); // el editor se cerró de golpe: queda «incompleta», no «grabando»
    }

    #[test]
    fn prune_keeps_the_newest_sessions() {
        let root = tempfile::tempdir().unwrap();
        for i in 0..5 {
            write_session(root.path(), &format!("20261110-10000{i}"), r#"{"status":"finished"}"#, &[]);
        }
        let deleted = prune_sessions(root.path(), 3);
        assert_eq!(deleted, ["20261110-100001", "20261110-100000"]);
        assert_eq!(list_sessions(root.path()).len(), 3);
    }
}
