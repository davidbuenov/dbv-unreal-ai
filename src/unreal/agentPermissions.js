// =============================================================================
// DBV Unreal AI — Permisos de los agentes: las consultas no preguntan
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-12
//
// Un agente (Claude Code, Antigravity, Codex…) pide permiso por ACP para cada herramienta del servidor MCP de la
// app, también para las que solo consultan: con una receta eran decenas de tarjetas seguidas (lo vio el usuario el
// 2026-11-01). Las consultas de NUESTRO servidor (lista de Rust, `READ_ONLY_TOOLS`) y `unreal_call_tool` con una
// herramienta de lectura (la misma regla del verbo que el asistente integrado, `classifyTool`) se conceden solas.
// Lo demás —modificar, Python, gastar créditos y las herramientas propias del agente— sigue preguntando.

import { classifyTool } from './permissions.js';

/** El nombre de la herramienta de nuestro servidor en el título que pone cada agente, o `null`. */
const OWN_TOOL = /(?:^|[\s(`"'])(?:mcp__)?dbv-unreal-ai(?:__|[_.:/])([a-z][a-z0-9_]*)/i;

/** Herramienta de nuestro servidor a la que se refiere la petición de permiso, o `null` si es de otro. */
export function ownTool(params) {
  const call = params?.toolCall ?? {};
  const match = OWN_TOOL.exec(String(call.title ?? '')) ?? OWN_TOOL.exec(String(call.toolName ?? call.name ?? ''));
  return match ? match[1].toLowerCase() : null;
}

/**
 * Si la petición es una consulta de nuestro servidor, la opción con que se concede sola (y qué se consultó, para
 * dejarlo dicho en la conversación). `null` si hay que preguntar.
 * @param {object} params los de `session/request_permission`.
 * @param {string[]} readOnlyTools `READ_ONLY_TOOLS` de Rust.
 * @returns {{optionId: string, tool: string} | null}
 */
export function autoAllow(params, readOnlyTools) {
  const tool = ownTool(params);
  const raw = params?.toolCall?.rawInput ?? {};
  const inner = tool === 'unreal_call_tool' ? String(raw.tool ?? raw.tool_name ?? '') : '';
  const read = Boolean(tool) && (readOnlyTools.includes(tool) || (inner !== '' && classifyTool(inner) === 'read'));
  const options = params?.options ?? [];
  const option = read ? options.find((o) => o.kind === 'allow_once') ?? options.find((o) => String(o.kind ?? '').startsWith('allow')) : null;
  return option ? { optionId: option.optionId, tool: inner || tool } : null;
}
