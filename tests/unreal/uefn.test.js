// =============================================================================
// DBV Unreal AI — Tests: UEFN en la vista Proyecto
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it, vi } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { canEnablePython, toolsForUefn, uefnDiagnosticCard, uefnRows, uefnSection } from '../../src/unreal/uefn.js';

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

const project = (title, extra = {}) => ({ title, path: `D:/UEFN/${title}/${title}.uefnproject`, linked: false, compatibilityVersion: '43.10', open: false, newerThanInstalled: false, ...extra });
const overview = (projects) => ({ install: { root: 'C:/F', version: { version: '43.12', changelist: 1, source: 'build' } }, running: true, projects });

describe('UEFN en la vista Proyecto (REQ-122)', () => {
  it('pone primero el proyecto abierto y lleva la versión instalada', () => {
    // @spec REQ-122
    const { version, rows } = uefnRows(overview([project('Uno'), project('Abierto', { open: true })]));
    expect(version).toBe('43.12');
    expect(rows.map((r) => r.title)).toEqual(['Abierto', 'Uno']);
  });

  it('sin UEFN instalado (o fuera de Windows) no muestra nada', () => {
    // @spec REQ-122
    expect(uefnSection({ install: null, running: false, projects: [project('Uno')] }, { select: vi.fn() })).toEqual([]);
    expect(uefnSection(null, { select: vi.fn() })).toEqual([]);
  });

  it('muestra el distintivo [UEFN] con la versión y avisa si el proyecto pide una UEFN más nueva', () => {
    // @spec REQ-122
    const select = vi.fn();
    const nodes = uefnSection(overview([project('Nuevo', { compatibilityVersion: '43.20', newerThanInstalled: true })]), { select });
    const text = nodes.map((n) => n.textContent).join(' ');
    expect(text).toContain('[UEFN] 43.12');
    expect(text).toContain('UEFN 43.20');
    expect(nodes[1].querySelector('.warning')).not.toBeNull();
    nodes[1].querySelector('button').click();
    expect(select).toHaveBeenCalledWith('D:/UEFN/Nuevo/Nuevo.uefnproject');
  });

  it('con UEFN y sin proyectos lo dice', () => {
    // @spec REQ-122
    const nodes = uefnSection(overview([]), { select: vi.fn() });
    expect(nodes.map((n) => n.textContent).join(' ')).toContain('no se ha encontrado ningún proyecto');
  });
});

const diag = (overrides = {}) =>
  [
    { step: 1, key: 'uefn.diag.installed', ok: true, optional: false, detail: '43.12', fix: null },
    { step: 2, key: 'uefn.diag.editorOpen', ok: true, optional: false, detail: '', fix: null },
    { step: 3, key: 'uefn.diag.projectOpen', ok: true, optional: false, detail: 'DBVUnrealAITest', fix: null },
    { step: 4, key: 'uefn.diag.python', ok: false, optional: true, detail: '', fix: 'uefn.fix.enablePython' },
    { step: 5, key: 'uefn.diag.server', ok: true, optional: false, detail: '8000', fix: null },
    { step: 6, key: 'uefn.diag.toolsets', ok: true, optional: false, detail: '63', fix: null },
  ].map((row) => ({ ...row, ...(overrides[row.step] ?? {}) }));

describe('diagnóstico de UEFN (REQ-123)', () => {
  it('solo muestra filas de UEFN, sin las de Unreal Engine', () => {
    // @spec REQ-123
    const card = uefnDiagnosticCard({ title: 'MiIsla' }, diag(), { connect: vi.fn(), enablePython: vi.fn(), connected: false });
    const text = card.textContent;
    expect(card.querySelectorAll('li')).toHaveLength(6);
    expect(text).toContain('UEFN instalado (43.12)');
    expect(text).toContain('puerto 8000');
    for (const ue of ['PythonScriptPlugin', 'DBVUnrealAI', 'Fab', 'C++']) expect(text).not.toContain(ue);
  });

  it('Python es opcional: se marca con «–» y no impide conectar', () => {
    // @spec REQ-123
    const connect = vi.fn();
    const card = uefnDiagnosticCard({ title: 'P' }, diag(), { connect, enablePython: vi.fn(), connected: false });
    const marks = [...card.querySelectorAll('li > span')].map((s) => s.textContent);
    expect(marks).toEqual(['✓', '✓', '✓', '–', '✓', '✓']);
    const connectButton = [...card.querySelectorAll('button')].find((b) => b.textContent === 'Conectar con UEFN');
    expect(connectButton.className).toContain('btn-primary');
    connectButton.click();
    expect(connect).toHaveBeenCalled();
  });

  it('con el puerto ocupado explica cómo arreglarlo', () => {
    // @spec REQ-123
    const rows = diag({ 5: { ok: false, fix: 'uefn.fix.portBusy' }, 6: { ok: false, detail: '0', fix: 'uefn.fix.toolsets' } });
    const card = uefnDiagnosticCard({ title: 'P' }, rows, { connect: vi.fn(), enablePython: vi.fn(), connected: false });
    expect(card.textContent).toContain('Stop Server');
    expect(canEnablePython(rows)).toBe(false);
  });

  it('«Activar Python» solo aparece con el servidor de UEFN respondiendo y Python apagado', () => {
    // @spec REQ-123
    expect(canEnablePython(diag())).toBe(true);
    expect(canEnablePython(diag({ 4: { ok: true, fix: null } }))).toBe(false);
    const card = uefnDiagnosticCard({ title: 'P' }, diag({ 4: { ok: true, fix: null } }), { connect: vi.fn(), enablePython: vi.fn(), connected: true });
    expect(card.textContent).toContain('Conectado a UEFN');
    expect(card.textContent).not.toContain('Activar Python');
  });
});

describe('herramientas de la app con UEFN (REQ-123)', () => {
  const policy = { tools: ['unreal_call_tool', 'unreal_list_toolsets', 'library_list'], prefixes: ['generate_', 'verse_', 'project_memory_'] };
  const tool = (name) => ({ name });

  it('solo deja las que no necesitan nuestro plugin', () => {
    // @spec REQ-123
    const all = ['unreal_call_tool', 'unreal_run_python', 'unreal_dress_metahuman', 'library_send_to_unreal', 'generate_3d_model', 'level_zones_propose', 'verse_reference', 'project_memory_read', 'fab_install'].map(tool);
    expect(toolsForUefn(all, policy).map((t) => t.name)).toEqual(['unreal_call_tool', 'generate_3d_model', 'verse_reference', 'project_memory_read']);
  });

  it('sin la regla cargada no ofrece ninguna', () => {
    // @spec REQ-123
    expect(toolsForUefn([tool('unreal_call_tool')], null)).toEqual([]);
  });
});
