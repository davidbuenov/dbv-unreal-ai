// =============================================================================
// DBV Unreal AI — Tests del seguimiento de los trabajos de generación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { describe, expect, it, vi } from 'vitest';

/** `window.__TAURI__` con un evento que se puede emitir a mano y un `invoke` que emite al lanzar. */
function fakeTauri({ failImmediately }) {
  const listeners = [];
  const emit = (payload) => listeners.forEach((cb) => cb({ payload }));
  window.__TAURI__ = {
    event: { listen: vi.fn(async (_name, cb) => (listeners.push(cb), () => {})) },
    core: {
      invoke: vi.fn(async (command) => {
        if (command !== 'generation_launch') return null;
        // El backend falla al instante (clave mala) y emite ANTES de que nadie llame a waitJob.
        if (failImmediately) emit({ jobId: 'job-1', stage: 'failed', error: 'invalid api key' });
        return ['job-1'];
      }),
    },
  };
  return { emit };
}

describe('trabajos de generación (REQ-15)', () => {
  it('un fallo inmediato no deja la interfaz esperando: el evento se guarda @spec REQ-15', async () => {
    vi.resetModules();
    fakeTauri({ failImmediately: true });
    const api = await import('../../src/app/backend.js');
    const started = await api.launch('generate_3d_model', { prompt: 'x', name: 'X' });
    expect(started).toEqual({ ok: true, value: ['job-1'] });
    // Solo se lanza ya confirmado: la tarjeta de coste va antes, en quien llama.
    expect(window.__TAURI__.core.invoke).toHaveBeenCalledWith('generation_launch', { tool: 'generate_3d_model', args: { prompt: 'x', name: 'X' }, confirmCost: true });
    const end = await api.waitJob('job-1', 5000);
    expect(end).toMatchObject({ stage: 'failed', error: 'invalid api key' });
  });

  it('sigue el progreso hasta el final y solo de su trabajo @spec REQ-15', async () => {
    vi.resetModules();
    const { emit } = fakeTauri({ failImmediately: false });
    const api = await import('../../src/app/backend.js');
    await api.launch('generate_3d_model', { prompt: 'x', name: 'X' });
    const seen = [];
    const waiting = api.waitJob('job-1', 5000, (p) => seen.push(p.stage));
    emit({ jobId: 'otro', stage: 'done' });
    emit({ jobId: 'job-1', stage: 'running', progress: 40 });
    emit({ jobId: 'job-1', stage: 'done', entry: { id: 'a' } });
    expect(await waiting).toMatchObject({ stage: 'done', entry: { id: 'a' } });
    expect(seen).toEqual(['running', 'done']);
  });

  it('agota el tiempo si el trabajo no termina @spec REQ-15, RNF-03', async () => {
    vi.resetModules();
    fakeTauri({ failImmediately: false });
    const api = await import('../../src/app/backend.js');
    expect(await api.waitJob('nunca', 20)).toMatchObject({ stage: 'failed', error: 'timeout' });
  });
});
