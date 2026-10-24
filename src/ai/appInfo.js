// =============================================================================
// DBV Unreal AI — Lo que la IA necesita saber de TU aplicación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// ADAPTAR al convertir el starter en tu app (ver docs/IA.md, «Qué adaptar»): el nombre se manda a
// los agentes ACP y a los modelos en las instrucciones, y el tipo de documento dice con qué trabaja.
// La lista de extensiones que la IA puede leer y proponer vive en `paths.js` (primera barrera) y en
// `src-tauri/src/ai/files.rs` (`TEXT_EXTENSIONS`, la barrera de verdad).

/** Nombre de la aplicación, tal como se lo presentamos a la IA. */
export const APP_NAME = 'DBV Unreal AI';

/** Con qué trabaja el usuario, en inglés (las instrucciones a los modelos van en inglés). */
export const DOCUMENT_KIND = 'Markdown (.md) documents';
