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

  /**
   * Regresión (2026-11-01, la vio el usuario en el log de Unreal: «Unknown tool: get_project_info» en cada conexión):
   * se llamaba sin toolset y se leía el sobre `{text, isError, value}` en vez de `value`, así que nunca se comprobaba.
   * @spec REQ-07
   */
  it('la identidad del editor se pide al toolset DBV y lee su valor; sin toolset no se pide', async () => {
    const endpoint = { port: 8001, path: '/mcp' };
    window.__TAURI__.core.invoke.mockImplementation(async (command, args) => {
      invoked.push([command, args]);
      return command === 'unreal_call_tool' ? { text: '', isError: false, value: { project_name: 'Otro' } } : null;
    });
    const { editorIdentity } = await import('../../src/app/projectView.js');

    expect(await editorIdentity(endpoint, [{ name: 'EditorToolset.LogsToolset' }, { name: 'DBVUnrealAICpp.DBVUnrealAICppToolset' }])).toEqual({ project_name: 'Otro' });
    expect(invoked.find(([c]) => c === 'unreal_call_tool')[1]).toMatchObject({ endpoint, toolset: 'DBVUnrealAICpp.DBVUnrealAICppToolset', tool: 'get_project_info' });

    invoked.length = 0;
    expect(await editorIdentity(endpoint, [{ name: 'EditorToolset.LogsToolset' }])).toBeNull();
    expect(invoked).toEqual([]);
  });

  it('no se conecta a un editor que tiene abierto otro proyecto @spec REQ-07', async () => {
    const base = window.__TAURI__.core.invoke.getMockImplementation();
    window.__TAURI__.core.invoke.mockImplementation(async (command, args) => {
      if (command === 'unreal_connect') { invoked.push([command, args]); return [{ name: 'PythonTypes.DBVUnrealAIToolset_0x1' }]; }
      if (command === 'unreal_call_tool') { invoked.push([command, args]); return { text: '', isError: false, value: { project_name: 'Otro' } }; }
      return base(command, args);
    });
    const host = document.createElement('div');
    const deps = { state: { connection: null, setConnection: vi.fn() }, toast: vi.fn(), openModal: vi.fn(), closeModal: vi.fn(), onMemoryChanged: vi.fn(), showView: vi.fn() };
    const view = createProjectView(host, deps);
    await view.select(PROJECT);
    [...host.querySelectorAll('button')].find((b) => b.textContent === 'Conectar').click();
    await vi.waitFor(() => expect(deps.toast).toHaveBeenCalled());
    expect(deps.toast).toHaveBeenCalledWith('El editor en el puerto 8000 pertenece a «Otro», no a «Demo».', 'error');
    expect(deps.state.setConnection).toHaveBeenLastCalledWith({ status: 'offline' }, true);
  });
});
