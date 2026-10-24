// =============================================================================
// DBV Unreal AI — Vista Generar: texto o imagen → modelo con Tripo
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-15, REQ-27, REQ-28, REQ-29, REQ-32, RNF-04, RNF-IA.2
//
// Escenario D. Nada se lanza sin la tarjeta de coste (con el foco en «Cancelar», DESIGN.md), que enseña el
// total de todas las variantes y lo que se enviará a Tripo (RNF-IA.2, REQ-27). Lanza las MISMAS herramientas
// que la IA (`generate_3d_model`, `image_to_3d_model`), así que lo que ve el usuario aquí es lo que vería un
// agente. Abajo, los trabajos de la sesión con su consola, también los que lanzó un agente.

import { t } from '../ai/i18nBridge.js';
import { createJobCard, createJobWatcher } from '../generation/jobCard.js';
import * as api from './backend.js';
import { baseName, button, el } from './dom.js';

/** Tarjeta de coste común a la vista y a la herramienta de la IA. `true` si el usuario acepta. */
export async function confirmCost(confirm, request) {
  const variants = request.variants > 1 ? t('gen.variantsNote', { n: String(request.variants) }) : '';
  const choice = await confirm({
    title: t('gen.costTitle'),
    text: t('gen.costText', { name: request.name, operation: request.operation, cost: String(request.total), variants, spent: String(request.spent ?? 0), budget: request.budget > 0 ? String(request.budget) : '∞' }),
    detail: `POST ${request.endpoint}\n${JSON.stringify(request.params ?? {}, null, 2)}`,
    // El último botón recibe el foco: «Cancelar» va el último para que un Intro no gaste créditos.
    choices: [
      { key: 'accept', label: t('gen.accept', { cost: String(request.total) }), tone: 'primary' },
      { key: 'cancel', label: t('gen.cancel') },
    ],
  });
  return choice === 'accept';
}

/**
 * Prepara una herramienta de generación, enseña la tarjeta de coste y, si se acepta, la lanza. La usan Generar y la
 * Biblioteca (riggear, animar, imagen → 3D). Devuelve los ids de los trabajos, o `null` si no se lanzó.
 * @param {object} deps `{confirm, toast}`.
 */
export async function confirmAndLaunch(deps, tool, args) {
  const prepared = await api.generationPrepare(tool, args);
  let ids = null;
  if (!prepared.ok) deps.toast(prepared.error.message, 'error');
  else {
    const status = await api.generationStatus();
    if (status.ok && !status.value.hasKey?.tripo) deps.toast(t('gen.noKey'), 'error');
    else if (await confirmCost(deps.confirm, { ...prepared.value, spent: status.value?.spent ?? 0, budget: status.value?.budget ?? 0 })) {
      const launched = await api.launch(tool, args);
      if (launched.ok) ids = launched.value;
      else deps.toast(launched.error.message, 'error');
    }
  }
  return ids;
}

/** Argumentos de la herramienta a partir del formulario. */
export function formArgs({ mode, prompt, image, name, variants, faceLimit, lowPoly, textureQuality }) {
  const options = textureQuality && textureQuality !== 'standard' ? { texture_quality: textureQuality } : undefined;
  const common = { name: name.trim(), variants: Number(variants) || 1, ...(options ? { options } : {}) };
  const args =
    mode === 'image'
      ? { image, ...common }
      : { prompt: prompt.trim(), ...common, ...(Number(faceLimit) ? { face_limit: Number(faceLimit) } : {}), ...(lowPoly ? { smart_low_poly: true } : {}) };
  return { tool: mode === 'image' ? 'image_to_3d_model' : 'generate_3d_model', args };
}

/**
 * @param {HTMLElement} host
 * @param {object} deps `{confirm, toast, onSpent, openLibrary, suggestName?(prompt) → Promise<string|null>, renderResult?(record, slot)}`.
 */
