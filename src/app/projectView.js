// =============================================================================
// DBV Unreal AI — Vista Proyecto: editores, preparación y conexión
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-01, REQ-02, REQ-03, REQ-04, REQ-05, REQ-87
//
// Escenarios A y B: la app detecta el editor abierto, dice qué le falta al proyecto y lo
// prepara enseñando ANTES el diff de cada fichero, con copia de seguridad y «Revertir».

import { t } from '../ai/i18nBridge.js';
import { lineOps } from '../ai/diff.js';
import * as api from './backend.js';
import { baseName, button, dirName, el } from './dom.js';
import { convaiCard, convaiLive, directAccessBanner, hasConvaiToolsets } from './convaiCard.js';
import { isDbvToolset } from '../unreal/unrealTools.js';
import { shortVersion, versionMismatch } from '../unreal/engineVersion.js';
import { uefnDiagnosticCard, uefnSection } from '../unreal/uefn.js';

const BACKUP_KEY = 'dbv-unreal-ai:lastBackup:';

/** Plugins de Epic que usa «vestir a un MetaHuman» (los mismos que `prepare::DRESSING_PLUGINS`). @spec REQ-87 */
export const DRESSING_PLUGINS = ['GeometryScripting', 'ChaosClothAssetToolset', 'PythonFoundationPackages'];
/** Plugins que «Preparar» pide solo desde UE 5.9 (los mismos que `prepare::UE59_PLUGINS`). @spec REQ-110 */
export const UE59_PLUGINS = ['AllToolsets'];

/** Texto y explicación de una comprobación del diagnóstico. */
export function checkLabel(check) {
  const [kind, name] = check.id.split(':');
  const label = kind === 'plugin' ? t('check.plugin', { name }) : t(`check.${kind}`, { detail: check.detail || '—' });
  const special = kind === 'plugin' && DRESSING_PLUGINS.includes(name) ? 'dressing' : kind === 'plugin' && UE59_PLUGINS.includes(name) ? 'ue59' : kind;
  const why = t(`check.why.${special}`);
  return { label, why };
}

/** Solución de una comprobación que falla, en una frase (REQ-120). */
export function checkFix(check) {
  const [kind] = check.id.split(':');
  const key = `check.fix.${kind}`;
  const text = t(key);
  return text === key ? '' : text;
}

/**
 * Pasos del diagnóstico en orden (REQ-120): las comprobaciones de «Preparar», el editor abierto, su servidor MCP y el
 * plugin C++. Cada uno con su texto, si está bien y, si no, cómo arreglarlo.
 */
export function diagnosticSteps(readiness, editor) {
  const steps = (readiness?.checks ?? []).map((check) => ({ ok: check.ok, label: checkLabel(check).label, fix: check.ok ? '' : checkFix(check) }));
  const port = String(editor?.mcp?.port ?? readiness?.mcp?.port ?? '');
  steps.push({ ok: Boolean(editor), label: t('step.editor'), fix: editor ? '' : t('step.editorFix') });
  steps.push({ ok: Boolean(editor?.listening), label: t('step.mcp', { port }), fix: editor?.listening ? '' : t(editor ? 'step.mcpFix' : 'step.editorFix') });
  if (readiness?.cppPlugin && readiness.cppPlugin !== 'absent') {
    const compiled = readiness.cppPlugin === 'compiled';
    steps.push({ ok: compiled, label: t(compiled ? 'step.cppCompiled' : 'step.cppNeedsCompile'), fix: compiled ? '' : t(readiness.cppBinaries ? 'step.cppFixPrepare' : 'step.cppFixCompile') });
  }
  return steps;
}

