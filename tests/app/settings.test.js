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
});
