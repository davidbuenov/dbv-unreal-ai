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
        return null;
      }),
    },
  };
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
});
