// =============================================================================
// DBV Unreal AI — Caja negra: almacenamiento y lector de sesiones grabadas
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-91
//
// Lee lo que graba el plugin de Unreal (`dbv_unreal_ai/session`) en `Saved/DBVUnrealAI/Sessions/<id>/`:
//   · `meta.json`    — plan, estado, coste medido, avisos (texto);
//   · `frames.bin`   — señales por fotograma, por bloques con CRC (contrato en `session/frames.py`);
//   · `events.jsonl` — marcas y eventos, una línea JSON por evento.
// Todo se lee en la app (no en el editor), así que funciona con el editor cerrado o colgado. La lectura es tolerante:
// un bloque a medias o una línea cortada se descartan y el resto se lee (R6). Nunca modifica una sesión ajena.

pub mod analysis;
pub mod commands;
pub mod control;
pub mod store;
pub mod tools;

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

pub use store::{
    delete_session, list_sessions, prune_sessions, read_events, read_frames, read_meta, Frames, FramesRead,
    SessionInfo,
};

/// Sesiones que se conservan por proyecto (las últimas 20, REQ-91); al parar una grabación se podan las más antiguas.
pub const DEFAULT_KEEP_SESSIONS: usize = 20;

/// Las diez herramientas `session_*` (D69): lo que publican el MCP propio y el asistente integrado.
pub fn specs() -> Vec<Value> {
    let mut all = tools::specs();
    all.extend(control::specs());
    all
}

/// ¿Es una herramienta de la caja negra?
pub fn is_tool(name: &str) -> bool {
    let out = specs().iter().any(|s| s["name"].as_str() == Some(name));
    out
}

/// Ejecuta una herramienta `session_*`. `root` es `Saved/DBVUnrealAI/Sessions` del proyecto; `editor` lo implementa
/// cada superficie (solo lo tocan las que graban o paran Play).
pub fn run_tool(name: &str, args: &Value, root: &Path, editor: &mut dyn control::Editor) -> Result<String, String> {
    let out = match name {
        "session_list" => tools::list_text(root, args),
        "session_analyze" => tools::analyze_text(root, args),
        "session_query" => tools::query_text(root, args),
        "session_wait" => tools::wait_text(root, args, Duration::from_millis(500)),
        "session_delete" => tools::delete_text(root, args),
        "session_plan" => control::plan_text(args),
        "session_start" => control::start_text(editor, args),
        "session_status" => control::status_text(editor),
        "session_mark" => control::mark_text(editor, args),
        "session_stop" => control::stop_text(editor, root, args),
        other => Err(format!("unknown session tool «{other}»")),
    };
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_the_ten_tools_and_is_tool_knows_them() {
        let names: Vec<String> = specs().iter().map(|s| s["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names.len(), 10);
        assert!(names.iter().all(|n| n.starts_with("session_") && is_tool(n)));
        assert!(!is_tool("session_nada") && !is_tool("unreal_playtest"));
    }
}
