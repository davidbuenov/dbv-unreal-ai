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
pub mod store;
pub mod tools;

pub use store::{
    delete_session, list_sessions, prune_sessions, read_events, read_frames, read_meta, Frames, FramesRead,
    SessionInfo,
};