/** Texto para «Copiar diagnóstico» (REQ-120): los pasos legibles y el informe anonimizado de Rust. */
export function diagnosticsText(projectName, steps, report) {
  const lines = [`# ${t('project.diagnosticsTitle', { project: projectName })}`, ''];
  for (const step of steps) lines.push(`${step.ok ? '✓' : '✗'} ${step.label}${step.fix ? ` — ${step.fix}` : ''}`);
  lines.push('', '```json', JSON.stringify(report ?? {}, null, 2), '```');
  return lines.join('\n');
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

/** Misma ruta de proyecto, aunque cambien las barras o las mayúsculas (Windows). */
export const sameProjectPath = (a, b) => Boolean(a && b) && String(a).replace(/\\/g, '/').toLowerCase() === String(b).replace(/\\/g, '/').toLowerCase();

/**
 * Proyecto que tiene abierto el editor de un puerto, según el toolset DBV (`get_project_info` vive en él, no es una
 * herramienta suelta del MCP). Sin el toolset no se puede saber: `null`, y la conexión sigue. @spec REQ-07
 * @param {{port: number, path: string}} endpoint
 * @param {Array<{name: string}>} toolsets los que devuelve `unrealConnect`.
 * @returns {Promise<{project_name?: string} | null>}
 */
export async function editorIdentity(endpoint, toolsets) {
  const dbv = (toolsets ?? []).find((toolset) => isDbvToolset(toolset?.name))?.name;
  const info = dbv ? await api.unrealCallTool(endpoint, dbv, 'get_project_info', {}) : null;
  const value = info?.ok && !info.value?.isError ? info.value?.value : null;
  return value && typeof value === 'object' ? value : null;
}

/**
 * @param {HTMLElement} host
 * @param {object} deps `{state, toast, confirmModal, openModal, closeModal}`.
 */
export function createProjectView(host, deps) {
  let selected = null;
  let readiness = null;
  // Proyecto UEFN elegido (REQ-122): va aparte de `selected`, que es un `.uproject` con su diagnóstico de UE.
  let selectedUefn = null;

  async function connect(project, mcp) {
    const endpoint = { port: mcp.port, path: mcp.path };
    // REQ-118: la versión del motor resuelto, también si la asociación es un GUID o un nombre registrado.
    const projectVersion = project.engineVersion ? shortVersion(project.engineVersion) : project.engineAssociation;
    deps.state.setConnection({ projectPath: project.path, projectName: project.name, engineVersion: projectVersion, endpoint, status: 'connecting', toolsets: 0 });
    const result = await api.unrealConnect(endpoint);
    if (result.ok) {
      const toolsets = Array.isArray(result.value) ? result.value : (result.value.toolsets ?? []);
      const actualEndpoint = Array.isArray(result.value) ? endpoint : (result.value.endpoint ?? endpoint);
      const identity = await editorIdentity(actualEndpoint, toolsets);
      if (identity?.project_name && project.name && identity.project_name.toLowerCase() !== project.name.toLowerCase()) {
        deps.state.setConnection({ status: 'offline' }, true);
        deps.toast(t('unreal.identityMismatch', { port: String(actualEndpoint.port), found: identity.project_name, expected: project.name }), 'error');
        return;
      }
      // El editor conectado puede ser de otra versión que el proyecto si el usuario lo abrió a mano con otro motor:
      // se avisa (convertir no tiene vuelta atrás) y la píldora y la IA usan la versión real (REQ-109).
      const serverVersion = Array.isArray(result.value) ? '' : (result.value.serverVersion ?? '');
      const mismatch = versionMismatch(projectVersion, serverVersion);
      const engineVersion = serverVersion ? shortVersion(serverVersion) : projectVersion;
      // Con Convai en el editor, la conversación recibe sus reglas (REQ-56).
      // REQ-115: las instrucciones que el usuario puso en el MCP del editor (UE 5.9) van al prompt de la IA integrada.
      const editorInstructions = Array.isArray(result.value) ? '' : (result.value.instructions ?? '');
      deps.state.setConnection({ status: 'online', endpoint: actualEndpoint, engineVersion, toolsets: toolsets.length, convai: hasConvaiToolsets(toolsets), editorInstructions }, true);
      deps.toast(mismatch ? t('unreal.versionMismatch', mismatch) : t('project.connected', { n: String(toolsets.length) }), mismatch ? 'error' : undefined);
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
        const fix = check.ok ? '' : checkFix(check);
        return el('li', { className: 'check' }, el('span', { className: check.ok ? 'check-ok' : 'check-fail', text: check.ok ? '✓' : '✗', attrs: { 'aria-label': check.ok ? 'ok' : 'falta' } }), el('div', {}, el('span', { text: label }), fix ? el('p', { className: 'check-fix', text: fix }) : null, check.ok ? null : el('details', {}, el('summary', { text: '?' }), why)));
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

  /**
   * Diagnóstico de un proyecto UEFN y su conexión (REQ-123). El MCP de UEFN está en el puerto de la fila 5.
   */
  async function uefnPanel(project, install) {
    const [rows, skillStatus] = await Promise.all([api.uefnDiagnostics(project.path), api.uefnSkillStatus(project.path)]);
    const skill = skillStatus.ok ? skillStatus.value : null;
    if (!rows.ok) return el('div', { className: 'card' }, el('p', { className: 'error', text: rows.error.message }));
    const port = Number(rows.value.find((r) => r.step === 5)?.detail) || 8000;
    const endpoint = { port, path: '/mcp' };
    const connection = deps.state.connection;
    const connected = connection?.status === 'online' && connection?.uefn === true && sameProjectPath(connection.projectPath, project.path);
    const connectUefn = async () => {
      deps.state.setConnection({ projectPath: project.path, projectName: project.title, engineVersion: `UEFN ${install?.version?.version ?? ''}`.trim(), endpoint, status: 'connecting', toolsets: 0, uefn: true });
      const result = await api.unrealConnect(endpoint);
      if (!result.ok || !result.value.uefn) {
        deps.state.setConnection({ status: 'offline' }, true);
        deps.toast(result.ok ? t('uefn.fix.otherEditor', { detail: String(port) }) : result.error.message, 'error');
      } else {
        deps.state.setConnection({ status: 'online', endpoint: result.value.endpoint ?? endpoint, toolsets: result.value.toolsets.length, editorInstructions: result.value.instructions ?? '', uefn: true }, true);
        deps.toast(t('project.connected', { n: String(result.value.toolsets.length) }));
      }
      await render();
    };
    const enablePython = async () => {
      const choice = await deps.confirm({ title: t('uefn.enablePython'), text: t('uefn.enablePythonConfirm'), choices: [{ key: 'cancel', label: t('gen.cancel') }, { key: 'ok', label: t('uefn.enablePython'), tone: 'primary' }] });
      if (choice !== 'ok') return;
      const enabled = await api.uefnEnablePython(endpoint);
      deps.toast(enabled.ok ? t('uefn.pythonEnabled') : enabled.error.message, enabled.ok ? undefined : 'error');
      await render();
    };
    // Skill de Verse de Epic (REQ-124): sin licencia, se descarga de GitHub a la máquina del usuario.
    const installSkill = async () => {
      const scope = await deps.confirm({ title: t('uefn.skill.install'), text: t('uefn.skill.where'), choices: [{ key: 'cancel', label: t('gen.cancel') }, { key: 'user', label: t('uefn.skill.user') }, { key: 'project', label: t('uefn.skill.project'), tone: 'primary' }] });
      if (scope !== 'project' && scope !== 'user') return;
      deps.toast(t('uefn.skill.installing'));
      const installed = await api.uefnSkillInstall(project.path, scope);
      deps.toast(installed.ok ? t('uefn.skill.installed', { path: installed.value.path }) : installed.error.message, installed.ok ? undefined : 'error');
      await render();
    };
    const updateSkill = async () => {
      const updated = await api.uefnSkillUpdate(project.path);
      deps.toast(updated.ok ? t('uefn.skill.updated', { commit: updated.value.commit }) : updated.error.message, updated.ok ? undefined : 'error');
      await render();
    };
    const openUrl = (url) => window.__TAURI__?.opener?.openUrl?.(url);
    return uefnDiagnosticCard(project, rows.value, { connect: connectUefn, enablePython, connected, skill, installSkill, updateSkill, openUrl });
  }

  async function selectUefn(path) {
    selected = null;
    readiness = null;
    selectedUefn = path;
    await render();
  }

  async function select(path) {
    selected = path;
    selectedUefn = null;
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
    const [editors, recents, uefn] = await Promise.all([api.unrealEditors(), api.unrealRecentProjects(), api.uefnOverview()]);
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
    sections.push(...uefnSection(uefn.ok ? uefn.value : null, { select: selectUefn }));
    const uefnProject = selectedUefn && uefn.ok ? uefn.value.projects.find((p) => sameProjectPath(p.path, selectedUefn)) : null;
    if (uefnProject) sections.push(await uefnPanel(uefnProject, uefn.value.install));
    if (selected && readiness) {
      // Encima del diagnóstico: el agente vería el editor dos veces (REQ-52).
      sections.push(directAccessBanner(readiness.directAccess, { reveal: (file) => window.__TAURI__?.opener?.revealItemInDir?.(`${dirName(selected)}/${file}`) }));
      sections.push(readinessCard(open.find((e) => e.editor.project.toLowerCase() === selected.toLowerCase())));
      // Convai (REQ-55): toolsets y guías solo se comprueban con ESTE proyecto conectado.
      const connection = deps.state.connection;
      const connected = connection?.status === 'online' && String(connection.projectPath ?? '').toLowerCase() === selected.toLowerCase();
      sections.push(convaiCard(readiness.convai, readiness.convai?.cited || readiness.convai?.version ? await convaiLive(api, connected ? connection.endpoint : null) : null));
    }
    if (selected) sections.push(await memoryCard(selected));
    host.replaceChildren(...sections.filter(Boolean));
  }

  /**
   * Memoria del proyecto (REQ-34): qué es y, antes de activarla, qué ficheros crea y con qué contenido. Es aparte de
   * «Preparar»: «Revertir preparación» no la toca (REJECTED-03).
   */
  async function memoryCard(path) {
    const status = await api.memoryStatus(path);
    if (!status.ok) return el('div', { className: 'card' }, el('p', { className: 'error', text: status.error.message }));
    const memory = status.value;
    const folder = `${dirName(path)}/dbv-unreal-ai`;
    const activate = async () => {
      const activated = await api.memoryActivate(path);
      if (!activated.ok) return deps.toast(activated.error.message, 'error');
      deps.toast(t('memory.activated'));
      deps.onMemoryChanged?.();
      return render();
    };
    const preview = () =>
      deps.openModal([
        el('h3', { text: t('memory.previewTitle') }),
        el('p', { className: 'hint', text: t('memory.previewHint') }),
        ...memory.missing.flatMap((file) => [el('div', { className: 'mono subheading', text: file.file }), el('pre', { className: 'diff', text: file.text })]),
        el('div', { className: 'row' }, button(t('memory.activate'), async () => {
          deps.closeModal();
          await activate();
        }, 'btn-primary'), button(t('memory.cancel'), deps.closeModal, 'btn-ghost')),
      ]);
    const open = memory.taskList.filter((task) => !task.done).length;
    return el(
      'div',
      { className: 'card stack' },
      el('h3', { text: t('memory.title') }),
      memory.active
        ? el('p', { text: t('memory.activeSummary', { lines: String(memory.memory.text.split('\n').length), open: String(open) }) })
        : el('p', { className: 'hint', text: t('memory.intro') }),
      el(
        'div',
        { className: 'row' },
        memory.active ? null : button(t('memory.activate'), preview),
        memory.active ? button(t('memory.openPlan'), () => deps.showView?.('plan')) : null,
        memory.active ? button(t('lib.reveal'), () => window.__TAURI__.opener.revealItemInDir(folder), 'btn-ghost') : null,
      ),
    );
  }

  function readinessCard(editor) {
    const project = readiness.project;
    const variant = el('select', { attrs: { 'aria-label': t('project.variant') } }, el('option', { text: t('project.variant.python'), attrs: { value: 'python' } }), el('option', { text: t('project.variant.cpp'), attrs: { value: 'cpp' } }));
    // REQ-121: con el plugin C++ ya compilado para este motor, la variante C++ (Fab, splats) no necesita Visual Studio,
    // también en un proyecto de solo Blueprints, y va preseleccionada.
    if (!project.hasCpp && !readiness.cppBinaries) variant.disabled = true;
    if (readiness.cppBinaries) variant.value = 'cpp';
    const port = el('input', { attrs: { type: 'number', min: '1024', max: '65535', value: String(readiness.proposedPort ?? readiness.mcp.port), 'aria-label': t('project.port') } });
    let backup = null;
    try {
      backup = localStorage.getItem(BACKUP_KEY + selected);
    } catch {
      backup = null;
    }
    return el(
      'div',
      { className: 'card' },
      el('div', { className: 'row' }, el('h3', { text: `${project.name} · ${t('project.readiness')}` }), el('span', { className: 'spacer' }), button(t('project.copyDiagnostics'), () => copyDiagnostics(project, editor), 'btn-ghost')),
      readiness.ready ? el('p', { className: 'check-ok', text: t('project.ready') }) : null,
      renderChecks(readiness),
      extraSteps(editor),
      engineChoice(readiness),
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

  /** Pasos del diagnóstico que no son de «Preparar»: editor, servidor MCP y plugin C++ (REQ-120). */
  function extraSteps(editor) {
    const steps = diagnosticSteps({ ...readiness, checks: [] }, editor);
    return el('ul', { className: 'checklist' }, steps.map((step) => el('li', { className: 'check' }, el('span', { className: step.ok ? 'check-ok' : 'check-fail', text: step.ok ? '✓' : '✗' }), el('div', {}, el('span', { text: step.label }), step.fix ? el('p', { className: 'check-fix', text: step.fix }) : null))));
  }

  /** «Copiar diagnóstico» (REQ-120): pasos legibles e informe sin claves ni la carpeta del usuario. */
  async function copyDiagnostics(project, editor) {
    const report = await api.appDiagnostics(project.path ?? selected);
    if (!report.ok) return deps.toast(report.error.message, 'error');
    await navigator.clipboard.writeText(diagnosticsText(project.name, diagnosticSteps(readiness, editor), report.value));
    return deps.toast(t('settings.diagnosticsCopied'));
  }

  /**
   * Motor elegido a mano (REQ-119): si no se encuentra, el botón para elegir la carpeta; si se eligió, cuál y cómo
   * volver a la detección automática.
   */
  function engineChoice(info) {
    const engineCheck = (info.checks ?? []).find((check) => check.id === 'engine');
    const missing = engineCheck ? !engineCheck.ok : !info.engine;
    const chosen = info.engineSource === 'chosen';
    let node = null;
    if (missing || chosen) {
      node = el(
        'div',
        { className: 'row engine-choice' },
        chosen && info.engine ? el('span', { className: 'hint', text: t('project.engineChosenHint', { version: info.engine.version, root: info.engine.root }) }) : el('span', { className: 'hint', text: t('project.engineMissingHint') }),
        button(t('project.pickEngine'), () => pickEngine(info.project.path), missing ? 'btn-primary' : ''),
        chosen ? button(t('project.engineAuto'), async () => {
          const cleared = await api.unrealClearEngine(info.project.path);
          if (!cleared.ok) return deps.toast(cleared.error.message, 'error');
          return select(info.project.path);
        }, 'btn-ghost') : null,
      );
    }
    return node;
  }

  async function pickEngine(projectPath) {
    const folder = await window.__TAURI__.dialog.open({ directory: true, multiple: false, title: t('project.pickEngine') });
    if (typeof folder !== 'string') return;
    const picked = await api.unrealPickEngine(projectPath, folder);
    if (!picked.ok) return deps.toast(picked.error.message, 'error');
    deps.toast(t('project.engineChosen', { version: picked.value.version }));
    await select(projectPath);
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

  /** Conecta el editor de ESE proyecto si está escuchando (tras reabrirlo desde la app, REQ-22). `true` si lo hizo. */
  async function connectProject(path) {
    const editors = await api.unrealEditors();
    const live = editors.ok ? editors.value.find((e) => e.listening && sameProjectPath(e.editor?.project ?? e.project?.path, path)) : null;
    if (live) {
      await select(live.editor.project);
      await connect(live.project, live.mcp);
    }
    return Boolean(live);
  }

  return { render, autoConnect, select, connectProject };
}
