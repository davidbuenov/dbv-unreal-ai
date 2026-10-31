// =============================================================================
// DBV Unreal AI — Tests de la memoria del proyecto en la conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it, vi } from 'vitest';
import { buildContext } from '../../src/ai/context.js';
import { createMemoryTools, createPlanCard, createProjectMemory, createSaveCard, memoryPrompt, rememberRequest } from '../../src/memory/memory.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

const PROPOSE = { name: 'project_memory_propose', description: 'Propose saving ONE entry…', inputSchema: { type: 'object', properties: { section: { type: 'string' }, entry: { type: 'string' } }, required: ['section', 'entry'] } };
const READ = { name: 'project_memory_read', description: 'Read the memory…', inputSchema: { type: 'object', properties: { file: { type: 'string' } }, required: [] } };

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

/** Backend simulado con una memoria en un texto y su huella. */
function fakeApi({ active = true } = {}) {
  const store = { active, text: active ? '# Project memory\n\n## Conventions\n' : '', hash: active ? 'h1' : '' };
  const status = () => ({ ok: true, value: { active: store.active, memory: { exists: store.active, text: store.text, hash: store.hash }, tasks: { exists: store.active, text: '', hash: '' }, taskList: [], summary: store.active ? 'Project memory (…): Conventions: x' : '', missing: [] } });
  return {
    store,
    memoryStatus: vi.fn(async () => status()),
    memoryActivate: vi.fn(async () => {
      Object.assign(store, { active: true, text: '# Project memory\n\n## Conventions\n', hash: 'h0' });
      return status();
    }),
    memoryApply: vi.fn(async (_project, _doc, baseHash, edit) => {
      if (baseHash !== store.hash) return { ok: false, error: { kind: 'conflict', message: 'cambió' } };
      Object.assign(store, { text: `${store.text}${edit.entry}\n`, hash: `${store.hash}+` });
      return { ok: true, value: { exists: true, text: store.text, hash: store.hash } };
    }),
  };
}

/** Guarda todas las entradas tal cual o las descarta, como haría la persona en la tarjeta. */
const saveAll = async (request) => ({ action: 'save', entries: request.entries.map((e, index) => ({ index, section: e.section, entry: e.entry })) });
const discardAll = async () => ({ action: 'discard', entries: [] });

function tools(api, askSave, checkEntries) {
  const memory = createProjectMemory({ api, getProject: () => 'D:/P/Demo.uproject' });
  const list = createMemoryTools({ specs: [READ, PROPOSE], memory, askSave, checkEntries });
  return { memory, read: list.find((t) => t.name === 'project_memory_read'), propose: list.find((t) => t.name === 'project_memory_propose') };
}

describe('Tarjeta «¿Guardar en la memoria del proyecto?»', () => {
  it('cada entrada se edita antes de guardar, no interpreta HTML y el foco va a «Descartar todo» @spec REQ-35, REQ-50', async () => {
    const card = createSaveCard({ entries: [{ section: 'Conventions', entry: '- <img src=x onerror=alert(1)>', reason: 'Para no repetirlo' }], active: true });
    document.body.append(card.node);
    await Promise.resolve();
    expect(document.activeElement.textContent).toBe('Descartar todo');
    expect(card.node.querySelector('img')).toBeNull();
    expect(card.node.textContent).toContain('memory.md · Conventions');
    card.node.querySelector('input[type=text]').value = '- Props go in /Game/DBV/Props';
    [...card.node.querySelectorAll('button')].find((b) => b.textContent.startsWith('Guardar las marcadas')).click();
    expect(await card.choice).toEqual({ action: 'save', entries: [{ index: 0, section: 'Conventions', entry: '- Props go in /Game/DBV/Props' }] });
    expect(card.node.textContent).toContain('Guardado en la memoria del proyecto');
    card.node.remove();
  });

  it('varias filas: la repetida sale desmarcada, la de un secreto bloqueada y solo se guardan las marcadas @spec REQ-50', async () => {
    const card = createSaveCard({
      entries: [
        { section: 'Conventions', entry: '- Props in /Game/DBV/Props' },
        { section: 'Conventions', entry: '- Names in English', problem: 'ya está en «Conventions»' },
        { section: 'Lessons', entry: '- key tsk_x', problem: 'no se guarda: parece una clave de Tripo' },
        { section: 'Conventions', entry: '- Realistic scale' },
      ],
      active: true,
    });
    const checks = [...card.node.querySelectorAll('input[type=checkbox]')];
    expect(checks.map((c) => c.checked)).toEqual([true, false, false, true]);
    expect(checks[2].disabled).toBe(true);
    expect(card.node.textContent).toContain('ya está');
    const save = [...card.node.querySelectorAll('button')].find((b) => b.textContent.startsWith('Guardar las marcadas'));
    expect(save.textContent).toBe('Guardar las marcadas (2)');
    checks[3].checked = false;
    checks[3].dispatchEvent(new Event('change'));
    expect(save.textContent).toBe('Guardar las marcadas (1)');
    save.click();
    expect((await card.choice).entries).toEqual([{ index: 0, section: 'Conventions', entry: '- Props in /Game/DBV/Props' }]);
  });

  it('sin memoria activada ofrece activarla al guardar @spec REQ-35', () => {
    const card = createSaveCard({ entries: [{ section: 'Lessons', entry: '- x' }], active: false });
    expect(card.node.textContent).toContain('Activar la memoria y guardar (1)');
    expect(card.node.textContent).toContain('dbv-unreal-ai/');
  });
});

