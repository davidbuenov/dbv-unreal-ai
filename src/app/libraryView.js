// =============================================================================
// DBV Unreal AI — Vista Biblioteca
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-17, REQ-19, REQ-16, REQ-28, REQ-30
//
// Escenario G: todo lo generado o importado, de cualquier proyecto, con búsqueda. Cada asset dice qué es
// (modelo, rig, animación, imagen) y de cuál sale, y ofrece el paso siguiente: un modelo se riggea, un rig se
// anima, una imagen se convierte en 3D; todo pasa por la tarjeta de coste. Se pueden importar modelos e
// imágenes propios u otra biblioteca entera, y renombrar.

import { t } from '../ai/i18nBridge.js';
import { sendToUnreal } from '../unreal/sendToUnreal.js';
import * as api from './backend.js';
import { button, el } from './dom.js';
import { confirmAndLaunch } from './generateView.js';

/** Extensiones que se pueden importar (`library::MODEL_EXTENSIONS` e imágenes). */
export const IMPORT_EXTENSIONS = ['glb', 'gltf', 'fbx', 'obj', 'usdz', 'png', 'jpg', 'jpeg', 'webp'];

/** Animaciones que se pueden elegir de una vez (límite de Tripo por tarea). */
export const MAX_ANIMATIONS = 5;

/** Entradas que contienen `query` en el nombre, el prompt, el tipo o el proveedor. */
export function filterEntries(entries, query) {
  const needle = String(query ?? '').trim().toLowerCase();
  return needle ? entries.filter((e) => `${e.name} ${e.prompt} ${e.provider} ${e.kind}`.toLowerCase().includes(needle)) : entries;
}

/** ¿El rig se hizo con el modelo v1.0 (101 animaciones)? Igual que `catalog::is_v1_rig`. */
export function isV1Rig(entry) {
  const model = entry?.parameters?.request?.model;
  return !model || String(model).startsWith('v1.0');
}

/** Backend que necesita `sendToUnreal` para el proyecto conectado. */
export function sendBackend(connection) {
  return {
    listToolsets: () => api.unrealListToolsets(connection.endpoint),
    callTool: (toolset, tool, args, timeout) => api.unrealCallTool(connection.endpoint, toolset, tool, args, timeout),
    libraryFilePath: api.libraryFilePath,
    libraryRecordSent: api.libraryRecordSent,
  };
}

/** Miniatura de un asset (la que trajo el proveedor, o una imagen del propio asset), o su icono. */
export async function assetThumbnail(entry, className = 'asset-thumb') {
  const box = el('div', { className, text: { rig: '🦴', animation: '💃', image: '🖼' }[entry.kind] ?? '◆' });
  const image = entry.thumbnail ?? (entry.kind === 'image' ? entry.files?.[0] : null);
  if (image) {
    const path = await api.libraryFilePath(entry.id, image);
    if (path.ok) box.replaceChildren(el('img', { attrs: { src: window.__TAURI__.core.convertFileSrc(path.value), alt: entry.name } }));
  }
  return box;
}

/**
 * @param {HTMLElement} host
 * @param {object} deps `{state, confirm, toast, openModal, closeModal, onLaunched}`.
 */
