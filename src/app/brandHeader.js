// =============================================================================
// DBV Unreal AI — Cabecera de tarjeta con el logo de un servicio
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-60
//
// Los logos son claros (Tripo y World Labs en blanco, Convai sobre negro): van sobre una pastilla oscura para
// verse igual en los temas oscuro, claro y sepia.

import { el } from './dom.js';

/** Logo de cada servicio, en `src/assets/brands/`. */
export const BRAND_LOGOS = {
  tripo: 'assets/brands/tripo.webp',
  worldlabs: 'assets/brands/worldlabs.svg',
  convai: 'assets/brands/convai.png',
};

/**
 * Título de la tarjeta a la izquierda y logo del servicio a la derecha.
 * @param {string} title
 * @param {keyof typeof BRAND_LOGOS} brand
 */
export function brandHeader(title, brand) {
  return el(
    'div',
    { className: 'brand-header' },
    el('h3', { text: title }),
    el('span', { className: 'brand-logo' }, el('img', { attrs: { src: BRAND_LOGOS[brand], alt: title, height: '18' } })),
  );
}
