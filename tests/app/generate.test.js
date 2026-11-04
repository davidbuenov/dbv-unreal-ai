// =============================================================================
// DBV Unreal AI — Tests de la vista Generar: logos y «Mejorar con IA»
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { confirmCost, createGenerateView } from '../../src/app/generateView.js';
import { barPercent, createJobCard, stageText } from '../../src/generation/jobCard.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { cleanImproved, IMPROVE_LIMITS, improveMessages, improvePrompt } from '../../src/generation/promptImprover.js';

const READ_ONLY = new Set(['generation_status', 'generation_jobs', 'generation_balance', 'generation_worldlabs_balance', 'generation_prepare']);
let invoked;

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

beforeEach(() => {
  invoked = [];
  window.__TAURI__ = {
    core: {
      invoke: vi.fn(async (command) => {
        invoked.push(command);
        if (command === 'generation_status') return { spent: 0, budget: 300, hasKey: { tripo: true, worldlabs: true } };
        if (command === 'generation_jobs') return [];
        if (command === 'generation_balance') return { balance: 100, frozen: 0 };
        if (command === 'generation_prepare') return { total: 1580 };
        if (command === 'generation_worldlabs_balance') return { balance: 5000 };
        return null;
      }),
    },
  };
});

async function renderView(deps = {}) {
  const host = document.createElement('div');
  await createGenerateView(host, { confirm: vi.fn(), toast: vi.fn(), onSpent: vi.fn(), openLibrary: vi.fn(), ...deps }).render();
  return host;
}

describe('Generar', () => {
  it('cada pestaña lleva el logo del servicio con el que genera @spec REQ-65', async () => {
    const host = await renderView();
    const logos = [...host.querySelectorAll('[data-mode]')].map((tab) => tab.querySelector('.brand-logo img')).filter(Boolean);
    expect(logos.map((img) => img.getAttribute('alt'))).toEqual(['Tripo', 'Tripo', 'World Labs']);
    for (const img of logos) expect(existsSync(resolve('src', img.getAttribute('src')))).toBe(true);
    // El texto de la pestaña sigue ahí, junto al logo.
    expect(host.querySelector('[data-mode="world"] span').textContent.trim()).not.toBe('');
  });

  it('«Mejorar con IA» sustituye el texto sin generar nada y «Deshacer» lo recupera @spec REQ-66, REQ-27', async () => {
    const improve = vi.fn(async (service, idea) => ({ kind: 'text', text: `A sunlit Mediterranean old town (${service}, ${idea})` }));
    const host = await renderView({ improvePrompt: improve, improveAvailable: async () => true });
    host.querySelector('[data-mode="world"]').click();
    const field = host.querySelectorAll('textarea')[1];
    field.value = 'ciudad bonita';
    const button = host.querySelector('[data-improve="worldlabs"]');
    expect(button.disabled).toBe(false);
    button.click();
    await vi.waitFor(() => expect(field.value).toBe('A sunlit Mediterranean old town (worldlabs, ciudad bonita)'));
    expect(improve).toHaveBeenCalledWith('worldlabs', 'ciudad bonita');
    expect(invoked.every((command) => READ_ONLY.has(command)), invoked.join(', ')).toBe(true);

    const undo = [...button.parentElement.querySelectorAll('button')].find((b) => b.textContent === 'Deshacer');
    expect(undo.hidden).toBe(false);
    undo.click();
    expect(field.value).toBe('ciudad bonita');
  });

  it('sin IA conectada el botón está desactivado y dice por qué; si el agente está ocupado lo dice y no toca el campo @spec REQ-66', async () => {
    const host = await renderView({ improvePrompt: vi.fn(), improveAvailable: async () => false });
    const button = host.querySelector('[data-improve="tripo"]');
    await vi.waitFor(() => expect(button.disabled).toBe(true));
    expect(button.title).toContain('Conecta una IA');

    const toast = vi.fn();
    const busy = await renderView({ toast, improvePrompt: async () => ({ kind: 'busy' }), improveAvailable: async () => true });
    const field = busy.querySelector('textarea');
    field.value = 'cofre del tesoro';
    busy.querySelector('[data-improve="tripo"]').click();
    await vi.waitFor(() => expect(toast).toHaveBeenCalledWith(expect.stringContaining('cuando termine'), 'error'));
    expect(field.value).toBe('cofre del tesoro');
  });
});

