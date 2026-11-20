// =============================================================================
// DBV Unreal AI — UEFN (Unreal Editor for Fortnite) y Verse: módulo raíz
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Todo lo propio de UEFN vive aquí y solo se activa con un proyecto UEFN (v0.7.0, §3.26):
//   · `discovery`: instalación, versión, proyectos y proyecto abierto (REQ-122).
//   · `server`: identificar el MCP de UEFN, diagnóstico, Python y lo que dice el log del editor (REQ-123).
//
// UEFN solo existe para Windows: en macOS y Linux las funciones devuelven «no hay UEFN».

pub mod discovery;
pub mod server;
