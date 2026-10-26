// =============================================================================
// DBV Unreal AI — Vista previa 3D de un asset (ventana y en línea)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-31
//
// Dos formas de ver un modelo: en una ventana (Biblioteca, «Ver en 3D») y en línea, dentro de la conversación,
// cuando termina una generación o la IA va a enviar un asset a Unreal. En línea hay como mucho
// `MAX_INLINE` visores vivos: el más antiguo vuelve a ser su miniatura (el WebView tiene pocos contextos WebGL).
// Si el asset no traía miniatura (uno importado), la hace el visor.

import { t } from '../ai/i18nBridge.js';
import * as api from '../app/backend.js';
import { button, el } from '../app/dom.js';
import { clipLabel, createModelViewer, renderModelThumbnail } from './modelViewer.js';

/** Visores en línea vivos a la vez. */
export const MAX_INLINE = 2;

/** Fichero que se puede ver en 3D (GLB o glTF), o `null`. */
export function viewableFile(entry) {
  return (entry?.files ?? []).find((f) => /\.(glb|gltf)$/i.test(f)) ?? null;
}

async function urlFor(entry) {
  const file = viewableFile(entry);
  const path = file ? await api.libraryFilePath(entry.id, file) : null;
  return path?.ok && path.value && window.__TAURI__?.core ? window.__TAURI__.core.convertFileSrc(path.value) : null;
}

/**
 * Genera la miniatura de un modelo 3D que no la tiene y la guarda en la biblioteca.
 * @param {object} entry Entrada de la biblioteca.
 * @returns {Promise<string|null>} Nombre del fichero guardado ('thumbnail.png') o null.
 */
export async function generateThumbnailForEntry(entry) {
  let saved = null;
  if (entry?.thumbnail) {
    saved = entry.thumbnail;
  } else {
    const file = viewableFile(entry);
    if (file) {
      const path = await api.libraryFilePath(entry.id, file);
      const url = path?.ok && path.value && window.__TAURI__?.core?.convertFileSrc
        ? window.__TAURI__.core.convertFileSrc(path.value)
        : null;
      if (url) {
        const png = await renderModelThumbnail(url).catch(() => null);
        if (png) {
          const res = await api.librarySaveThumbnail(entry.id, Array.from(png));
          if (res.ok) {
            entry.thumbnail = 'thumbnail.png';
            saved = 'thumbnail.png';
          }
        }
      }
    }
  }
  return saved;
}

/** Si el asset no tiene miniatura, guarda la del visor. */
async function keepThumbnail(entry, viewer, onSaved) {
  if (entry.thumbnail) return;
  try {
    const png = await viewer.snapshot().catch(() => null);
    if (png) {
      const res = await api.librarySaveThumbnail(entry.id, Array.from(png));
      if (res.ok) {
        entry.thumbnail = 'thumbnail.png';
        onSaved?.(entry);
      }
    }
  } catch {
    // ignorar
  }
}

/** Selector de animaciones del GLB, si trae alguna. */
function clipPicker(viewer) {
  if (viewer.clips.length < 2) return viewer.clips.length ? el('span', { className: 'hint', text: clipLabel(viewer.clips[0]) }) : null;
  const select = el('select', { attrs: { 'aria-label': t('preview.animation') } }, ...viewer.clips.map((name) => el('option', { text: clipLabel(name), attrs: { value: name } })));
  select.addEventListener('change', () => viewer.play(select.value));
  return select;
}

/** Selector de rigs de personajes para ver la animación sobre su cuerpo. */
export function rigPicker(rigs, currentRigId, onChange) {
  if (!rigs || !rigs.length) return null;
  const skeletonOpt = el('option', { text: t('preview.skeletonOnly'), attrs: { value: '' } });
  if (!currentRigId) skeletonOpt.selected = true;
  const options = [
    skeletonOpt,
    ...rigs.map((r) => {
      const opt = el('option', { text: `🎭 ${r.name}`, attrs: { value: r.id } });
      if (r.id === currentRigId) opt.selected = true;
      return opt;
    }),
  ];
  const select = el('select', { attrs: { 'aria-label': t('preview.rigPicker') } }, ...options);
  select.value = currentRigId || '';
  select.addEventListener('change', () => onChange(select.value));
  return el('label', { className: 'row', style: 'align-items: center; gap: 6px; font-size: 13px;' }, el('span', { text: t('preview.rigPicker') }), select);
}

/**
 * Ventana con el visor. `deps`: `{openModal(children, {onClose}), closeModal(), onSend?(entry), onThumbnail?(entry)}`.
 */
