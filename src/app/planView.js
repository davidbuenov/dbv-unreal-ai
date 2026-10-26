// =============================================================================
// DBV Unreal AI — Vista Plan: tareas y memoria del proyecto
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-36, REQ-34
//
// Lo que el usuario hace aquí (marcar, reabrir, editar, borrar) se escribe al momento: es su acción, no la de la
// IA. Toda escritura va sobre la huella del texto leído, así que si el fichero cambió (a mano, `git pull`, un
// agente) no se pisa: se avisa y se vuelve a leer. «Planificar» y «Hacer esta tarea» van a la conversación.

import { getLanguage, t } from '../ai/i18nBridge.js';
import * as api from './backend.js';
import { button, el } from './dom.js';

/** Línea de una tarea con otro título (conserva la casilla, el criterio y el coste). */
export function retitle(task, title) {
  const clean = String(title ?? '').replace(/[\r\n]+/g, ' ').trim();
  const at = task.line.indexOf(task.title);
  return at < 0 || !clean ? task.line : `${task.line.slice(0, at)}${clean}${task.line.slice(at + task.title.length)}`;
}

/**
 * Cambio de una línea ENTERA de `task.md` (`null` la borra): buscar solo el texto encontraría también el ejemplo que
 * la plantilla cita con el mismo título.
 */
export function wholeLine(line, replacement) {
  return { op: 'replace', search: `\n${line}\n`, replace: replacement === null ? '\n' : `\n${replacement}\n` };
}

/** Línea de una tarea marcada o desmarcada. */
export function toggled(task) {
  return task.done ? task.line.replace(/\[[xX]\]/, '[ ]') : task.line.replace('[ ]', '[x]');
}

/** ¿El criterio es una comprobación que la app sabe hacer en el editor (REQ-37)? */
export const isCheck = (criterion) => /^(asset exists|actor exists|level is|no errors in log)\b/i.test(String(criterion ?? '').trim());

/** Petición de «Planificar» para la conversación. */
export function planRequest(goal, lang) {
  return lang === 'es'
    ? `Planifica este objetivo para el proyecto y propón el plan con project_tasks_propose (tareas concretas y en orden, con su comprobación de «hecho» cuando se pueda comprobar en el editor y las generaciones de pago que necesite): ${goal}`
    : `Plan this goal for the project and propose the plan with project_tasks_propose (concrete tasks in order, each with its done-check when it can be checked in the editor and the paid generations it needs): ${goal}`;
}

/** Petición de «Hacer esta tarea». */
export function taskRequest(task, lang) {
  const check = task.criterion ? (lang === 'es' ? ` Estará hecha cuando: ${task.criterion}.` : ` It is done when: ${task.criterion}.`) : '';
  return lang === 'es' ? `Hagamos esta tarea del plan: «${task.title}».${check}` : `Let's do this task of the plan: "${task.title}".${check}`;
}

/**
 * @param {HTMLElement} host
 * @param {object} deps `{state, toast, openModal, closeModal, ask: (text) => void, showView: (name) => void, onChanged?: () => void, renderMarkdown?: () => Promise<(md: string) => Node>}`.
 */
