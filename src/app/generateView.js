// =============================================================================
// DBV Unreal AI — Vista Generar: texto → modelo con Tripo
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-15, RNF-04, RNF-IA.2
//
// Escenario D. Nada se lanza sin la tarjeta de coste (con el foco en «Cancelar», DESIGN.md), que
// además avisa de que el prompt sale hacia Tripo (RNF-IA.2).

import { t } from '../ai/i18nBridge.js';
import * as api from './backend.js';
import { button, el } from './dom.js';

/** Tarjeta de coste común a la vista y a la herramienta de la IA. `true` si el usuario acepta. */
export async function confirmCost(confirm, { name, prompt, cost, spent, budget }) {
  const choice = await confirm({
    title: t('gen.costTitle'),
    text: t('gen.costText', { name: name || prompt, cost: String(cost), spent: String(spent), budget: budget > 0 ? String(budget) : '∞' }),
    // El último botón recibe el foco: «Cancelar» va el último para que un Intro no gaste créditos.
    choices: [
      { key: 'accept', label: t('gen.accept', { cost: String(cost) }), tone: 'primary' },
      { key: 'cancel', label: t('gen.cancel') },
    ],
  });
  return choice === 'accept';
}

/**
 * @param {HTMLElement} host
 * @param {object} deps `{confirm, toast, onSpent, openLibrary}`.
 */
export function createGenerateView(host, deps) {
  async function render() {
    const status = await api.generationStatus();
    const hasKey = status.ok && status.value.hasKey?.tripo;
    const prompt = el('textarea', { attrs: { rows: '3', 'aria-label': t('gen.prompt') } });
    const name = el('input', { attrs: { type: 'text', 'aria-label': t('gen.name') } });
    const faces = el('input', { attrs: { type: 'number', min: '500', max: '200000', step: '500', 'aria-label': t('gen.faceLimit') } });
    const lowPoly = el('input', { attrs: { type: 'checkbox' } });
    const progressBar = el('div');
    const progressText = el('p', { className: 'hint', attrs: { 'aria-live': 'polite' } });
    const progress = el('div', { className: 'stack hidden' }, el('div', { className: 'progress' }, progressBar), progressText);
    let jobId = null;
    const cancel = button(t('gen.cancelJob'), () => jobId && api.generationCancel(jobId), 'btn-ghost hidden');
    const start = button(
      t('gen.start'),
      async () => {
        const text = prompt.value.trim();
        if (!text) return prompt.focus();
        const current = await api.generationStatus();
        const ok = await confirmCost(deps.confirm, { name: name.value.trim(), prompt: text, cost: current.value?.tripoTextToModel ?? 30, spent: current.value?.spent ?? 0, budget: current.value?.budget ?? 0 });
        if (!ok) return null;
        const started = await api.generationTripoStart({ prompt: text, name: name.value.trim() || null, faceLimit: Number(faces.value) || null, smartLowPoly: lowPoly.checked || null, confirmCost: true });
        if (!started.ok) return deps.toast(started.error.message, 'error');
        jobId = started.value;
        start.disabled = true;
        cancel.classList.remove('hidden');
        progress.classList.remove('hidden');
        deps.onSpent();
        const end = await api.waitJob(jobId, 12 * 60 * 1000, (p) => {
          progressBar.style.width = `${Math.round(p.progress ?? 0)}%`;
          progressText.textContent = t(`gen.stage.${p.stage}`, { progress: String(Math.round(p.progress ?? 0)), error: p.error ?? '' });
        });
        progressText.textContent = t(`gen.stage.${end.stage}`, { progress: '100', error: end.error ?? '' });
        start.disabled = false;
        cancel.classList.add('hidden');
        jobId = null;
        if (end.stage === 'done') deps.openLibrary();
        return null;
      },
      'btn-primary',
    );
    if (!hasKey) start.disabled = true;
    host.replaceChildren(
      el('div', { className: 'view-header' }, el('h2', { text: t('gen.title') })),
      el(
        'div',
        { className: 'card stack' },
        hasKey ? null : el('p', { className: 'check-fail', text: t('gen.noKey') }),
        el('label', {}, t('gen.prompt'), prompt),
        el('div', { className: 'row' }, el('label', {}, t('gen.name'), name), el('label', {}, t('gen.faceLimit'), faces), el('label', { className: 'row' }, lowPoly, t('gen.lowPoly'))),
        el('div', { className: 'row' }, start, cancel, status.ok ? el('span', { className: 'cost', text: `≈ ${status.value.tripoTextToModel} · ${t('gen.spent', { spent: String(status.value.spent) })}` }) : null),
        progress,
        el('p', { className: 'hint', text: t('gen.imageNote') }),
      ),
    );
  }
  return { render };
}
