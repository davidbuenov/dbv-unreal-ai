// =============================================================================
// DBV Unreal AI — Tests de la consola y del seguimiento de los trabajos
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Lo que se protege: que la consola enseñe la petición EXACTA enviada a Tripo (REQ-27) y que, mientras un
// agente genera en su subproceso `--mcp`, la conversación muestre el trabajo en vez de parecer parada.

import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { createChatJobs } from '../../src/generation/chatJobs.js';
import { consoleText, createJobCard } from '../../src/generation/jobCard.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});
afterEach(() => vi.useRealTimers());

const now = () => Math.floor(Date.now() / 1000);

function job(over = {}) {
  return {
    id: 'job-1',
    source: 'agent',
    kind: 'text_to_model',
    name: 'Pirata',
    prompt: 'pirate',
    endpoint: '/generation/text-to-model',
    request: { prompt: 'stylized pirate in T-pose' },
    stage: 'running',
    progress: 40,
    estimatedCredits: 20,
    startedAt: now(),
    updatedAt: now(),
    log: ['+0s en cola', '+1s tarea creada: task_1'],
    output: null,
    ...over,
  };
}

describe('tarjeta de un trabajo (REQ-27)', () => {
  /** @spec REQ-27 */
  it('la consola enseña la petición exacta y el registro', () => {
    const text = consoleText(job());
    expect(text).toContain('POST /generation/text-to-model');
    expect(text).toContain('"prompt": "stylized pirate in T-pose"');
    expect(text).toContain('tarea creada: task_1');
    expect(consoleText(job({ output: { riggable: true, rig_type: 'biped' } }))).toContain('"rig_type": "biped"');
  });

  /** @spec REQ-27 */
  it('avanza, termina una sola vez y no ofrece cancelar lo que lanzó un agente', () => {
    const renderResult = vi.fn();
    const card = createJobCard(job(), { onCancel: vi.fn(), renderResult });
    expect(card.node.querySelector('.job-stage').textContent).toBe('Generando… 40%');
    expect(card.node.querySelector('.job-meta').textContent).toContain('lanzado por el agente');
    expect(card.node.querySelector('.btn-link').classList.contains('hidden')).toBe(true);
    card.update(job({ stage: 'done', progress: 100, entryId: 'pirata-1' }));
    card.update(job({ stage: 'done', progress: 100, entryId: 'pirata-1', updatedAt: now() + 1 }));
    expect(renderResult).toHaveBeenCalledTimes(1);
    expect(createJobCard(job({ source: 'app' }), { onCancel: vi.fn() }).node.querySelector('.btn-link').classList.contains('hidden')).toBe(false);
  });
});

describe('trabajos en la conversación (REQ-27)', () => {
  /**
   * Claude Code llama a `generate_3d_model` en su subproceso: la ventana descubre el trabajo en los ficheros y lo
   * pinta en la conversación mientras el turno sigue abierto. Lo anterior al turno no se pinta.
   * @spec REQ-27
   */
  it('mientras un agente responde, sus trabajos nuevos aparecen en la conversación', async () => {
    vi.useFakeTimers();
    let records = [job({ id: 'viejo', startedAt: now() - 600, stage: 'done' })];
    const list = vi.fn(async () => ({ ok: true, value: records }));
    const added = [];
    const panel = { addNode: (node) => added.push(node) };
    const chat = createChatJobs({ list });
    let busy = true;
    chat.follow(panel, () => busy);
    records = [...records, job({ id: 'nuevo' })];
    await vi.advanceTimersByTimeAsync(1600);
    expect(added).toHaveLength(1);
    expect(added[0].querySelector('.job-name').textContent).toBe('Pirata');
    records = records.map((r) => (r.id === 'nuevo' ? { ...r, stage: 'done', progress: 100, updatedAt: r.updatedAt + 5 } : r));
    busy = false;
    await vi.advanceTimersByTimeAsync(1600);
    expect(added[0].dataset.stage).toBe('done');
    chat.stop();
  });

  /** @spec REQ-27 */
  it('los trabajos del asistente integrado aparecen porque su herramienta los pide', async () => {
    const records = [job({ id: 'job-a', source: 'app' }), job({ id: 'job-b', source: 'app' }), job({ id: 'otro', source: 'app' })];
    const added = [];
    const chat = createChatJobs({ list: async () => ({ ok: true, value: records }) });
    chat.track({ addNode: (node) => added.push(node) }, ['job-a', 'job-b']);
    await vi.waitFor(() => expect(added).toHaveLength(2));
    chat.stop();
  });
});
