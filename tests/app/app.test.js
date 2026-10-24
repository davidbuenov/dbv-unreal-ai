// =============================================================================
// DBV Unreal AI — Tests de la interfaz (lógica sin Tauri)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it } from 'vitest';
import { pillState } from '../../src/app/app.js';
import { checkLabel, diffPreview } from '../../src/app/projectView.js';
import { filterEntries } from '../../src/app/libraryView.js';
import { summarizeArgs, toMarkdown } from '../../src/app/actionLog.js';
import { mcpClientConfig } from '../../src/app/settingsView.js';
import { createState } from '../../src/app/state.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { createApprovalCard } from '../../src/unreal/approvalCard.js';
import { confirmCost, formArgs } from '../../src/app/generateView.js';

// jsdom arranca en inglés: las aserciones de texto están escritas en español.
beforeAll(() => window.DBV_I18N.setLang('es'));

describe('conexión y píldora (REQ-01, REQ-02)', () => {
  it('la píldora dice con qué proyecto está conectada la app @spec REQ-02, REQ-08', () => {
    expect(pillState(null)).toEqual({ text: 'Sin editor', className: 'pill' });
    const online = pillState({ projectName: 'DBVMCPFirst', engineVersion: '5.8', endpoint: { port: 8000 }, status: 'online' });
    expect(online.text).toBe('DBVMCPFirst · UE 5.8 · :8000');
    expect(online.className).toBe('pill pill--online');
    expect(pillState({ projectName: 'X', status: 'prep' }).className).toBe('pill pill--prep');
  });

  it('el estado avisa a los suscriptores y fusiona cambios @spec REQ-02', () => {
    const state = createState();
    const seen = [];
    state.subscribe((c) => seen.push(c?.status));
    state.setConnection({ projectName: 'A', status: 'connecting' });
    state.setConnection({ status: 'online' }, true);
    expect(state.connection).toEqual({ projectName: 'A', status: 'online' });
    expect(seen).toEqual(['connecting', 'online']);
  });
});

describe('preparación (REQ-04, REQ-05)', () => {
  it('cada comprobación tiene su texto y su porqué @spec REQ-04', () => {
    expect(checkLabel({ id: 'plugin:EditorToolset', ok: false, detail: '' }).label).toBe('Plugin EditorToolset activo');
    expect(checkLabel({ id: 'autoStart', ok: false, detail: '8000' }).label).toMatch(/puerto 8000/);
    expect(checkLabel({ id: 'engine', ok: true, detail: '5.8.2' }).why).toMatch(/5\.8/);
  });

  it('el diff enseña solo lo que cambia y un fichero nuevo entero @spec REQ-05', () => {
    const before = ['a', 'b', 'c', 'd', 'e', 'f', 'g'].join('\r\n');
    const after = ['a', 'b', 'c', 'D', 'e', 'f', 'g'].join('\n');
    const lines = diffPreview(before, after, 1);
    expect(lines.map((l) => l.text)).toEqual(['…', '  c', '- d', '+ D', '  e', '…']);
    expect(lines.filter((l) => l.kind === 'del').map((l) => l.text)).toEqual(['- d']);
    expect(lines.filter((l) => l.kind === 'add').map((l) => l.text)).toEqual(['+ D']);
    expect(lines.some((l) => l.text === '  a')).toBe(false);
    expect(diffPreview(null, 'x\ny').map((l) => l.kind)).toEqual(['del', 'add', 'add']);
  });
});