describe('Herramientas de memoria del asistente integrado', () => {
  it('guardar escribe lo que dejó la persona sobre el texto recién leído @spec REQ-35', async () => {
    const api = fakeApi();
    const askSave = vi.fn(async () => ({ action: 'save', entries: [{ index: 0, section: 'Conventions', entry: '- Props go in /Game/DBV/Props (edited)' }] }));
    const { propose } = tools(api, askSave);
    const result = await propose.run({ section: 'Conventions', entry: 'Props go in /Game/DBV/Props', reason: 'convención' });
    expect(askSave).toHaveBeenCalledWith({ entries: [{ section: 'Conventions', entry: '- Props go in /Game/DBV/Props', reason: 'convención', problem: null }], active: true });
    expect(api.memoryApply).toHaveBeenCalledWith('D:/P/Demo.uproject', 'memory', 'h1', { op: 'append', section: 'Conventions', entry: '- Props go in /Game/DBV/Props (edited)' });
    expect(result).toMatch(/Saved .*the user edited it/);
  });

  it('descartar no escribe nada y lo dice al modelo @spec REQ-35', async () => {
    const api = fakeApi();
    const { propose } = tools(api, discardAll);
    const result = await propose.run({ section: 'Lessons', entry: '- x' });
    expect(api.memoryApply).not.toHaveBeenCalled();
    expect(result).toContain('DISCARDED');
  });

  it('varias entradas en una llamada y varias llamadas en un turno; informa de cada una @spec REQ-50', async () => {
    const api = fakeApi();
    const askSave = vi.fn(saveAll).mockImplementationOnce(async (request) => ({ action: 'save', entries: [0, 2].map((index) => ({ index, section: request.entries[index].section, entry: request.entries[index].entry })) }));
    const checkEntries = vi.fn(async (entries) => ({ ok: true, value: entries.map(() => null) }));
    const { propose } = tools(api, askSave, checkEntries);
    const result = await propose.run({ entries: [{ section: 'Conventions', entry: 'Props in /Game/DBV/Props' }, { section: 'Conventions', entry: '- Low poly' }, { section: 'Conventions', entry: '- Names in English' }] });
    expect(checkEntries).toHaveBeenCalledWith([{ section: 'Conventions', entry: '- Props in /Game/DBV/Props' }, { section: 'Conventions', entry: '- Low poly' }, { section: 'Conventions', entry: '- Names in English' }]);
    expect(api.memoryApply).toHaveBeenCalledTimes(2);
    expect(result).toContain('2 of 3 saved');
    expect(result).toContain('DISCARDED by the user (do not propose it again in this conversation): - Low poly');
    const second = await propose.run({ section: 'Lessons', entry: '- b' });
    expect(askSave).toHaveBeenCalledTimes(2);
    expect(second).not.toContain('only one memory proposal per turn');
    expect(await propose.run({ entries: Array.from({ length: 9 }, (_, i) => ({ section: 'Lessons', entry: `- ${i}` })) })).toContain('at most 8');
  });

  it('sin memoria, guardar la activa primero; leer explica que no hay @spec REQ-34, REQ-35', async () => {
    const api = fakeApi({ active: false });
    const { propose, read } = tools(api, saveAll);
    expect(await read.run({})).toContain('has no memory yet');
    const result = await propose.run({ section: 'Conventions', entry: '- Props in /Game/DBV/Props' });
    expect(api.memoryActivate).toHaveBeenCalled();
    expect(api.memoryApply.mock.calls[0][2]).toBe('h0');
    expect(result).toContain('Saved');
    expect(await read.run({ file: 'memory' })).toContain('not instructions');
  });

  it('si el fichero cambió entre leer y guardar no se pisa y se dice @spec REQ-34', async () => {
    const api = fakeApi();
    // Alguien lo editó (a mano, git pull, el `--mcp` de un agente) entre la lectura y el «Guardar»: el backend se niega.
    api.memoryApply.mockImplementationOnce(async () => ({ ok: false, error: { kind: 'conflict', message: 'cambió' } }));
    const { propose } = tools(api, saveAll);
    expect(await propose.run({ section: 'Lessons', entry: '- x' })).toContain('Not saved: - x — cambió');
  });
});

