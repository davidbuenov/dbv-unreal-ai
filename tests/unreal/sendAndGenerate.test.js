// =============================================================================
// DBV Unreal AI — Tests de «Enviar a Unreal» y de las herramientas de generación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { describe, expect, it, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { assetName, cutMismatch, GENERATED_FOLDER, listMetahumans, OUTFITS_FOLDER, sendToUnreal } from '../../src/unreal/sendToUnreal.js';
import { garmentKindOf } from '../../src/app/libraryView.js';
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

describe('enviar prendas a Unreal (REQ-81)', () => {
  it('una prenda se importa en la carpeta de MetaHumanOutfits, no en Generated @spec REQ-81', async () => {
    const b = backend();
    await sendToUnreal({ backend: b }, { ...ENTRY, parameters: { garment_slot: 'Torso' } }, { project: 'p', place: false });
    expect(b.calls[0].args.folder_path).toBe(OUTFITS_FOLDER);
  });

  it('lo que no es prenda sigue en Generated y se respeta una carpeta pedida @spec REQ-19', async () => {
    const plain = backend();
    await sendToUnreal({ backend: plain }, ENTRY, { project: 'p', place: false });
    expect(plain.calls[0].args.folder_path).toBe(GENERATED_FOLDER);
    const chosen = backend();
    await sendToUnreal({ backend: chosen }, { ...ENTRY, parameters: { garment_slot: 'Legs' } }, { project: 'p', place: false, folder: '/Game/Otra' });
    expect(chosen.calls[0].args.folder_path).toBe('/Game/Otra');
  });

  it('si el editor rechaza la importación (Play), devuelve el motivo y no lo anota como enviado @spec REQ-81', async () => {
    const b = backend({ importResult: { success: false, error: 'the editor is in Play mode' } });
    const out = await sendToUnreal({ backend: b }, { ...ENTRY, parameters: { garment_slot: 'Torso' } }, { project: 'p' });
    expect(out).toEqual({ ok: false, error: 'the editor is in Play mode' });
    expect(b.libraryRecordSent).not.toHaveBeenCalled();
    expect(b.calls).toHaveLength(1);
  });
});

