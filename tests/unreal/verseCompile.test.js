// =============================================================================
// DBV Unreal AI — Tests: aplicar y compilar Verse en UEFN
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it, vi } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { createVerseCompiler, DEFAULT_ATTEMPTS, maxAttempts, verseBuildCard } from '../../src/unreal/verseCompile.js';

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

const error = (message = 'This variable expects to be initialized with a value of type int', code = 3509) => ({ severity: 'Error', code, message, versePath: '/mod/zz_test.verse', relative: 'Content/zz_test.verse', line: 9, column: 27 });
const failed = (...errors) => ({ ok: false, sessionRunning: false, errors: errors.length ? errors : [error()], warnings: [], experimental: false });
const compiled = { ok: true, sessionRunning: false, errors: [], warnings: [], experimental: false };
const connection = { uefn: true, status: 'online', projectPath: 'D:/U/P/P.uefnproject', endpoint: { port: 8000, path: '/mcp' } };

function harness(results, extra = {}) {
  const nodes = [];
  const ai = { ask: vi.fn(async () => {}), panel: { addNode: (node) => nodes.push(node) } };
  const queue = [...results];
  const build = vi.fn(async () => ({ ok: true, value: queue.shift() }));
  const compiler = createVerseCompiler({ getConnection: () => extra.connection ?? connection, getAi: () => ai, build, storage: extra.storage ?? null, pushVerse: extra.pushVerse });
  const text = () => nodes.map((n) => n.textContent).join('\n');
  return { compiler, ai, build, nodes, text };
}

describe('aplicar y compilar Verse (REQ-127)', () => {
  it('solo compila con UEFN y si la propuesta tocó .verse', async () => {
    // @spec REQ-127
    const { compiler, build } = harness([compiled]);
    await compiler.afterApply(['README.md']);
    expect(build).not.toHaveBeenCalled();
    await compiler.afterApply(['Content/zz_test.verse']);
    expect(build).toHaveBeenCalledWith(connection.endpoint, connection.projectPath);
  });

  it('«Compila» solo con la lista vacía', async () => {
    // @spec REQ-127
    const { compiler, text } = harness([compiled]);
    await compiler.afterApply(['Content/zz_test.verse']);
    expect(text()).toContain('Compila: BuildAll no ha dado ningún error.');
  });

  it('los errores vuelven a la IA con fichero, línea desde 1, código y mensaje', async () => {
    // @spec REQ-127
    const { compiler, ai, text } = harness([failed()]);
    await compiler.afterApply(['Content/zz_test.verse']);
    expect(text()).toContain('Content/zz_test.verse:9 · 3509');
    expect(text()).toContain('Intento 1 de 5');
    const [message] = ai.ask.mock.calls[0];
    expect(message).toContain('attempt 1 of 5');
    expect(message).toContain('Content/zz_test.verse:9 · 3509');
    expect(message).toContain('verse_reference');
  });

  it('errores → arreglo → compila: el contador vuelve a cero', async () => {
    // @spec REQ-127
    const { compiler, ai, text } = harness([failed(), compiled]);
    await compiler.afterApply(['Content/zz_test.verse']);
    expect(compiler.attempts()).toBe(1);
    await compiler.afterApply(['Content/zz_test.verse']);
    expect(compiler.attempts()).toBe(0);
    expect(ai.ask).toHaveBeenCalledTimes(1);
    expect(text()).toContain('Compila');
  });

  it('se para en el tope de intentos', async () => {
    // @spec REQ-127
    const storage = { getItem: () => '2' };
    const { compiler, ai, text } = harness([failed(error('a')), failed(error('b')), failed(error('c'))], { storage });
    for (let i = 0; i < 3; i += 1) await compiler.afterApply(['Content/zz_test.verse']);
    expect(ai.ask).toHaveBeenCalledTimes(2);
    expect(text()).toContain('lo ha intentado 2 veces');
  });

  it('el mismo error tras un arreglo: compila otra vez y, si sigue, sugiere reiniciar UEFN', async () => {
    // @spec REQ-127
    const { compiler, build, ai, text } = harness([failed(), failed(), failed()]);
    await compiler.afterApply(['Content/zz_test.verse']);
    await compiler.afterApply(['Content/zz_test.verse']);
    expect(build).toHaveBeenCalledTimes(3);
    expect(ai.ask).toHaveBeenCalledTimes(1);
    expect(text()).toContain('reinicia UEFN');
  });

  it('sin MCP dice «Sin verificar» y cómo compilar en UEFN', async () => {
    // @spec REQ-127
    const { compiler, build, text } = harness([], { connection: { ...connection, status: 'offline' } });
    await compiler.afterApply(['Content/zz_test.verse']);
    expect(build).not.toHaveBeenCalled();
    expect(text()).toContain('Sin verificar');
    expect(text()).toContain('Build Verse Code');
  });

  it('con una sesión en marcha no compila y ofrece enviar los cambios', async () => {
    // @spec REQ-127, REQ-128
    const pushVerse = vi.fn();
    const { compiler, nodes, text } = harness([{ ...compiled, ok: false, sessionRunning: true }], { pushVerse });
    await compiler.afterApply(['Content/zz_test.verse']);
    expect(text()).toContain('sesión de juego en marcha');
    nodes[0].querySelector('button').click();
    expect(pushVerse).toHaveBeenCalled();
  });

  it('una API experimental avisa de que no se podrá publicar', () => {
    // @spec REQ-127
    const card = verseBuildCard({ ...compiled, experimental: true }, { attempt: 1, max: 5 });
    expect(card.textContent).toContain('No se podrá publicar');
  });

  it('el tope se puede ajustar de 1 a 10', () => {
    // @spec REQ-127
    expect(maxAttempts({ getItem: () => '10' })).toBe(10);
    expect(maxAttempts({ getItem: () => '0' })).toBe(DEFAULT_ATTEMPTS);
    expect(maxAttempts({ getItem: () => '11' })).toBe(DEFAULT_ATTEMPTS);
    expect(maxAttempts(null)).toBe(DEFAULT_ATTEMPTS);
  });
});
