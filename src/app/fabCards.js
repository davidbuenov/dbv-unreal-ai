// =============================================================================
// DBV Unreal AI — Tarjetas de Fab (conversación y vista Fab)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-117, REQ-106
//
// Cada listing de Fab se ve antes de instalarlo: miniatura, título, vendedor, gratis / en tu biblioteca / de pago,
// formatos compatibles con el editor y el enlace «Ver en fab.com». Las tarjetas salen de los datos de la herramienta,
// no del texto del modelo. Sin innerHTML (con withGlobalTauri, un `<img onerror>` colado podría llamar al backend):
// todo se construye con nodos y textContent. La miniatura solo se carga desde https://media.fab.com/ y el enlace
// solo se abre si es https://www.fab.com/listings/. Nada lleva a comprar: lo de pago que no se tiene solo ofrece
// el enlace.

import { el } from './dom.js';
import { t } from '../ai/i18nBridge.js';

export const THUMBNAIL_PREFIX = 'https://media.fab.com/';
export const LISTING_PREFIX = 'https://www.fab.com/listings/';
const FORMAT_LABELS = { 'unreal-engine': 'UE', glb: 'GLB', gltf: 'glTF', fbx: 'FBX', 'converted-files': 'Files' };

/** ¿Se puede cargar esta miniatura? Solo de Fab y por https. */
export const safeThumbnail = (url) => (typeof url === 'string' && url.startsWith(THUMBNAIL_PREFIX) ? url : null);

/** ¿Se puede abrir este enlace? Solo fichas de fab.com. */
export const safeListingUrl = (url) => (typeof url === 'string' && url.startsWith(LISTING_PREFIX) ? url : null);

/** ¿Se puede instalar? Gratis o en la biblioteca, y con algún formato compatible con el editor (REQ-104). */
export const canInstall = (listing) => Boolean(listing && (listing.isFree || listing.owned) && listing.compatible && (listing.compatibleFormats ?? []).length > 0);

/** Formato que se instala por defecto: el pack de Unreal si vale, si no el primero compatible. */
export function defaultFormat(listing) {
  const formats = listing?.compatibleFormats ?? [];
  const chosen = formats.includes('unreal-engine') ? 'unreal-engine' : formats[0] ?? null;
  return chosen;
}

/** Texto de la píldora de precio. */
export function priceLabel(listing) {
  let label = t('fab.paid');
  if (listing?.owned) label = t('fab.owned');
  else if (listing?.isFree) label = t('fab.free');
  return label;
}

/**
 * Una tarjeta. `actions.onOpen(url)` abre el enlace en el navegador del sistema; `actions.onInstall(listing, format)`
 * pide la aprobación e instala (si falta, no hay botón).
 */
export function fabCard(listing, actions = {}) {
  const thumbUrl = safeThumbnail(listing.thumbnail);
  const thumb = thumbUrl
    ? el('img', { className: 'fab-card__thumb', attrs: { src: thumbUrl, alt: listing.title ?? '', loading: 'lazy', referrerpolicy: 'no-referrer' } })
    : el('div', { className: 'fab-card__thumb fab-card__thumb--empty', text: '⬚' });
  if (listing.mature) thumb.classList.add('fab-card__thumb--mature');
  const pills = el(
    'div',
    { className: 'fab-card__pills' },
    el('span', { className: `fab-pill ${listing.isFree || listing.owned ? 'fab-pill--ok' : 'fab-pill--paid'}`, text: priceLabel(listing) }),
    listing.mature ? el('span', { className: 'fab-pill fab-pill--warn', text: '18+' }) : null,
    ...(listing.compatibleFormats ?? []).map((code) => el('span', { className: 'fab-pill', text: FORMAT_LABELS[code] ?? code })),
    listing.compatible ? null : el('span', { className: 'fab-pill fab-pill--warn', text: t('fab.incompatible') }),
  );
  const rating = listing.rating == null ? t('fab.noRatings') : `★ ${Number(listing.rating).toFixed(1)} (${listing.ratingCount ?? 0})`;
  const link = safeListingUrl(listing.fabUrl);
  const footer = el('div', { className: 'fab-card__actions' });
  if (link) {
    footer.append(el('a', { className: 'fab-card__link', text: t('fab.view'), attrs: { href: '#', title: link }, on: { click: (event) => { event.preventDefault(); actions.onOpen?.(link); } } }));
  }
  if (actions.onInstall && canInstall(listing)) {
    const install = el('button', { className: 'btn-primary fab-card__install', text: t('fab.install'), attrs: { type: 'button' } });
    install.addEventListener('click', () => actions.onInstall(listing, defaultFormat(listing)));
    footer.append(install);
  }
  return el(
    'div',
    { className: 'fab-card', attrs: { 'data-uid': listing.uid ?? '' } },
    thumb,
    el('div', { className: 'fab-card__title', text: listing.title ?? '', title: listing.title ?? '' }),
    el('div', { className: 'fab-card__seller', text: `${listing.sellerName ?? ''} · ${rating}` }),
    pills,
    footer,
  );
}

/** Tira horizontal de tarjetas para la conversación (REQ-117). */
export function fabCardStrip(listings, actions = {}) {
  const strip = el('div', { className: 'fab-strip' }, ...(listings ?? []).map((listing) => fabCard(listing, actions)));
  return strip;
}
