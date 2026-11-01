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
import { createJobCard } from '../../src/generation/jobCard.js';
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
    const logos = [...host.querySelectorAll('[data-mode]')].map((tab) => tab.querySelector('.brand-logo img'));
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

  it('sin IA conectada el botón está desactivado y dice por qué; con un agente avisa de que va a la conversación @spec REQ-66', async () => {
    const host = await renderView({ improvePrompt: vi.fn(), improveAvailable: async () => false });
    const button = host.querySelector('[data-improve="tripo"]');
    await vi.waitFor(() => expect(button.disabled).toBe(true));
    expect(button.title).toContain('Conecta una IA');

    const toast = vi.fn();
    const withAgent = await renderView({ toast, improvePrompt: async () => ({ kind: 'sent' }), improveAvailable: async () => true });
    const field = withAgent.querySelector('textarea');
    field.value = 'cofre del tesoro';
    withAgent.querySelector('[data-improve="tripo"]').click();
    await vi.waitFor(() => expect(toast).toHaveBeenCalledWith(expect.stringContaining('conversación')));
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

  it('con un modelo es una consulta suelta; con un agente va a la conversación; sin IA no hace nada @spec REQ-66', async () => {
    const model = { activeConnection: () => ({ provider: 'ollama' }), complete: vi.fn(async () => 'A cozy village'), ask: vi.fn() };
    expect(await improvePrompt(model, 'worldlabs', 'pueblo', 'l')).toEqual({ kind: 'text', text: 'A cozy village' });
    expect(model.ask).not.toHaveBeenCalled();

    // Con un agente, su respuesta vuelve al campo aunque añada una frase delante.
    const agent = { activeConnection: () => ({ provider: 'agent' }), complete: vi.fn(), ask: vi.fn(async () => 'Here is an improved prompt:\n\nAn old wooden treasure chest with iron bands and a brass lock, stylized.') };
    expect(await improvePrompt(agent, 'tripo', 'cofre', 'Mejorar el prompt: «cofre»')).toEqual({ kind: 'text', text: 'An old wooden treasure chest with iron bands and a brass lock, stylized.' });
    expect(agent.ask.mock.calls[0][0]).toContain('Idea: cofre');
    expect(agent.ask.mock.calls[0][1]).toEqual({ label: 'Mejorar el prompt: «cofre»' });
    expect(agent.complete).not.toHaveBeenCalled();
    // Si el agente no devuelve nada (se paró, falló), la respuesta queda en la conversación.
    expect(await improvePrompt({ ...agent, ask: async () => '' }, 'tripo', 'cofre', 'l')).toEqual({ kind: 'sent' });

    expect(await improvePrompt(null, 'tripo', 'x', 'l')).toEqual({ kind: 'none' });
    expect(await improvePrompt({ ...model, complete: async () => null }, 'tripo', 'x', 'l')).toEqual({ kind: 'empty' });
  });
});
