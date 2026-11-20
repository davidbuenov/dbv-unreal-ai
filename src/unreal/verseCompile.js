// =============================================================================
// DBV Unreal AI — Aplicar y compilar Verse en UEFN, con la IA corrigiendo en bucle
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-127
//
// Al aplicar una propuesta que toca `.verse` con UEFN conectado, la app compila con `VerseToolset.BuildAll` y enseña
// el resultado. Los errores vuelven a la IA solos hasta el tope de intentos (5 por defecto, de 1 a 10); cada intento
// sigue pasando por la aprobación de la propuesta. «Compila» solo con la lista vacía. Si el mismo error vuelve tal cual
// tras un arreglo, se compila otra vez desde cero; si sigue, se sugiere reiniciar UEFN (su compilación y su LSP a veces
// dan errores viejos).

import { t } from '../ai/i18nBridge.js';
import { button, el } from '../app/dom.js';

export const DEFAULT_ATTEMPTS = 5;
const ATTEMPTS_KEY = 'dbv-unreal-ai:verseAttempts';

/** Tope de intentos guardado (1..10), o el de por defecto. */
export function maxAttempts(storage = globalThis.localStorage) {
  let stored = NaN;
  try {
    stored = Number(storage?.getItem(ATTEMPTS_KEY));
  } catch {
    // Sin almacenamiento local se usa el de por defecto.
  }
  return Number.isInteger(stored) && stored >= 1 && stored <= 10 ? stored : DEFAULT_ATTEMPTS;
}

/** Firma de los errores para saber si un arreglo no cambió nada. */
export const errorSignature = (build) => build.errors.map((e) => `${e.versePath}:${e.line}:${e.code}:${e.message}`).join('|');

/** `Content/a.verse:9 · 3509 · mensaje` (línea desde 1). */
export const diagnosticLine = (d) => `${d.relative || d.versePath}:${d.line} · ${d.code} · ${d.message}`;

/** Errores para la IA (en inglés: lo lee el modelo). */
export function errorsForAi(build, attempt, max) {
  const errors = build.errors.map((d) => `- ${diagnosticLine(d)}`).join('\n');
  return `UEFN did not compile your Verse (VerseToolset BuildAll, attempt ${attempt} of ${max}). Compiler errors:\n${errors}\nRead verse_reference "error-codes" for these codes if needed, check signatures with verse_api_search, and fix them with propose_changes, changing only what the errors need.`;
}

/**
 * Tarjeta del resultado (DESIGN.md v0.7.0): badge `Verse`, «Compila» en verde solo con la lista vacía, errores
 * `fichero:línea · código · mensaje`, el contador de intentos y, si hace falta, el aviso de API experimental.
 * @param {{ok: boolean, sessionRunning?: boolean, errors: object[], warnings: object[], experimental: boolean} | null} build `null` = sin MCP.
 * @param {{attempt: number, max: number, onOpen?: (relative: string) => void, actions?: Node[]}} options
 */
export function verseBuildCard(build, { attempt, max, onOpen, actions = [] }) {
  const badge = el('span', { className: 'badge badge--accent', text: 'Verse' });
  let body;
  if (!build) body = [el('p', { className: 'warning', text: t('verse.unverified') })];
  else if (build.sessionRunning) body = [el('p', { className: 'warning', text: t('verse.sessionRunning') })];
  else if (build.ok) body = [el('p', { className: 'check-ok', text: t('verse.compiles') })];
  else {
    const items = build.errors.map((d) => {
      const link = el('a', { className: 'mono', text: diagnosticLine(d), attrs: { href: '#' } });
      link.addEventListener('click', (event) => {
        event.preventDefault();
        if (d.relative) onOpen?.(d.relative);
      });
      return el('li', {}, link);
    });
    body = [el('p', { className: 'error', text: t('verse.failed', { n: String(build.errors.length) }) }), el('ul', { className: 'verse-errors' }, items)];
  }
  const experimental = build?.experimental ? el('p', { className: 'warning', text: t('verse.experimental') }) : null;
  const counter = build && !build.ok && !build.sessionRunning ? el('p', { className: 'hint', text: t('verse.attempt', { n: String(attempt), max: String(max) }) }) : null;
  return el('div', { className: 'ai-review code-compile' }, el('strong', {}, badge, ' ', t('verse.title')), ...body, experimental, counter, actions.length ? el('div', { className: 'row' }, ...actions) : null);
}

