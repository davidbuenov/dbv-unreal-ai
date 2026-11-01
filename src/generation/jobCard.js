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

/** ¿Es de World Labs? Sus operaciones van a `/marble/…`. */
export const isWorldLabs = (record) => String(record?.endpoint ?? '').startsWith('/marble/');

/** Lo que suele tardar un mundo de World Labs (s), según su documentación: para la barra estimada. */
export const WORLD_EXPECTED_SECS = 300;

/** Segundos desde que empezó el trabajo. */
export const elapsedSecs = (record, nowMs = Date.now()) => Math.max(0, Math.floor(nowMs / 1000) - (record?.startedAt ?? Math.floor(nowMs / 1000)));

/** `m:ss`. */
export const clock = (secs) => `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, '0')}`;

/**
 * Etapa legible de un trabajo. World Labs no da porcentaje (solo «IN_PROGRESS»): en vez de un 0 % fijo se ve el
 * tiempo que lleva y lo que suele tardar (2026-11-01, lo vio el usuario). @spec REQ-18, REQ-27
 */
export function stageText(record, nowMs = Date.now()) {
  const progress = String(Math.round(record.progress ?? 0));
  const world = isWorldLabs(record);
  let key = `gen.stage.${record.stage}`;
  if (world && ['submitted', 'downloading'].includes(record.stage)) key = `gen.stage.${record.stage}WorldLabs`;
  else if (world && record.stage === 'running' && record.progress == null) key = record.kind === 'generate_world' ? 'gen.stage.runningWorld' : 'gen.stage.runningElapsed';
  return t(key, { progress, error: record.error ?? '', elapsed: clock(elapsedSecs(record, nowMs)) });
}

/** Ancho de la barra (%): el progreso real o, en un mundo sin progreso, una estimación por tiempo que no pasa del 95 %. */
export function barPercent(record, nowMs = Date.now()) {
  let percent = Math.round(record.progress ?? 0);
  if (isFinal(record)) percent = 100;
  else if (record.progress == null && record.stage === 'running' && record.kind === 'generate_world') percent = Math.min(95, Math.round((elapsedSecs(record, nowMs) / WORLD_EXPECTED_SECS) * 100));
  return percent;
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
  // Las operaciones de World Labs van a `/marble/…`: su consola no es «lo que se envía a Tripo».
  const consoleLabel = isWorldLabs(record) ? t('job.consoleWorldLabs') : t('job.console');
  const consoleBox = el('details', { className: 'job-console' }, el('summary', { text: consoleLabel }), pre);
  const result = el('div', { className: 'job-result' });
  const cancel = el('button', { className: 'btn-ghost btn-link', text: t('gen.cancelJob'), attrs: { type: 'button' }, on: { click: () => deps.onCancel?.(current.id) } });
  const node = el('div', { className: 'job-card' }, el('div', { className: 'job-head' }, title, meta, el('span', { className: 'spacer' }), stage, cancel), progress, consoleBox, result);
  let current = record;
  let rendered = false;
  // Sin porcentaje (World Labs), el reloj avanza aunque el fichero de trabajo no cambie.
  let ticker = null;

  function paintStage() {
    stage.textContent = stageText(current);
    bar.style.width = `${barPercent(current)}%`;
  }

  function update(next) {
    current = next;
    node.dataset.stage = next.stage;
    title.textContent = next.name;
    const cost = next.credits ?? next.estimatedCredits;
    const source = next.source === 'agent' ? ` · ${t('job.byAgent')}` : '';
    meta.textContent = `${next.kind}${cost ? ` · ${t('gen.spent', { spent: String(cost) })}` : ''}${source}`;
    paintStage();
    const ticking = isWorldLabs(next) && !isFinal(next) && next.progress == null;
    if (ticking && !ticker) {
      ticker = setInterval(() => (isFinal(current) ? (clearInterval(ticker), (ticker = null)) : paintStage()), 1000);
    } else if (!ticking && ticker) {
      clearInterval(ticker);
      ticker = null;
    }
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
