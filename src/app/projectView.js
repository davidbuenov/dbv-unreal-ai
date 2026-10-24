// =============================================================================
// DBV Unreal AI — Vista Proyecto: editores, preparación y conexión
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-01, REQ-02, REQ-03, REQ-04, REQ-05
//
// Escenarios A y B: la app detecta el editor abierto, dice qué le falta al proyecto y lo
// prepara enseñando ANTES el diff de cada fichero, con copia de seguridad y «Revertir».

import { t } from '../ai/i18nBridge.js';
import { lineOps } from '../ai/diff.js';
import * as api from './backend.js';
import { baseName, button, el } from './dom.js';

const BACKUP_KEY = 'dbv-unreal-ai:lastBackup:';

/** Texto y explicación de una comprobación del diagnóstico. */
export function checkLabel(check) {
  const [kind, name] = check.id.split(':');
  const label = kind === 'plugin' ? t('check.plugin', { name }) : t(`check.${kind}`, { detail: check.detail || '—' });
  const why = t(`check.why.${kind}`);
  return { label, why };
}

/**
 * Líneas de un diff para enseñar (`+`, `-`, ` `), con `context` líneas iguales alrededor de cada cambio.
 * Un fichero nuevo se enseña entero como añadido.
 */
export function diffPreview(before, after, context = 2) {
  const ops = lineOps(String(before ?? '').replace(/\r\n/g, '\n').split('\n'), String(after ?? '').replace(/\r\n/g, '\n').split('\n'));
  const keep = new Set();
  ops.forEach((op, index) => {
    if (op.type === 'equal') return;
    for (let i = Math.max(0, index - context); i <= Math.min(ops.length - 1, index + context); i += 1) keep.add(i);
  });
  const lines = [];
  let skipped = false;
  ops.forEach((op, index) => {
    if (!keep.has(index)) {
      skipped = true;
      return;
    }
    if (skipped) lines.push({ kind: 'skip', text: '…' });
    skipped = false;
    lines.push({ kind: op.type === 'insert' ? 'add' : op.type === 'delete' ? 'del' : 'same', text: `${op.type === 'insert' ? '+' : op.type === 'delete' ? '-' : ' '} ${op.line}` });
  });
  if (skipped) lines.push({ kind: 'skip', text: '…' });
  return lines;
}

/**
 * @param {HTMLElement} host
 * @param {object} deps `{state, toast, confirmModal, openModal, closeModal}`.
 */
