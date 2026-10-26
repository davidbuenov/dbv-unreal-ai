// =============================================================================
// DBV Unreal AI — Tests de «Enviar a Unreal» y de las herramientas de generación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { describe, expect, it, vi } from 'vitest';
import { assetName, GENERATED_FOLDER, sendToUnreal } from '../../src/unreal/sendToUnreal.js';
import { createGenerationTools } from '../../src/generation/generationTools.js';

const ENTRY = { id: 'linterna-1', name: 'Linterna mágica', provider: 'tripo', kind: 'model', prompt: 'lantern', files: ['model.glb'], sentTo: [] };
const DBV = 'PythonTypes.DBVUnrealAIToolset_0x3D7D6EDD';

function backend({ toolsets = [{ name: DBV, description: '' }], importResult, playResult, splatResult } = {}) {
  const calls = [];
  return {
    calls,
    listToolsets: async () => ({ ok: true, value: toolsets }),
    libraryFilePath: async (id, file) => ({ ok: true, value: `C:/data/library/${id}/${file}` }),
    libraryRecordSent: vi.fn(async () => ({ ok: true })),
    callTool: async (toolset, tool, args, timeout) => {
      calls.push({ toolset, tool, args, timeout });
      if (tool === 'import_model') return { ok: true, value: { text: '', isError: false, value: importResult ?? { success: true, imported: ['/Game/DBV/Generated/Linterna_magica/StaticMeshes/Linterna_magica.Linterna_magica'], primary: '/Game/DBV/Generated/Linterna_magica/StaticMeshes/Linterna_magica.Linterna_magica' } } };
      if (tool === 'play_animation') return playResult ?? { ok: true, value: { text: '', isError: false, value: { success: true } } };
      if (tool === 'load_gaussian_splat') return splatResult ?? { ok: true, value: { text: '', isError: false, value: { success: true, actor: args.actor_label } } };
      return { ok: true, value: { text: '', isError: false, value: { success: true, actor: 'Linterna_magica' } } };
    },
  };
}

