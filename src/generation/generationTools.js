// =============================================================================
// DBV Unreal AI — Herramientas de generación y biblioteca para el asistente
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-15, REQ-16, REQ-17, REQ-19, REQ-27, REQ-33, RNF-04
//
// Las herramientas de Tripo NO se definen aquí: llegan de Rust (`generation/tools.rs`, comando
// `generation_tool_specs`), las mismas que ve un agente por el MCP propio (D10). Este módulo solo las adapta
// al bucle del asistente integrado: la IA puede proponer gastar, pero NO gasta: cada lanzamiento enseña la
// tarjeta de coste con el total y lo que se enviará, y solo si el usuario la acepta se lanza. Mientras Tripo
// trabaja, la conversación muestra la tarjeta viva del trabajo (REQ-27). Enviar a Unreal es una modificación
// del proyecto y también pide permiso (REQ-12).

/** Espera máxima de un trabajo de generación desde la herramienta (ms). */
export const JOB_WAIT_MS = 12 * 60 * 1000;

/** Herramientas de Rust que lanzan una operación con coste (`tools::LAUNCHERS`). */
export const LAUNCHERS = ['generate_3d_model', 'image_to_3d_model', 'rig_model', 'animate_model', 'tripo_run'];

const object = (properties, required = []) => ({ type: 'object', properties, required, additionalProperties: false });

/** Línea compacta de una entrada de la biblioteca para el modelo. */
export function describeEntry(entry) {
  const sent = entry.sentTo?.length ? `, sent to ${entry.sentTo.length} project(s)` : '';
  const from = entry.parent ? ` from ${entry.parent}` : '';
  const prompt = entry.prompt ? `, prompt: "${entry.prompt}"` : '';
  return `- ${entry.id}: "${entry.name}" (${entry.provider} ${entry.kind}${from}, files: ${(entry.files ?? []).join(', ')}${prompt}${sent})`;
}

/** Texto para el modelo con el final de cada trabajo. */
export function describeEnds(ends) {
  const lines = ends.map((end) => (end.stage === 'done' ? (end.entry ? describeEntry(end.entry) : `- done: ${JSON.stringify(end.output ?? {})}`) : `- ${end.stage}: ${end.error ?? ''}`));
  const done = ends.some((end) => end.stage === 'done' && end.entry);
  return `${lines.join('\n')}${done ? '\nThe results are in the library and the user sees them in the conversation. Ask before bringing one into Unreal with library_send_to_unreal.' : ''}`;
}

/**
 * @param {object} deps
 * @param {object[]} deps.specs Definiciones de Rust (`generation_tool_specs`).
 * @param {() => Promise<{ok: boolean, value?: object}>} deps.status `generation_status`.
 * @param {(tool: string, args: object) => Promise<{ok: boolean, value?: object, error?: object}>} deps.prepare Coste y petición, sin gastar.
 * @param {(tool: string, args: object) => Promise<{ok: boolean, value?: string[], error?: object}>} deps.launch
 * @param {(jobId: string, timeoutMs: number) => Promise<{stage: string, entry?: object, error?: string}>} deps.waitJob
 * @param {(ids: string[]) => void} [deps.showJobs] Tarjetas vivas de los trabajos en la conversación.
 * @param {(operation?: string) => Promise<{ok: boolean, value?: string}>} deps.operations
 * @param {() => Promise<{ok: boolean, value?: object}>} deps.balance
 * @param {() => Promise<{ok: boolean, value?: object[]}>} deps.listLibrary
 * @param {(id: string, name: string) => Promise<{ok: boolean, value?: object}>} deps.rename
 * @param {(entry: object) => Promise<{ok: boolean, assetPath?: string, actor?: string, error?: string}>} deps.send
 * @param {(request: object) => Promise<'once'|'always'|'deny'>} deps.askApproval Tarjeta de aprobación (permiso).
 * @param {(request: object) => Promise<boolean>} deps.confirmCost Tarjeta de coste.
 */
