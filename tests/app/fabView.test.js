// =============================================================================
// DBV Unreal AI — Tests de la vista «Fab»
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-106

import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { createFabView, parseToolJson } from '../../src/app/fabView.js';

const CAR = { uid: 'c1', title: 'Camo muscle car', sellerName: 'FoulQuince', isFree: true, owned: false, rating: 5, ratingCount: 1, fabUrl: 'https://www.fab.com/listings/c1', thumbnail: 'https://media.fab.com/c1.jpg', mature: false, compatibleFormats: ['fbx'], compatible: true };
const WRECK = { ...CAR, uid: 'm1', title: 'Battle Wreck Car', mature: true, fabUrl: 'https://www.fab.com/listings/m1' };
let invoked;

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

beforeEach(() => {
  invoked = [];
  window.__TAURI__ = {
    core: {
      invoke: vi.fn(async (command, args) => {
        invoked.push([command, args]);
        if (command !== 'fab_run_tool') return null;
        const page = (items, next) => ({ text: `${JSON.stringify({ listings: [], next_cursor: next })}\nNever state a total.`, fabListings: items });
        if (args.name === 'fab_search') return args.args.cursor ? page([WRECK], null) : page([CAR], 'bz0x');
        if (args.name === 'fab_sign_in') return { text: 'The Fab session of the editor is active.', fabListings: [] };
        if (args.name === 'fab_listing_details') return { text: `${JSON.stringify({ description: 'A car.', licenses: ['Personal'] })}\nShow the user the fab_url.`, fabListings: [args.args.listing_uid === 'm1' ? WRECK : CAR] };
        if (args.name === 'fab_install') return { text: '{"success":true,"folder":"/Game/Fab/Camo_muscle_car"}', fabListings: [] };
        return { text: '{"listings":[]}', fabListings: [] };
      }),
    },
    opener: { openUrl: vi.fn() },
  };
});

const runs = (name) => invoked.filter(([c, a]) => c === 'fab_run_tool' && a.name === name);
const byText = (host, text) => [...host.querySelectorAll('button')].find((b) => b.textContent === text);

function view(connection = null, confirmAnswer = 'install') {
  const host = document.createElement('div');
  const modal = document.createElement('div');
  const deps = { state: { connection }, toast: vi.fn(), confirm: vi.fn(async () => confirmAnswer), openModal: (children) => modal.replaceChildren(...children.filter(Boolean)), closeModal: () => modal.replaceChildren() };
  return { host, modal, deps, fab: createFabView(host, deps) };
}

const ONLINE = { status: 'online', endpoint: { port: 8000, path: '/mcp' }, engineVersion: '5.8', projectPath: 'D:/P/A.uproject', projectName: 'A' };

describe('Vista Fab (REQ-106)', () => {
  it('busca sin editor, enseña tarjetas sin total y carga más con el cursor', async () => {
    const { host, fab } = view();
    await fab.render();
    expect(runs('fab_search')[0][1].args).toMatchObject({ free_only: true, cursor: '' });
    expect(runs('fab_search')[0][1].endpoint).toBeNull();
    expect(host.querySelectorAll('.fab-card')).toHaveLength(1);
    expect(host.textContent).toContain('Sin editor conectado');
    expect(host.querySelector('.fab-card__install'), 'sin editor no se instala').toBeNull();
    byText(host, 'Cargar más').click();
    await vi.waitFor(() => expect(host.querySelectorAll('.fab-card')).toHaveLength(2));
    expect(runs('fab_search')[1][1].args.cursor).toBe('bz0x');
    expect(host.textContent).not.toMatch(/\d+ resultados/);
  });

  it('con el editor, enseña la sesión e instala tras confirmar', async () => {
    const { host, deps, fab } = view(ONLINE);
    await fab.render();
    expect(host.textContent).toContain('● Sesión de Fab');
    host.querySelector('.fab-card__install').click();
    await vi.waitFor(() => expect(runs('fab_install')).toHaveLength(1));
    expect(deps.confirm).toHaveBeenCalled();
    expect(runs('fab_install')[0][1]).toMatchObject({ args: { listing_uid: 'c1', asset_format: 'fbx' }, project: 'D:/P/A.uproject' });
    await vi.waitFor(() => expect(deps.toast).toHaveBeenCalledWith('«Camo muscle car» instalado en /Game/Fab/Camo_muscle_car.', undefined));
  });

  it('la ficha de un listing maduro se pregunta antes y solo se pide con confirm_mature si se acepta', async () => {
    const { host, deps, fab } = view(null, 'cancel');
    await fab.render();
    byText(host, 'Cargar más').click();
    await vi.waitFor(() => expect(host.querySelectorAll('.fab-card')).toHaveLength(2));
    host.querySelectorAll('.fab-card__title')[1].click();
    await vi.waitFor(() => expect(deps.confirm).toHaveBeenCalled());
    expect(runs('fab_listing_details')).toHaveLength(0);
    deps.confirm.mockResolvedValue('open');
    host.querySelectorAll('.fab-card__title')[1].click();
    await vi.waitFor(() => expect(runs('fab_listing_details')).toHaveLength(1));
    expect(runs('fab_listing_details')[0][1].args).toEqual({ listing_uid: 'm1', confirm_mature: true });
  });

  it('«Mi biblioteca» sin editor no consulta nada y lo dice', async () => {
    const { host, fab } = view();
    await fab.render();
    byText(host, 'Mi biblioteca').click();
    expect(host.textContent).toContain('Conecta el editor de Unreal para ver tu biblioteca e instalar.');
    expect(runs('fab_my_library')).toHaveLength(0);
  });

  it('lee el JSON de la primera línea del texto de una herramienta', () => {
    expect(parseToolJson('{"next_cursor":"x"}\nNever state a total.')).toEqual({ next_cursor: 'x' });
    expect(parseToolJson('error: nope')).toBeNull();
  });
});