describe('enviar a Unreal (REQ-19)', () => {
  it('importa el GLB en /Game/DBV/Generated, lo coloca y lo anota en la biblioteca @spec REQ-19, REQ-17', async () => {
    const b = backend();
    const out = await sendToUnreal({ backend: b }, ENTRY, { project: 'D:/P/P.uproject' });
    expect(out).toEqual({ ok: true, assetPath: '/Game/DBV/Generated/Linterna_magica/StaticMeshes/Linterna_magica.Linterna_magica', actor: 'Linterna_magica', kind: 'static', animations: [], playing: null });
    expect(b.calls[0]).toMatchObject({ toolset: DBV, tool: 'import_model', args: { source_file: 'C:/data/library/linterna-1/model.glb', folder_path: GENERATED_FOLDER, asset_name: 'Linterna_magica' }, timeout: 300 });
    expect(b.calls[1]).toMatchObject({ tool: 'place_asset', args: { folder: 'DBV/Generated', distance: 300 } });
    expect(b.libraryRecordSent).toHaveBeenCalledWith('linterna-1', 'D:/P/P.uproject', out.assetPath);
  });

  it('un personaje de Tripo se coloca reproduciendo su primera animación @spec REQ-16, REQ-19', async () => {
    const rigged = { success: true, imported: ['/Game/DBV/Generated/Pirata', '/Game/DBV/Generated/Pirata_Skeleton', '/Game/DBV/Generated/dance_01'], primary: '/Game/DBV/Generated/Pirata', kind: 'skeletal', skeleton: '/Game/DBV/Generated/Pirata_Skeleton', animations: ['/Game/DBV/Generated/dance_01'] };
    const b = backend({ importResult: rigged });
    const out = await sendToUnreal({ backend: b }, { ...ENTRY, kind: 'animation' }, { project: 'p' });
    expect(out).toMatchObject({ ok: true, kind: 'skeletal', animations: ['/Game/DBV/Generated/dance_01'], playing: '/Game/DBV/Generated/dance_01' });
    expect(b.calls.map((c) => c.tool)).toEqual(['import_model', 'place_asset', 'play_animation']);
    expect(b.calls[2].args).toEqual({ actor_label: 'Linterna_magica', animation_path: '/Game/DBV/Generated/dance_01', looping: true });
  });

  it('con un toolset anterior a 0.2.0 el personaje se importa igual y se avisa de actualizarlo @spec REQ-16, REQ-05', async () => {
    const rigged = { success: true, imported: ['/Game/A'], primary: '/Game/A', kind: 'skeletal', animations: ['/Game/anim'] };
    const old = backend({ importResult: rigged, playResult: { ok: true, value: { text: 'unknown tool play_animation', isError: true, value: null } } });
    const out = await sendToUnreal({ backend: old }, ENTRY, { project: 'p' });
    expect(out).toMatchObject({ ok: true, playing: null, note: 'toolsetOutdated' });
    expect(old.libraryRecordSent).toHaveBeenCalled();
  });

  /** @spec REQ-41, REQ-42 */
  it('un escenario World Labs carga el Gaussian Splat nativo y coloca su malla de colisión @spec REQ-41, REQ-42', async () => {
    const worldEntry = {
      id: 'isla-pirata',
      name: 'Isla Pirata',
      kind: 'world',
      files: ['splat.spz', 'collision_mesh.glb', 'thumbnail.webp'],
      sentTo: [],
    };
    const b = backend();
    const out = await sendToUnreal({ backend: b }, worldEntry, { project: 'D:/P/P.uproject' });
    expect(out).toMatchObject({
      ok: true,
      kind: 'world',
      assetPath: 'Isla_Pirata_Splat',
      actor: 'Isla_Pirata_Splat',
      colliderActor: 'Linterna_magica',
    });
    expect(b.calls.map((c) => c.tool)).toEqual(['load_gaussian_splat', 'import_model', 'place_asset']);
    expect(b.calls[0].args).toEqual({
      source_file: 'C:/data/library/isla-pirata/splat.spz',
      actor_label: 'Isla_Pirata_Splat',
    });
    expect(b.libraryRecordSent).toHaveBeenCalledWith('isla-pirata', 'D:/P/P.uproject', 'Isla_Pirata_Splat');
  });

  it('sin colocar, sin toolset o con una importación fallida lo explica y no anota nada @spec REQ-19', async () => {
    const quiet = backend();
    await sendToUnreal({ backend: quiet }, ENTRY, { project: 'p', place: false });
    expect(quiet.calls.map((c) => c.tool)).toEqual(['import_model']);
    expect(await sendToUnreal({ backend: backend({ toolsets: [] }) }, ENTRY, { project: 'p' })).toEqual({ ok: false, error: 'toolsetMissing' });
    const failing = backend({ importResult: { success: false, error: 'the import produced no assets' } });
    expect(await sendToUnreal({ backend: failing }, ENTRY, { project: 'p' })).toEqual({ ok: false, error: 'the import produced no assets' });
    expect(failing.libraryRecordSent).not.toHaveBeenCalled();
    expect(await sendToUnreal({ backend: backend() }, { ...ENTRY, files: ['thumbnail.webp'] }, { project: 'p' })).toEqual({ ok: false, error: 'noModelFile' });
  });

  it('el nombre del asset es válido para Unreal @spec REQ-19', () => {
    expect(assetName('Linterna mágica (v2)!')).toBe('Linterna_magica_v2');
    expect(assetName('3 sillas')).toBe('SM_3_sillas');
    expect(assetName('¡¡!!')).toBe('Generated');
  });
});

