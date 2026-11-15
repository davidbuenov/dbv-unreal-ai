// =============================================================================
// DBV Unreal AI — Herramientas de Fab para el asistente integrado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-105, REQ-117
//
// La definición viene de Rust (`fab_tool_specs`, la misma que el `--mcp`). Buscar, ver la ficha y la biblioteca
// solo leen; instalar e iniciar sesión piden aprobación (REQ-12). Los listings de cada resultado se pintan como
// tarjetas en la conversación (`showListings`) y al modelo solo le llega el texto.

export const FAB_WRITE_TOOLS = new Set(['fab_install', 'fab_sign_in']);

/**
 * @param {object} deps
 * @param {Array} deps.specs Definiciones de `fab_tool_specs`.
 * @param {(name: string, args: object) => Promise<{ok: boolean, value?: {text: string, fabListings: Array}, error?: {message: string}}>} deps.run
 * @param {(request: object) => Promise<string>} deps.askApproval `once`, `always` o `deny`.
 * @param {(listings: Array) => void} [deps.showListings] Pinta las tarjetas en la conversación.
 * @param {(entry: object) => void} [deps.onAction] Registro de acciones.
 * @param {{isAllowed: Function, allow: Function}} [deps.permissions]
 */
export function createFabTools(deps) {
  const now = () => Date.now();
  const tools = (deps.specs ?? []).map((spec) => {
    const writes = FAB_WRITE_TOOLS.has(spec.name);
    const properties = writes
      ? { ...spec.inputSchema.properties, purpose: { type: 'string', description: 'One sentence for the user: what will be installed or why.' } }
      : spec.inputSchema.properties;
    return {
      name: spec.name,
      description: spec.description,
      parameters: { ...spec.inputSchema, properties, additionalProperties: false },
      label: (args) => (spec.name === 'fab_search' && args?.query ? `Fab: ${args.query}` : `Fab: ${spec.name.replace('fab_', '')}`),
      run: async (args = {}) => {
        const { purpose = '', ...rest } = args ?? {};
        let result;
        if (writes && !deps.permissions?.isAllowed?.('Fab', spec.name)) {
          const choice = await deps.askApproval({ kind: 'tool', toolset: 'Fab', tool: spec.name, args: rest, purpose });
          if (choice === 'always') deps.permissions?.allow?.('Fab', spec.name);
          if (choice !== 'once' && choice !== 'always') result = 'The user DENIED this action. It was not executed.';
        }
        if (result === undefined) {
          const started = now();
          const ran = await deps.run(spec.name, rest);
          deps.onAction?.({ at: started, toolset: 'Fab', tool: spec.name, kind: writes ? 'modify' : 'read', args: rest, ms: now() - started, outcome: ran.ok ? 'ok' : 'error' });
          if (ran.ok && ran.value?.fabListings?.length) deps.showListings?.(ran.value.fabListings);
          result = ran.ok ? ran.value.text : `error: ${ran.error?.message ?? ran.error}`;
        }
        return result;
      },
    };
  });
  return tools;
}
