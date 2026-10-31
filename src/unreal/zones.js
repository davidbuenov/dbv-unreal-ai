// =============================================================================
// DBV Unreal AI — Zonas del nivel en la conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-57
//
// Las herramientas se DEFINEN en Rust (`zones.rs`), las mismas que ve un agente por el MCP propio (D10). Aquí cambia
// quién aprueba: la IA propone las zonas, la persona las revisa en una tarjeta y, al aceptarla, la app las crea en el
// editor con su código fijo (D29). Nada cambia en el nivel sin pulsar «Crear zonas».

import { t } from '../ai/i18nBridge.js';
import { button, el } from '../app/dom.js';

/** Metros con dos decimales a partir de centímetros. */
const meters = (cm) => Math.round(Number(cm) || 0) / 100;

/**
 * Tarjeta de zonas propuestas (DESIGN.md v0.3.0): una fila editable por zona que se puede quitar; «actualizar» si
 * la zona ya existe en el nivel. El foco va a «Descartar».
 * @param {Array<{name: string, description: string, label: string, x: number, y: number, z: number, radius: number}>} zones En cm (Rust).
 * @param {string[]} existing Etiquetas de las zonas que ya hay.
 * @returns {{node: HTMLElement, choice: Promise<{action: 'accept'|'discard', zones: object[]}>}}
 */
export function createZonesCard(zones, existing = []) {
  const rows = zones.map((zone) => {
    const name = el('input', { attrs: { type: 'text', 'aria-label': t('zones.name') } });
    name.value = zone.name;
    const description = el('input', { attrs: { type: 'text', 'aria-label': t('zones.description') } });
    description.value = zone.description;
    const where = el('span', { className: 'mono hint', text: `(${meters(zone.x)}, ${meters(zone.y)}, ${meters(zone.z)}) m · r ${zone.radius} m` });
    const tag = existing.includes(zone.label) ? el('span', { className: 'pill', text: t('zones.update') }) : null;
    const row = el('div', { className: 'plan-proposal__row' }, el('div', { className: 'stack' }, name, description, where), tag ?? el('span'));
    const entry = { zone, name, description, row, removed: false };
    const remove = button('×', () => {
      row.remove();
      entry.removed = true;
      refresh();
    }, 'btn-ghost button--compact');
    remove.setAttribute('aria-label', t('zones.remove'));
    row.append(remove);
    return entry;
  });
  const accept = button('', () => {}, 'btn-primary');
  const discard = button(t('memory.discard'), () => {}, 'btn-ghost');
  function refresh() {
    const count = rows.filter((r) => !r.removed).length;
    accept.textContent = t('zones.create', { n: String(count) });
    accept.disabled = count === 0;
  }
  refresh();
  const actions = el('div', { className: 'unreal-approval__actions' }, accept, discard);
  const card = el(
    'div',
    { className: 'memory-save', attrs: { role: 'group', 'aria-label': t('zones.title') } },
    el('div', { className: 'unreal-approval__title', text: t('zones.title') }),
    el('div', { className: 'unreal-approval__target mono', text: 'Outliner · DBV/Zones' }),
    ...rows.map((r) => r.row),
    el('p', { className: 'hint', text: t('zones.hint') }),
    actions,
  );
  const choice = new Promise((resolve) => {
    const decide = (action, label) => {
      for (const r of rows) {
        r.name.disabled = true;
        r.description.disabled = true;
      }
      actions.replaceWith(el('div', { className: 'unreal-approval__decision', text: label }));
      const kept = rows.filter((r) => !r.removed).map((r) => ({ ...r.zone, name: r.name.value.trim(), description: r.description.value.trim() })).filter((z) => z.name);
      resolve({ action, zones: action === 'accept' ? kept : [] });
    };
    accept.addEventListener('click', () => decide('accept', t('zones.created')));
    discard.addEventListener('click', () => decide('discard', t('memory.discarded')));
  });
  queueMicrotask(() => discard.focus());
  return { node: card, choice };
}

/** Texto para el modelo con lo que devolvió `run_python` (como `zones::report_text` de Rust). */
export function reportText(value) {
  return value?.success ? JSON.stringify(value.result ?? null) : `error: ${value?.error ?? 'the editor did not run the zones code'}`;
}

/**
 * Herramientas de zonas para el asistente integrado.
 * @param {object} deps
 * @param {object[]} deps.specs Definiciones de Rust (`zones_tool_specs`).
 * @param {(args: object) => Promise<{ok: boolean, value?: object[], error?: object}>} deps.preview `zones_preview`.
 * @param {(zones: object[]) => Promise<{ok: boolean, value?: object, error?: object}>} deps.apply `zones_apply`.
 * @param {() => Promise<{ok: boolean, value?: object, error?: object}>} deps.list `zones_list`.
 * @param {(zones: object[], existing: string[]) => Promise<{action: string, zones: object[]}>} deps.askZones Tarjeta.
 */
export function createZoneTools(deps) {
  async function propose(args) {
    const preview = await deps.preview(args);
    let text;
    if (!preview.ok) text = `error: ${preview.error.message}`;
    else {
      const listed = await deps.list();
      const existing = listed.ok ? (listed.value?.result?.zones ?? []).map((z) => z.label) : [];
      const decision = await deps.askZones(preview.value, existing);
      if (decision.action !== 'accept' || !decision.zones.length) text = 'The user DISCARDED these zones: nothing changed in the level. Ask what to change if they want other zones.';
      else {
        // Lo editado vuelve a pasar por Rust: el nombre cambia la etiqueta del actor.
        const edited = await deps.preview({ zones: decision.zones.map((z) => ({ name: z.name, description: z.description, x: z.x / 100, y: z.y / 100, z: z.z / 100, radius: z.radius })) });
        const applied = edited.ok ? await deps.apply(edited.value) : edited;
        text = applied.ok ? `Zones saved in the level (Outliner DBV/Zones; the user may have edited them): ${reportText(applied.value)}` : `error: ${applied.error.message}`;
      }
    }
    return text;
  }

  async function list() {
    const listed = await deps.list();
    return listed.ok ? reportText(listed.value) : `error: ${listed.error.message}`;
  }

  const runners = { level_zones_propose: propose, level_zones_list: list };
  return (deps.specs ?? [])
    .filter((spec) => runners[spec.name])
    .map((spec) => ({
      name: spec.name,
      description: spec.description,
      parameters: { ...spec.inputSchema, additionalProperties: false },
      label: () => t('zones.title'),
      run: (args) => runners[spec.name](args ?? {}),
    }));
}
