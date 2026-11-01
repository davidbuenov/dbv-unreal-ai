// =============================================================================
// DBV Unreal AI — Tests de la vista Ajustes con el backend de Tauri simulado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { createSettingsView } from '../../src/app/settingsView.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

const DIAGNOSTICS = { app: 'DBV Unreal AI', version: '0.1.0', os: 'windows', engines: [{ version: '5.8.2' }], editors: [] };
let invoked;

// En la app los registra `app.js` antes de crear las vistas.
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
        if (command === 'generation_status') return { spent: 0, budget: 300, hasKey: { tripo: true }, tripoTextToModel: 30 };
        if (command === 'app_mcp_command') return { command: 'C:/app/dbv-unreal-ai.exe', args: ['--mcp', '--port', '8000'] };
        if (command === 'app_diagnostics') return DIAGNOSTICS;
        if (command === 'generation_set_key') return 'system';
        if (command === 'generation_balance') return { balance: 2370, frozen: 30 };
        if (command === 'library_location') return { current: 'C:/Users/u/AppData/Roaming/app/library', default: 'C:/Users/u/AppData/Roaming/app/library', isDefault: true };
        if (command === 'library_set_location') return { current: args.path, default: 'C:/x', isDefault: false };
        if (command === 'convai_characters_list') return [{ name: 'Victoria', characterId: 'c1fca5ca-03b4-11ef-ae52-42010a7be00e', notes: '' }];
        if (command === 'convai_characters_save') return args.characters;
        return null;
      }),
    },
  };
  window.__TAURI__.dialog = { open: vi.fn(async () => 'D:/Compartida/Assets') };
  window.__TAURI__.opener = { revealItemInDir: vi.fn() };
  Object.assign(navigator, { clipboard: { writeText: vi.fn(async () => {}) } });
});

const buttonWithText = (host, text) => [...host.querySelectorAll('button')].find((b) => b.textContent === text);

