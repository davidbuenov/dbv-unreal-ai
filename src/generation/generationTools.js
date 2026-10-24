// =============================================================================
// DBV Unreal AI — Herramientas de generación y biblioteca para el asistente
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-15, REQ-17, REQ-19, RNF-04
//
// La IA puede proponer generar un modelo, pero NO gasta: la herramienta enseña la tarjeta de
// coste y solo si el usuario la acepta se lanza el trabajo (`confirmCost`). Enviar a Unreal es una
// modificación del proyecto y también pide permiso (REQ-12).

/** Espera máxima de un trabajo de generación desde la herramienta (ms). */
export const JOB_WAIT_MS = 12 * 60 * 1000;

const object = (properties, required = []) => ({ type: 'object', properties, required, additionalProperties: false });

/** Línea compacta de una entrada de la biblioteca para el modelo. */
export function describeEntry(entry) {
  const sent = entry.sentTo?.length ? `, sent to ${entry.sentTo.length} project(s)` : '';
  return `- ${entry.id}: "${entry.name}" (${entry.provider} ${entry.kind}, prompt: "${entry.prompt}"${sent})`;
}

/**
 * @param {object} deps
 * @param {() => Promise<{ok: boolean, value?: object}>} deps.status `generation_status`.
 * @param {(start: object) => Promise<{ok: boolean, value?: string, error?: object}>} deps.startTripo
 * @param {(jobId: string, timeoutMs: number) => Promise<{stage: string, entry?: object, error?: string}>} deps.waitJob
 * @param {() => Promise<{ok: boolean, value?: object[]}>} deps.listLibrary
 * @param {(entry: object) => Promise<{ok: boolean, assetPath?: string, actor?: string, error?: string}>} deps.send
 * @param {(request: object) => Promise<'once'|'always'|'deny'>} deps.askApproval Tarjeta de aprobación (permiso).
 * @param {(request: object) => Promise<boolean>} deps.confirmCost Tarjeta de coste.
 */
export function createGenerationTools(deps) {
  const generate = {
    name: 'generate_3d_model',
    description:
      'Generate a 3D model from a text description with Tripo (costs credits: the user sees the cost and must accept it). The result is saved in the asset library; use library_send_to_unreal to bring it into the project.',
    parameters: object(
      {
        prompt: { type: 'string', description: 'Visual description in English works best, e.g. "stylized wooden treasure chest, game prop".' },
        name: { type: 'string', description: 'Short name for the library.' },
        face_limit: { type: 'integer', description: 'Approximate triangle budget (a hint, not exact).' },
        smart_low_poly: { type: 'boolean' },
      },
      ['prompt'],
    ),
    label: (args) => `Tripo: ${args.name ?? args.prompt ?? ''}`,
    run: async ({ prompt, name = '', face_limit: faceLimit, smart_low_poly: lowPoly }) => {
      const status = await deps.status();
      const cost = status.ok ? status.value.tripoTextToModel : 30;
      let text;
      if (!String(prompt ?? '').trim()) text = 'error: empty prompt';
      else if (status.ok && !status.value.hasKey?.tripo) text = 'error: there is no Tripo API key. Tell the user to add it in Settings.';
      else if (!(await deps.confirmCost({ provider: 'tripo', prompt, name, cost, spent: status.value?.spent ?? 0, budget: status.value?.budget ?? 0 }))) {
        text = 'The user did NOT accept the cost: nothing was generated. Do not retry unless the user asks.';
      } else {
        const started = await deps.startTripo({ prompt, name, faceLimit, smartLowPoly: lowPoly, confirmCost: true });
        if (!started.ok) text = `error: ${started.error.message}`;
        else {
          const end = await deps.waitJob(started.value, JOB_WAIT_MS);
          text = end.stage === 'done' ? `Generated and saved in the library:\n${describeEntry(end.entry)}` : `error: generation ${end.stage}: ${end.error ?? ''}`;
        }
      }
      return text;
    },
  };

  const list = {
    name: 'library_list',
    description: 'List the generated assets in the local library (all projects), optionally filtered by text.',
    parameters: object({ filter: { type: 'string' } }),
    label: () => 'Library',
    run: async ({ filter = '' }) => {
      const listed = await deps.listLibrary();
      const needle = String(filter).toLowerCase();
      const entries = listed.ok ? listed.value.filter((e) => !needle || `${e.name} ${e.prompt}`.toLowerCase().includes(needle)) : [];
      return listed.ok ? (entries.length ? entries.slice(0, 50).map(describeEntry).join('\n') : 'the library is empty or nothing matches') : `error: ${listed.error.message}`;
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

  return [generate, list, send];
}
