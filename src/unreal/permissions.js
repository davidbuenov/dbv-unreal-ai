// =============================================================================
// DBV Unreal AI — Clasificación lectura/modificación de las herramientas de Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-12
//
// El servidor MCP de Unreal 5.8 no publica `annotations` (comprobado en vivo): no dice qué
// herramientas solo leen. Se decide por el VERBO con que empieza el nombre, con una lista
// blanca de verbos de consulta; todo lo demás es «modificación» y pide permiso (ADR-V010-001
// D5). Fallo seguro: una herramienta desconocida nunca se ejecuta sin preguntar.

/** Verbos que solo consultan. */
export const READ_VERBS = new Set([
  'get', 'list', 'find', 'describe', 'exists', 'is', 'can', 'capture', 'search', 'count', 'validate',
  'discover', 'observe', 'snapshot', 'trace', 'has', 'query', 'inspect', 'load',
]);

/**
 * Herramientas que SIEMPRE piden permiso aunque su verbo sea de lectura: ejecutan código o tocan
 * ficheros fuera del proyecto (`AssetTools.read_file` lee cualquier ruta del disco, RNF-IA.3).
 */
export const ALWAYS_ASK = new Set(['run_python', 'runpython', 'execute_tool_script', 'read_file', 'write_file']);

/** Primera palabra de un nombre `snake_case` o `CamelCase`, en minúsculas. */
export function firstWord(name) {
  return String(name ?? '')
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .toLowerCase()
    .split(/[_\s.-]+/)
    .filter(Boolean)[0] ?? '';
}

/** `'read'` o `'modify'` para una herramienta (`tool` sin el toolset delante). */
export function classifyTool(tool) {
  const bare = String(tool ?? '').split('.').pop();
  const lowered = bare.toLowerCase();
  let kind = 'modify';
  if (!ALWAYS_ASK.has(lowered) && READ_VERBS.has(firstWord(bare))) kind = 'read';
  return kind;
}

/** Permisos «siempre en esta conversación» (REQ-12). Uno por conversación. */
export function createPermissionMemory() {
  const allowed = new Set();
  const key = (toolset, tool) => `${toolset ?? ''}::${String(tool ?? '').toLowerCase()}`;
  return {
    isAllowed: (toolset, tool) => allowed.has(key(toolset, tool)),
    allow: (toolset, tool) => allowed.add(key(toolset, tool)),
    clear: () => allowed.clear(),
  };
}