/**
 * @param {{
 *   getConnection: () => ({uefn?: boolean, status?: string, projectPath?: string, endpoint?: {port: number, path: string}} | null),
 *   getAi: () => ({ask: (text: string, options?: object) => Promise<void>, panel: {addNode: (node: Node) => void}} | null),
 *   build: (endpoint: object, project: string) => Promise<{ok: boolean, value?: object, error?: {message: string}}>,
 *   pushVerse?: () => Promise<void>,
 *   openFile?: (relative: string) => void,
 *   storage?: Storage,
 * }} deps
 */
export function createVerseCompiler(deps) {
  let attempt = 0;
  let lastSignature = '';
  let rebuiltOnce = false;

  const addCard = (build, actions = []) => deps.getAi()?.panel.addNode(verseBuildCard(build, { attempt: Math.max(attempt, 1), max: maxAttempts(deps.storage), onOpen: deps.openFile, actions }));

  async function compile(connection) {
    const result = await deps.build(connection.endpoint, connection.projectPath);
    return result.ok ? result.value : { ok: false, errors: [], warnings: [], experimental: false, failure: result.error?.message ?? '' };
  }

  /** Lo que pasa tras un resultado con errores: a la IA, otra compilación, o parar en el tope. */
  /** El mismo error tras un arreglo: se compila otra vez desde cero; si sigue igual, se sugiere reiniciar UEFN. */
  async function rebuild(connection, signature) {
    rebuiltOnce = true;
    const again = await compile(connection);
    if (again.ok) finish(again);
    else if (errorSignature(again) === signature) {
      addCard(again, []);
      deps.getAi()?.panel.addNode(el('p', { className: 'warning', text: t('verse.restartUefn') }));
    } else await onErrors(connection, again);
  }

  /** Lo que pasa tras un resultado con errores: otra compilación, parar en el tope o devolverlos a la IA. */
  async function onErrors(connection, build) {
    const max = maxAttempts(deps.storage);
    const signature = errorSignature(build);
    const repeated = signature === lastSignature && attempt > 0;
    lastSignature = signature;
    if (repeated && !rebuiltOnce) await rebuild(connection, signature);
    else if (attempt >= max) {
      addCard(build);
      deps.getAi()?.panel.addNode(el('p', { className: 'hint', text: t('verse.giveUp', { max: String(max) }) }));
    } else {
      attempt += 1;
      addCard(build);
      await deps.getAi()?.ask(errorsForAi(build, attempt, max), { label: t('verse.retry', { n: String(attempt), max: String(max) }) });
    }
  }

  function finish(build) {
    addCard(build);
    attempt = 0;
    lastSignature = '';
    rebuiltOnce = false;
  }

  /**
   * Tras aplicar una propuesta: compila si tocó `.verse` con UEFN conectado.
   * @param {string[]} paths relativas al proyecto.
   */
  async function afterApply(paths) {
    const connection = deps.getConnection();
    if (!connection?.uefn || !paths.some((p) => /\.verse$/i.test(p))) return;
    if (connection.status !== 'online' || !connection.endpoint) return void addCard(null);
    const build = await compile(connection);
    if (build.failure) deps.getAi()?.panel.addNode(el('p', { className: 'error', text: build.failure }));
    else if (build.sessionRunning) addCard(build, deps.pushVerse ? [button(t('verse.pushChanges'), deps.pushVerse, 'btn-primary')] : []);
    else if (build.ok) finish(build);
    else await onErrors(connection, build);
  }

  return { afterApply, attempts: () => attempt };
}
