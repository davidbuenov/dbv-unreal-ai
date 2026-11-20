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
//   · `external`: repositorios de Epic que se descargan en la máquina del usuario, nunca se incluyen (REQ-124, REQ-131).
//   · `session`: probar en una sesión de UEFN y leer lo que imprime Verse (REQ-128).
//   · `verse`: conocimiento de Verse para cualquier IA y compilación (REQ-125..REQ-127).
//
// UEFN solo existe para Windows: en macOS y Linux las funciones devuelven «no hay UEFN».

pub mod discovery;
pub mod external;
pub mod server;
pub mod session;
pub mod verse;
