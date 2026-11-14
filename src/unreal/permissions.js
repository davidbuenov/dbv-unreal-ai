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

/**
 * `'read'` o `'modify'` para una herramienta (`tool` sin el toolset delante). UE 5.9 publica `accessMode` (`read`,
 * `write`, `execute`) y entonces manda la marca: `read` es lectura salvo las de `ALWAYS_ASK`; `write` y `execute`
 * piden permiso aunque el verbo parezca de consulta (p. ej. `LoadLevel`). Sin marca (UE 5.8) sigue la regla del verbo,
 * que ya pide permiso para todo verbo desconocido. @spec REQ-12, REQ-112
 * @param {string} tool
 * @param {string|null} [accessMode]
 */
export function classifyTool(tool, accessMode = null) {
  const bare = String(tool ?? '').split('.').pop();
  const lowered = bare.toLowerCase();
  const mode = String(accessMode ?? '').toLowerCase();
  const readByMark = mode === 'read';
  const readByVerb = mode === '' && READ_VERBS.has(firstWord(bare));
  let kind = 'modify';
  if (!ALWAYS_ASK.has(lowered) && (readByMark || readByVerb)) kind = 'read';
  return kind;
}

/**
 * Niveles de autonomía, como los del Unreal Assistant de UE 5.9 (REQ-115): `ask` pregunta todo, también lo que solo
 * lee; `read` solo deja leer y rechaza lo demás sin preguntar; `edit` (por defecto, el comportamiento de siempre) lee
 * sin preguntar y pide permiso para modificar; `auto` modifica sin preguntar pero pide permiso para ejecutar código o
 * tocar ficheros (`ALWAYS_ASK`); `full` no pregunta nada. El coste de Tripo o World Labs se confirma aparte en todos.
 */
export const AUTONOMY_LEVELS = ['ask', 'read', 'edit', 'auto', 'full'];
export const DEFAULT_AUTONOMY = 'edit';
const AUTONOMY_KEY = 'dbv.autonomy';

/** Qué hacer con una acción de tipo `kind` (`read`, `modify`, `execute`) en el nivel `level`: `run`, `ask` o `deny`. @spec REQ-115, REQ-12 */
export function autonomyDecision(level, kind) {
  const table = {
    ask: { read: 'ask', modify: 'ask', execute: 'ask' },
    read: { read: 'run', modify: 'deny', execute: 'deny' },
    edit: { read: 'run', modify: 'ask', execute: 'ask' },
    auto: { read: 'run', modify: 'run', execute: 'ask' },
    full: { read: 'run', modify: 'run', execute: 'run' },
  };
  const row = table[level] ?? table[DEFAULT_AUTONOMY];
  return row[kind] ?? row.modify;
}

/**
 * Herramientas de UE 5.9 que ejecutan código o comandos aunque el editor no lo diga (los agentes no ven `accessMode`):
 * el Python de Epic (sin la validación de la app), la consola y Live Coding. @spec REQ-115
 */
export const EXECUTE_TOOLS = new Set(['execute_python', 'runconsolecommand', 'compilelivecoding', 'enablelivecoding']);

/** Toolsets cuyas herramientas que no solo leen tocan ficheros de cualquier ruta del disco (UE 5.9). */
const FILE_TOOLSETS = /(^|\.)FileSystemToolset(\.|$)/i;

/**
 * ¿Ejecuta código o comandos, o toca ficheros? Lo que el nivel *Automático* sigue preguntando: la marca
 * `accessMode: execute` de UE 5.9, un nombre de `ALWAYS_ASK` o `EXECUTE_TOOLS`, o un toolset de ficheros. Sin esto,
 * en *Automático* el Python de Epic o un comando de consola se ejecutaban sin preguntar. @spec REQ-115, REQ-12
 * @param {string} toolset
 * @param {string} tool
 * @param {string|null} [accessMode]
 */
export function isExecution(toolset, tool, accessMode = null) {
  const bare = String(tool ?? '').split('.').pop().toLowerCase();
  return ALWAYS_ASK.has(bare) || EXECUTE_TOOLS.has(bare) || String(accessMode ?? '').toLowerCase() === 'execute' || FILE_TOOLSETS.test(String(toolset ?? ''));
}

/** Tipo de una petición de aprobación: `execute` si ejecuta código o toca ficheros; si no, `modify`. @spec REQ-115 */
export function requestKind(request) {
  return request?.kind === 'python' || isExecution(request?.toolset, request?.tool, request?.accessMode) ? 'execute' : 'modify';
}

/** Nivel guardado (el de siempre si no hay o no es válido). */
export function loadAutonomy(storage = globalThis.localStorage) {
  const stored = storage?.getItem?.(AUTONOMY_KEY) ?? null;
  return AUTONOMY_LEVELS.includes(stored) ? stored : DEFAULT_AUTONOMY;
}

/** Guarda el nivel (si es uno de los conocidos). */
export function saveAutonomy(level, storage = globalThis.localStorage) {
  if (AUTONOMY_LEVELS.includes(level)) storage?.setItem?.(AUTONOMY_KEY, level);
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
