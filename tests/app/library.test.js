// =============================================================================
// DBV Unreal AI — Tests de la vista Biblioteca
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Lo que se protege: cada asset ofrece el paso siguiente (un modelo se riggea, un rig se anima, una imagen se
// convierte en 3D), siempre por la tarjeta de coste, y nunca más de cinco animaciones por tarea.

import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { createLibraryView, filterEntries, filterEntriesByTab, isV1Rig, IMPORT_EXTENSIONS } from '../../src/app/libraryView.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

const base = { provider: 'tripo', prompt: '', createdAt: 1, files: ['model.glb'], sentTo: [], parameters: {} };
const MODEL = { ...base, id: 'pirata-1', name: 'Pirata', kind: 'model' };
const RIG = { ...base, id: 'pirata-rig', name: 'Pirata (rig)', kind: 'rig', parent: 'pirata-1', parameters: { request: { model: 'v1.0-20240301' } } };
const IMAGE = { ...base, id: 'barco-ref', name: 'Barco', kind: 'image', provider: 'import', files: ['barco.png'] };

let invoked = [];

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

beforeEach(() => {
  invoked = [];
  window.__TAURI__ = {
    core: {
      convertFileSrc: (p) => p,
      invoke: vi.fn(async (command, args) => {
        invoked.push([command, args]);
        const answers = {
          library_list: [MODEL, RIG, IMAGE],
          library_location: { current: 'C:/lib', default: 'C:/lib', isDefault: true },
          library_file_path: 'C:/lib/x',
          generation_status: { spent: 0, budget: 300, hasKey: { tripo: true } },
          generation_prepare: { operation: 'rig', endpoint: '/animations/rig', name: 'Pirata (rig)', variants: 1, params: { input: 'pirata-1' }, total: 25, pricing: '25' },
          generation_launch: ['job-9'],
          generation_presets: { biped: [{ group: 'dance', names: ['dance_01', 'dance_02', 'dance_03', 'dance_04', 'dance_05', 'dance_06'] }], generic: ['idle', 'walk'] },
        };
        return answers[command] ?? null;
      }),
    },
    event: { listen: vi.fn(async () => () => {}) },
    dialog: { open: vi.fn() },
    opener: { revealItemInDir: vi.fn() },
  };
});

const byText = (host, text) => [...host.querySelectorAll('button')].find((b) => b.textContent === text);

function view(over = {}) {
  const host = document.createElement('div');
  const modal = document.createElement('div');
  const deps = {
    state: { connection: null },
    confirm: vi.fn(async () => 'accept'),
    toast: vi.fn(),
    openModal: (children) => modal.replaceChildren(...children),
    closeModal: () => modal.replaceChildren(),
    onLaunched: vi.fn(),
    ...over,
  };
  return { host, modal, deps, library: createLibraryView(host, deps) };
}

describe('Biblioteca (REQ-16, REQ-30)', () => {
  /** @spec REQ-17 */
  it('busca también por tipo y reconoce el rig de las 101 animaciones', () => {
    expect(filterEntries([MODEL, RIG, IMAGE], 'rig').map((e) => e.id)).toEqual(['pirata-rig']);
    expect(isV1Rig(RIG)).toBe(true);
    expect(isV1Rig({ parameters: { request: { model: 'v3.0-20260909' } } })).toBe(false);
  });

  /** @spec REQ-16, RNF-04 */
  it('cada tipo ofrece su paso siguiente y riggear pasa por la tarjeta de coste', async () => {
    const { host, deps, library } = view();
    await library.render();
    const cards = [...host.querySelectorAll('.asset-card')];
    expect(cards.map((c) => c.dataset.kind)).toEqual(['model', 'rig', 'image']);
    expect(cards[1].textContent).toContain('Sale de «Pirata»');
    expect(byText(cards[0], 'Riggear…')).toBeTruthy();
    expect(byText(cards[1], 'Animar…')).toBeTruthy();
    expect(byText(cards[2], 'Generar 3D…')).toBeTruthy();
    expect(byText(cards[2], 'Enviar a Unreal'), 'una imagen no se envía a Unreal como modelo').toBeFalsy();

    byText(cards[0], 'Riggear…').click();
    await vi.waitFor(() => expect(deps.onLaunched).toHaveBeenCalledWith(['job-9']));
    expect(invoked.find(([c]) => c === 'generation_prepare')[1]).toEqual({ tool: 'rig_model', args: { id: 'pirata-1' } });
    expect(deps.confirm).toHaveBeenCalledWith(expect.objectContaining({ detail: expect.stringContaining('POST /animations/rig') }));
  });

  /** @spec REQ-16 */
  it('las animaciones se eligen por grupos y como mucho cinco', async () => {
    const { host, modal, library } = view();
    await library.render();
    byText(host.querySelectorAll('.asset-card')[1], 'Animar…').click();
    await vi.waitFor(() => expect(modal.querySelectorAll('input[type=checkbox]').length).toBe(6));
    const boxes = [...modal.querySelectorAll('input[type=checkbox]')];
    for (const box of boxes) {
      box.checked = true;
      box.dispatchEvent(new Event('change'));
    }
    expect(boxes.filter((b) => b.checked)).toHaveLength(5);
    const go = [...modal.querySelectorAll('button')].find((b) => b.textContent.startsWith('Animar ('));
    expect(go.textContent).toBe('Animar (50 créditos)');
    go.click();
    await vi.waitFor(() => expect(invoked.some(([c]) => c === 'generation_launch')).toBe(true));
    expect(invoked.find(([c]) => c === 'generation_prepare')[1]).toEqual({ tool: 'animate_model', args: { id: 'pirata-rig', animations: ['dance_01', 'dance_02', 'dance_03', 'dance_04', 'dance_05'] } });
  });

  /** @spec REQ-30, REQ-41 */
  it('filtra por apartados Objetos y Escenarios (Gaussian Splatting)', () => {
    const world = { ...base, id: 'isla-1', name: 'Isla pirata', kind: 'world', files: ['isla.spz'] };
    const all = [MODEL, RIG, IMAGE, world];
    expect(filterEntriesByTab(all, 'all')).toHaveLength(4);
    expect(filterEntriesByTab(all, 'objects').map((e) => e.id)).toEqual(['pirata-1', 'pirata-rig', 'barco-ref']);
    expect(filterEntriesByTab(all, 'worlds').map((e) => e.id)).toEqual(['isla-1']);
  });

  /** @spec REQ-30 */
  it('las pestañas de la cabecera conmutan la vista y permiten importar desde URL', async () => {
    const world = { ...base, id: 'isla-1', name: 'Isla pirata', kind: 'world', files: ['isla.spz'] };
    const { host, modal, library } = view();
    // Inyectamos la lista con objetos y escenario
    window.__TAURI__.core.invoke = vi.fn(async (command, args) => {
      invoked.push([command, args]);
      if (command === 'library_list') return [MODEL, RIG, IMAGE, world];
      if (command === 'library_location') return { current: 'C:/lib', default: 'C:/lib', isDefault: true };
      if (command === 'library_import_url') return { ...base, id: 'nuevo-1', name: args.name || 'Descargado', kind: 'world', files: ['descargado.spz'] };
      return null;
    });

    await library.render();

    // Verificamos presencia de los botones de pestañas y botones de importación
    const tabAll = byText(host, 'Todos');
    const tabObjects = byText(host, 'Objetos');
    const tabWorlds = byText(host, 'Escenarios');
    expect(tabAll).toBeTruthy();
    expect(tabObjects).toBeTruthy();
    expect(tabWorlds).toBeTruthy();

    const btnImportFiles = byText(host, 'Importar…');
    const btnImportUrl = byText(host, 'Importar URL…');
    const btnMergeLib = byText(host, 'Fusionar biblioteca…');
    expect(btnImportFiles).toBeTruthy();
    expect(btnImportUrl).toBeTruthy();
    expect(btnMergeLib).toBeTruthy();

    // Estado inicial: Todos (4 tarjetas)
    expect(host.querySelectorAll('.asset-card')).toHaveLength(4);

    // Conmutar a Objetos: 3 tarjetas
    tabObjects.click();
    await vi.waitFor(() => expect(host.querySelectorAll('.asset-card')).toHaveLength(3));
    expect([...host.querySelectorAll('.asset-card')].map((c) => c.dataset.kind)).toEqual(['model', 'rig', 'image']);

    // Conmutar a Escenarios: 1 tarjeta (isla pirata) con icono 🏝️
    tabWorlds.click();
    await vi.waitFor(() => expect(host.querySelectorAll('.asset-card')).toHaveLength(1));
    expect(host.querySelector('.asset-card').dataset.kind).toBe('world');
    expect(host.querySelector('.asset-thumb').textContent).toBe('🏝️');

    // Probar modal de Importar URL
    btnImportUrl.click();
    const urlInput = modal.querySelector('input[type=text]');
    expect(urlInput).toBeTruthy();
    urlInput.value = 'https://cdn.worldlabs.ai/scenes/cueva.spz';
    const submitBtn = byText(modal, 'Descargar e importar');
    expect(submitBtn).toBeTruthy();
    submitBtn.click();

    await vi.waitFor(() => expect(invoked.some(([c]) => c === 'library_import_url')).toBe(true));
    expect(invoked.find(([c]) => c === 'library_import_url')[1]).toEqual({ url: 'https://cdn.worldlabs.ai/scenes/cueva.spz', name: null });
  });

  /** @spec REQ-42 */
  it('IMPORT_EXTENSIONS admite json y un escenario World Labs muestra miniatura y ficha', async () => {
    expect(IMPORT_EXTENSIONS).toContain('json');
    expect(IMPORT_EXTENSIONS).toContain('spz');
    expect(IMPORT_EXTENSIONS).toContain('ply');

    const worldManifestEntry = {
      ...base,
      id: 'isla-wl',
      name: 'Isla Calavera',
      kind: 'world',
      provider: 'worldlabs',
      thumbnail: 'thumbnail.webp',
      files: ['isla.spz', 'isla_collider.glb', 'manifest.json'],
      parameters: {
        metric_scale_factor: 2.1,
        ground_plane_offset: 1.2,
      },
    };

    const { host, library } = view();
    window.__TAURI__.core.invoke.mockImplementation(async (command) => {
      if (command === 'library_list') return [worldManifestEntry];
      if (command === 'library_location') return { current: 'C:/lib', default: 'C:/lib', isDefault: true };
      if (command === 'library_file_path') return 'C:/lib/isla-wl/thumbnail.webp';
      return null;
    });

    await library.render();
    const card = host.querySelector('.asset-card');
    expect(card).toBeTruthy();
    expect(card.dataset.kind).toBe('world');
    expect(card.textContent).toContain('Isla Calavera');
  });
});