export function createPlanView(host, deps) {
  let tab = 'tasks';

  async function write(project, status, edit) {
    const saved = await api.memoryApply(project, 'tasks', status.tasks.hash, edit);
    if (!saved.ok) deps.toast(saved.error.kind === 'conflict' ? t('plan.changed') : saved.error.message, 'error');
    deps.onChanged?.();
    return render();
  }

  function taskRow(project, status, task) {
    const title = el('span', { className: 'plan-task__title', text: task.title });
    const box = el('input', { attrs: { type: 'checkbox', 'aria-label': task.title } });
    box.checked = task.done;
    box.addEventListener('change', async () => {
      // Con una comprobación, «hecha» la decide el editor (REQ-37); sin editor queda pendiente de comprobar.
      if (task.done || !isCheck(task.criterion)) return write(project, status, wholeLine(task.line, toggled(task)));
      const endpoint = deps.state.connection?.status === 'online' ? deps.state.connection.endpoint : null;
      const updated = await api.memoryTaskUpdate(project, endpoint, task.title, true, false);
      deps.toast(updated.ok ? updated.value.message : updated.error.message, updated.ok && updated.value.saved ? 'info' : 'error');
      deps.onChanged?.();
      return render();
    });
    const edit = () => {
      const field = el('input', { className: 'plan-task__edit', attrs: { type: 'text', 'aria-label': t('plan.edit') } });
      field.value = task.title;
      field.addEventListener('keydown', (event) => {
        if (event.key === 'Enter') write(project, status, wholeLine(task.line, retitle(task, field.value)));
        if (event.key === 'Escape') render();
      });
      title.replaceWith(field);
      field.focus();
    };
    return el(
      'li',
      { className: `plan-task${task.done ? ' plan-task--done' : ''}` },
      box,
      el('div', { className: 'plan-task__body' }, title, task.criterion ? el('span', { className: `plan-task__criterion caption${isCheck(task.criterion) ? ' mono' : ''}`, text: task.criterion }) : null),
      task.credits ? el('span', { className: 'plan-task__cost cost', text: t('plan.credits', { credits: String(task.credits) }) }) : null,
      el(
        'span',
        { className: 'plan-task__actions' },
        task.done ? null : button(t('plan.doTask'), () => deps.ask(taskRequest(task, getLanguage())), 'btn-primary button--compact'),
        button(t('plan.edit'), edit, 'btn-ghost button--compact'),
        button(t('plan.delete'), () => write(project, status, wholeLine(task.line, null)), 'btn-ghost btn-danger button--compact'),
      ),
    );
  }

  function section(text) {
    return String(text ?? '').trim();
  }

  function tasksTab(project, status) {
    const sections = Object.fromEntries((status.tasks.text.match(/^## .*$/gm) ?? []).map((heading) => [heading.slice(3).trim(), '']));
    const lines = status.tasks.text.split('\n');
    let current = null;
    for (const line of lines) {
      if (line.startsWith('## ')) current = line.slice(3).trim();
      else if (current && !/^_.*_$/.test(line.trim())) sections[current] = `${sections[current]}\n${line}`;
    }
    const goal = section(sections.Goal);
    const next = section(sections['Next step']);
    const open = status.taskList.filter((task) => !task.done);
    const total = open.reduce((sum, task) => sum + (task.credits ?? 0), 0);
    return [
      el('div', { className: 'subheading', text: t('plan.goal') }),
      el('p', { className: goal ? '' : 'hint', text: goal || t('plan.noGoal') }),
      status.taskList.length ? el('ul', { className: 'plan-tasks' }, status.taskList.map((task) => taskRow(project, status, task))) : el('p', { className: 'empty', text: t('plan.noTasks') }),
      el('div', { className: 'row' }, el('span', { className: 'subheading', text: t('plan.next') }), el('span', { className: 'spacer' }), total ? el('span', { className: 'cost', text: t('plan.total', { credits: String(total) }) }) : null),
      el('p', { className: next ? '' : 'hint', text: next || '—' }),
    ];
  }

  async function memoryTab(status) {
    const box = el('div', { className: 'md plan-memory' });
    const render = await deps.renderMarkdown?.();
    if (render) box.replaceChildren(render(status.memory.text));
    else box.textContent = status.memory.text;
    return [box];
  }

  function planDialog() {
    const goal = el('textarea', { attrs: { rows: '4', 'aria-label': t('plan.goal') } });
    deps.openModal([
      el('h3', { text: t('plan.plan') }),
      el('p', { className: 'hint', text: t('plan.planHint') }),
      goal,
      el('div', { className: 'row' }, button(t('plan.send'), () => {
        const text = goal.value.trim();
        if (!text) return;
        deps.closeModal();
        deps.ask(planRequest(text, getLanguage()));
      }, 'btn-primary'), button(t('memory.cancel'), deps.closeModal, 'btn-ghost')),
    ]);
    queueMicrotask(() => goal.focus());
  }

  async function render() {
    const project = deps.state.connection?.projectPath ?? null;
    const header = el('div', { className: 'view-header' }, el('h2', { text: t('plan.title') }), el('span', { className: 'spacer' }));
    if (!project) return host.replaceChildren(header, el('p', { className: 'empty', text: t('plan.noProject') }));
    const read = await api.memoryStatus(project);
    if (!read.ok) return host.replaceChildren(header, el('p', { className: 'error', text: read.error.message }));
    const status = read.value;
    if (!status.active) {
      return host.replaceChildren(header, el('div', { className: 'card stack' }, el('p', { text: t('memory.intro') }), el('div', { className: 'row' }, button(t('memory.activate'), () => deps.showView('project'), 'btn-primary'))));
    }
    header.append(button(t('plan.plan'), planDialog, 'btn-primary'));
    const tabs = el(
      'div',
      { className: 'segmented', attrs: { role: 'group' } },
      ...['tasks', 'memory'].map((name) =>
        button(t(`plan.tab.${name}`), () => {
          tab = name;
          render();
        }, 'segmented-btn'),
      ),
    );
    for (const node of tabs.children) node.setAttribute('aria-pressed', String(node.textContent === t(`plan.tab.${tab}`)));
    const body = tab === 'tasks' ? tasksTab(project, status) : await memoryTab(status);
    return host.replaceChildren(header, el('div', { className: 'card stack' }, tabs, ...body));
  }

  return { render };
}