export function createLibraryView(host, deps) {
  let entries = [];
  const search = el('input', { attrs: { type: 'search', placeholder: t('lib.search'), 'aria-label': t('lib.search') } });
  const grid = el('div', { className: 'library-grid' });
  search.addEventListener('input', () => paint());

  async function send(entry, statusLine) {
    const connection = deps.state.connection;
    if (connection?.status !== 'online') return deps.toast(t('lib.noProject'), 'error');
    statusLine.textContent = t('lib.sending');
    const result = await sendToUnreal({ backend: sendBackend(connection) }, entry, { project: connection.projectPath });
    statusLine.textContent = result.ok ? t('lib.sent', { path: result.assetPath }) : '';
    if (!result.ok) deps.toast(t(`lib.error.${result.error}`) === `lib.error.${result.error}` ? result.error : t(`lib.error.${result.error}`), 'error');
    return null;
  }

  async function launch(tool, args) {
    const ids = await confirmAndLaunch(deps, tool, args);
    if (ids) deps.onLaunched?.(ids);
  }

  /** Elegir hasta cinco animaciones para un rig, por grupos, y lanzarlas. */
  async function pickAnimations(rig) {
    const presets = await api.generationPresets();
    if (!presets.ok) return deps.toast(presets.error.message, 'error');
    const groups = isV1Rig(rig) ? presets.value.biped : [{ group: 'generic', names: presets.value.generic }];
    const chosen = new Set();
    const counter = el('span', { className: 'hint' });
    const go = button('', async () => {
      deps.closeModal();
      await launch('animate_model', { id: rig.id, animations: [...chosen] });
    }, 'btn-primary');
    const refresh = () => {
      counter.textContent = t('lib.animationsChosen', { n: String(chosen.size), max: String(MAX_ANIMATIONS) });
      go.textContent = t('lib.animateGo', { cost: String(chosen.size * 10) });
      go.disabled = chosen.size === 0;
    };
    const boxes = groups.map(({ group, names }) =>
      el(
        'fieldset',
        { className: 'preset-group' },
        el('legend', { text: t(`lib.presetGroup.${group}`) === `lib.presetGroup.${group}` ? group : t(`lib.presetGroup.${group}`) }),
        ...names.map((name) => {
          const box = el('input', { attrs: { type: 'checkbox', value: name } });
          box.addEventListener('change', () => {
            if (box.checked && chosen.size >= MAX_ANIMATIONS) box.checked = false;
            else if (box.checked) chosen.add(name);
            else chosen.delete(name);
            refresh();
          });
          return el('label', { className: 'preset' }, box, name.replace(/_/g, ' '));
        }),
      ),
    );
    refresh();
    deps.openModal([
      el('h3', { text: t('lib.animateTitle', { name: rig.name }) }),
      el('p', { className: 'hint', text: t(isV1Rig(rig) ? 'lib.animateHintV1' : 'lib.animateHintV3') }),
      el('div', { className: 'preset-list' }, ...boxes),
      el('div', { className: 'modal-actions' }, counter, el('span', { className: 'spacer' }), button(t('gen.cancel'), deps.closeModal), go),
    ]);
    return null;
  }

  async function rename(entry) {
    const input = el('input', { attrs: { type: 'text', value: entry.name, 'aria-label': t('lib.rename') } });
    const save = async () => {
      const renamed = await api.libraryRename(entry.id, input.value);
      if (!renamed.ok) return deps.toast(renamed.error.message, 'error');
      deps.closeModal();
      return render();
    };
    input.addEventListener('keydown', (event) => event.key === 'Enter' && save());
    deps.openModal([el('h3', { text: t('lib.rename') }), input, el('div', { className: 'modal-actions' }, button(t('gen.cancel'), deps.closeModal), button(t('settings.save'), save, 'btn-primary'))]);
    input.select();
  }

  /** El paso siguiente que tiene sentido para cada tipo de asset. */
  function nextStep(entry) {
    const steps = {
      model: () => button(t('lib.rig'), () => launch('rig_model', { id: entry.id })),
      rig: () => button(t('lib.animate'), () => pickAnimations(entry)),
      image: () => button(t('lib.imageTo3d'), () => launch('image_to_3d_model', { image: entry.id, name: entry.name })),
    };
    return steps[entry.kind]?.() ?? null;
  }

  async function card(entry) {
    const statusLine = el('div', { className: 'asset-meta', attrs: { 'aria-live': 'polite' } });
    const date = new Date(entry.createdAt * 1000).toLocaleDateString();
    const parent = entry.parent ? entries.find((e) => e.id === entry.parent) : null;
    const kind = t(`lib.kind.${entry.kind}`) === `lib.kind.${entry.kind}` ? entry.kind : t(`lib.kind.${entry.kind}`);
    const canSend = ['model', 'rig', 'animation'].includes(entry.kind);
    return el(
      'article',
      { className: 'asset-card', attrs: { 'data-kind': entry.kind } },
      await assetThumbnail(entry),
      el(
        'div',
        { className: 'asset-body' },
        el('div', { className: 'asset-name', text: entry.name }),
        el('div', { className: 'asset-meta', text: `${kind} · ${entry.provider} · ${date}${entry.credits ? ` · ${t('lib.credits', { n: String(entry.credits) })}` : ''}` }),
        parent ? el('div', { className: 'asset-meta', text: t('lib.from', { name: parent.name }) }) : null,
        statusLine,
      ),
      el(
        'div',
        { className: 'asset-actions' },
        canSend ? button(t('lib.send'), () => send(entry, statusLine), 'btn-primary') : null,
        nextStep(entry),
        button(t('lib.rename'), () => rename(entry), 'btn-ghost'),
        button(t('lib.reveal'), async () => {
          const dir = await api.libraryEntryDir(entry.id);
          if (dir.ok) await window.__TAURI__.opener.revealItemInDir(dir.value);
        }, 'btn-ghost'),
        button(t('lib.remove'), async () => {
          const choice = await deps.confirm({ title: t('lib.remove'), text: t('lib.removeConfirm', { name: entry.name }), choices: [{ key: 'delete', label: t('lib.remove'), tone: 'danger' }, { key: 'cancel', label: t('gen.cancel') }] });
          if (choice !== 'delete') return;
          const removed = await api.libraryRemove(entry.id);
          if (!removed.ok) deps.toast(removed.error.message, 'error');
          await render();
        }, 'btn-ghost btn-danger'),
      ),
    );
  }

  async function importPaths(paths) {
    if (!paths?.length) return null;
    const imported = await api.libraryImport(paths.map(String));
    if (!imported.ok) return deps.toast(imported.error.message, 'error');
    deps.toast(t('lib.imported', { n: String(imported.value.length) }));
    return render();
  }

  async function paint() {
    const shown = filterEntries(entries, search.value);
    grid.replaceChildren(...(await Promise.all(shown.map(card))));
    if (!shown.length) grid.replaceChildren(el('p', { className: 'empty', text: t('lib.empty') }));
  }

  async function render() {
    const [listed, location] = await Promise.all([api.libraryList(), api.libraryLocation()]);
    entries = listed.ok ? listed.value : [];
    const dialog = window.__TAURI__.dialog;
    host.replaceChildren(
      el(
        'div',
        { className: 'view-header' },
        el('h2', { text: t('lib.title') }),
        el('span', { className: 'spacer' }),
        search,
        button(t('lib.importFiles'), async () => importPaths(await dialog.open({ multiple: true, filters: [{ name: t('lib.importable'), extensions: IMPORT_EXTENSIONS }] }))),
        button(t('lib.importLibrary'), async () => {
          const folder = await dialog.open({ directory: true });
          return importPaths(folder ? [folder] : []);
        }, 'btn-ghost'),
      ),
      location.ok && location.value ? el('p', { className: 'hint mono', text: t('lib.location', { path: location.value.current }) }) : null,
      grid,
    );
    await paint();
  }

  return { render };
}