describe('biblioteca, registro y ajustes (REQ-14, REQ-17, REQ-20)', () => {
  it('filtra por nombre, prompt o proveedor @spec REQ-17', () => {
    const entries = [{ name: 'Cofre', prompt: 'treasure chest', provider: 'tripo' }, { name: 'Silla', prompt: 'chair', provider: 'tripo' }];
    expect(filterEntries(entries, 'CHEST').map((e) => e.name)).toEqual(['Cofre']);
    expect(filterEntries(entries, '')).toHaveLength(2);
  });

  it('el registro se exporta a Markdown sin romper la tabla @spec REQ-14', () => {
    const md = toMarkdown([{ at: 0, toolset: 'SceneTools', tool: 'add|x', kind: 'modify', outcome: 'ok', ms: 12, args: { code: 'a\nb\nc' } }], { project: 'Demo' });
    expect(md).toMatch(/^# DBV Unreal AI — Demo/);
    expect(md).toMatch(/add\\\|x/);
    expect(md).toMatch(/code: 3 lines/);
    expect(summarizeArgs({ path: 'x'.repeat(80) })).toMatch(/…$/);
  });

  it('la configuración MCP para otros agentes usa el propio ejecutable @spec REQ-20', () => {
    const config = JSON.parse(mcpClientConfig({ command: 'C:/app/dbv-unreal-ai.exe', args: ['--mcp', '--port', '8000'] }));
    expect(config.mcpServers['dbv-unreal-ai']).toEqual({ command: 'C:/app/dbv-unreal-ai.exe', args: ['--mcp', '--port', '8000'] });
  });

  it('español e inglés tienen las mismas claves @spec REQ-24', () => {
    expect(Object.keys(APP_TRANSLATIONS.en).sort()).toEqual(Object.keys(APP_TRANSLATIONS.es).sort());
  });
});

describe('tarjeta de aprobación (REQ-12)', () => {
  it('enseña el código entero como texto y resuelve con la elección; el foco empieza en «Denegar» @spec REQ-12, RNF-07', async () => {
    const code = '<img src=x onerror=alert(1)>\nprint(1)';
    const card = createApprovalCard({ kind: 'python', toolset: 'DBV', tool: 'run_python', code, purpose: 'Probar', validated: true, findings: { findings: [{ kind: 'deprecated', line: 1, message: 'old' }] } });
    document.body.append(card.node);
    expect(card.node.querySelector('pre').textContent).toBe(code);
    expect(card.node.querySelector('img')).toBeNull();
    expect(card.node.textContent).toMatch(/Obsoleto/);
    await Promise.resolve();
    expect(document.activeElement.textContent).toBe('Denegar');
    card.node.querySelectorAll('button')[1].click();
    expect(await card.choice).toBe('always');
    expect(card.node.querySelector('button')).toBeNull();
  });

  it('pliega el código largo y avisa si no se pudo validar @spec REQ-12, REQ-26', () => {
    const code = Array.from({ length: 30 }, (_, i) => `x${i} = ${i}`).join('\n');
    const card = createApprovalCard({ kind: 'python', toolset: 'P', tool: 'RunPython', code, validated: false });
    expect(card.node.querySelector('pre').textContent.split('\n')).toHaveLength(20);
    expect(card.node.textContent).toMatch(/no se ha podido validar/);
    card.node.querySelector('.btn-link').click();
    expect(card.node.querySelector('pre').textContent).toBe(code);
  });
});

describe('privacidad y coste (RNF-IA.2, RNF-04)', () => {
  it('la tarjeta de coste da el total, enseña lo que se envía a Tripo y deja «Cancelar» con el foco @spec RNF-IA.2, RNF-04, RNF-07, REQ-27', async () => {
    let seen = null;
    const accepted = await confirmCost(async (options) => {
      seen = options;
      return 'cancel';
    }, { name: 'Cofre', operation: 'text_to_model', endpoint: '/generation/text-to-model', params: { prompt: 'chest', smart_low_poly: true }, variants: 2, total: 60, spent: 60, budget: 300 });
    expect(accepted).toBe(false);
    expect(seen.text).toMatch(/se envía a Tripo/);
    expect(seen.text).toMatch(/60 créditos/);
    expect(seen.text).toMatch(/2 variantes/);
    expect(seen.detail).toBe('POST /generation/text-to-model\n{\n  "prompt": "chest",\n  "smart_low_poly": true\n}');
    expect(seen.choices.at(-1).key).toBe('cancel');
  });

  it('el formulario de Generar se traduce en la misma herramienta que usa la IA @spec REQ-15, REQ-29, REQ-32', () => {
    expect(formArgs({ mode: 'text', prompt: ' chest ', name: 'Cofre ', variants: '3', faceLimit: '3000', lowPoly: true, textureQuality: 'detailed' })).toEqual({
      tool: 'generate_3d_model',
      args: { prompt: 'chest', name: 'Cofre', variants: 3, options: { texture_quality: 'detailed' }, face_limit: 3000, smart_low_poly: true },
    });
    expect(formArgs({ mode: 'image', image: 'C:/refs/barco.png', name: 'Barco', variants: '1', textureQuality: 'standard' })).toEqual({ tool: 'image_to_3d_model', args: { image: 'C:/refs/barco.png', name: 'Barco', variants: 1 } });
  });
});
