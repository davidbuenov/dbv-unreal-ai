// =============================================================================
// DBV Unreal AI — Vista «Fab»: buscar, ver la ficha e instalar sin escribir a la IA
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-106
//
// Usa las mismas herramientas que la IA (`fab_run_tool`), así que valen las mismas reglas, comprobadas en el código:
// solo lo compatible con el editor, solo se instala lo gratuito o lo que ya tienes (nada se compra desde la app) y un
// listing maduro solo se abre si lo confirmas. Buscar funciona sin el editor; «Mi biblioteca» e instalar lo necesitan.
// Las tarjetas son las de la conversación (`fabCards.js`), en cuadrícula. Nunca se enseña un total: Fab no lo da.

import { t } from '../ai/i18nBridge.js';
import { openExternalUrl } from '../ai/backend.js';
import * as api from './backend.js';
import { button, el } from './dom.js';
import { defaultFormat, fabCard, safeListingUrl, safeThumbnail } from './fabCards.js';

const FORMATS = ['', 'unreal-engine', 'glb', 'gltf', 'fbx'];
const SORTS = ['', '-createdAt', '-averageRating'];

/** La primera línea del texto de una herramienta `fab_*` es su JSON (`listings`, `next_cursor`…). */
export function parseToolJson(text) {
  let value = null;
  try {
    value = JSON.parse(String(text ?? '').split('\n')[0]);
  } catch {
    value = null;
  }
  return value;
}

/**
 * @param {HTMLElement} host
 * @param {{state: object, toast: Function, confirm: Function, openModal: Function, closeModal: Function}} deps
 */
