// =============================================================================
// DBV Unreal AI — Tarjeta de aprobación de acciones en el editor
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-12
//
// Lo que la IA quiere hacer, ANTES de hacerlo: herramienta, proyecto, motivo y el detalle
// completo (el código Python entero o los argumentos). Todo se pinta con `textContent`: el
// código o los nombres que manda el modelo nunca se interpretan como HTML. El foco inicial
// va a «Denegar» para que un Intro accidental no apruebe nada (DESIGN.md).

import { t } from '../ai/i18nBridge.js';

/** Líneas visibles del código antes de plegarlo. */
export const VISIBLE_LINES = 20;

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * @param {object} request `{kind: 'python'|'tool', toolset, tool, code?, args?, purpose?, findings?, validated?}`.
 * @returns {{node: HTMLElement, choice: Promise<'once'|'always'|'deny'>}}
 */
export function createApprovalCard(request) {
  const card = el('div', 'unreal-approval');
  card.setAttribute('role', 'group');
  const title = request.kind === 'python' ? t('unreal.approval.python') : t('unreal.approval.tool');
  card.setAttribute('aria-label', title);
  card.append(el('div', 'unreal-approval__title', title));
  card.append(el('div', 'unreal-approval__target mono', `${request.toolset} · ${request.tool}`));
  if (request.purpose) card.append(el('p', 'unreal-approval__purpose', request.purpose));

  if (request.kind === 'python') {
    const lines = String(request.code ?? '').split('\n');
    const pre = el('pre', 'unreal-approval__code mono', lines.length > VISIBLE_LINES ? lines.slice(0, VISIBLE_LINES).join('\n') : request.code);
    card.append(pre);
    if (lines.length > VISIBLE_LINES) {
      const more = el('button', 'btn-link', t('unreal.approval.showAll', { n: String(lines.length) }));
      more.type = 'button';
      more.addEventListener('click', () => {
        pre.textContent = request.code;
        more.remove();
      });
      card.append(more);
    }
    if (!request.validated) card.append(el('p', 'unreal-approval__warning', t('unreal.approval.notValidated')));
    for (const finding of request.findings?.findings ?? []) {
      card.append(el('p', 'unreal-approval__warning', `${t(`unreal.finding.${finding.kind}`)} · ${t('unreal.line')} ${finding.line}: ${finding.message}`));
    }
  } else {
    card.append(el('pre', 'unreal-approval__code mono', JSON.stringify(request.args ?? {}, null, 2)));
  }

  const actions = el('div', 'unreal-approval__actions');
  const allow = el('button', 'btn-primary', t('unreal.approval.allow'));
  const always = el('button', 'btn-secondary', t('unreal.approval.always'));
  const deny = el('button', 'btn-ghost', t('unreal.approval.deny'));
  for (const button of [allow, always, deny]) button.type = 'button';
  actions.append(allow, always, deny);
  card.append(actions);

  const choice = new Promise((resolve) => {
    const decide = (value, label) => {
      for (const button of [allow, always, deny]) button.disabled = true;
      card.classList.add(`unreal-approval--${value}`);
      actions.replaceWith(el('div', 'unreal-approval__decision', label));
      resolve(value);
    };
    allow.addEventListener('click', () => decide('once', t('unreal.approval.allowed')));
    always.addEventListener('click', () => decide('always', t('unreal.approval.allowedAlways')));
    deny.addEventListener('click', () => decide('deny', t('unreal.approval.denied')));
  });
  queueMicrotask(() => deny.focus());
  return { node: card, choice };
}
