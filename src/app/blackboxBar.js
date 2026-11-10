// =============================================================================
// DBV Unreal AI — Panel de grabación de la caja negra («Marcar momento» y «Terminar»)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-94
//
// Mientras se graba, el turno de la IA ocupa la conversación y la persona no puede escribir: este panel, siempre a la
// vista, le deja marcar un momento y terminar. Se enciende solo cuando hay una grabación escribiéndose en el proyecto
// (la app lo pregunta cada par de segundos leyendo ficheros), también si la lanzó un agente por el `--mcp`.

import { button, el } from './dom.js';

/** `mm:ss` de un número de segundos. */
export function clock(seconds) {
  const whole = Math.max(0, Math.floor(seconds));
  const out = `${String(Math.floor(whole / 60)).padStart(2, '0')}:${String(whole % 60).padStart(2, '0')}`;
  return out;
}

/**
 * @param {{
 *   active: () => Promise<{ok: boolean, value?: {id: string, startedAt: string, marks: number} | null}>,
 *   run: (name: string, args: object) => Promise<{ok: boolean, value?: string, error?: {message: string}}>,
 *   t: (key: string, vars?: object) => string,
 *   enabled?: () => boolean,
 *   nowMs?: () => number,
 * }} deps
 */
export function createBlackboxBar({ active, run, t, enabled = () => true, nowMs = () => Date.now() }) {
  const status = el('span', { className: 'blackbox-bar__status' });
  const note = el('span', { className: 'blackbox-bar__note hint' });
  const markButton = button(t('blackbox.mark'), () => mark(), 'btn-primary button--compact');
  const finishButton = button(t('blackbox.finish'), () => finish(), 'button--compact');
  const node = el(
    'div',
    { className: 'blackbox-bar hidden', attrs: { role: 'status', 'aria-live': 'polite' } },
    el('span', { className: 'blackbox-bar__dot', attrs: { 'aria-hidden': 'true' }, text: '⏺' }),
    status,
    markButton,
    finishButton,
    note,
  );
  let current = null;
  let firstSeen = 0;
  let busy = false;
  let pollTimer = null;
  let clockTimer = null;

  function elapsed() {
    const started = Date.parse(current?.startedAt ?? '');
    const from = Number.isFinite(started) && started <= nowMs() ? started : firstSeen;
    return (nowMs() - from) / 1000;
  }

  function render() {
    node.classList.toggle('hidden', !current);
    if (current) status.textContent = `${t('blackbox.recording')} · ${clock(elapsed())} · ${t('blackbox.marks', { n: current.marks })}`;
    markButton.disabled = busy || !current;
    finishButton.disabled = busy || !current;
  }

  /** Mira si hay una grabación en marcha y actualiza el panel. */
  async function refresh() {
    let info = null;
    if (enabled()) {
      const result = await active();
      info = result?.ok && result.value ? result.value : null;
    }
    if (info && (!current || current.id !== info.id)) firstSeen = nowMs();
    if (!info) note.textContent = '';
    current = info;
    render();
  }

  async function act(name, args, pending, done) {
    if (busy || !current) return;
    busy = true;
    note.textContent = pending;
    render();
    const result = await run(name, args);
    busy = false;
    note.textContent = result.ok ? done : result.error?.message ?? String(result.error ?? '');
    await refresh();
  }

  const mark = () => act('session_mark', { label: t('blackbox.moment', { n: (current?.marks ?? 0) + 1 }) }, '', t('blackbox.marked', { n: (current?.marks ?? 0) + 1 }));
  const finish = () => act('session_stop', {}, t('blackbox.finishing'), t('blackbox.finished'));

  return {
    node,
    refresh,
    /** Empieza a preguntar cada `intervalMs` (y a mover el reloj cada segundo). */
    start(intervalMs = 2000) {
      if (pollTimer) return;
      pollTimer = setInterval(refresh, intervalMs);
      clockTimer = setInterval(render, 1000);
      refresh();
    },
    stop() {
      clearInterval(pollTimer);
      clearInterval(clockTimer);
      pollTimer = null;
      clockTimer = null;
    },
  };
}
