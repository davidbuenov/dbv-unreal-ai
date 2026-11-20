// =============================================================================
// DBV Unreal AI — Tests: UEFN en la vista Proyecto
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it, vi } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { canEnablePython, createSessionTools, createVerseTools, skillAttribution, toolsForUefn, uefnDiagnosticCard, uefnPrompt, uefnRows, uefnSection, VERSE_RULES, verseHeader } from '../../src/unreal/uefn.js';

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

describe('skill de Verse de Epic (REQ-124)', () => {
  const repository = 'https://github.com/magnusenebakk-epic/verse-skill';
  const base = { connect: vi.fn(), enablePython: vi.fn(), connected: true, installSkill: vi.fn(), updateSkill: vi.fn(), openUrl: vi.fn() };

  it('sin skill ofrece instalarla y dice de quién es', () => {
    // @spec REQ-124
    const card = uefnDiagnosticCard({ title: 'P' }, diag(), { ...base, skill: { installed: null, updateAvailable: false, repository } });
    const install = [...card.querySelectorAll('button')].find((b) => b.textContent === 'Instalar la skill de Verse de Epic');
    expect(install).toBeTruthy();
    install.click();
    expect(base.installSkill).toHaveBeenCalled();
    expect(card.textContent).toContain('Epic Games (Magnus Enebakk)');
  });

  it('instalada muestra commit y fecha, y «Actualizar» solo si hay commits nuevos', () => {
    // @spec REQ-124
    const installed = { commit: 'abc1234', date: '2026-10-14', path: 'x' };
    const card = uefnDiagnosticCard({ title: 'P' }, diag(), { ...base, skill: { installed, updateAvailable: false, repository } });
    expect(card.textContent).toContain('versión abc1234 del 2026-10-14');
    expect(card.textContent).not.toContain('Actualizar la skill');
    expect(card.textContent).not.toContain('Instalar la skill');
    const newer = uefnDiagnosticCard({ title: 'P' }, diag(), { ...base, skill: { installed, updateAvailable: true, repository } });
    expect(newer.textContent).toContain('Actualizar la skill de Verse');
  });

  it('el enlace abre el repositorio de Epic', () => {
    // @spec REQ-124
    const openUrl = vi.fn();
    const node = skillAttribution({ installed: null, repository }, openUrl);
    node.querySelector('a').click();
    expect(openUrl).toHaveBeenCalledWith(repository);
    expect(skillAttribution(null, openUrl)).toBeNull();
  });
});

describe('conocimiento de Verse para cualquier IA (REQ-125)', () => {
  it('el prompt lleva la regla de efectos, Scene Graph primero y BuildAll', () => {
    // @spec REQ-125
    const prompt = uefnPrompt({ projectName: 'DBVUnrealAITest', version: '43.12' });
    expect(VERSE_RULES).toHaveLength(10);
    expect(prompt).toContain('<transacts><no_rollback>');
    expect(prompt).toContain('class<final_super>(component)');
    expect(prompt).toContain('BuildAll');
    expect(prompt).toContain('verse_reference');
    expect(prompt).toContain('verse_api_search');
  });

  it('no enseña creative_device como plantilla de jugabilidad ni habla de Python del motor', () => {
    // @spec REQ-125
    const prompt = uefnPrompt({});
    expect(prompt).toContain('Do NOT write gameplay in creative_device');
    expect(prompt).not.toMatch(/:= class\(creative_device\)/);
    expect(prompt).not.toContain('unreal_run_python');
    expect(prompt).not.toContain('OnBegin');
  });

  it('funciona aunque la skill no esté instalada: las reglas van en el prompt', () => {
    // @spec REQ-125
    expect(uefnPrompt({}).split('\n').filter((line) => /^\d+\. /.test(line))).toHaveLength(10);
  });

  it('las herramientas de Verse llaman a Rust y devuelven su texto', async () => {
    // @spec REQ-125
    const run = vi.fn(async () => ({ ok: true, value: 'Topics of Epic' }));
    const [reference] = createVerseTools({ specs: [{ name: 'verse_reference', description: 'd', inputSchema: { type: 'object', properties: { topic: { type: 'string' } }, required: [] } }], run });
    expect(reference.label({ topic: 'gotchas' })).toBe('Verse: reference · gotchas');
    expect(await reference.run({ topic: 'gotchas' })).toBe('Topics of Epic');
    expect(run).toHaveBeenCalledWith('verse_reference', { topic: 'gotchas' });
  });
});

describe('escribir Verse (REQ-127)', () => {
  it('todo .verse nuevo lleva la cabecera DBV de 6 líneas con #', () => {
    // @spec REQ-127, REQ-69
    const lines = verseHeader('DBVUnrealAITest').split('\n').slice(1);
    expect(lines).toHaveLength(6);
    expect(lines.every((line) => line.startsWith('# '))).toBe(true);
    expect(lines[1]).toContain('DBVUnrealAITest');
    expect(uefnPrompt({ projectName: 'DBVUnrealAITest' })).toContain('# Generated with DBV Unreal AI');
  });

  it('el prompt limita los cambios a Verse de Content/ y nombra zz_ lo de prueba', () => {
    // @spec REQ-127
    const prompt = uefnPrompt({});
    expect(prompt).toContain('Verse files under Content/');
    expect(prompt).toContain('zz_');
  });
});

describe('sesión de UEFN (REQ-128)', () => {
  const spec = (name) => ({ name, description: 'd', inputSchema: { type: 'object', properties: {}, required: [] } });
  const specs = ['uefn_session_start', 'uefn_session_push', 'uefn_session_status', 'uefn_verse_output'].map(spec);

  it('arrancar pide aprobación y avisa de que guarda el nivel y tarda unos 2 minutos', async () => {
    // @spec REQ-128
    const askApproval = vi.fn(async () => 'once');
    const run = vi.fn(async () => ({ ok: true, value: 'Session: Completed' }));
    const [start] = createSessionTools({ specs, run, askApproval });
    expect(start.parameters.properties.purpose).toBeTruthy();
    expect(await start.run({ purpose: 'Probar la zona' })).toBe('Session: Completed');
    const request = askApproval.mock.calls[0][0];
    expect(request.purpose).toContain('2 minutos');
    expect(request.purpose).toContain('Probar la zona');
    expect(run).toHaveBeenCalledWith('uefn_session_start', {});
  });

  it('si se deniega no se ejecuta', async () => {
    // @spec REQ-128
    const run = vi.fn();
    const [, push] = createSessionTools({ specs, run, askApproval: async () => 'deny' });
    expect(await push.run({ wait_for: 'ZZTEST' })).toContain('DENIED');
    expect(run).not.toHaveBeenCalled();
  });

  it('leer el estado y la salida de Verse no pide permiso', async () => {
    // @spec REQ-128
    const askApproval = vi.fn();
    const run = vi.fn(async () => ({ ok: true, value: 'zz_test.verse:8 ZZTEST hola' }));
    const tools = createSessionTools({ specs, run, askApproval });
    expect(await tools[3].run({ pattern: 'ZZTEST' })).toContain('ZZTEST hola');
    await tools[2].run({});
    expect(askApproval).not.toHaveBeenCalled();
    expect(tools[3].parameters.properties.purpose).toBeUndefined();
  });
});