export function createFabView(host, deps) {
  let tab = 'catalog';
  let listings = [];
  let cursor = null;
  let query = '';
  let format = '';
  let freeOnly = true;
  let sort = '';
  let session = null;
  let busy = false;
  let message = '';

  const online = () => deps.state.connection?.status === 'online';
  const run = (name, args) => api.fabRunTool(online() ? deps.state.connection.endpoint : null, deps.state.connection?.engineVersion ?? null, name, args, deps.state.connection?.projectPath ?? null);

  async function load(more = false) {
    if (busy) return;
    busy = true;
    message = '';
    const args = tab === 'catalog'
      ? { query: query || 'asset', free_only: freeOnly, formats: format ? [format] : [], sort_by: sort, cursor: more ? cursor : '' }
      : { query, cursor: more ? cursor : '' };
    const ran = await run(tab === 'catalog' ? 'fab_search' : 'fab_my_library', args);
    busy = false;
    if (!ran.ok || String(ran.value?.text ?? '').startsWith('error')) {
      message = ran.ok ? ran.value.text.replace(/^error:\s*/, '') : ran.error?.message ?? String(ran.error);
      if (!more) listings = [];
    } else {
      const parsed = parseToolJson(ran.value.text);
      listings = more ? [...listings, ...(ran.value.fabListings ?? [])] : ran.value.fabListings ?? [];
      cursor = parsed?.next_cursor ?? null;
      if (!listings.length) message = t('fabview.empty');
    }
    paint();
  }

  async function checkSession() {
    session = null;
    if (online()) {
      const ran = await run('fab_sign_in', {});
      session = ran.ok && /is active/i.test(String(ran.value?.text ?? ''));
    }
    paint();
  }

  async function install(listing, chosenFormat) {
    if (!online()) {
      deps.toast(t('lib.fabNeedsEditor'), 'error');
      return;
    }
    const choice = await deps.confirm({ title: t('fab.install'), text: t('lib.fabInstallConfirm', { name: listing.title, format: chosenFormat, project: deps.state.connection?.projectName ?? '' }), choices: [{ key: 'install', label: t('fab.install') }, { key: 'cancel', label: t('gen.cancel') }] });
    if (choice !== 'install') return;
    deps.toast(t('fab.installing', { title: listing.title }));
    const ran = await run('fab_install', { listing_uid: listing.uid, asset_format: chosenFormat });
    const text = ran.ok ? String(ran.value?.text ?? '') : String(ran.error?.message ?? '');
    const failed = !ran.ok || text.startsWith('error');
    const folder = /"folder":"([^"]+)"/.exec(text)?.[1] ?? '';
    deps.toast(failed ? t('fab.installFailed', { title: listing.title, message: text.slice(0, 200) }) : t('fab.installed', { title: listing.title, folder }), failed ? 'error' : undefined);
  }

  async function details(listing) {
    let confirmMature = false;
    if (listing.mature) {
      const choice = await deps.confirm({ title: '18+', text: t('fabview.matureConfirm', { name: listing.title }), choices: [{ key: 'open', label: t('fabview.open') }, { key: 'cancel', label: t('gen.cancel') }] });
      if (choice !== 'open') return;
      confirmMature = true;
    }
    const ran = await run('fab_listing_details', { listing_uid: listing.uid, confirm_mature: confirmMature });
    const info = ran.ok ? parseToolJson(ran.value?.text) : null;
    if (!info) {
      deps.toast(ran.ok ? String(ran.value?.text ?? '') : ran.error?.message ?? '', 'error');
      return;
    }
    const card = ran.value.fabListings?.[0] ?? listing;
    const formats = card.compatibleFormats ?? [];
    const select = el('select', { attrs: { 'aria-label': t('fabview.format') } }, ...formats.map((code) => el('option', { text: code, attrs: { value: code } })));
    select.value = defaultFormat(card) ?? '';
    const installable = (card.isFree || card.owned) && formats.length > 0;
    const link = safeListingUrl(card.fabUrl);
    const thumb = safeThumbnail(card.thumbnail);
    deps.openModal([
      el('h3', { text: card.title }),
      thumb ? el('img', { className: 'fab-detail__thumb', attrs: { src: thumb, alt: card.title, referrerpolicy: 'no-referrer' } }) : null,
      el('p', { className: 'hint', text: `${card.sellerName ?? ''} · ${(info.licenses ?? []).join(', ')}` }),
      el('p', { className: 'fab-detail__text', text: info.description ?? '' }),
      formats.length ? null : el('p', { className: 'hint', text: t('fabview.noCompatible') }),
      el(
        'div',
        { className: 'modal-actions' },
        link ? button(t('fab.view'), () => openExternalUrl(link), 'btn-ghost') : null,
        installable ? select : null,
        installable ? button(t('fab.install'), () => (deps.closeModal(), install(card, select.value)), 'btn-primary') : el('span', { className: 'hint', text: formats.length ? t('fabview.paidHint') : '' }),
        button(t('gen.cancel'), deps.closeModal),
      ),
    ]);
  }

  function filters() {
    const search = el('input', { attrs: { type: 'search', placeholder: t('fabview.search'), value: query, 'aria-label': t('fabview.search') } });
    search.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') {
        query = search.value.trim();
        load();
      }
    });
    const formatSelect = el('select', { attrs: { 'aria-label': t('fabview.format') } }, ...FORMATS.map((code) => el('option', { text: code || t('fabview.anyFormat'), attrs: { value: code } })));
    formatSelect.value = format;
    formatSelect.addEventListener('change', () => { format = formatSelect.value; load(); });
    const free = el('input', { attrs: { type: 'checkbox' } });
    free.checked = freeOnly;
    free.addEventListener('change', () => { freeOnly = free.checked; load(); });
    const sortSelect = el('select', { attrs: { 'aria-label': t('fabview.sort') } }, ...SORTS.map((code) => el('option', { text: t(`fabview.sort.${code || 'relevance'}`), attrs: { value: code } })));
    sortSelect.value = sort;
    sortSelect.addEventListener('change', () => { sort = sortSelect.value; load(); });
    const catalogOnly = tab === 'catalog';
    return el('div', { className: 'stack' }, el('div', { className: 'row' }, search, button(t('fabview.go'), () => { query = search.value.trim(); load(); }, 'btn-primary')), catalogOnly ? el('div', { className: 'row' }, formatSelect, el('label', {}, free, ' ', t('fabview.freeOnly')), sortSelect) : null);
  }

  function sessionPill() {
    let node = el('span', { className: 'hint', text: t('fabview.noEditor') });
    if (online() && session === true) node = el('span', { className: 'fab-pill fab-pill--ok', text: t('fabview.session') });
    else if (online() && session === false) node = el('span', { className: 'hint', text: t('fabview.noSession') });
    return node;
  }

  function paint() {
    const tabs = el('div', { className: 'row fab-tabs' }, ...['catalog', 'library'].map((name) => button(t(`fabview.tab.${name}`), () => { tab = name; listings = []; cursor = null; message = ''; paint(); if (name === 'catalog' || online()) load(); }, tab === name ? 'btn-primary' : '')));
    const needsEditor = tab === 'library' && !online();
    const grid = el('div', { className: 'fab-grid' }, ...listings.map((listing) => {
      const node = fabCard(listing, { onOpen: openExternalUrl, onInstall: online() ? install : null });
      const title = node.querySelector('.fab-card__title');
      title.classList.add('fab-card__title--link');
      title.addEventListener('click', () => details(listing));
      node.querySelector('.fab-card__thumb')?.addEventListener('click', () => details(listing));
      return node;
    }));
    host.replaceChildren(
      el('div', { className: 'view-header' }, el('h2', { text: t('fabview.title') }), el('span', { className: 'spacer' }), sessionPill()),
      el('p', { className: 'hint', text: t('fabview.hint') }),
      tabs,
      filters(),
      needsEditor ? el('p', { className: 'hint', text: t('fabview.libraryNeedsEditor') }) : null,
      busy ? el('p', { className: 'hint', text: t('fabview.loading') }) : null,
      message ? el('p', { className: 'hint', text: message }) : null,
      grid,
      cursor && listings.length ? button(t('fabview.more'), () => load(true)) : null,
    );
  }

  async function render() {
    paint();
    await checkSession();
    if (!listings.length && (tab === 'catalog' || online())) await load();
  }

  return { render };
}