export async function openPreview(entry, deps) {
  const stage = el('div', { className: 'viewer-stage', attrs: { role: 'img', 'aria-label': entry.name } });
  const tools = el('div', { className: 'row' });
  let viewer = null;
  let showingRender = false;

  const isWorld = entry?.kind === 'world';
  const isAnim = entry?.kind === 'animation'
    || (entry?.name && /(?:^|[\s_-])anim(?:[\s_-]|$)|_anim|anim_|_dance|_wave/i.test(entry.name))
    || (entry?.files && entry.files.some((f) => /anim|dance|wave/i.test(f)));
  const notice = isWorld ? el('p', {
    className: 'hint',
    style: 'margin: 4px 0 8px 0; padding: 6px 10px; background: rgba(0, 0, 0, 0.05); border-radius: 6px; font-size: 13px;',
    text: t('preview.worldColliderNotice'),
  }) : (isAnim ? el('p', {
    className: 'hint',
    style: 'margin: 4px 0 8px 0; padding: 6px 10px; background: rgba(0, 0, 0, 0.05); border-radius: 6px; font-size: 13px;',
    text: t('preview.animationNotice'),
  }) : null);

  let toggleBtn = null;
  if (isWorld && entry?.thumbnail) {
    toggleBtn = button(t('preview.viewRender'), async () => {
      showingRender = !showingRender;
      if (showingRender) {
        toggleBtn.textContent = t('preview.viewMesh');
        const p = await api.libraryFilePath(entry.id, entry.thumbnail);
        if (p.ok && p.value && window.__TAURI__?.core) {
          const img = el('img', {
            attrs: {
              src: window.__TAURI__.core.convertFileSrc(p.value),
              alt: entry.name,
              style: 'max-width: 100%; max-height: 480px; object-fit: contain; border-radius: 8px; display: block; margin: auto;',
            },
          });
          stage.replaceChildren(img);
        }
      } else {
        toggleBtn.textContent = t('preview.viewRender');
        const url = await urlFor(entry);
        if (url) {
          stage.replaceChildren(el('p', { className: 'hint', text: t('preview.loading') }));
          viewer?.dispose();
          viewer = await createModelViewer(stage, url);
        }
      }
    }, 'btn-ghost');
  }

  deps.openModal(
    [
      el('h3', { text: entry.name }),
      notice,
      stage,
      tools,
      el('div', { className: 'modal-actions' }, toggleBtn, deps.onSend ? button(t('lib.send'), () => deps.onSend(entry), 'btn-primary') : null, button(t('action.close'), deps.closeModal)),
    ].filter(Boolean),
    { onClose: () => viewer?.dispose(), wide: true },
  );

  const url = await urlFor(entry);
  if (!url) {
    if (entry?.thumbnail) {
      const p = await api.libraryFilePath(entry.id, entry.thumbnail);
      if (p.ok && p.value && window.__TAURI__?.core) {
        stage.replaceChildren(el('img', {
          attrs: {
            src: window.__TAURI__.core.convertFileSrc(p.value),
            alt: entry.name,
            style: 'max-width: 100%; max-height: 480px; object-fit: contain; border-radius: 8px; display: block; margin: auto;',
          },
        }));
        return;
      }
    }
    stage.replaceChildren(el('p', { className: 'hint', text: t('preview.notViewable') }));
    return;
  }

  let rigs = [];
  let currentRigId = null;
  if (isAnim) {
    try {
      const res = await api.libraryList();
      const all = (res?.ok && Array.isArray(res.value)) ? res.value : (Array.isArray(res) ? res : []);
      rigs = all.filter((e) => e.id !== entry.id && (e.kind === 'rig' || e.name.toLowerCase().includes('rig')));
      if (entry.parent) {
        currentRigId = rigs.find((r) => r.id === entry.parent)?.id ?? null;
      }
      if (!currentRigId) {
        const stem = entry.name.toLowerCase();
        currentRigId = rigs.find((r) => {
          const rName = r.name.toLowerCase().replace(/[\s_]rig$/i, '').trim();
          return rName.length > 0 && (stem.startsWith(rName) || stem.includes(rName));
        })?.id ?? null;
      }
      if (!currentRigId && rigs.length > 0) {
        currentRigId = rigs[0].id;
      }
    } catch {
      rigs = [];
    }
  }

  async function renderViewer(rigId) {
    stage.replaceChildren(el('p', { className: 'hint', text: t('preview.loading') }));
    viewer?.dispose();
    const rigSelector = isAnim && rigs.length ? rigPicker(rigs, rigId, (newId) => renderViewer(newId)) : null;
    tools.replaceChildren(...[el('span', { className: 'hint', text: t('preview.help') }), rigSelector].filter(Boolean));
    try {
      let rigUrl = null;
      if (rigId) {
        const rigEntry = rigs.find((r) => r.id === rigId);
        if (rigEntry) rigUrl = await urlFor(rigEntry);
      }
      viewer = await createModelViewer(stage, url, { rigUrl });
      const picker = clipPicker(viewer);
      tools.replaceChildren(...[el('span', { className: 'hint', text: t('preview.help') }), picker, rigSelector].filter(Boolean));
      keepThumbnail(entry, viewer, deps?.onThumbnail);
    } catch (error) {
      stage.replaceChildren(el('p', { className: 'check-fail', text: t('preview.failed', { error: String(error?.message ?? error) }) }));
    }
  }

  await renderViewer(currentRigId);
}

/** Visores en línea vivos (el más antiguo, primero). */
const inline = [];

/**
 * Visor en línea en `slot` (conversación). Si ya hay `MAX_INLINE`, el más antiguo vuelve a su miniatura.
 * @param {HTMLElement} slot
 * @param {object} entry Asset de la biblioteca.
 * @param {(entry: object) => Promise<HTMLElement>} fallback Miniatura para cuando el visor se retire o falle.
 */
export async function mountInlineViewer(slot, entry, fallback) {
  const url = await urlFor(entry);
  if (!url) {
    slot.replaceChildren(await fallback(entry));
    return;
  }
  while (inline.length >= MAX_INLINE) {
    const oldest = inline.shift();
    oldest.viewer.dispose();
    oldest.slot.replaceChildren(await fallback(oldest.entry));
  }
  const stage = el('div', { className: 'viewer-stage viewer-stage--inline' });
  slot.replaceChildren(stage);
  try {
    const viewer = await createModelViewer(stage, url);
    inline.push({ viewer, slot, entry });
    const picker = clipPicker(viewer);
    if (picker) slot.append(picker);
    keepThumbnail(entry, viewer);
  } catch {
    slot.replaceChildren(await fallback(entry));
  }
}
