// =============================================================================
// DBV Unreal AI — Tests de las tarjetas y las herramientas de Fab
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-105, REQ-117

import { beforeAll, describe, expect, it, vi } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { canInstall, defaultFormat, fabCard, fabCardStrip, safeListingUrl, safeThumbnail } from '../../src/app/fabCards.js';
import { createFabTools } from '../../src/unreal/fabTools.js';

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

const listing = (overrides = {}) => ({
  uid: 'l1',
  title: 'Camo muscle car',
  sellerName: 'FoulQuince',
  isFree: true,
  owned: false,
  rating: 5,
  ratingCount: 1,
  fabUrl: 'https://www.fab.com/listings/l1',
  thumbnail: 'https://media.fab.com/image_previews/a.jpg',
  mature: false,
  compatibleFormats: ['fbx'],
  compatible: true,
  ...overrides,
});

describe('tarjetas de Fab (REQ-117)', () => {
  it('llevan miniatura, título, vendedor, gratis, formatos y el enlace a fab.com', () => {
    const onOpen = vi.fn();
    const card = fabCard(listing(), { onOpen, onInstall: vi.fn() });
    expect(card.querySelector('img').getAttribute('src')).toBe('https://media.fab.com/image_previews/a.jpg');
    expect(card.textContent).toContain('Camo muscle car');
    expect(card.textContent).toContain('FoulQuince');
    expect(card.textContent).toContain('Gratis');
    expect(card.textContent).toContain('FBX');
    card.querySelector('.fab-card__link').click();
    expect(onOpen).toHaveBeenCalledWith('https://www.fab.com/listings/l1');
  });

  it('una miniatura de otro dominio no se carga y un enlace que no es de Fab no se ofrece', () => {
    const card = fabCard(listing({ thumbnail: 'https://evil.example/a.jpg', fabUrl: 'javascript:alert(1)' }), { onOpen: vi.fn() });
    expect(card.querySelector('img')).toBeNull();
    expect(card.querySelector('.fab-card__thumb--empty')).not.toBeNull();
    expect(card.querySelector('.fab-card__link')).toBeNull();
    expect(safeThumbnail('http://media.fab.com/a.jpg')).toBeNull();
    expect(safeListingUrl('https://www.fab.com/listings/x')).toBe('https://www.fab.com/listings/x');
  });

  it('el texto del listing no se interpreta como HTML', () => {
    const card = fabCard(listing({ title: '<img src=x onerror=alert(1)>' }));
    expect(card.querySelectorAll('img')).toHaveLength(1);
    expect(card.querySelector('.fab-card__title').textContent).toBe('<img src=x onerror=alert(1)>');
  });

  it('lo maduro sale difuminado y con 18+', () => {
    const card = fabCard(listing({ mature: true }));
    expect(card.querySelector('.fab-card__thumb--mature')).not.toBeNull();
    expect(card.textContent).toContain('18+');
  });

  it('solo se instala lo gratuito o lo que ya tiene el usuario, y compatible con el editor (REQ-104)', () => {
    const onInstall = vi.fn();
    const paid = fabCard(listing({ isFree: false, owned: false, compatibleFormats: ['unreal-engine'] }), { onInstall });
    expect(paid.querySelector('.fab-card__install')).toBeNull();
    expect(paid.textContent).toContain('De pago');
    expect(paid.textContent.toLowerCase()).not.toContain('comprar');
    const owned = fabCard(listing({ isFree: false, owned: true, compatibleFormats: ['fbx', 'unreal-engine'] }), { onInstall });
    expect(owned.textContent).toContain('En tu biblioteca');
    owned.querySelector('.fab-card__install').click();
    expect(onInstall).toHaveBeenCalledWith(expect.objectContaining({ uid: 'l1' }), 'unreal-engine');
    const incompatible = fabCard(listing({ compatible: false, compatibleFormats: [] }), { onInstall });
    expect(incompatible.querySelector('.fab-card__install')).toBeNull();
    expect(incompatible.textContent).toContain('No compatible');
    expect(canInstall(listing())).toBe(true);
    expect(defaultFormat(listing({ compatibleFormats: ['fbx'] }))).toBe('fbx');
  });

  it('sin valoraciones no pone 0 y la tira pinta una tarjeta por listing', () => {
    expect(fabCard(listing({ rating: null, ratingCount: 0 })).textContent).toContain('Sin valoraciones');
    expect(fabCardStrip([listing(), listing({ uid: 'l2' })]).querySelectorAll('.fab-card')).toHaveLength(2);
  });
});

describe('herramientas de Fab en la ventana (REQ-105)', () => {
  const specs = [
    { name: 'fab_search', description: 'Search', inputSchema: { type: 'object', properties: { query: { type: 'string' } }, required: ['query'] } },
    { name: 'fab_install', description: 'Install', inputSchema: { type: 'object', properties: { listing_uid: { type: 'string' } }, required: ['listing_uid'] } },
  ];

  it('buscar no pide permiso y pinta las tarjetas aparte del texto', async () => {
    const showListings = vi.fn();
    const askApproval = vi.fn();
    const run = vi.fn(async () => ({ ok: true, value: { text: '{"listings":[]}', fabListings: [listing()] } }));
    const [search] = createFabTools({ specs, run, askApproval, showListings });
    const text = await search.run({ query: 'car' });
    expect(text).toBe('{"listings":[]}');
    expect(askApproval).not.toHaveBeenCalled();
    expect(showListings).toHaveBeenCalledWith([expect.objectContaining({ uid: 'l1' })]);
  });

  it('abrir un listing maduro con confirm_mature lo pregunta a la persona @spec REQ-105', async () => {
    const detailSpecs = [{ name: 'fab_listing_details', description: 'Details', inputSchema: { type: 'object', properties: { listing_uid: { type: 'string' }, confirm_mature: { type: 'boolean' } }, required: ['listing_uid'] } }];
    const run = vi.fn(async () => ({ ok: true, value: { text: 'details', fabListings: [] } }));
    const askApproval = vi.fn(async () => 'deny');
    const [details] = createFabTools({ specs: detailSpecs, run, askApproval });
    expect(await details.run({ listing_uid: 'm1', confirm_mature: true })).toContain('DENIED');
    expect(run).not.toHaveBeenCalled();
    expect(await details.run({ listing_uid: 'n1' })).toBe('details');
    expect(askApproval).toHaveBeenCalledTimes(1);
  });

  it('instalar pide aprobación y, si se niega, no se ejecuta', async () => {
    const run = vi.fn();
    const tools = createFabTools({ specs, run, askApproval: vi.fn(async () => 'deny') });
    const install = tools.find((tool) => tool.name === 'fab_install');
    expect(install.parameters.properties.purpose).toBeDefined();
    expect(await install.run({ listing_uid: 'l1', purpose: 'un coche' })).toContain('DENIED');
    expect(run).not.toHaveBeenCalled();
    const allowed = createFabTools({ specs, run: vi.fn(async () => ({ ok: true, value: { text: 'ok', fabListings: [] } })), askApproval: vi.fn(async () => 'once') });
    expect(await allowed.find((tool) => tool.name === 'fab_install').run({ listing_uid: 'l1' })).toBe('ok');
  });
});