describe('herramientas de generación (REQ-15, REQ-33, RNF-04)', () => {
  const status = async () => ({ ok: true, value: { spent: 0, budget: 300, hasKey: { tripo: true } } });
  // Lo que da Rust (`generation_tool_specs`): aquí basta con el nombre y el esquema.
  const specs = ['generate_3d_model', 'image_to_3d_model', 'rig_model', 'animate_model', 'tripo_run', 'tripo_operations', 'tripo_balance'].map((name) => ({ name, description: name, inputSchema: { type: 'object', properties: {}, required: [] } }));
  const prepared = { operation: 'text_to_model', endpoint: '/generation/text-to-model', name: 'Linterna', variants: 2, params: { prompt: 'lantern' }, total: 40, pricing: '…' };
  const tools = (over = {}) => {
    const deps = {
      specs,
      status,
      prepare: vi.fn(async () => ({ ok: true, value: prepared })),
      launch: vi.fn(async () => ({ ok: true, value: ['job-1', 'job-2'] })),
      waitJob: vi.fn(async () => ({ stage: 'done', entry: ENTRY })),
      showJobs: vi.fn(),
      operations: vi.fn(async () => ({ ok: true, value: '- rig …' })),
      balance: vi.fn(async () => ({ ok: true, value: { balance: 2370, frozen: 30 } })),
      listLibrary: vi.fn(async () => ({ ok: true, value: [ENTRY] })),
      rename: vi.fn(async (id, name) => ({ ok: true, value: { ...ENTRY, name } })),
      send: vi.fn(),
      askApproval: vi.fn(),
      confirmCost: vi.fn(async () => true),
      ...over,
    };
    const list = createGenerationTools(deps);
    return { deps, byName: (name) => list.find((tool) => tool.name === name), list };
  };

  it('publica las herramientas de Rust (las mismas que un agente) y las de la biblioteca @spec REQ-33', () => {
    const { list } = tools();
    expect(list.map((tool) => tool.name)).toEqual([...specs.map((spec) => spec.name), 'library_list', 'library_rename', 'library_send_to_unreal']);
  });

  it('sin aceptar el coste no se lanza nada @spec REQ-15, RNF-04', async () => {
    const { deps, byName } = tools({ confirmCost: vi.fn(async () => false) });
    expect(await byName('generate_3d_model').run({ prompt: 'lantern', name: 'Linterna', variants: 2 })).toMatch(/did NOT accept/);
    // La tarjeta recibe el TOTAL de las variantes y lo que se enviará (REQ-27, REQ-29).
    expect(deps.confirmCost).toHaveBeenCalledWith(expect.objectContaining({ total: 40, variants: 2, endpoint: '/generation/text-to-model', params: { prompt: 'lantern' }, budget: 300 }));
    expect(deps.launch).not.toHaveBeenCalled();
  });

  it('aceptado, lanza, enseña los trabajos en la conversación y espera a todas las variantes @spec REQ-15, REQ-27, REQ-29', async () => {
    const { deps, byName } = tools();
    const out = await byName('generate_3d_model').run({ prompt: 'lantern', name: 'Linterna', variants: 2 });
    expect(deps.launch).toHaveBeenCalledWith('generate_3d_model', { prompt: 'lantern', name: 'Linterna', variants: 2 });
    expect(deps.showJobs).toHaveBeenCalledWith(['job-1', 'job-2']);
    expect(deps.waitJob).toHaveBeenCalledTimes(2);
    expect(out).toMatch(/linterna-1/);
  });

  it('riggear y animar pasan por la misma tarjeta de coste @spec REQ-16, REQ-33', async () => {
    const { deps, byName } = tools();
    await byName('rig_model').run({ id: 'linterna-1' });
    await byName('animate_model').run({ id: 'rig-1', animations: ['dance_01'] });
    expect(deps.prepare.mock.calls.map(([tool]) => tool)).toEqual(['rig_model', 'animate_model']);
    expect(deps.confirmCost).toHaveBeenCalledTimes(2);
  });

  it('un error al preparar (sin nombre, animación inexistente) vuelve al modelo sin enseñar coste @spec REQ-28', async () => {
    const { deps, byName } = tools({ prepare: vi.fn(async () => ({ ok: false, error: { message: '`name` is required' } })) });
    expect(await byName('generate_3d_model').run({ prompt: 'x' })).toMatch(/`name` is required/);
    expect(deps.confirmCost).not.toHaveBeenCalled();
  });

  it('sin clave de Tripo lo dice sin enseñar el coste @spec REQ-15, RNF-IA.4', async () => {
    const { deps, byName } = tools({ status: async () => ({ ok: true, value: { spent: 0, budget: 300, hasKey: { tripo: false } } }) });
    expect(await byName('generate_3d_model').run({ prompt: 'x', name: 'X' })).toMatch(/no Tripo API key/);
    expect(deps.confirmCost).not.toHaveBeenCalled();
  });

  it('el saldo y el catálogo no cuestan nada @spec REQ-33', async () => {
    const { deps, byName } = tools();
    expect(await byName('tripo_balance').run({})).toMatch(/2370 credits available \(30 reserved/);
    expect(await byName('tripo_operations').run({ operation: 'rig' })).toMatch(/rig/);
    expect(deps.confirmCost).not.toHaveBeenCalled();
  });

  it('enviar a Unreal desde la IA pide permiso @spec REQ-12, REQ-19', async () => {
    const send = vi.fn(async () => ({ ok: true, assetPath: '/Game/DBV/Generated/X.X', actor: 'X' }));
    const deny = tools({ send, askApproval: async () => 'deny' });
    expect(await deny.byName('library_send_to_unreal').run({ id: 'linterna-1' })).toMatch(/DENIED/);
    expect(send).not.toHaveBeenCalled();
    const allow = tools({ send, askApproval: async () => 'once' });
    expect(await allow.byName('library_send_to_unreal').run({ id: 'linterna-1' })).toMatch(/Imported as \/Game\/DBV\/Generated\/X\.X/);
    expect(await allow.byName('library_list').run({ filter: 'lantern' })).toMatch(/linterna-1/);
    expect(await allow.byName('library_send_to_unreal').run({ id: 'nope' })).toMatch(/no library asset/);
    expect(await allow.byName('library_rename').run({ id: 'linterna-1', name: 'Farol' })).toMatch(/"Farol"/);
  });
});
