// =============================================================================
// DBV Unreal AI — Tests de la vista Plan con el backend de Tauri simulado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPlanView, isCheck, planRequest, retitle, taskRequest, toggled } from '../../src/app/planView.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

const CABIN = { done: false, title: 'Build the cabin', criterion: 'asset exists /Game/DBV/Props/Cabin', credits: 20, line: '- [ ] Build the cabin — done when: asset exists /Game/DBV/Props/Cabin — est: 20 Tripo credits' };
const STYLE = { done: true, title: 'Pick a style', criterion: null, credits: null, line: '- [x] Pick a style' };
const TASKS_TEXT = `# Tasks\n\n## Goal\n\nA test level\n\n## Tasks\n\n${STYLE.line}\n${CABIN.line}\n\n## Next step\n\nBuild the cabin\n`;
let invoked;
let applyResult;

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

beforeEach(() => {
  invoked = [];
  applyResult = { exists: true, text: '', hash: 'h2' };
  window.__TAURI__ = {
    core: {
      invoke: vi.fn(async (command, args) => {
        invoked.push([command, args]);
        if (command === 'memory_status') return { active: true, memory: { exists: true, text: '# Project memory\n\n## Conventions\n\n- Props in /Game/DBV/Props\n', hash: 'm1' }, tasks: { exists: true, text: TASKS_TEXT, hash: 'h1' }, taskList: [STYLE, CABIN], summary: '', missing: [] };
        if (command === 'memory_apply') {
          if (applyResult instanceof Error) throw { kind: 'conflict', message: 'cambió' };
          return applyResult;
        }
        return null;
      }),
    },
  };
});

function view(extra = {}) {
  const host = document.createElement('div');
  const deps = { state: { connection: { projectPath: 'D:/P/Demo.uproject' } }, toast: vi.fn(), openModal: vi.fn(), closeModal: vi.fn(), ask: vi.fn(), showView: vi.fn(), onChanged: vi.fn(), ...extra };
  return { host, deps, plan: createPlanView(host, deps) };
}

describe('Líneas de tareas', () => {
  it('marcar, reabrir y retitular conservan el criterio y el coste @spec REQ-36', () => {
    expect(toggled(CABIN)).toBe(CABIN.line.replace('[ ]', '[x]'));
    expect(toggled(STYLE)).toBe('- [ ] Pick a style');
    expect(retitle(CABIN, 'Model the cabin')).toBe('- [ ] Model the cabin — done when: asset exists /Game/DBV/Props/Cabin — est: 20 Tripo credits');
    expect(retitle(CABIN, '  ')).toBe(CABIN.line);
    expect(isCheck(CABIN.criterion)).toBe(true);
    expect(isCheck('the user likes it')).toBe(false);
    expect(planRequest('un nivel', 'es')).toContain('project_tasks_propose');
    expect(taskRequest(CABIN, 'es')).toContain('Estará hecha cuando: asset exists');
  });
});