export function createProjectView(host, deps) {
  let selected = null;
  let readiness = null;

  async function connect(project, mcp) {
    const endpoint = { port: mcp.port, path: mcp.path };
    deps.state.setConnection({ projectPath: project.path, projectName: project.name, engineVersion: project.engineAssociation, endpoint, status: 'connecting', toolsets: 0 });
    const result = await api.unrealConnect(endpoint);
    if (result.ok) {
      deps.state.setConnection({ status: 'online', toolsets: result.value.length }, true);
      deps.toast(t('project.connected', { n: String(result.value.length) }));
      await api.unrealRememberProject(project.path);
    } else {
      deps.state.setConnection({ status: readiness?.ready === false ? 'prep' : 'offline' }, true);
      deps.toast(result.error.message, 'error');
    }
  }

  function renderChecks(report) {
    return el(
      'ul',
      { className: 'checklist' },
      report.checks.map((check) => {
        const { label, why } = checkLabel(check);
        return el('li', { className: 'check' }, el('span', { className: check.ok ? 'check-ok' : 'check-fail', text: check.ok ? '✓' : '✗', attrs: { 'aria-label': check.ok ? 'ok' : 'falta' } }), el('div', {}, el('span', { text: label }), check.ok ? null : el('details', {}, el('summary', { text: '?' }), why)));
      }),
    );
  }

  async function showPlan(variant, port) {
    const plan = await api.unrealPreparePlan(selected, variant, port);
    if (!plan.ok) return deps.toast(plan.error.message, 'error');
    const changes = plan.value.changes;
    const body = changes.length
      ? changes.map((change) =>
          el(
            'details',
            { attrs: change.relative.endsWith('.uproject') || change.relative.endsWith('.ini') ? { open: '' } : {} },
            el('summary', { className: 'mono', text: `${change.relative}${change.before === null ? ` ${t('project.newFile')}` : ''}` }),
            el('pre', { className: 'diff' }, diffPreview(change.before, change.after).map((line) => el('div', { className: `diff-${line.kind}`, text: line.text }))),
          ),
        )
      : [el('p', { text: t('project.planEmpty') })];
    deps.openModal([
      el('h3', { text: t('project.planTitle') }),
      el('p', { className: 'hint', text: `${t('project.planIntro')} ${t('project.files', { n: String(changes.length) })}` }),
      ...body,
      el(
        'div',
        { className: 'modal-actions' },
        button(t('gen.cancel'), deps.closeModal),
        changes.length
          ? button(
              t('project.apply'),
              async () => {
                const applied = await api.unrealPrepareApply(selected, variant, port);
                deps.closeModal();
                if (!applied.ok) return deps.toast(applied.error.message, 'error');
                try {
                  localStorage.setItem(BACKUP_KEY + selected, applied.value);
                } catch {
                  // Sin almacenamiento local el revertir sigue disponible desde la carpeta de copias.
                }
                deps.toast(t('project.applied', { id: applied.value }));
                await select(selected);
              },
              'btn-primary',
            )
          : null,
      ),
    ]);
  }

  async function select(path) {
    selected = path;
    const report = await api.unrealReadiness(path);
    readiness = report.ok ? report.value : null;
    await render();
  }

  function projectCard(path, status) {
    const name = baseName(path).replace(/\.uproject$/i, '');
    return el(
      'div',
      { className: 'card editor-card' },
      el('div', { className: 'row' }, el('div', {}, el('strong', { text: name }), el('div', { className: 'hint mono', text: path })), el('div', { className: 'row' }, status, button(t('project.select'), () => select(path)))),
    );
  }

  async function render() {
    const [editors, recents] = await Promise.all([api.unrealEditors(), api.unrealRecentProjects()]);
    const open = editors.ok ? editors.value.filter((e) => e.project) : [];
    const sections = [el('div', { className: 'view-header' }, el('h2', { text: t('project.title') }), el('span', { className: 'spacer' }), button(t('project.refresh'), () => render()), button(t('project.openFile'), pickFile))];
    sections.push(el('div', { className: 'subheading', text: t('project.editors') }));
    if (!open.length) sections.push(el('p', { className: 'empty', text: t('project.noEditors') }));
    for (const editor of open) {
      const status = el('span', { className: `pill ${editor.listening ? 'pill--online' : 'pill--prep'}`, text: editor.listening ? t('project.listening', { port: String(editor.mcp?.port ?? '') }) : t('project.notListening', { port: String(editor.mcp?.port ?? 8000) }) });
      sections.push(projectCard(editor.editor.project, status));
    }
    const openPaths = new Set(open.map((e) => e.editor.project.toLowerCase()));
    const recentList = recents.ok ? recents.value.filter((p) => !openPaths.has(p.toLowerCase())) : [];
    if (recentList.length) {
      sections.push(el('div', { className: 'subheading', text: t('project.recents') }));
      for (const path of recentList) sections.push(projectCard(path, button(t('project.openInUnreal'), () => openInUnreal(path))));
    }
    if (selected && readiness) sections.push(readinessCard(open.find((e) => e.editor.project.toLowerCase() === selected.toLowerCase())));
    host.replaceChildren(...sections);
  }

  function readinessCard(editor) {
    const project = readiness.project;
    const variant = el('select', { attrs: { 'aria-label': t('project.variant') } }, el('option', { text: t('project.variant.python'), attrs: { value: 'python' } }), el('option', { text: t('project.variant.cpp'), attrs: { value: 'cpp' } }));
    if (!project.hasCpp) variant.disabled = true;
    const port = el('input', { attrs: { type: 'number', min: '1024', max: '65535', value: String(readiness.mcp.port), 'aria-label': t('project.port') } });
    let backup = null;
    try {
      backup = localStorage.getItem(BACKUP_KEY + selected);
    } catch {
      backup = null;
    }
    return el(
      'div',
      { className: 'card' },
      el('div', { className: 'row' }, el('h3', { text: `${project.name} · ${t('project.readiness')}` }), el('span', { className: 'spacer' })),
      readiness.ready ? el('p', { className: 'check-ok', text: t('project.ready') }) : null,
      renderChecks(readiness),
      el(
        'div',
        { className: 'row' },
        readiness.ready ? null : el('label', {}, t('project.variant'), variant),
        readiness.ready ? null : el('label', {}, t('project.port'), port),
        readiness.ready ? null : button(t('project.prepare'), () => showPlan(variant.value, Number(port.value) || null), 'btn-primary'),
        editor?.listening ? button(t('project.connect'), () => connect(project, editor.mcp ?? readiness.mcp), readiness.ready ? 'btn-primary' : '') : button(t('project.openInUnreal'), () => openInUnreal(selected)),
        backup ? button(t('project.revert'), async () => {
          const reverted = await api.unrealPrepareRevert(selected, backup);
          if (!reverted.ok) return deps.toast(reverted.error.message, 'error');
          try {
            localStorage.removeItem(BACKUP_KEY + selected);
          } catch {
            // Nada que limpiar.
          }
          deps.toast(t('project.reverted'));
          await select(selected);
        }, 'btn-ghost') : null,
      ),
    );
  }

  async function openInUnreal(path) {
    const opened = await api.unrealOpenProject(path);
    if (!opened.ok) deps.toast(opened.error.message, 'error');
    else await api.unrealRememberProject(path);
  }

  async function pickFile() {
    const picked = await window.__TAURI__.dialog.open({ multiple: false, filters: [{ name: 'Unreal Engine', extensions: ['uproject'] }] });
    if (typeof picked === 'string') {
      await api.unrealRememberProject(picked);
      await select(picked);
    }
  }

  /** Al abrir la app: si hay un editor con el servidor MCP activo, se conecta solo (escenario A). */
  async function autoConnect() {
    const editors = await api.unrealEditors();
    const live = editors.ok ? editors.value.find((e) => e.project && e.listening) : null;
    if (live) {
      await select(live.editor.project);
      await connect(live.project, live.mcp);
    } else await render();
  }

  return { render, autoConnect, select };
}
