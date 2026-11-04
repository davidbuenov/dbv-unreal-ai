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
import { brandLogo } from './brandHeader.js';
import { serviceOf, withSessionCap } from '../generation/generationTools.js';
import { baseName, button, el } from './dom.js';
import { openHelp } from './helpModal.js';

/** Tarjeta de coste común a la vista y a la herramienta de la IA. `true` si el usuario acepta. */
export async function confirmCost(confirm, request) {
  const variants = request.variants > 1 ? t('gen.variantsNote', { n: String(request.variants) }) : '';
  const choice = await confirm({
    title: t('gen.costTitle'),
    text: t(serviceOf(request) === 'worldlabs' ? 'gen.costTextWorldLabs' : 'gen.costText', { name: request.name, operation: request.operation, cost: String(request.total), variants, spent: String(request.spent ?? 0), budget: request.budget > 0 ? String(request.budget) : '∞' }),
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
    const service = tool === 'generate_world' || tool === 'export_world_mesh' ? 'worldlabs' : 'tripo';
    if (status.ok && !status.value.hasKey?.[service]) {
      deps.toast(service === 'worldlabs' ? t('gen.noKeyWorldLabs') : t('gen.noKey'), 'error');
    } else if (await confirmCost(deps.confirm, withSessionCap(prepared.value, status.value))) {
      const launched = await api.generationLaunch(tool, args);
      if (launched.ok) ids = launched.value;
      else deps.toast(launched.error.message, 'error');
    }
  }
  return ids;
}

/** Argumentos de la herramienta a partir del formulario. */
export function formArgs({ mode, prompt, image, name, variants, faceLimit, lowPoly, textureQuality, worldModel }) {
  let result = null;
  if (mode === 'world') {
    const args = {
      prompt: (prompt || '').trim(),
      name: (name || '').trim(),
      ...(worldModel ? { model: worldModel } : {}),
      ...(image ? { image } : {}),
    };
    result = { tool: 'generate_world', args };
  } else {
    const options = textureQuality && textureQuality !== 'standard' ? { texture_quality: textureQuality } : undefined;
    const common = { name: (name || '').trim(), variants: Number(variants) || 1, ...(options ? { options } : {}) };
    const args =
      mode === 'image'
        ? { image, ...common }
        : { prompt: (prompt || '').trim(), ...common, ...(Number(faceLimit) ? { face_limit: Number(faceLimit) } : {}), ...(lowPoly ? { smart_low_poly: true } : {}) };
    result = { tool: mode === 'image' ? 'image_to_3d_model' : 'generate_3d_model', args };
  }
  return result;
}

/**
 * @param {HTMLElement} host
 * @param {object} deps `{confirm, toast, onSpent, openLibrary, suggestName?(prompt) → Promise<string|null>, renderResult?(record, slot),
 *   improvePrompt?(service, idea) → Promise<{kind, text?}>, improveAvailable?() → Promise<boolean>}`.
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
    let mode = 'text';
    let image = '';
    const prompt = el('textarea', { attrs: { rows: '3', 'aria-label': t('gen.prompt') } });
    const name = el('input', { attrs: { type: 'text', 'aria-label': t('gen.name') } });
    const variants = el('select', { attrs: { 'aria-label': t('gen.variants') } }, ...[1, 2, 3, 4].map((n) => el('option', { text: String(n), attrs: { value: String(n) } })));
    const faces = el('input', { attrs: { type: 'number', min: '500', max: '200000', step: '500', 'aria-label': t('gen.faceLimit') } });
    const lowPoly = el('input', { attrs: { type: 'checkbox' } });
    const quality = el('select', { attrs: { 'aria-label': t('gen.textureQuality') } }, ...['standard', 'detailed', 'extreme'].map((q) => el('option', { text: t(`gen.quality.${q}`), attrs: { value: q } })));
    const worldPrompt = el('textarea', { attrs: { rows: '3', 'aria-label': t('gen.prompt') } });
    const worldModel = el('select', { attrs: { 'aria-label': t('gen.worldModel') } },
      el('option', { text: t('gen.worldModel.full'), attrs: { value: 'marble-1.1' } }),
      el('option', { text: t('gen.worldModel.draft'), attrs: { value: 'marble-1.0-draft' } }),
    );

    const estimate = el('span', { className: 'cost' });
    const balance = el('span', { className: 'hint' });
    const keyWarning = el('p', { className: 'check-fail hidden' });
    const imageName = el('span', { className: 'mono' });
    const imageThumb = el('div', { className: 'ref-thumb hidden' });

    const values = () => ({
      mode,
      prompt: mode === 'world' ? worldPrompt.value : prompt.value,
      image,
      name: name.value,
      variants: variants.value,
      faceLimit: faces.value,
      lowPoly: lowPoly.checked,
      textureQuality: quality.value,
      worldModel: worldModel.value,
    });

    let estimateTimer = null;
    function refreshEstimate() {
      clearTimeout(estimateTimer);
      estimateTimer = setTimeout(async () => {
        const { tool, args } = formArgs({ ...values(), name: name.value.trim() || '—' });
        const ready = mode === 'image' ? Boolean(image) : Boolean((mode === 'world' ? worldPrompt.value : prompt.value).trim());
        const prepared = ready ? await api.generationPrepare(tool, args) : null;
        estimate.textContent = prepared?.ok && prepared.value?.total != null ? t('gen.estimate', { cost: String(prepared.value.total), spent: String(withSessionCap(prepared.value, status.value).spent) }) : '';
      }, 250);
    }
    for (const input of [prompt, worldPrompt, name, variants, faces, lowPoly, quality, worldModel]) input.addEventListener('input', refreshEstimate);

    async function suggest() {
      const currentPrompt = mode === 'world' ? worldPrompt.value.trim() : prompt.value.trim();
      const text = mode === 'image' ? baseName(image) : currentPrompt;
      if (!text) {
        (mode === 'world' ? worldPrompt : prompt).focus();
      } else {
        const fromAi = await deps.suggestName?.(text).catch(() => null);
        const fallback = fromAi ? null : await api.generationSuggestName(text);
        name.value = fromAi || (fallback?.ok ? fallback.value : '');
        refreshEstimate();
      }
    }

    /**
     * «Mejorar con IA» bajo un prompt: sustituye el texto por la descripción de la IA, sin generar nada, con
     * «Deshacer». Nunca lleva a la conversación, tampoco con un agente. @spec REQ-66
     */
    function improveControls(field, service) {
      let previous = null;
      const undo = button(t('gen.improveUndo'), () => {
        field.value = previous ?? field.value;
        undo.hidden = true;
        refreshEstimate();
      }, 'btn-ghost');
      undo.hidden = true;
      const improve = button(t('gen.improve'), async () => {
        const idea = field.value.trim();
        if (!idea) {
          field.focus();
          return;
        }
        improve.disabled = true;
        improve.textContent = t('gen.improving');
        const result = (await deps.improvePrompt?.(service, idea).catch(() => null)) ?? { kind: 'none' };
        improve.disabled = false;
        improve.textContent = t('gen.improve');
        if (result.kind === 'text') {
          previous = field.value;
          field.value = result.text;
          undo.hidden = false;
          refreshEstimate();
        } else if (result.kind === 'error') deps.toast(result.message, 'error');
        else deps.toast(t({ empty: 'gen.improveEmpty', busy: 'gen.improveBusy' }[result.kind] ?? 'gen.improveNoAi'), 'error');
      }, 'btn-ghost');
      improve.dataset.improve = service;
      improve.title = t('gen.improveHint');
      deps.improveAvailable?.().then((available) => {
        if (available === false) {
          improve.disabled = true;
          improve.title = t('gen.improveNoAi');
        }
      });
      return el('div', { className: 'row' }, improve, undo);
    }

    const textPane = el(
      'div',
      { className: 'stack' },
      el('div', { className: 'row', style: 'justify-content: flex-end;' }, button(`📖 ${t('gen.helpTripo')}`, () => openHelp('tripo'), 'btn-ghost button--compact')),
      el('label', {}, t('gen.prompt'), prompt),
      improveControls(prompt, 'tripo'),
      el('div', { className: 'row' }, el('label', {}, t('gen.faceLimit'), faces), el('label', { className: 'row' }, lowPoly, t('gen.lowPoly'))),
    );
    const worldPane = el(
      'div',
      { className: 'stack hidden' },
      el('div', { className: 'row', style: 'justify-content: space-between; align-items: center;' },
        el('p', { className: 'hint', text: t('gen.worldHint') }),
        button(`📖 ${t('gen.helpWorldLabs')}`, () => openHelp('worldlabs'), 'btn-ghost button--compact'),
      ),
      el('label', {}, t('gen.prompt'), worldPrompt),
      improveControls(worldPrompt, 'worldlabs'),
      el('div', { className: 'row' }, el('label', {}, t('gen.worldModel'), worldModel)),
    );
    const imagePane = el(
      'div',
      { className: 'stack hidden' },
      el('div', { className: 'row', style: 'justify-content: space-between; align-items: center;' },
        el('p', { className: 'hint', text: t('gen.imageHint') }),
        button(`📖 ${t('gen.helpTripo')}`, () => openHelp('tripo'), 'btn-ghost button--compact'),
      ),
      el(
        'div',
        { className: 'row' },
        button(t('gen.pickImage'), async () => {
          const picked = await window.__TAURI__.dialog.open({ multiple: false, filters: [{ name: t('gen.images'), extensions: ['png', 'jpg', 'jpeg', 'webp'] }] });
          if (!picked) return;
          image = String(picked);
          await api.generationAllowReference(image);
          imageName.textContent = baseName(image);
          imageThumb.replaceChildren(el('img', { attrs: { src: window.__TAURI__.core.convertFileSrc(image), alt: baseName(image) } }));
          imageThumb.classList.remove('hidden');
          refreshEstimate();
        }),
        imageName,
      ),
      imageThumb,
    );

    const soundPrompt = el('textarea', { attrs: { rows: '3', placeholder: t('gen.soundPromptPlaceholder'), 'aria-label': t('gen.soundPrompt') } });
    const soundProvider = el('select', { attrs: { 'aria-label': t('gen.soundProvider') } },
      el('option', { text: t('gen.soundProvider.local'), attrs: { value: 'local' } }),
      el('option', { text: t('gen.soundProvider.elevenlabs'), attrs: { value: 'elevenlabs' } }),
      el('option', { text: t('gen.soundProvider.freesound'), attrs: { value: 'freesound' } }),
    );
    const soundDuration = el('input', { attrs: { type: 'number', min: '1', max: '30', step: '0.5', value: '5', 'aria-label': t('gen.soundDuration') } });
    const soundLoop = el('input', { attrs: { type: 'checkbox', 'aria-label': t('gen.soundLoop') } });
    const soundLocalStatus = el('p', { className: 'hint', text: '' });
    const soundResultsContainer = el('div', { className: 'sound-results-list hidden' });

    function stopAllAudio() {
      soundResultsContainer.querySelectorAll('audio').forEach((audio) => {
        if (!audio.paused) {
          audio.pause();
          audio.currentTime = 0;
        }
      });
    }

    function renderSoundResults(items, query) {
      stopAllAudio();
      soundResultsContainer.innerHTML = '';
      if (!items || items.length === 0) {
        soundResultsContainer.appendChild(el('p', { className: 'hint', text: t('gen.soundNoResults') }));
        return;
      }

      const header = el('div', { className: 'row', style: 'justify-content: space-between; align-items: center; margin-bottom: 4px;' },
        el('strong', { text: t('gen.soundResultsTitle', { count: String(items.length) }), style: 'font-size: 13px;' }),
      );
      soundResultsContainer.appendChild(header);

      for (const item of items) {
        const itemTitle = el('span', { className: 'sound-item-title', text: item.name, attrs: { title: item.name } });
        const metaDuration = el('span', { text: `⏱️ ${Number(item.duration || 0).toFixed(1)}s` });
        const tagsText = (item.tags || []).slice(0, 4).join(', ');
        const metaTags = tagsText ? el('span', { className: 'sound-item-tags', text: tagsText }) : null;
        const meta = el('div', { className: 'sound-item-meta' }, metaDuration, metaTags);
        const info = el('div', { className: 'sound-item-info' }, itemTitle, meta);

        let player = null;
        if (item.previewUrl) {
          player = el('audio', {
            className: 'sound-item-player',
            attrs: { controls: '', preload: 'none', src: item.previewUrl },
          });
          player.addEventListener('play', () => {
            soundResultsContainer.querySelectorAll('audio').forEach((other) => {
              if (other !== player && !other.paused) other.pause();
            });
          });
        }

        const addBtn = button(`+ ${t('gen.soundAdd')}`, async () => {
          addBtn.disabled = true;
          addBtn.textContent = '...';
          const targetName = name.value.trim() || item.name;
          const res = await api.generationSaveSoundUrl({
            url: item.previewUrl || item.downloadUrl,
            name: targetName,
            prompt: query,
            duration: item.duration,
          });
          if (res.ok) {
            addBtn.textContent = t('gen.soundAdded');
            addBtn.className = 'btn-ghost button--compact';
            deps.toast(t('gen.soundGenerated'), 'info');
            if (name.value.trim()) name.value = '';
          } else {
            addBtn.disabled = false;
            addBtn.textContent = `+ ${t('gen.soundAdd')}`;
            deps.toast(res.error?.message || 'Error al guardar sonido', 'error');
          }
        }, 'btn-primary button--compact');

        const actions = el('div', { className: 'sound-item-actions' }, addBtn);
        const card = el('div', { className: 'sound-item-card' }, info, player, actions);
        soundResultsContainer.appendChild(card);
      }
    }

    soundPrompt.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && !e.shiftKey && soundProvider.value === 'freesound') {
        e.preventDefault();
        start.click();
      }
    });

    soundProvider.addEventListener('change', () => {
      soundDuration.closest('label').classList.toggle('hidden', soundProvider.value === 'freesound');
      soundLoop.closest('label').classList.toggle('hidden', soundProvider.value === 'freesound');
      if (soundProvider.value !== 'freesound') {
        stopAllAudio();
        soundResultsContainer.classList.add('hidden');
      } else if (soundResultsContainer.children.length > 0) {
        soundResultsContainer.classList.remove('hidden');
      }
      updateServiceState();
    });

    const soundPane = el(
      'div',
      { className: 'stack hidden' },
      el('div', { className: 'row', style: 'justify-content: space-between; align-items: center;' },
        el('p', { className: 'hint', text: t('gen.soundHint') }),
        button(`📖 ${t('gen.helpAudio')}`, () => openHelp('audio'), 'btn-ghost button--compact'),
      ),
      el('div', { className: 'row' },
        el('label', {}, t('gen.soundProvider'), soundProvider),
        el('label', {}, t('gen.soundDuration'), soundDuration),
        el('label', { className: 'row' }, soundLoop, t('gen.soundLoop')),
      ),
      el('label', {}, t('gen.soundPrompt'), soundPrompt),
      soundLocalStatus,
      soundResultsContainer,
    );

    const variantsLabel = el('label', {}, t('gen.variants'), variants);
    const qualityLabel = el('label', {}, t('gen.textureQuality'), quality);

    const start = button(
      t('gen.start'),
      async () => {
        let ids = null;
        if (mode === 'sound') {
          const textVal = soundPrompt.value.trim();
          if (!textVal) {
            soundPrompt.focus();
            return null;
          }
          if (soundProvider.value === 'freesound') {
            start.disabled = true;
            start.textContent = t('gen.soundSearching');
            soundResultsContainer.innerHTML = '';
            soundResultsContainer.classList.remove('hidden');
            try {
              const res = await api.generationSearchFreesound(textVal, 15);
              if (res.ok) {
                renderSoundResults(res.value || [], textVal);
              } else {
                deps.toast(res.error?.message || 'Error en búsqueda', 'error');
              }
            } finally {
              updateServiceState();
            }
            return null;
          }
          start.disabled = true;
          start.textContent = t('gen.soundGenerating');
          try {
            const res = await api.generationGenerateSound({
              provider: soundProvider.value,
              prompt: textVal,
              name: name.value.trim() || null,
              duration: Number(soundDuration.value) || 5,
              loop: soundLoop.checked,
            });
            if (res.ok) {
              deps.toast(t('gen.soundGenerated'), 'info');
              soundPrompt.value = '';
              deps.openLibrary?.();
            } else {
              deps.toast(res.error.message, 'error');
            }
          } finally {
            updateServiceState();
          }
          return null;
        }
        const textVal = mode === 'world' ? worldPrompt.value.trim() : prompt.value.trim();
        if ((mode === 'text' || mode === 'world') && !textVal) {
          (mode === 'world' ? worldPrompt : prompt).focus();
        } else if (mode === 'image' && !image) {
          deps.toast(t('gen.noImage'), 'error');
        } else {
          if (!name.value.trim()) await suggest();
          const { tool, args } = formArgs(values());
          ids = await confirmAndLaunch(deps, tool, args);
          if (ids) {
            deps.onSpent();
            watcher.poke();
          }
        }
        return ids;
      },
      'btn-primary',
    );

    function updateServiceState() {
      if (mode === 'sound') {
        const prov = soundProvider.value;
        start.disabled = false;
        start.textContent = prov === 'freesound' ? t('gen.soundSearch') : t('gen.start');
        keyWarning.classList.add('hidden');
        balance.textContent = '';
        soundLocalStatus.classList.toggle('hidden', prov !== 'local');
        if (prov === 'elevenlabs') {
          const has = Boolean(status.ok && status.value.hasKey?.elevenlabs);
          start.disabled = !has;
          if (!has) {
            keyWarning.textContent = t('settings.elevenlabs');
            keyWarning.classList.remove('hidden');
          }
        } else if (prov === 'freesound') {
          const has = Boolean(status.ok && status.value.hasKey?.freesound);
          start.disabled = !has;
          if (!has) {
            keyWarning.textContent = t('settings.freesound');
            keyWarning.classList.remove('hidden');
          }
        } else if (prov === 'local') {
          api.generationCheckLocalAudio().then((st) => {
            if (st?.ready) {
              soundLocalStatus.textContent = t('gen.soundLocalReady', { device: st.device || 'GPU' });
              soundLocalStatus.className = 'hint check-ok';
            } else {
              soundLocalStatus.textContent = t('gen.soundLocalOffline');
              soundLocalStatus.className = 'hint check-fail';
            }
          });
        }
        return;
      }
      start.textContent = t('gen.start');
      const service = mode === 'world' ? 'worldlabs' : 'tripo';
      const has = Boolean(status.ok && status.value.hasKey?.[service]);
      start.disabled = !has;
      if (!has) {
        keyWarning.textContent = service === 'worldlabs' ? t('gen.noKeyWorldLabs') : t('gen.noKey');
        keyWarning.classList.remove('hidden');
        balance.textContent = '';
      } else {
        keyWarning.classList.add('hidden');
        if (service === 'worldlabs') {
          api.generationWorldLabsBalance().then((result) => {
            // Si falla (clave rechazada, cuenta sin API), se ve el motivo en vez de quedar en blanco.
            balance.textContent = result.ok ? t('gen.worldLabsBalance', { balance: String(result.value.balance) }) : (result.error?.message ?? '');
          });
        } else {
          api.generationBalance().then((result) => {
            balance.textContent = result.ok ? t('gen.balance', { balance: String(result.value.balance), frozen: String(result.value.frozen) }) : '';
          });
        }
      }
    }

    // REQ-65: cada pestaña dice con qué servicio se genera.
    const tabs = ['text', 'image', 'world', 'sound'].map((key) =>
      el('button', {
        className: 'segmented-btn segmented-btn--brand',
        attrs: { type: 'button', 'aria-pressed': String(key === mode), 'data-mode': key },
        on: {
          click: () => {
            mode = key;
            for (const tab of tabs) tab.setAttribute('aria-pressed', String(tab.dataset.mode === key));
            textPane.classList.toggle('hidden', mode !== 'text');
            imagePane.classList.toggle('hidden', mode !== 'image');
            worldPane.classList.toggle('hidden', mode !== 'world');
            soundPane.classList.toggle('hidden', mode !== 'sound');
            if (mode !== 'sound') stopAllAudio();
            variantsLabel.classList.toggle('hidden', mode === 'world' || mode === 'sound');
            qualityLabel.classList.toggle('hidden', mode === 'world' || mode === 'sound');
            updateServiceState();
            refreshEstimate();
          },
        },
      }, el('span', { text: t(`gen.mode.${key}`) }), key === 'sound' ? el('span', { className: 'brand-logo brand-logo--small', text: '🎵', style: 'display:inline-flex;align-items:center;justify-content:center;' }) : brandLogo(key === 'world' ? 'worldlabs' : 'tripo', true)),
    );

    updateServiceState();

    host.replaceChildren(
      el('div', { className: 'view-header' }, el('h2', { text: t('gen.title') }), el('span', { className: 'spacer' }), balance),
      el(
        'div',
        { className: 'card stack' },
        keyWarning,
        el('div', { className: 'segmented', attrs: { role: 'group', 'aria-label': t('gen.mode.label') } }, ...tabs),
        textPane,
        imagePane,
        worldPane,
        soundPane,
        el('div', { className: 'row' }, el('label', {}, t('gen.name'), el('div', { className: 'row' }, name, button(t('gen.suggestName'), suggest, 'btn-ghost'))), variantsLabel, qualityLabel),
        el('div', { className: 'row' }, start, estimate),
      ),
      el('div', { className: 'view-header' }, el('h3', { text: t('gen.jobs') })),
      jobsList,
    );
    watcher.poke();
  }
  return { render, watch: () => watcher.poke() };
}
