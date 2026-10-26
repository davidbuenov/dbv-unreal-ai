// =============================================================================
// DBV Unreal AI — Tests de la vista Proyecto con el backend de Tauri simulado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { createProjectView } from '../../src/app/projectView.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

const PROJECT = 'D:/Unreal/Demo/Demo.uproject';
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
        if (command === 'unreal_editors') return [{ editor: { project: PROJECT }, project: { path: PROJECT, name: 'Demo', engineAssociation: '5.8' }, listening: true, mcp: { port: 8000, path: '/mcp' } }];
        if (command === 'unreal_recent_projects') return [];
        if (command === 'unreal_readiness') return { project: { name: 'Demo', hasCpp: false }, mcp: { port: 8000 }, checks: [{ id: 'engine', ok: true, detail: '5.8.4' }], ready: true };
        if (command === 'memory_status') return { active: false, memory: { exists: false, text: '', hash: '' }, tasks: { exists: false, text: '', hash: '' }, taskList: [], summary: '', missing: [{ file: 'dbv-unreal-ai/memory.md', text: '# Project memory' }, { file: 'dbv-unreal-ai/task.md', text: '# Tasks' }] };
        if (command === 'memory_activate') return {};
        return null;
      }),
    },
  };
});

describe('Vista Proyecto', () => {
  /**
   * Regresión (2026-10-26, la vio el usuario): con un proyecto seleccionado la vista se quedaba en blanco porque la
   * tarjeta de memoria usaba `dirName` sin importarlo.
   * @spec REQ-34, REQ-04
   */
  it('con un proyecto seleccionado pinta la preparación y la memoria, y enseña los ficheros antes de activarla', async () => {
    const host = document.createElement('div');
    const deps = { state: { connection: null, setConnection: vi.fn() }, toast: vi.fn(), openModal: vi.fn(), closeModal: vi.fn(), onMemoryChanged: vi.fn(), showView: vi.fn() };
    const view = createProjectView(host, deps);
    await view.select(PROJECT);
    expect(host.textContent).toContain('Demo');
    expect(host.textContent).toContain('Memoria del proyecto');
    expect(invoked.find(([c]) => c === 'memory_status')[1]).toEqual({ project: PROJECT });
    [...host.querySelectorAll('button')].find((b) => b.textContent === 'Activar memoria del proyecto').click();
    const modal = deps.openModal.mock.calls[0][0];
    expect(modal.map((n) => n.textContent).join('\n')).toContain('dbv-unreal-ai/task.md');
    expect(invoked.some(([c]) => c === 'memory_activate')).toBe(false);
  });
});