describe('destino explícito del MetaHuman (REQ-83)', () => {
  const equipCalls = (b) => b.calls.filter((c) => c.tool === 'run_python' && String(c.args.code).includes('equip_metahuman_outfit'));
  const GARMENT = { ...ENTRY, parameters: { garment_slot: 'Torso' } };
  const withRunPython = (b, response) => {
    const inner = b.callTool;
    b.callTool = async (toolset, tool, args, timeout) => {
      let out;
      if (tool === 'run_python') {
        b.calls.push({ toolset, tool, args, timeout });
        out = { ok: true, value: { text: '', isError: false, value: { success: true, result: response } } };
      } else {
        out = await inner(toolset, tool, args, timeout);
      }
      return out;
    };
    return b;
  };

  it('sin destino la prenda solo se importa: no se equipa a nadie ni se coloca delante de la cámara @spec REQ-83', async () => {
    const b = backend();
    const out = await sendToUnreal({ backend: b }, GARMENT, { project: 'p' });
    expect(out).toMatchObject({ ok: true, note: 'noMetahumanTarget', actor: null });
    expect(out.equippedSlot).toBeUndefined();
    expect(equipCalls(b)).toHaveLength(0);
    expect(b.calls.some((c) => c.tool === 'place_asset')).toBe(false);
  });

  it('un destino en blanco cuenta como sin destino @spec REQ-83', async () => {
    const b = backend();
    const out = await sendToUnreal({ backend: b }, GARMENT, { project: 'p', metahuman: '   ' });
    expect(out.note).toBe('noMetahumanTarget');
    expect(equipCalls(b)).toHaveLength(0);
  });

  it('con destino equipa solo a ese personaje y lo dice @spec REQ-83', async () => {
    const b = withRunPython(backend(), { success: true, actor: 'BP_Dos', slot: 'Torso', rigged_asset: '/Game/DBV/MetaHumanOutfits/Chaqueta_Rigged' });
    const out = await sendToUnreal({ backend: b }, GARMENT, { project: 'p', metahuman: 'BP_Dos' });
    expect(out).toMatchObject({ ok: true, actor: 'BP_Dos', equippedSlot: 'Torso', kind: 'metahuman_garment' });
    const hex = Array.from(new TextEncoder().encode('BP_Dos'), (x) => x.toString(16).padStart(2, '0')).join('');
    expect(equipCalls(b)).toHaveLength(1);
    expect(equipCalls(b)[0].args.code).toContain(hex);
  });

  it('si el editor no puede equipar, el error llega tal cual y no se coloca la prenda suelta @spec REQ-83', async () => {
    const reason = "MetaHuman 'BP_X' not found in level. Available MetaHumans: ['BP_Uno']";
    const b = withRunPython(backend(), { success: false, error: reason });
    const out = await sendToUnreal({ backend: b }, GARMENT, { project: 'p', metahuman: 'BP_X' });
    expect(out).toEqual({ ok: false, error: reason });
    expect(b.calls.some((c) => c.tool === 'place_asset')).toBe(false);
    expect(b.libraryRecordSent).not.toHaveBeenCalled();
  });

  it('listMetahumans devuelve los personajes del nivel para el selector @spec REQ-83', async () => {
    const b = withRunPython(backend(), { count: 2, metahumans: [{ label: 'BP_Uno', build: 'f_med_nrw' }, { label: 'BP_Dos', build: 'm_tal_nrw' }] });
    const out = await listMetahumans(b);
    expect(out.ok).toBe(true);
    expect(out.metahumans.map((m) => m.label)).toEqual(['BP_Uno', 'BP_Dos']);
    expect(await listMetahumans(backend({ toolsets: [] }))).toEqual({ ok: false, error: 'toolsetMissing' });
  });

  it('el tipo de una prenda sale de garment_kind o, en entradas antiguas, de la ranura y la descripción @spec REQ-84', () => {
    expect(garmentKindOf({ parameters: { garment_kind: 'dress', garment_slot: 'Torso' } })).toBe('dress');
    expect(garmentKindOf({ parameters: { garment_slot: 'Torso' } })).toBe('top');
    expect(garmentKindOf({ parameters: { garment_slot: 'Feet' } })).toBe('shoes');
    expect(garmentKindOf({ parameters: { garment_slot: 'Legs' }, prompt: 'tartan kilt' })).toBe('skirt');
    expect(garmentKindOf({ parameters: { garment_slot: 'Legs' }, prompt: 'denim jeans', name: 'Vaqueros' })).toBe('pants');
    expect(garmentKindOf({ parameters: {} })).toBeNull();
    expect(garmentKindOf(null)).toBeNull();
  });

  it('avisa si el corte de la prenda no es el del cuerpo elegido @spec REQ-84', () => {
    const female = { parameters: { garment_cut: 'female' } };
    expect(cutMismatch(female, 'm_tal_nrw')).toEqual({ garment: 'female', body: 'male' });
    expect(cutMismatch(female, 'f_med_nrw')).toBeNull();
    expect(cutMismatch({ parameters: { garment_cut: 'neutral' } }, 'm_tal_nrw')).toBeNull();
    expect(cutMismatch({ parameters: {} }, 'm_tal_nrw')).toBeNull();
    expect(cutMismatch(female, '')).toBeNull();
  });

  it('al equipar con un corte que no encaja, el resultado lo dice y la IA lo cuenta @spec REQ-84', async () => {
    const b = withRunPython(backend(), { success: true, actor: 'BP_Dos', slot: 'Torso', build: 'm_tal_nrw', rigged_asset: '/Game/DBV/MetaHumanOutfits/X' });
    const entry = { ...ENTRY, parameters: { garment_slot: 'Torso', garment_kind: 'top', garment_cut: 'female' } };
    const out = await sendToUnreal({ backend: b }, entry, { project: 'p', metahuman: 'BP_Dos' });
    expect(out.cutMismatch).toEqual({ garment: 'female', body: 'male' });
    const tools = createGenerationTools({
      specs: [],
      listLibrary: async () => ({ ok: true, value: [{ ...entry, id: 'g-1' }] }),
      askApproval: async () => 'once',
      send: async () => out,
    });
    const text = await tools.find((t) => t.name === 'library_send_to_unreal').run({ id: 'g-1', metahuman: 'BP_Dos' });
    expect(text).toContain('WARNING: the garment has a female cut and the body is male');
  });

  it('library_send_to_unreal pasa el destino de la IA a la app y avisa si falta @spec REQ-83', async () => {
    const seen = [];
    const tools = createGenerationTools({
      specs: [],
      listLibrary: async () => ({ ok: true, value: [{ ...GARMENT, id: 'g-1' }] }),
      askApproval: async () => 'once',
      send: async (entry, options) => {
        seen.push(options);
        return options.metahuman ? { ok: true, assetPath: '/Game/X', actor: options.metahuman, equippedSlot: 'Torso' } : { ok: true, assetPath: '/Game/X', actor: null, note: 'noMetahumanTarget' };
      },
    });
    const send = tools.find((t) => t.name === 'library_send_to_unreal');
    expect(await send.run({ id: 'g-1' })).toContain('NO target was given');
    expect(await send.run({ id: 'g-1', metahuman: 'BP_Dos' })).toContain('equipped (Torso) on the MetaHuman "BP_Dos"');
    expect(seen).toEqual([{ metahuman: '' }, { metahuman: 'BP_Dos' }]);
  });

  it('ninguna ruta de equipado lleva nombres de personaje escritos en el código @spec REQ-83', () => {
    const files = ['src/unreal/sendToUnreal.js', 'src/unreal/unrealTools.js', 'src/generation/generationTools.js', 'src/app/libraryView.js', 'unreal-plugin/DBVUnrealAI/Content/Python/dbv_unreal_ai/metahuman.py'];
    for (const file of files) {
      const text = readFileSync(resolve(process.cwd(), file), 'utf8');
      expect(text, file).not.toMatch(/Victoria|Pablo|David3|candidates\[0\]|first MetaHuman/);
    }
    const rust = readFileSync(resolve(process.cwd(), 'src-tauri/src/mcp_server.rs'), 'utf8').split('#[cfg(test)]')[0];
    expect(rust).not.toMatch(/Victoria|Pablo|David3|first MetaHuman/);
  });
});

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

  it('un mundo con escala y suelo se coloca con el código fijo de Rust, no delante de la cámara @spec REQ-18', async () => {
    const worldEntry = { id: 'arid', name: 'Arid Desert', kind: 'world', files: ['w.spz', 'c.glb'], sentTo: [] };
    const b = backend();
    b.libraryWorldPlaceCode = vi.fn(async () => ({ ok: true, value: 'result = editor.place_world(...)' }));
    const run = b.callTool;
    b.callTool = async (toolset, tool, args, timeout) =>
      tool === 'run_python'
        ? (b.calls.push({ toolset, tool, args, timeout }), { ok: true, value: { text: '', isError: false, value: { success: true, result: { success: true, collider: 'Arid_Desert_Collider', metric_scale: 1.98, ground_lift_cm: 217.2 } } } })
        : run(toolset, tool, args, timeout);
    const out = await sendToUnreal({ backend: b }, worldEntry, { project: 'p' });
    expect(b.calls.map((c) => c.tool)).toEqual(['load_gaussian_splat', 'import_model', 'run_python']);
    expect(b.libraryWorldPlaceCode).toHaveBeenCalledWith('arid', 'Arid_Desert_Splat', '/Game/DBV/Generated/Linterna_magica/StaticMeshes/Linterna_magica.Linterna_magica', 'Arid_Desert');
    expect(out).toMatchObject({ ok: true, colliderActor: 'Arid_Desert_Collider', world: { ground_lift_cm: 217.2 } });
  });

  it('sin metadatos de mundo se coloca como antes @spec REQ-18', async () => {
    const b = backend();
    b.libraryWorldPlaceCode = vi.fn(async () => ({ ok: true, value: null }));
    await sendToUnreal({ backend: b }, { id: 'iglesia', name: 'Iglesia', kind: 'world', files: ['scene.ply', 'c.glb'], sentTo: [] }, { project: 'p' });
    expect(b.calls.map((c) => c.tool)).toEqual(['load_gaussian_splat', 'import_model', 'place_asset']);
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
  const specs = ['generate_3d_model', 'image_to_3d_model', 'rig_model', 'animate_model', 'tripo_run', 'tripo_operations', 'tripo_balance', 'generate_world', 'worldlabs_balance'].map((name) => ({ name, description: name, inputSchema: { type: 'object', properties: {}, required: [] } }));
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
      worldlabsBalance: vi.fn(async () => ({ ok: true, value: { balance: 4200 } })),
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

  /**
   * Regresión (2026-11-01): un mundo de World Labs pedía la clave de Tripo y se medía con el tope de Tripo.
   * @spec REQ-18, RNF-04
   */
  it('World Labs usa su clave, su gasto y su tope, no los de Tripo', async () => {
    const world = { operation: 'generate_world', endpoint: '/marble/v1/worlds:generate', name: 'Ciudad', variants: 1, params: { prompt: 'city' }, total: 1580, pricing: '…', service: 'worldlabs' };
    const both = async () => ({ ok: true, value: { spent: 120, budget: 300, spentByService: { tripo: 120, worldlabs: 0 }, budgetByService: { tripo: 300, worldlabs: 2000 }, hasKey: { tripo: false, worldlabs: true } } });
    const { deps, byName } = tools({ status: both, prepare: vi.fn(async () => ({ ok: true, value: world })), launch: vi.fn(async () => ({ ok: true, value: ['w-1'] })) });
    // Regresión (2026-11-01): generate_world no estaba en LAUNCHERS y el asistente integrado respondía «unknown tool».
    await byName('generate_world').run({ prompt: 'city', name: 'Ciudad' });
    expect(deps.prepare).toHaveBeenCalledWith('generate_world', { prompt: 'city', name: 'Ciudad' });
    expect(deps.confirmCost).toHaveBeenCalledWith(expect.objectContaining({ service: 'worldlabs', total: 1580, spent: 0, budget: 2000 }));
    expect(deps.launch).toHaveBeenCalled();

    const noKey = async () => ({ ok: true, value: { ...(await both()).value, hasKey: { tripo: true, worldlabs: false } } });
    const { byName: withoutKey } = tools({ status: noKey, prepare: vi.fn(async () => ({ ok: true, value: world })) });
    expect(await withoutKey('generate_world').run({ prompt: 'city', name: 'Ciudad' })).toBe('error: there is no World Labs API key. Tell the user to add it in Settings.');
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
    // World Labs es otra cuenta: su saldo, con su herramienta (REQ-18).
    expect(await byName('worldlabs_balance').run({})).toBe('World Labs balance: 4200 credits available.');
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
