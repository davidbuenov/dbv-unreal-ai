// =============================================================================
// DBV Unreal AI — Tests de la compilación tras aplicar cambios de código de la IA
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { createCodeCompiler, errorsForAi, MAX_RETRIES } from '../../src/app/codeCompile.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

const CONNECTION = { projectPath: 'D:/Unreal/Demo/Demo.uproject', endpoint: { port: 8000, path: '/mcp' }, status: 'online' };
const FAILED = { ok: false, summary: 'Failure', errors: ["DemoActor.cpp(12): error C2065: 'Speed': undeclared identifier"] };
let invoked;
let outcomes;

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

beforeEach(() => {
  invoked = [];
  outcomes = [];
  window.__TAURI__ = {
    core: {
      invoke: vi.fn(async (command, args) => {
        invoked.push([command, args]);
        if (command === 'unreal_compile_plan') {
          const structural = args.paths.filter((p) => /\.(h|hpp|cs)$/.test(p));
          const cpp = args.paths.some((p) => /\.(cpp|h|hpp|cs)$/.test(p));
          return !cpp ? { mode: 'nothing' } : structural.length ? { mode: 'editorClosed', files: structural } : { mode: 'liveCoding' };
        }
        if (command === 'unreal_compile_live' || command === 'unreal_build_project') return outcomes.shift();
        return null;
      }),
    },
  };
});

function fakeAi() {
  const host = document.createElement('div');
  return { host, ask: vi.fn(async () => {}), panel: { addNode: (node) => host.append(node), addNote: vi.fn() } };
}

describe('Compilar tras aplicar cambios de código', () => {
  it('solo .cpp: Live Coding; los errores vuelven solos a la IA dos veces y luego decide la persona @spec REQ-22', async () => {
    const ai = fakeAi();
    const compiler = createCodeCompiler({ getConnection: () => CONNECTION, getAi: () => ai });
    outcomes.push(FAILED, FAILED, FAILED);

    await compiler.afterApply(['Source/Demo/Private/DemoActor.cpp']);
    expect(invoked.find(([c]) => c === 'unreal_compile_live')[1]).toEqual({ endpoint: CONNECTION.endpoint });
    expect(ai.ask).toHaveBeenCalledTimes(1);
    expect(ai.ask.mock.calls[0][0]).toContain("error C2065: 'Speed'");
    expect(ai.ask.mock.calls[0][1]).toEqual({ label: `Errores de compilación: se los paso a la IA (intento 1 de ${MAX_RETRIES})` });

    await compiler.afterApply(['Source/Demo/Private/DemoActor.cpp']);
    await compiler.afterApply(['Source/Demo/Private/DemoActor.cpp']);
    expect(ai.ask).toHaveBeenCalledTimes(MAX_RETRIES);
    expect(ai.host.textContent).toContain('La IA ya lo ha intentado dos veces');
    [...ai.host.querySelectorAll('button')].find((b) => b.textContent === 'Pedir a la IA que lo corrija').click();
    await vi.waitFor(() => expect(ai.ask).toHaveBeenCalledTimes(MAX_RETRIES + 1));
  });

  it('una compilación buena vuelve a dar dos reintentos @spec REQ-22', async () => {
    const ai = fakeAi();
    const compiler = createCodeCompiler({ getConnection: () => CONNECTION, getAi: () => ai });
    outcomes.push(FAILED, { ok: true, summary: 'Success', errors: [] });
    await compiler.afterApply(['Source/Demo/Private/DemoActor.cpp']);
    await compiler.afterApply(['Source/Demo/Private/DemoActor.cpp']);
    expect(compiler.retries()).toBe(0);
    expect(ai.panel.addNote).toHaveBeenLastCalledWith('Compilado (Success).', 'info');
  });

  it('cabeceras o Build.cs: tarjeta para cerrar el editor, compilar con UBT y reabrirlo @spec REQ-22', async () => {
    const ai = fakeAi();
    const compiler = createCodeCompiler({ getConnection: () => CONNECTION, getAi: () => ai });
    outcomes.push({ ok: true, summary: 'Success', errors: [] });
    await compiler.afterApply(['Source/Demo/Public/DemoActor.h', 'Source/Demo/Private/DemoActor.cpp']);
    expect(invoked.some(([c]) => c === 'unreal_compile_live')).toBe(false);
    expect(ai.host.textContent).toContain('Source/Demo/Public/DemoActor.h');

    const buttons = () => [...ai.host.querySelectorAll('button')];
    expect(buttons().find((b) => b.textContent === 'Abrir el editor').hidden).toBe(true);
    buttons().find((b) => b.textContent === 'Compilar').click();
    await vi.waitFor(() => expect(invoked.find(([c]) => c === 'unreal_build_project')?.[1]).toEqual({ path: CONNECTION.projectPath }));
    await vi.waitFor(() => expect(buttons().find((b) => b.textContent === 'Abrir el editor').hidden).toBe(false));
    buttons().find((b) => b.textContent === 'Abrir el editor').click();
    await vi.waitFor(() => expect(invoked.some(([c, a]) => c === 'unreal_open_project' && a.path === CONNECTION.projectPath)).toBe(true));
  });

  it('Python o .ini no se compilan, y sin editor conectado no se lanza Live Coding @spec REQ-22', async () => {
    const ai = fakeAi();
    await createCodeCompiler({ getConnection: () => CONNECTION, getAi: () => ai }).afterApply(['Content/Python/tools.py', 'Config/DefaultGame.ini']);
    expect(invoked.some(([c]) => c === 'unreal_compile_live' || c === 'unreal_build_project')).toBe(false);

    await createCodeCompiler({ getConnection: () => ({ ...CONNECTION, status: 'offline' }), getAi: () => ai }).afterApply(['Source/Demo/Private/DemoActor.cpp']);
    expect(invoked.some(([c]) => c === 'unreal_compile_live')).toBe(false);
    expect(ai.panel.addNote).toHaveBeenCalledWith(expect.stringContaining('No hay un editor conectado'), 'error');
  });

  it('el mensaje para la IA lleva los errores y el intento @spec REQ-22', () => {
    const text = errorsForAi(FAILED, 2);
    expect(text).toContain('Failure');
    expect(text).toContain("- DemoActor.cpp(12): error C2065");
    expect(text).toContain(`attempt 2 of ${MAX_RETRIES}`);
  });
});