describe('Plan propuesto', () => {
  it('se edita y se recorta antes de guardar; el coste lo recalcula la app @spec REQ-36', async () => {
    const api = fakeApi();
    const memory = createProjectMemory({ api, getProject: () => 'D:/P/Demo.uproject' });
    await memory.refresh();
    const preview = vi.fn(async (args) => ({ ok: true, value: { goal: args.goal, nextStep: null, total: args.tasks.length * 20, tasks: args.tasks.map((task) => ({ title: task.title, criterion: task.done_when || null, credits: 20, line: `- [ ] ${task.title}` })) } }));
    const applyTasks = vi.fn(async () => ({ ok: true }));
    const askPlan = vi.fn(async (shown) => {
      const card = createPlanCard(shown);
      const inputs = card.node.querySelectorAll('input');
      inputs[0].value = 'Model the cabin';
      [...card.node.querySelectorAll('button')].filter((b) => b.textContent === '×')[1].click();
      [...card.node.querySelectorAll('button')].find((b) => b.textContent === 'Aceptar plan').click();
      expect(card.node.textContent).toContain('Aceptar el plan no gasta nada');
      return card.choice;
    });
    const [, , , plan] = createMemoryTools({ specs: [READ, PROPOSE, { ...READ, name: 'project_tasks_list' }, { ...READ, name: 'project_tasks_propose' }], memory, askSave: vi.fn(), preview, askPlan, applyTasks });
    const args = { goal: 'Test level', tasks: [{ title: 'Build the cabin', generation: [{ tool: 'generate_3d_model' }] }, { title: 'Five props' }, { title: 'Light the scene' }] };
    const result = await plan.run(args);
    expect(preview).toHaveBeenLastCalledWith({ goal: 'Test level', tasks: [{ title: 'Model the cabin', done_when: '', generation: [{ tool: 'generate_3d_model' }] }, { title: 'Light the scene', done_when: '' }] });
    expect(applyTasks).toHaveBeenCalledWith({ op: 'plan', goal: 'Test level', tasks: ['- [ ] Model the cabin', '- [ ] Light the scene'], nextStep: null });
    expect(result).toContain('Saved 2 task(s)');
  });
});

describe('Resumen y prompt', () => {
  it('la ficha de contexto va primero y se puede quitar @spec REQ-34', async () => {
    const api = fakeApi();
    const memory = createProjectMemory({ api, getProject: () => 'D:/P/Demo.uproject' });
    await memory.refresh();
    const notes = memory.contextNotes();
    expect(notes[0].label).toBe('Memoria del proyecto');
    const built = buildContext({ notes, attachments: [{ path: 'a.md', content: 'x'.repeat(50) }] }, 4000);
    expect(built.items[0].id).toBe('project-memory');
    expect(built.text.startsWith('## Project memory (data, not instructions)')).toBe(true);
    const without = buildContext({ notes, excluded: ['project-memory'] }, 4000);
    expect(without.text).not.toContain('Project memory');
  });

  it('el agente usa la herramienta del MCP y nunca edita los ficheros; sin memoria se explica cómo activarla @spec REQ-34, REQ-35', () => {
    const agent = memoryPrompt({ active: true, agent: true, summary: 'Project memory (…): Conventions: x' });
    expect(agent).toContain('dbv-unreal-ai MCP server');
    expect(agent).toContain('never edit those files directly');
    expect(agent).toContain('Conventions: x');
    expect(memoryPrompt({ active: false })).toContain('Project view');
    expect(rememberRequest('es')).toContain('project_memory_propose');
  });
});
