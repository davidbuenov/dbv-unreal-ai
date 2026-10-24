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
import { clipLabel, createModelViewer } from './modelViewer.js';

/** Visores en línea vivos a la vez. */
export const MAX_INLINE = 2;

/** Fichero que se puede ver en 3D (GLB o glTF), o `null`. */
export function viewableFile(entry) {
  return (entry?.files ?? []).find((f) => /\.(glb|gltf)$/i.test(f)) ?? null;
}

async function urlFor(entry) {
  const file = viewableFile(entry);
  const path = file ? await api.libraryFilePath(entry.id, file) : null;
  return path?.ok && path.value ? window.__TAURI__.core.convertFileSrc(path.value) : null;
}

/** Si el asset no tiene miniatura, guarda la del visor (tras un momento, con el modelo ya encuadrado). */
function keepThumbnail(entry, viewer) {
  if (entry.thumbnail) return;
  setTimeout(async () => {
    const png = await viewer.snapshot().catch(() => null);
    if (png) await api.librarySaveThumbnail(entry.id, Array.from(png));
  }, 1200);
}

/** Selector de animaciones del GLB, si trae alguna. */
function clipPicker(viewer) {
  if (viewer.clips.length < 2) return viewer.clips.length ? el('span', { className: 'hint', text: clipLabel(viewer.clips[0]) }) : null;
  const select = el('select', { attrs: { 'aria-label': t('preview.animation') } }, ...viewer.clips.map((name) => el('option', { text: clipLabel(name), attrs: { value: name } })));
  select.addEventListener('change', () => viewer.play(select.value));
  return select;
}

/**
 * Ventana con el visor. `deps`: `{openModal(children, {onClose}), closeModal(), onSend?(entry)}`.
 */
export async function openPreview(entry, deps) {
  const stage = el('div', { className: 'viewer-stage', attrs: { role: 'img', 'aria-label': entry.name } });
  const tools = el('div', { className: 'row' });
  let viewer = null;
  deps.openModal(
    [
      el('h3', { text: entry.name }),
      stage,
      tools,
      el('div', { className: 'modal-actions' }, deps.onSend ? button(t('lib.send'), () => deps.onSend(entry), 'btn-primary') : null, button(t('action.close'), deps.closeModal)),
    ],
    { onClose: () => viewer?.dispose(), wide: true },
  );
  const url = await urlFor(entry);
  if (!url) {
    stage.replaceChildren(el('p', { className: 'hint', text: t('preview.notViewable') }));
    return;
  }
  stage.replaceChildren(el('p', { className: 'hint', text: t('preview.loading') }));
  try {
    viewer = await createModelViewer(stage, url);
    // `replaceChildren` pintaría «null» como texto: solo los nodos que existen.
    tools.replaceChildren(...[el('span', { className: 'hint', text: t('preview.help') }), clipPicker(viewer)].filter(Boolean));
    keepThumbnail(entry, viewer);
  } catch (error) {
    stage.replaceChildren(el('p', { className: 'check-fail', text: t('preview.failed', { error: String(error?.message ?? error) }) }));
  }
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
