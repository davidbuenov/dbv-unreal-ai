// =============================================================================
// DBV Unreal AI — Tests de la vista Proyecto con el backend de Tauri simulado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { createProjectView, diagnosticSteps, diagnosticsText } from '../../src/app/projectView.js';
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
   * El amigo del usuario (2026-11-15) veía «no hay una instalación de Unreal {GUID}» sin salida: si no se encuentra el
   * motor, la vista ofrece elegir su carpeta; si se eligió a mano, dice cuál y cómo volver a la automática.
   * @spec REQ-119
   */
  it('sin motor ofrece elegir la carpeta de Unreal y guarda la elección', async () => {
    const base = window.__TAURI__.core.invoke.getMockImplementation();
    window.__TAURI__.core.invoke.mockImplementation(async (command, args) => {
      if (command === 'unreal_readiness') return { project: { name: 'Demo', path: PROJECT, hasCpp: false }, engine: null, mcp: { port: 8000 }, checks: [{ id: 'engine', ok: false, detail: '{95B798E8-4CE8-821F-3703-E085BBF457EF}' }], ready: false };
      if (command === 'unreal_pick_engine') { invoked.push([command, args]); return { version: '5.8.4', root: 'E:/UE_5.8' }; }
      return base(command, args);
    });
    window.__TAURI__.dialog = { open: vi.fn(async () => 'E:/UE_5.8/Engine') };
    const host = document.createElement('div');
    const deps = { state: { connection: null, setConnection: vi.fn() }, toast: vi.fn(), openModal: vi.fn(), closeModal: vi.fn() };
    const view = createProjectView(host, deps);
    await view.select(PROJECT);
    const pick = [...host.querySelectorAll('button')].find((b) => b.textContent === 'Elegir la carpeta de Unreal…');
    expect(pick).toBeDefined();
    pick.click();
    await vi.waitFor(() => expect(invoked.some(([c]) => c === 'unreal_pick_engine')).toBe(true));
    expect(invoked.find(([c]) => c === 'unreal_pick_engine')[1]).toEqual({ project: PROJECT, folder: 'E:/UE_5.8/Engine' });
    await vi.waitFor(() => expect(deps.toast).toHaveBeenCalledWith('Listo: este proyecto usará Unreal 5.8.4.'));
  });

  it('con el plugin C++ ya compilado, un proyecto de solo Blueprints puede elegir la variante C++ y va marcada @spec REQ-121', async () => {
    const base = window.__TAURI__.core.invoke.getMockImplementation();
    const readinessFor = (cppBinaries) => ({ project: { name: 'Demo', path: PROJECT, hasCpp: false }, engine: { version: '5.8.4', root: 'E:/UE_5.8' }, cppBinaries, mcp: { port: 8000 }, checks: [{ id: 'engine', ok: true, detail: '5.8.4' }, { id: 'toolset', ok: false, detail: '' }], ready: false });
    const deps = { state: { connection: null, setConnection: vi.fn() }, toast: vi.fn(), openModal: vi.fn(), closeModal: vi.fn() };
    window.__TAURI__.core.invoke.mockImplementation(async (command, args) => (command === 'unreal_readiness' ? readinessFor(true) : base(command, args)));
    const withBinaries = document.createElement('div');
    await createProjectView(withBinaries, deps).select(PROJECT);
    const select = withBinaries.querySelector('select');
    expect(select.disabled).toBe(false);
    expect(select.value).toBe('cpp');
    window.__TAURI__.core.invoke.mockImplementation(async (command, args) => (command === 'unreal_readiness' ? readinessFor(false) : base(command, args)));
    const without = document.createElement('div');
    await createProjectView(without, deps).select(PROJECT);
    expect(without.querySelector('select').disabled).toBe(true);
  });

  it('con el motor encontrado no enseña el botón; elegido a mano, enseña cuál y la vuelta a la automática @spec REQ-119', async () => {
    const host = document.createElement('div');
    const deps = { state: { connection: null, setConnection: vi.fn() }, toast: vi.fn(), openModal: vi.fn(), closeModal: vi.fn() };
    await createProjectView(host, deps).select(PROJECT);
    expect(host.textContent).not.toContain('Elegir la carpeta de Unreal');
    const base = window.__TAURI__.core.invoke.getMockImplementation();
    window.__TAURI__.core.invoke.mockImplementation(async (command, args) => (command === 'unreal_readiness' ? { project: { name: 'Demo', path: PROJECT, hasCpp: false }, engine: { version: '5.8.4', root: 'E:/UE_5.8' }, engineSource: 'chosen', mcp: { port: 8000 }, checks: [{ id: 'engine', ok: true, detail: '5.8.4' }], ready: true } : base(command, args)));
    const chosen = document.createElement('div');
    await createProjectView(chosen, deps).select(PROJECT);
    expect(chosen.textContent).toContain('Motor elegido a mano: Unreal 5.8.4 en E:/UE_5.8.');
    expect([...chosen.querySelectorAll('button')].some((b) => b.textContent === 'Usar la detección automática')).toBe(true);
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

describe('Diagnóstico en lenguaje llano (REQ-120)', () => {
  const readiness = {
    mcp: { port: 8000 },
    cppPlugin: 'needsCompile',
    cppBinaries: true,
    checks: [{ id: 'engine', ok: false, detail: '{95B798E8}' }, { id: 'plugin:ModelContextProtocol', ok: true, detail: '' }, { id: 'autoStart', ok: false, detail: '8000' }],
  };

  it('cada paso que falla lleva su solución en una frase; sin editor, se dice cómo abrirlo', () => {
    const steps = diagnosticSteps(readiness, null);
    const engine = steps.find((s) => s.label.startsWith('Unreal Engine 5.8'));
    expect(engine.ok).toBe(false);
    expect(engine.fix).toContain('Elegir la carpeta de Unreal');
    expect(steps.find((s) => s.label.startsWith('Plugin ModelContextProtocol')).fix).toBe('');
    expect(steps.find((s) => s.label.startsWith('Arranque automático')).fix).toContain('Preparar proyecto');
    expect(steps.find((s) => s.label === 'Editor de Unreal abierto con este proyecto').fix).toContain('Abrir en Unreal');
    expect(steps.find((s) => s.label.startsWith('Plugin C++ sin compilar')).fix).toContain('ya compilado');
  });

  it('con el editor abierto y el MCP respondiendo, esos pasos salen bien', () => {
    const steps = diagnosticSteps({ ...readiness, cppPlugin: 'compiled' }, { listening: true, mcp: { port: 8001 } });
    expect(steps.find((s) => s.label === 'Servidor MCP del editor respondiendo (puerto 8001)').ok).toBe(true);
    expect(steps.find((s) => s.label.startsWith('Plugin C++ compilado')).ok).toBe(true);
  });

  it('el texto copiado lleva los pasos legibles y el informe', () => {
    const text = diagnosticsText('Demo', diagnosticSteps(readiness, null), { version: '0.6.1', project: { engineAssociation: '{95B798E8}' } });
    expect(text.startsWith('# Diagnóstico de DBV Unreal AI · Demo')).toBe(true);
    expect(text).toContain('✗ Unreal Engine 5.8 o posterior instalado ({95B798E8}) — Instala Unreal 5.8');
    expect(text).toContain('```json');
    expect(text).toContain('"engineAssociation": "{95B798E8}"');
  });
});
