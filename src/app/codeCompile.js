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
import { sameProjectPath } from './projectView.js';

/** Reintentos automáticos seguidos tras un fallo de compilación. */
export const MAX_RETRIES = 2;

/**
 * Dónde escucha AHORA el editor del proyecto, o `null` si no está abierto o aún arranca. No vale la conexión guardada:
 * al reabrir el editor el puerto puede ser otro (2026-11-01: la app compiló contra el 8000 y el editor estaba en el 8001).
 */
export async function liveEndpoint(projectPath) {
  const editors = await api.unrealEditors();
  const own = editors.ok ? editors.value.find((e) => sameProjectPath(e.editor?.project ?? e.project?.path, projectPath)) : null;
  return own?.listening ? own.mcp : null;
}

/** Mensaje para la IA con los errores (en inglés: lo lee el modelo). */
export function errorsForAi(outcome, attempt) {
  const errors = outcome.errors.length ? outcome.errors.map((line) => `- ${line}`).join('\n') : '- (no compiler errors were captured)';
  return `The project did not compile after applying your change (${outcome.summary}). Compiler errors:\n${errors}\nFix them with propose_changes, changing only what the errors need (automatic attempt ${attempt} of ${MAX_RETRIES}).`;
}

/**
 * @param {{
 *   getConnection: () => ({projectPath?: string, endpoint?: {port: number, path: string}, status?: string} | null),
 *   getAi: () => ({ask: (text: string, options?: object) => Promise<void>, panel: {addNode: (node: Node) => void, addNote: (text: string, kind?: string) => void}} | null),
 *   reconnect?: (projectPath: string) => Promise<boolean>,
 *   pollMs?: number,
 * }} deps
 */
export function createCodeCompiler(deps) {
  let retries = 0;
  const pollMs = deps.pollMs ?? 3000;

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

  /** Un aviso con «Reintentar» (el editor aún arranca, o Live Coding no respondió). */
  function failedWithRetry(message, retry) {
    const again = button(t('code.retryNow'), () => {
      again.disabled = true;
      retry();
    }, 'btn-ghost');
    deps.getAi()?.panel.addNode(el('div', { className: 'ai-review code-compile' }, el('p', { className: 'hint', text: message }), el('div', { className: 'row' }, again)));
  }

  async function liveCoding(connection) {
    const endpoint = await liveEndpoint(connection.projectPath);
    if (!endpoint) {
      failedWithRetry(t('code.noEditor'), () => liveCoding(connection));
      return;
    }
    note(t('code.compiling'));
    const compiled = await api.unrealCompileLive(endpoint);
    if (compiled.ok) await report(compiled.value);
    else failedWithRetry(compiled.error.message, () => liveCoding(connection));
  }

  /** Tras reabrir el editor, espera a que su servidor MCP escuche (hasta 5 min) y conecta la app con él. */
  async function reconnectWhenReady(projectPath, status) {
    let endpoint = null;
    for (let waited = 0; !endpoint && waited < 300_000; waited += pollMs) {
      await new Promise((resolve) => setTimeout(resolve, pollMs));
      endpoint = await liveEndpoint(projectPath);
    }
    const connected = endpoint ? await deps.reconnect?.(projectPath) : false;
    status.textContent = connected ? t('code.reconnected') : endpoint ? '' : t('code.notReady');
  }

  /** Tarjeta para compilar con el editor cerrado: lo cierra la persona, que sabe si tiene algo sin guardar. */
  function editorClosedCard(connection, files) {
    const status = el('p', { className: 'hint' });
    const reopen = button(t('code.openEditor'), async () => {
      reopen.disabled = true;
      const opened = await api.unrealOpenProject(connection.projectPath);
      status.textContent = opened.ok ? t('code.opening') : opened.error.message;
      if (opened.ok) await reconnectWhenReady(connection.projectPath, status);
      else reopen.disabled = false;
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
    if (mode === 'liveCoding') await liveCoding(connection);
    else if (mode === 'editorClosed') deps.getAi()?.panel.addNode(editorClosedCard(connection, planned.value.files));
  }

  return { afterApply, retries: () => retries };
}
