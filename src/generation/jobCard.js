// =============================================================================
// DBV Unreal AI — Tarjeta viva de un trabajo de generación, con su consola
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-27
//
// Un trabajo (de la ventana o de un agente por `--mcp`) se ve igual en Generar y en la conversación: nombre,
// operación, etapa, barra de progreso, coste y una CONSOLA con la petición exacta enviada a Tripo (sin la
// clave) y su registro. Antes, esperando a Tripo, la conversación parecía bloqueada.
//
// Los datos llegan del fichero de trabajo (`generation_jobs`): `createJobWatcher` lo sondea mientras haya
// trabajos activos y avisa de los nuevos y de los cambios.

import { t } from '../ai/i18nBridge.js';
import { el } from '../app/dom.js';

/** Cada cuánto se miran los ficheros de trabajo mientras hay alguno activo (ms). */
export const WATCH_MS = 1200;

const FINAL = ['done', 'failed', 'cancelled'];

/** ¿Ha terminado el trabajo? */
export const isFinal = (record) => FINAL.includes(record?.stage);

/** Texto de la consola: la petición enviada y el registro, en el orden en que ocurrió. */
export function consoleText(record) {
  const request = record.request && Object.keys(record.request).length ? `POST ${record.endpoint}\n${JSON.stringify(record.request, null, 2)}` : '';
  const output = record.output && typeof record.output === 'object' && Object.keys(record.output).length ? `\n\n${t('job.output')}\n${JSON.stringify(record.output, null, 2)}` : '';
  return `${request}${request ? '\n\n' : ''}${(record.log ?? []).join('\n')}${output}`;
}

/** Etapa legible de un trabajo. */
export function stageText(record) {
  const progress = String(Math.round(record.progress ?? 0));
  return t(`gen.stage.${record.stage}`, { progress, error: record.error ?? '' });
}

/**
 * @param {object} record Fichero de trabajo (`JobRecord`).
 * @param {object} [deps] `{onCancel(id), renderResult(record, slot)}`: botones y vista previa del resultado.
 */
export function createJobCard(record, deps = {}) {
  const title = el('strong', { className: 'job-name' });
  const meta = el('span', { className: 'job-meta' });
  const stage = el('span', { className: 'job-stage', attrs: { 'aria-live': 'polite' } });
  const bar = el('div');
  const progress = el('div', { className: 'progress' }, bar);
  const pre = el('pre', { className: 'job-console-text' });
  const consoleBox = el('details', { className: 'job-console' }, el('summary', { text: t('job.console') }), pre);
  const result = el('div', { className: 'job-result' });
  const cancel = el('button', { className: 'btn-ghost btn-link', text: t('gen.cancelJob'), attrs: { type: 'button' }, on: { click: () => deps.onCancel?.(current.id) } });
  const node = el('div', { className: 'job-card' }, el('div', { className: 'job-head' }, title, meta, el('span', { className: 'spacer' }), stage, cancel), progress, consoleBox, result);
  let current = record;
  let rendered = false;

  function update(next) {
    current = next;
    node.dataset.stage = next.stage;
    title.textContent = next.name;
    const cost = next.credits ?? next.estimatedCredits;
    const source = next.source === 'agent' ? ` · ${t('job.byAgent')}` : '';
    meta.textContent = `${next.kind}${cost ? ` · ${t('gen.spent', { spent: String(cost) })}` : ''}${source}`;
    stage.textContent = stageText(next);
    bar.style.width = `${isFinal(next) ? 100 : Math.round(next.progress ?? 0)}%`;
    progress.classList.toggle('hidden', next.stage === 'failed' || next.stage === 'cancelled');
    pre.textContent = consoleText(next);
    cancel.classList.toggle('hidden', isFinal(next) || next.source === 'agent' || !deps.onCancel);
    if (next.stage === 'done' && !rendered && deps.renderResult) {
      rendered = true;
      deps.renderResult(next, result);
    }
  }

  update(record);
  return { node, update, id: () => current.id, record: () => current };
}

/**
 * Sondea los trabajos y avisa. Para mientras no hay ninguno activo y se reanuda con `poke()`.
 * @param {object} deps `{list(): Promise<{ok, value}>, onRecord(record, isNew), filter?(record)}`.
 */
export function createJobWatcher(deps) {
  const seen = new Map();
  let timer = null;
  let stopped = false;
  // Una consulta en vuelo: `poke()` durante ella no lanza otra.
  let busy = false;

  async function tick() {
    timer = null;
    busy = true;
    const listed = await deps.list().finally(() => (busy = false));
    const records = listed.ok && Array.isArray(listed.value) ? listed.value.filter((r) => !deps.filter || deps.filter(r)) : [];
    for (const record of records) {
      const before = seen.get(record.id);
      if (!before || before.updatedAt !== record.updatedAt || before.stage !== record.stage) {
        seen.set(record.id, record);
        deps.onRecord(record, !before);
      }
    }
    if (!stopped && records.some((r) => !isFinal(r))) timer = setTimeout(tick, WATCH_MS);
  }

  return {
    /** Mira ya (y sigue mirando mientras haya trabajos activos). */
    poke() {
      stopped = false;
      if (timer === null && !busy) tick();
    },
    /** Los trabajos que ya existen no son «nuevos». */
    async prime() {
      const listed = await deps.list();
      for (const record of listed.ok && Array.isArray(listed.value) ? listed.value : []) seen.set(record.id, record);
    },
    stop() {
      stopped = true;
      clearTimeout(timer);
      timer = null;
    },
  };
}