export function createGenerateView(host, deps) {
  const cards = new Map();
  const jobsList = el('div', { className: 'stack job-list' });
  const watcher = createJobWatcher({
    list: api.generationJobs,
    onRecord(record) {
      const known = cards.get(record.id);
      if (known) known.update(record);
      else {
        const card = createJobCard(record, { onCancel: api.generationCancel, renderResult: deps.renderResult });
        cards.set(record.id, card);
        jobsList.prepend(card.node);
      }
      if (record.stage === 'done') deps.onSpent();
    },
  });

  async function render() {
    const status = await api.generationStatus();
    const hasKey = status.ok && status.value.hasKey?.tripo;
    let mode = 'text';
    let image = '';
    const prompt = el('textarea', { attrs: { rows: '3', 'aria-label': t('gen.prompt') } });
    const name = el('input', { attrs: { type: 'text', 'aria-label': t('gen.name') } });
    const variants = el('select', { attrs: { 'aria-label': t('gen.variants') } }, ...[1, 2, 3, 4].map((n) => el('option', { text: String(n), attrs: { value: String(n) } })));
    const faces = el('input', { attrs: { type: 'number', min: '500', max: '200000', step: '500', 'aria-label': t('gen.faceLimit') } });
    const lowPoly = el('input', { attrs: { type: 'checkbox' } });
    const quality = el('select', { attrs: { 'aria-label': t('gen.textureQuality') } }, ...['standard', 'detailed', 'extreme'].map((q) => el('option', { text: t(`gen.quality.${q}`), attrs: { value: q } })));
    const estimate = el('span', { className: 'cost' });
    const balance = el('span', { className: 'hint' });
    const imageName = el('span', { className: 'mono' });
    const imageThumb = el('div', { className: 'ref-thumb hidden' });

    const values = () => ({ mode, prompt: prompt.value, image, name: name.value, variants: variants.value, faceLimit: faces.value, lowPoly: lowPoly.checked, textureQuality: quality.value });
    let estimateTimer = null;
    function refreshEstimate() {
      clearTimeout(estimateTimer);
      estimateTimer = setTimeout(async () => {
        const { tool, args } = formArgs({ ...values(), name: name.value.trim() || '—' });
        const ready = mode === 'image' ? Boolean(image) : Boolean(prompt.value.trim());
        const prepared = ready ? await api.generationPrepare(tool, args) : null;
        estimate.textContent = prepared?.ok ? t('gen.estimate', { cost: String(prepared.value.total), spent: String(status.value?.spent ?? 0) }) : '';
      }, 250);
    }
    for (const input of [prompt, name, variants, faces, lowPoly, quality]) input.addEventListener('input', refreshEstimate);

    async function suggest() {
      const text = mode === 'image' ? baseName(image) : prompt.value.trim();
      if (!text) return prompt.focus();
      const fromAi = await deps.suggestName?.(text).catch(() => null);
      const fallback = fromAi ? null : await api.generationSuggestName(text);
      name.value = fromAi || (fallback?.ok ? fallback.value : '');
      return refreshEstimate();
    }

    const textPane = el('div', { className: 'stack' }, el('label', {}, t('gen.prompt'), prompt), el('div', { className: 'row' }, el('label', {}, t('gen.faceLimit'), faces), el('label', { className: 'row' }, lowPoly, t('gen.lowPoly'))));
    const imagePane = el(
      'div',
      { className: 'stack hidden' },
      el('p', { className: 'hint', text: t('gen.imageHint') }),
      el(
        'div',
        { className: 'row' },
        button(t('gen.pickImage'), async () => {
          const picked = await window.__TAURI__.dialog.open({ multiple: false, filters: [{ name: t('gen.images'), extensions: ['png', 'jpg', 'jpeg', 'webp'] }] });
          if (!picked) return;
          image = String(picked);
          imageName.textContent = baseName(image);
          imageThumb.replaceChildren(el('img', { attrs: { src: window.__TAURI__.core.convertFileSrc(image), alt: baseName(image) } }));
          imageThumb.classList.remove('hidden');
          refreshEstimate();
        }),
        imageName,
      ),
      imageThumb,
    );
    const tabs = ['text', 'image'].map((key) =>
      el('button', {
        className: 'segmented-btn',
        text: t(`gen.mode.${key}`),
        attrs: { type: 'button', 'aria-pressed': String(key === mode), 'data-mode': key },
        on: {
          click: () => {
            mode = key;
            for (const tab of tabs) tab.setAttribute('aria-pressed', String(tab.dataset.mode === key));
            textPane.classList.toggle('hidden', mode !== 'text');
            imagePane.classList.toggle('hidden', mode !== 'image');
            refreshEstimate();
          },
        },
      }),
    );

    const start = button(
      t('gen.start'),
      async () => {
        if (mode === 'text' && !prompt.value.trim()) return prompt.focus();
        if (mode === 'image' && !image) return deps.toast(t('gen.noImage'), 'error');
        if (!name.value.trim()) await suggest();
        const { tool, args } = formArgs(values());
        const ids = await confirmAndLaunch(deps, tool, args);
        if (ids) {
          deps.onSpent();
          watcher.poke();
        }
        return null;
      },
      'btn-primary',
    );
    if (!hasKey) start.disabled = true;
    if (hasKey) {
      api.generationBalance().then((result) => {
        balance.textContent = result.ok ? t('gen.balance', { balance: String(result.value.balance), frozen: String(result.value.frozen) }) : '';
      });
    }
    host.replaceChildren(
      el('div', { className: 'view-header' }, el('h2', { text: t('gen.title') }), el('span', { className: 'spacer' }), balance),
      el(
        'div',
        { className: 'card stack' },
        hasKey ? null : el('p', { className: 'check-fail', text: t('gen.noKey') }),
        el('div', { className: 'segmented', attrs: { role: 'group', 'aria-label': t('gen.mode.label') } }, ...tabs),
        textPane,
        imagePane,
        el('div', { className: 'row' }, el('label', {}, t('gen.name'), el('div', { className: 'row' }, name, button(t('gen.suggestName'), suggest, 'btn-ghost'))), el('label', {}, t('gen.variants'), variants), el('label', {}, t('gen.textureQuality'), quality)),
        el('div', { className: 'row' }, start, estimate),
      ),
      el('div', { className: 'view-header' }, el('h3', { text: t('gen.jobs') })),
      jobsList,
    );
    watcher.poke();
  }
  return { render, watch: () => watcher.poke() };
}