export function createGenerationTools(deps) {
  const fromRust = (deps.specs ?? []).map((spec) => ({
    name: spec.name,
    description: spec.description,
    parameters: { ...spec.inputSchema, additionalProperties: false },
    label: (args) => `Tripo: ${args.name ?? args.operation ?? args.id ?? spec.name}`,
    run: (args) => run(spec.name, args ?? {}),
  }));

  async function launchWithCost(tool, args) {
    const prepared = await deps.prepare(tool, args);
    const status = await deps.status();
    let text;
    if (!prepared.ok) text = `error: ${prepared.error.message}`;
    else if (status.ok && !status.value.hasKey?.tripo) text = 'error: there is no Tripo API key. Tell the user to add it in Settings.';
    else if (prepared.value.total > 0 && !(await deps.confirmCost({ ...prepared.value, spent: status.value?.spent ?? 0, budget: status.value?.budget ?? 0 }))) {
      text = 'The user did NOT accept the cost: nothing was run. Do not retry unless the user asks.';
    } else {
      const launched = await deps.launch(tool, args);
      if (!launched.ok) text = `error: ${launched.error.message}`;
      else {
        deps.showJobs?.(launched.value);
        const ends = await Promise.all(launched.value.map((id) => deps.waitJob(id, JOB_WAIT_MS)));
        text = describeEnds(ends);
      }
    }
    return text;
  }

  async function run(tool, args) {
    let text;
    if (LAUNCHERS.includes(tool)) text = await launchWithCost(tool, args);
    else if (tool === 'tripo_operations') {
      const listed = await deps.operations(args.operation);
      text = listed.ok ? listed.value : `error: ${listed.error.message}`;
    } else if (tool === 'tripo_balance') {
      const balance = await deps.balance();
      text = balance.ok ? `Tripo balance: ${balance.value.balance} credits available (${balance.value.frozen} reserved by running tasks).` : `error: ${balance.error.message}`;
    } else text = `error: unknown tool ${tool}`;
    return text;
  }

  const list = {
    name: 'library_list',
    description: 'List the assets of the library (generated or imported, all projects): id, name, kind (model, rig, animation, image…), source asset and files. Use the id with rig_model, animate_model, tripo_run or library_send_to_unreal.',
    parameters: object({ filter: { type: 'string' } }),
    label: () => 'Library',
    run: async ({ filter = '' }) => {
      const listed = await deps.listLibrary();
      const needle = String(filter).toLowerCase();
      const entries = listed.ok ? listed.value.filter((e) => !needle || `${e.name} ${e.prompt} ${e.kind}`.toLowerCase().includes(needle)) : [];
      return listed.ok ? (entries.length ? entries.slice(0, 50).map(describeEntry).join('\n') : 'the library is empty or nothing matches') : `error: ${listed.error.message}`;
    },
  };

  const rename = {
    name: 'library_rename',
    description: "Give a library asset a short, meaningful name in the user's language.",
    parameters: object({ id: { type: 'string' }, name: { type: 'string' } }, ['id', 'name']),
    label: (args) => `Library: ${args.name ?? ''}`,
    run: async ({ id, name }) => {
      const renamed = await deps.rename(id, name);
      return renamed.ok ? `Renamed ${id} to "${renamed.value.name}"` : `error: ${renamed.error.message}`;
    },
  };

  const send = {
    name: 'library_send_to_unreal',
    description: 'Import a library asset into the connected Unreal project (under /Game/DBV/Generated) and place it in front of the editor camera. Needs the user approval.',
    parameters: object({ id: { type: 'string', description: 'Library id from library_list.' }, purpose: { type: 'string' } }, ['id']),
    label: (args) => `Unreal ← ${args.id ?? ''}`,
    run: async ({ id, purpose = '' }) => {
      const listed = await deps.listLibrary();
      const entry = listed.ok ? listed.value.find((e) => e.id === id) : null;
      let text;
      if (!entry) text = `error: no library asset with id "${id}". Call library_list.`;
      else if ((await deps.askApproval({ kind: 'tool', toolset: 'DBV Unreal AI', tool: 'library_send_to_unreal', args: { id, name: entry.name }, purpose })) === 'deny') {
        text = 'The user DENIED importing this asset. It was not imported.';
      } else {
        const sent = await deps.send(entry);
        text = sent.ok ? `Imported as ${sent.assetPath}${sent.actor ? ` and placed as actor "${sent.actor}"` : ''}.` : `error: ${sent.error}`;
      }
      return text;
    },
  };

  return [...fromRust, list, rename, send];
}