describe('Vista Plan', () => {
  it('enseña objetivo, tareas con su criterio y coste, y siguiente paso @spec REQ-36', async () => {
    const { host, plan } = view();
    await plan.render();
    expect(host.textContent).toContain('A test level');
    expect(host.textContent).toContain('Build the cabin');
    expect(host.querySelector('.plan-task__criterion.mono').textContent).toBe('asset exists /Game/DBV/Props/Cabin');
    expect(host.textContent).toContain('Pendiente: ≈20 créditos');
    expect(host.querySelector('.plan-task--done').textContent).toContain('Pick a style');
  });

  it('reabrir una tarea la escribe al momento, línea entera y sobre la huella leída @spec REQ-36', async () => {
    const { host, plan, deps } = view();
    await plan.render();
    const box = [...host.querySelectorAll('input[type="checkbox"]')].find((b) => b.getAttribute('aria-label') === 'Pick a style');
    box.checked = false;
    box.dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(invoked.some(([c]) => c === 'memory_apply')).toBe(true));
    expect(invoked.find(([c]) => c === 'memory_apply')[1]).toEqual({ project: 'D:/P/Demo.uproject', doc: 'tasks', baseHash: 'h1', edit: { op: 'replace', search: `\n${STYLE.line}\n`, replace: '\n- [ ] Pick a style\n' } });
    await vi.waitFor(() => expect(deps.onChanged).toHaveBeenCalled());
  });

  it('marcar una tarea con comprobación la decide el editor; sin editor queda pendiente @spec REQ-37', async () => {
    const { host, plan, deps } = view();
    window.__TAURI__.core.invoke = vi.fn(async (command, args) => {
      invoked.push([command, args]);
      if (command === 'memory_task_update') return { saved: false, task: 'Build the cabin', check: { passed: null, detail: 'no Unreal editor is connected' }, message: 'Not marked: «Build the cabin» could not be checked' };
      return { active: true, memory: { exists: true, text: '', hash: 'm1' }, tasks: { exists: true, text: TASKS_TEXT, hash: 'h1' }, taskList: [STYLE, CABIN], summary: '', missing: [] };
    });
    await plan.render();
    const box = [...host.querySelectorAll('input[type="checkbox"]')].find((b) => b.getAttribute('aria-label') === 'Build the cabin');
    box.checked = true;
    box.dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(deps.toast).toHaveBeenCalled());
    expect(invoked.find(([c]) => c === 'memory_task_update')[1]).toEqual({ project: 'D:/P/Demo.uproject', endpoint: null, title: 'Build the cabin', done: true, byAi: false });
    expect(invoked.some(([c]) => c === 'memory_apply')).toBe(false);
    expect(deps.toast.mock.calls[0]).toEqual(['Not marked: «Build the cabin» could not be checked', 'error']);
  });

  it('si el fichero cambió fuera, avisa en vez de pisar @spec REQ-34, REQ-36', async () => {
    applyResult = new Error('conflict');
    const { host, plan, deps } = view();
    await plan.render();
    [...host.querySelectorAll('button')].find((b) => b.textContent === 'Borrar').click();
    await vi.waitFor(() => expect(deps.toast).toHaveBeenCalled());
    expect(deps.toast.mock.calls[0][0]).toMatch(/cambió fuera de la app/);
  });

  it('«Hacer esta tarea» y «Planificar» van a la conversación @spec REQ-36', async () => {
    const { host, plan, deps } = view();
    await plan.render();
    [...host.querySelectorAll('button')].find((b) => b.textContent === 'Hacer esta tarea').click();
    expect(deps.ask).toHaveBeenCalledWith(taskRequest(CABIN, 'es'));
    [...host.querySelectorAll('button')].find((b) => b.textContent === 'Planificar').click();
    const modal = deps.openModal.mock.calls[0][0];
    const goal = modal.find((n) => n.tagName === 'TEXTAREA');
    goal.value = 'un claro de bosque con una cabaña';
    const send = modal.flatMap((n) => [...(n.querySelectorAll?.('button') ?? [])]).find((b) => b.textContent === 'Pedir el plan');
    send.click();
    expect(deps.ask).toHaveBeenLastCalledWith(planRequest('un claro de bosque con una cabaña', 'es'));
  });

  it('sin memoria activada explica qué es y lleva a activarla; la pestaña Memoria la enseña @spec REQ-34', async () => {
    const { host, plan } = view({ renderMarkdown: async () => (md) => document.createTextNode(`MD:${md}`) });
    await plan.render();
    [...host.querySelectorAll('button')].find((b) => b.textContent === 'Memoria').click();
    await vi.waitFor(() => expect(host.querySelector('.plan-memory')?.textContent).toContain('MD:# Project memory'));
    window.__TAURI__.core.invoke = vi.fn(async (command) => (command === 'memory_status' ? { active: false, memory: { exists: false, text: '', hash: '' }, tasks: { exists: false, text: '', hash: '' }, taskList: [], summary: '', missing: [] } : null));
    await plan.render();
    expect(host.textContent).toContain('Activar memoria del proyecto');
  });
});
