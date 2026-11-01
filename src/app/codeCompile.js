// =============================================================================
// DBV Unreal AI — Compilar el proyecto tras aplicar un cambio de código de la IA
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-22, R-31
//
// Al aceptar una propuesta que toca C++, la app compila (cómo, lo decide Rust: `compile.rs`). Si solo cambian `.cpp`,
// Live Coding con el editor abierto. Si cambian cabeceras o reglas del módulo, una tarjeta pide cerrar el editor y
// compila con UBT. Los errores vuelven a la IA solos como mucho dos veces seguidas; después decide la persona.

import { t } from '../ai/i18nBridge.js';
import * as api from './backend.js';
import { button, el } from './dom.js';

/** Reintentos automáticos seguidos tras un fallo de compilación. */
export const MAX_RETRIES = 2;

/** Mensaje para la IA con los errores (en inglés: lo lee el modelo). */
export function errorsForAi(outcome, attempt) {
  const errors = outcome.errors.length ? outcome.errors.map((line) => `- ${line}`).join('\n') : '- (no compiler errors were captured)';
  return `The project did not compile after applying your change (${outcome.summary}). Compiler errors:\n${errors}\nFix them with propose_changes, changing only what the errors need (automatic attempt ${attempt} of ${MAX_RETRIES}).`;
}

/**
 * @param {{
 *   getConnection: () => ({projectPath?: string, endpoint?: {port: number, path: string}, status?: string} | null),
 *   getAi: () => ({ask: (text: string, options?: object) => Promise<void>, panel: {addNode: (node: Node) => void, addNote: (text: string, kind?: string) => void}} | null),
 * }} deps
 */
export function createCodeCompiler(deps) {
  let retries = 0;

  function note(text, kind = 'info') {
    deps.getAi()?.panel.addNote(text, kind);
  }

  /** Errores de vuelta a la IA (sola mientras queden reintentos) o un botón para pedírselo a mano. */
  async function report(outcome) {
    if (outcome.ok) {
      retries = 0;
      note(t('code.compiled', { summary: outcome.summary }));
      return;
    }
    note(`${t('code.failed', { summary: outcome.summary, n: String(outcome.errors.length) })}\n${outcome.errors.join('\n')}`, 'error');
    if (retries < MAX_RETRIES) {
      retries += 1;
      await deps.getAi()?.ask(errorsForAi(outcome, retries), { label: t('code.retry', { n: String(retries), max: String(MAX_RETRIES) }) });
      return;
    }
    const again = button(t('code.askAgain'), async () => {
      retries = 1;
      again.disabled = true;
      await deps.getAi()?.ask(errorsForAi(outcome, retries), { label: t('code.retry', { n: String(retries), max: String(MAX_RETRIES) }) });
    });
    deps.getAi()?.panel.addNode(el('div', { className: 'ai-review code-compile' }, el('p', { text: t('code.giveUp') }), el('div', { className: 'row' }, again)));
  }

  async function liveCoding(connection) {
    note(t('code.compiling'));
    const compiled = await api.unrealCompileLive(connection.endpoint);
    if (compiled.ok) await report(compiled.value);
    else note(compiled.error.message, 'error');
  }

  /** Tarjeta para compilar con el editor cerrado: lo cierra la persona, que sabe si tiene algo sin guardar. */
  function editorClosedCard(connection, files) {
    const status = el('p', { className: 'hint' });
    const reopen = button(t('code.openEditor'), async () => {
      const opened = await api.unrealOpenProject(connection.projectPath);
      if (!opened.ok) status.textContent = opened.error.message;
    }, 'btn-ghost');
    reopen.hidden = true;
    const build = button(t('code.build'), async () => {
      build.disabled = true;
      status.textContent = t('code.building');
      const built = await api.unrealBuildProject(connection.projectPath);
      build.disabled = false;
      status.textContent = built.ok ? '' : built.error.message;
      if (built.ok) {
        build.hidden = built.value.ok;
        reopen.hidden = !built.value.ok;
        await report(built.value);
      }
    }, 'btn-primary');
    return el(
      'div',
      { className: 'ai-review code-compile' },
      el('strong', { text: t('code.closedTitle') }),
      el('p', { text: t('code.closedHint', { files: files.join(', ') }) }),
      status,
      el('div', { className: 'row' }, build, reopen),
    );
  }

  /**
   * Tras aplicar una propuesta: compila si tocó C++.
   * @param {string[]} paths relativas a la carpeta del proyecto.
   */
  async function afterApply(paths) {
    const connection = deps.getConnection();
    const planned = paths.length && connection?.projectPath ? await api.unrealCompilePlan(paths) : null;
    const mode = planned?.ok ? planned.value.mode : 'nothing';
    if (mode === 'liveCoding' && connection.status === 'online') await liveCoding(connection);
    else if (mode === 'liveCoding') note(t('code.noEditor'), 'error');
    else if (mode === 'editorClosed') deps.getAi()?.panel.addNode(editorClosedCard(connection, planned.value.files));
  }

  return { afterApply, retries: () => retries };
}