describe('Ajustes', () => {
  it('«Copiar diagnóstico» copia el informe del backend y avisa @spec RNF-08', async () => {
    const host = document.createElement('div');
    const toast = vi.fn();
    await createSettingsView(host, { toast, openConnect: vi.fn(), state: { connection: null } }).render();
    buttonWithText(host, 'Copiar diagnóstico').click();
    await vi.waitFor(() => expect(navigator.clipboard.writeText).toHaveBeenCalled());
    expect(JSON.parse(navigator.clipboard.writeText.mock.calls[0][0])).toEqual(DIAGNOSTICS);
    expect(toast).toHaveBeenCalledWith('Diagnóstico copiado');
  });

  it('la clave de Tripo solo viaja de ida y el campo se vacía @spec RNF-IA.4', async () => {
    const host = document.createElement('div');
    await createSettingsView(host, { toast: vi.fn(), openConnect: vi.fn(), state: { connection: null } }).render();
    const input = host.querySelector('input[type="password"]');
    expect(input.value).toBe('');
    expect(host.textContent).toMatch(/Hay una clave guardada/);
    input.value = 'sk-secreta';
    buttonWithText(host, 'Guardar').click();
    await vi.waitFor(() => expect(invoked.some(([c]) => c === 'generation_set_key')).toBe(true));
    expect(invoked.find(([c]) => c === 'generation_set_key')[1]).toEqual({ service: 'tripo', key: 'sk-secreta' });
    expect(input.value).toBe('');
  });

  it('la clave de World Labs tiene su propio campo y va al mismo llavero @spec REQ-18, RNF-IA.4', async () => {
    const host = document.createElement('div');
    await createSettingsView(host, { toast: vi.fn(), openConnect: vi.fn(), state: { connection: null } }).render();
    const input = host.querySelector('input[aria-label="Clave de API de World Labs"]');
    expect(input).not.toBeNull();
    expect(host.textContent).toContain('platform.worldlabs.ai');
    const card = input.closest('.card');
    expect(card.textContent).toMatch(/No hay clave/);
    input.value = 'wlt-secreta';
    [...card.querySelectorAll('button')].find((b) => b.textContent === 'Guardar').click();
    await vi.waitFor(() => expect(invoked.some(([c]) => c === 'generation_set_key')).toBe(true));
    expect(invoked.find(([c]) => c === 'generation_set_key')[1]).toEqual({ service: 'worldlabs', key: 'wlt-secreta' });
    expect(input.value).toBe('');
  });

  /**
   * La biblioteca puede vivir en una carpeta compartida con otras apps; por defecto se copia lo que ya había.
   * @spec REQ-30
   */
  it('cambia la carpeta de la biblioteca copiando lo que había, y enseña el saldo de Tripo', async () => {
    const host = document.createElement('div');
    const toast = vi.fn();
    await createSettingsView(host, { toast, openConnect: vi.fn(), state: { connection: null } }).render();
    expect(host.textContent).toContain('C:/Users/u/AppData/Roaming/app/library');
    await vi.waitFor(() => expect(host.textContent).toContain('Saldo de Tripo: 2370 créditos (30 reservados)'));
    buttonWithText(host, 'Cambiar carpeta…').click();
    await vi.waitFor(() => expect(invoked.some(([c]) => c === 'library_set_location')).toBe(true));
    expect(invoked.find(([c]) => c === 'library_set_location')[1]).toEqual({ path: 'D:/Compartida/Assets', copyExisting: true });
    await vi.waitFor(() => expect(toast).toHaveBeenCalledWith('La biblioteca está en la carpeta nueva'));
  });

  it('cada apartado va en su pestaña y la elegida se conserva al refrescar @spec REQ-60', async () => {
    const host = document.createElement('div');
    const view = createSettingsView(host, { toast: vi.fn(), openConnect: vi.fn(), state: { connection: null } });
    await view.render();
    const tabs = [...host.querySelectorAll('[data-settings-tab]')].map((b) => b.textContent);
    expect(tabs).toEqual(['Inteligencia artificial', 'Tripo', 'World Labs', 'Convai', 'Biblioteca', 'Agentes (MCP)', 'Diagnóstico']);
    const visible = () => [...host.querySelectorAll('[data-settings-panel]')].filter((p) => !p.classList.contains('hidden')).map((p) => p.dataset.settingsPanel);
    expect(visible()).toEqual(['ai']);
    buttonWithText(host, 'Convai').click();
    expect(visible()).toEqual(['convai']);
    await view.render();
    expect(visible()).toEqual(['convai']);
  });

  it('las pestañas de Tripo, World Labs y Convai llevan el logo del servicio, y el logo existe @spec REQ-60', async () => {
    const { existsSync } = await import('node:fs');
    const { resolve } = await import('node:path');
    const host = document.createElement('div');
    await createSettingsView(host, { toast: vi.fn(), openConnect: vi.fn(), state: { connection: null } }).render();
    for (const [panel, name] of [['tripo', 'Tripo'], ['worldlabs', 'World Labs'], ['convai', 'Convai']]) {
      const header = host.querySelector(`[data-settings-panel="${panel}"] .brand-header`);
      expect(header.querySelector('h3').textContent).toBe(name);
      const logo = header.querySelector('.brand-logo img');
      expect(logo.getAttribute('alt')).toBe(name);
      expect(existsSync(resolve('src', logo.getAttribute('src'))), logo.getAttribute('src')).toBe(true);
    }
  });

  it('Convai: sin clave, con los personajes registrados, añadir y guardar; el tutorial abre la ayuda @spec REQ-58, REQ-59', async () => {
    const host = document.createElement('div');
    const toast = vi.fn();
    await createSettingsView(host, { toast, openConnect: vi.fn(), state: { connection: null } }).render();
    const card = host.querySelector('[data-settings-panel="convai"]');
    expect(card.querySelector('input[type="password"]')).toBeNull();
    expect(card.textContent).toContain('No necesita clave');
    expect([...card.querySelectorAll('input[aria-label="Character ID"]')].map((i) => i.value)).toEqual(['c1fca5ca-03b4-11ef-ae52-42010a7be00e']);
    buttonWithText(card, 'Añadir personaje').click();
    const rows = card.querySelectorAll('.convai-character');
    rows[1].querySelector('input[aria-label="Nombre"]').value = 'Guía';
    rows[1].querySelector('input[aria-label="Character ID"]').value = 'abcdef12-0000';
    buttonWithText(card, 'Guardar').click();
    await vi.waitFor(() => expect(invoked.some(([c]) => c === 'convai_characters_save')).toBe(true));
    expect(invoked.find(([c]) => c === 'convai_characters_save')[1].characters.map((c) => c.name)).toEqual(['Victoria', 'Guía']);
    await vi.waitFor(() => expect(toast).toHaveBeenCalledWith('Personajes guardados: 2', 'info'));
    const opened = vi.fn();
    document.addEventListener('dbv-open-help', (e) => opened(e.detail.tab), { once: true });
    buttonWithText(card, 'Tutorial: qué necesitas y cómo se usa').click();
    expect(opened).toHaveBeenCalledWith('convai');
  });
});