describe('Tripo y World Labs, cada uno lo suyo @spec REQ-18, RNF-04', () => {
  it('la tarjeta de coste de un mundo habla de créditos de World Labs y lo que se envía a World Labs', async () => {
    const confirm = vi.fn(async () => 'cancel');
    await confirmCost(confirm, { service: 'worldlabs', name: 'Ciudad', operation: 'generate_world', variants: 1, total: 1580, spent: 0, budget: 2000, endpoint: '/marble/v1/worlds:generate', params: {} });
    const text = confirm.mock.calls[0][0].text;
    expect(text).toContain('1580 créditos de World Labs');
    expect(text).toContain('tope de 2000 créditos de World Labs');
    expect(text).not.toContain('Tripo:');
  });

  it('la consola de un trabajo de World Labs dice a quién se envía', () => {
    const world = createJobCard({ id: 'w', name: 'Ciudad', kind: 'generate_world', endpoint: '/marble/v1/worlds:generate', stage: 'running', log: [], request: {} });
    expect(world.node.querySelector('summary').textContent).toBe('Consola: lo que se envía a World Labs');
    const tripo = createJobCard({ id: 't', name: 'Cofre', kind: 'text_to_model', endpoint: '/generation/text-to-model', stage: 'running', log: [], request: {} });
    expect(tripo.node.querySelector('summary').textContent).toBe('Consola: lo que se envía a Tripo');
  });

  /**
   * Regresión (2026-11-01, lo vio el usuario): un mundo decía «Enviado a Tripo…» y se quedaba en «0 %» los ~5 min,
   * porque World Labs no da porcentaje (solo `IN_PROGRESS`). Ahora: su nombre, el tiempo y una barra estimada.
   * @spec REQ-18, REQ-27
   */
  it('un mundo dice World Labs y, sin porcentaje, enseña el tiempo y una barra estimada', () => {
    const now = 1_000_000_000;
    const world = (stage, extra = {}) => ({ kind: 'generate_world', endpoint: '/marble/v1/worlds:generate', stage, startedAt: now / 1000 - 150, ...extra });
    expect(stageText(world('submitted'), now)).toBe('Enviado a World Labs…');
    expect(stageText(world('running'), now)).toBe('Generando el mundo… 2:30 (suele tardar unos 5 min)');
    expect(barPercent(world('running'), now)).toBe(50);
    expect(barPercent(world('running', { startedAt: now / 1000 - 900 }), now)).toBe(95);
    expect(stageText(world('downloading'), now)).toBe('Descargando el mundo…');
    expect(barPercent(world('done'), now)).toBe(100);
    // Tripo sigue con su porcentaje real.
    const tripo = { kind: 'text_to_model', endpoint: '/generation/text-to-model', stage: 'running', progress: 42, startedAt: now / 1000 };
    expect(stageText(tripo, now)).toBe('Generando… 42%');
    expect(barPercent(tripo, now)).toBe(42);
    expect(stageText({ ...tripo, stage: 'submitted' }, now)).toBe('Enviado a Tripo…');
  });

  it('la tarjeta de un mundo avanza el reloj sola y para al terminar', () => {
    vi.useFakeTimers();
    try {
      const startedAt = Math.floor(Date.now() / 1000);
      const card = createJobCard({ id: 'w', name: 'Ciudad', kind: 'generate_world', endpoint: '/marble/v1/worlds:generate', stage: 'running', startedAt, log: [], request: {} });
      const stage = card.node.querySelector('.job-stage');
      expect(stage.textContent).toContain('0:00');
      vi.advanceTimersByTime(61_000);
      expect(stage.textContent).toContain('1:01');
      card.update({ ...card.record(), stage: 'done' });
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('Mejorar el prompt', () => {
  it('cada servicio tiene sus instrucciones y la idea va tal cual @spec REQ-66', () => {
    const [tripoSystem, tripoUser] = improveMessages('tripo', '  cofre del tesoro ');
    expect(tripoSystem.content).toContain('ONE object');
    expect(tripoUser).toEqual({ role: 'user', content: 'cofre del tesoro' });
    expect(improveMessages('worldlabs', 'x')[0].content).toContain('explorable 3D place');
  });

  it('limpia la respuesta y la recorta al tope del servicio @spec REQ-66', () => {
    expect(cleanImproved('<think>hmm</think>\n```text\nPrompt: "A wooden chest\nwith iron bands"\n```', 'tripo')).toBe('A wooden chest with iron bands');
    expect(cleanImproved('   ', 'tripo')).toBeNull();
    const long = cleanImproved(`${'word '.repeat(600)}`, 'tripo');
    expect(long.length).toBeLessThanOrEqual(IMPROVE_LIMITS.tripo);
    expect(long.endsWith('word')).toBe(true);
  });

  it('con un modelo o con un agente es una consulta suelta, nunca la conversación; sin IA no hace nada @spec REQ-66', async () => {
    const model = { activeConnection: () => ({ provider: 'ollama' }), complete: vi.fn(async () => 'A cozy village'), ask: vi.fn(), askAside: vi.fn() };
    expect(await improvePrompt(model, 'worldlabs', 'pueblo')).toEqual({ kind: 'text', text: 'A cozy village' });
    expect(model.ask).not.toHaveBeenCalled();

    // Regresión (2026-11-01): con un agente la petición iba a la conversación y la app saltaba a ella. Ahora va a una
    // sesión aparte y su respuesta vuelve al campo aunque añada una frase delante.
    const agent = { activeConnection: () => ({ provider: 'agent' }), complete: vi.fn(), ask: vi.fn(), askAside: vi.fn(async () => 'Here is an improved prompt:\n\nAn old wooden treasure chest with iron bands and a brass lock, stylized.') };
    expect(await improvePrompt(agent, 'tripo', 'cofre')).toEqual({ kind: 'text', text: 'An old wooden treasure chest with iron bands and a brass lock, stylized.' });
    expect(agent.askAside.mock.calls[0][0]).toContain('Idea: cofre');
    expect(agent.ask).not.toHaveBeenCalled();
    expect(agent.complete).not.toHaveBeenCalled();
    expect(await improvePrompt({ ...agent, askAside: async () => '' }, 'tripo', 'cofre')).toEqual({ kind: 'empty' });
    const busy = async () => Promise.reject(Object.assign(new Error('ocupado'), { kind: 'busy' }));
    expect(await improvePrompt({ ...agent, askAside: busy }, 'tripo', 'cofre')).toEqual({ kind: 'busy' });
    expect(await improvePrompt({ ...agent, askAside: async () => Promise.reject(new Error('sin red')) }, 'tripo', 'cofre')).toEqual({ kind: 'error', message: 'sin red' });

    expect(await improvePrompt(null, 'tripo', 'x')).toEqual({ kind: 'none' });
    expect(await improvePrompt({ ...model, complete: async () => null }, 'tripo', 'x')).toEqual({ kind: 'empty' });
  });

  it('los botones de ayuda en las pestañas de Generar abren la sección correspondiente', async () => {
    const host = await renderView();
    const opened = [];
    document.addEventListener('dbv-open-help', (e) => opened.push(e.detail.tab));

    const findButton = (text) => [...host.querySelectorAll('button')].find((b) => b.textContent.includes(text));

    findButton('Ayuda de Tripo')?.click();
    host.querySelector('[data-mode="world"]').click();
    findButton('Ayuda de World Labs')?.click();
    host.querySelector('[data-mode="sound"]').click();
    findButton('Ayuda de Sonidos')?.click();

    expect(opened).toEqual(['tripo', 'worldlabs', 'audio']);
  });

  it('permite buscar en Freesound, previsualizar audio en lista y añadir sonidos a la biblioteca @spec REQ-71', async () => {
    let savedSoundPayload = null;
    let searchedQuery = null;
    window.__TAURI__.core.invoke = vi.fn(async (command, payload) => {
      invoked.push(command);
      if (command === 'generation_status') return { spent: 0, budget: 300, hasKey: { tripo: true, worldlabs: true, freesound: true, elevenlabs: true } };
      if (command === 'generation_jobs') return [];
      if (command === 'generation_balance') return { balance: 100, frozen: 0 };
      if (command === 'generation_prepare') return { total: 1580 };
      if (command === 'generation_check_local_audio') return { ready: true, device: 'CUDA' };
      if (command === 'generation_search_freesound') {
        searchedQuery = payload.query;
        return [
          {
            id: 101,
            name: 'grua-movimiento.wav',
            tags: ['crane', 'heavy', 'metal'],
            duration: 4.5,
            previewUrl: 'https://freesound.org/previews/101.mp3',
            downloadUrl: null,
          },
          {
            id: 102,
            name: 'grua-motor.wav',
            tags: ['engine', 'motor'],
            duration: 8.2,
            previewUrl: 'https://freesound.org/previews/102.mp3',
            downloadUrl: null,
          },
        ];
      }
      if (command === 'generation_save_sound_url') {
        savedSoundPayload = payload;
        return { id: 'snd-101', name: payload.name };
      }
      return null;
    });

    const host = await renderView();
    host.querySelector('[data-mode="sound"]').click();

    const providerSelect = host.querySelector('select[aria-label="Proveedor"]');
    const nameInput = host.querySelector('input[aria-label="Nombre corto"]');
    const promptInput = host.querySelector('textarea[aria-label="Descripción del sonido"]');
    const startButton = host.querySelector('.btn-primary');

    expect(startButton.textContent).toBe('Generar…');

    // Cambiar a freesound
    providerSelect.value = 'freesound';
    providerSelect.dispatchEvent(new Event('change'));

    expect(startButton.textContent).toBe('Buscar en Freesound');

    promptInput.value = 'movimiento de grua';
    nameInput.value = 'sonidogrua1';

    // Buscar
    startButton.click();

    await vi.waitFor(() => expect(searchedQuery).toBe('movimiento de grua'));

    const resultsList = host.querySelector('.sound-results-list');
    expect(resultsList.classList.contains('hidden')).toBe(false);

    await vi.waitFor(() => expect(host.querySelectorAll('.sound-item-card').length).toBe(2));

    const cards = host.querySelectorAll('.sound-item-card');
    const firstCard = cards[0];
    expect(firstCard.querySelector('.sound-item-title').textContent).toBe('grua-movimiento.wav');
    expect(firstCard.querySelector('.sound-item-player').getAttribute('src')).toBe('https://freesound.org/previews/101.mp3');

    // Añadir el primer sonido a la biblioteca
    const addBtn = firstCard.querySelector('button');
    expect(addBtn.textContent).toBe('+ Añadir a la biblioteca');

    addBtn.click();

    await vi.waitFor(() => expect(savedSoundPayload).not.toBeNull());
    await vi.waitFor(() => expect(addBtn.textContent).toBe('✓ Añadido'));

    expect(savedSoundPayload).toEqual(
      expect.objectContaining({
        url: 'https://freesound.org/previews/101.mp3',
        name: 'sonidogrua1',
        prompt: 'movimiento de grua',
        duration: 4.5,
      }),
    );
  });

  it('permite generar sonido con ElevenLabs pasando duración, loop y nombre personalizado @spec REQ-71', async () => {
    let generatedPayload = null;
    window.__TAURI__.core.invoke = vi.fn(async (command, payload) => {
      invoked.push(command);
      if (command === 'generation_status') return { spent: 0, budget: 300, hasKey: { tripo: true, worldlabs: true, freesound: true, elevenlabs: true } };
      if (command === 'generation_jobs') return [];
      if (command === 'generation_balance') return { balance: 100, frozen: 0 };
      if (command === 'generation_prepare') return { total: 1580 };
      if (command === 'generation_generate_sound') {
        generatedPayload = payload;
        return { id: 'snd-el-1', name: payload.name || 'sonido' };
      }
      return null;
    });

    const host = await renderView();
    host.querySelector('[data-mode="sound"]').click();

    const providerSelect = host.querySelector('select[aria-label="Proveedor"]');
    providerSelect.value = 'elevenlabs';
    providerSelect.dispatchEvent(new Event('change'));

    const nameInput = host.querySelector('input[aria-label="Nombre corto"]');
    const promptInput = host.querySelector('textarea[aria-label="Descripción del sonido"]');
    const durationInput = host.querySelector('input[aria-label="Duración (segundos)"]');
    const loopInput = host.querySelector('input[aria-label="En bucle (loop)"]');
    const startButton = host.querySelector('.btn-primary');

    expect(startButton.textContent).toBe('Generar…');

    promptInput.value = 'sonido de movimiento de una grua';
    nameInput.value = 'sonidogrua_eleven';
    durationInput.value = '7';
    loopInput.checked = true;

    startButton.click();

    await vi.waitFor(() => expect(generatedPayload).not.toBeNull());

    expect(generatedPayload).toEqual(
      expect.objectContaining({
        provider: 'elevenlabs',
        prompt: 'sonido de movimiento de una grua',
        name: 'sonidogrua_eleven',
        duration: 7,
        loop: true,
      }),
    );
  });
});
