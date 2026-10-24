// =============================================================================
// DBV Unreal AI — Vista Biblioteca
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-17, REQ-19
//
// Escenario G: todo lo generado, de cualquier proyecto, con búsqueda y «Enviar a Unreal».

import { t } from '../ai/i18nBridge.js';
import { sendToUnreal } from '../unreal/sendToUnreal.js';
import * as api from './backend.js';
import { button, el } from './dom.js';

/** Entradas que contienen `query` en el nombre, el prompt o el proveedor. */
export function filterEntries(entries, query) {
  const needle = String(query ?? '').trim().toLowerCase();
  return needle ? entries.filter((e) => `${e.name} ${e.prompt} ${e.provider}`.toLowerCase().includes(needle)) : entries;
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

/**
 * @param {HTMLElement} host
 * @param {object} deps `{state, confirm, toast}`.
 */
export function createLibraryView(host, deps) {
  let entries = [];
  const search = el('input', { attrs: { type: 'search', placeholder: t('lib.search'), 'aria-label': t('lib.search') } });
  const grid = el('div', { className: 'library-grid' });
  search.addEventListener('input', () => paint());

  async function thumbnail(entry) {
    const box = el('div', { className: 'asset-thumb', text: '◆' });
    if (entry.thumbnail) {
      const path = await api.libraryFilePath(entry.id, entry.thumbnail);
      if (path.ok) box.replaceChildren(el('img', { attrs: { src: window.__TAURI__.core.convertFileSrc(path.value), alt: entry.name } }));
    }
    return box;
  }

  async function send(entry, statusLine) {
    const connection = deps.state.connection;
    if (connection?.status !== 'online') return deps.toast(t('lib.noProject'), 'error');
    statusLine.textContent = t('lib.sending');
    const result = await sendToUnreal({ backend: sendBackend(connection) }, entry, { project: connection.projectPath });
    statusLine.textContent = result.ok ? t('lib.sent', { path: result.assetPath }) : '';
    if (!result.ok) deps.toast(t(`lib.error.${result.error}`) === `lib.error.${result.error}` ? result.error : t(`lib.error.${result.error}`), 'error');
    return null;
  }

  async function card(entry) {
    const statusLine = el('div', { className: 'asset-meta', attrs: { 'aria-live': 'polite' } });
    const date = new Date(entry.createdAt * 1000).toLocaleDateString();
    return el(
      'article',
      { className: 'asset-card' },
      await thumbnail(entry),
      el('div', { className: 'asset-body' }, el('div', { className: 'asset-name', text: entry.name }), el('div', { className: 'asset-meta', text: `${entry.provider} · ${date}${entry.credits ? ` · ${t('lib.credits', { n: String(entry.credits) })}` : ''}` }), statusLine),
      el(
        'div',
        { className: 'asset-actions' },
        button(t('lib.send'), () => send(entry, statusLine), 'btn-primary'),
        button(t('lib.reveal'), async () => {
          const path = await api.libraryFilePath(entry.id, entry.files[0]);
          if (path.ok) await window.__TAURI__.opener.revealItemInDir(path.value);
        }),
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

  async function paint() {
    const shown = filterEntries(entries, search.value);
    grid.replaceChildren(...(await Promise.all(shown.map(card))));
    if (!shown.length) grid.replaceChildren(el('p', { className: 'empty', text: t('lib.empty') }));
  }

  async function render() {
    const listed = await api.libraryList();
    entries = listed.ok ? listed.value : [];
    host.replaceChildren(el('div', { className: 'view-header' }, el('h2', { text: t('lib.title') }), el('span', { className: 'spacer' }), search), grid);
    await paint();
  }

  return { render };
}
