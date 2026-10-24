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

function backend({ toolsets = [{ name: DBV, description: '' }], importResult } = {}) {
  const calls = [];
  return {
    calls,
    listToolsets: async () => ({ ok: true, value: toolsets }),
    libraryFilePath: async (id, file) => ({ ok: true, value: `C:/data/library/${id}/${file}` }),
    libraryRecordSent: vi.fn(async () => ({ ok: true })),
    callTool: async (toolset, tool, args, timeout) => {
      calls.push({ toolset, tool, args, timeout });
      if (tool === 'import_model') return { ok: true, value: { text: '', isError: false, value: importResult ?? { success: true, imported: ['/Game/DBV/Generated/Linterna_magica/StaticMeshes/Linterna_magica.Linterna_magica'], primary: '/Game/DBV/Generated/Linterna_magica/StaticMeshes/Linterna_magica.Linterna_magica' } } };
      return { ok: true, value: { text: '', isError: false, value: { success: true, actor: 'Linterna_magica' } } };
    },
  };
}

describe('enviar a Unreal (REQ-19)', () => {
  it('importa el GLB en /Game/DBV/Generated, lo coloca y lo anota en la biblioteca @spec REQ-19, REQ-17', async () => {
    const b = backend();
    const out = await sendToUnreal({ backend: b }, ENTRY, { project: 'D:/P/P.uproject' });
    expect(out).toEqual({ ok: true, assetPath: '/Game/DBV/Generated/Linterna_magica/StaticMeshes/Linterna_magica.Linterna_magica', actor: 'Linterna_magica' });
    expect(b.calls[0]).toMatchObject({ toolset: DBV, tool: 'import_model', args: { source_file: 'C:/data/library/linterna-1/model.glb', folder_path: GENERATED_FOLDER, asset_name: 'Linterna_magica' }, timeout: 300 });
    expect(b.calls[1]).toMatchObject({ tool: 'place_asset', args: { folder: 'DBV/Generated', distance: 300 } });
    expect(b.libraryRecordSent).toHaveBeenCalledWith('linterna-1', 'D:/P/P.uproject', out.assetPath);
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

describe('herramientas de generación (REQ-15, RNF-04)', () => {
  const status = async () => ({ ok: true, value: { spent: 0, budget: 300, hasKey: { tripo: true }, tripoTextToModel: 30 } });

  it('sin aceptar el coste no se genera nada @spec REQ-15, RNF-04', async () => {
    const startTripo = vi.fn();
    const confirmCost = vi.fn(async () => false);
    const [generate] = createGenerationTools({ status, startTripo, confirmCost, waitJob: vi.fn(), listLibrary: vi.fn(), send: vi.fn(), askApproval: vi.fn() });
    expect(await generate.run({ prompt: 'cofre' })).toMatch(/did NOT accept/);
    expect(confirmCost).toHaveBeenCalledWith(expect.objectContaining({ provider: 'tripo', cost: 30, budget: 300 }));
    expect(startTripo).not.toHaveBeenCalled();
  });

  it('aceptado, lanza con confirmCost y espera al trabajo @spec REQ-15', async () => {
    const startTripo = vi.fn(async () => ({ ok: true, value: 'job-1' }));
    const waitJob = vi.fn(async () => ({ stage: 'done', entry: ENTRY }));
    const [generate] = createGenerationTools({ status, startTripo, waitJob, confirmCost: async () => true, listLibrary: vi.fn(), send: vi.fn(), askApproval: vi.fn() });
    const out = await generate.run({ prompt: 'lantern', name: 'Linterna', face_limit: 3000 });
    expect(startTripo).toHaveBeenCalledWith({ prompt: 'lantern', name: 'Linterna', faceLimit: 3000, smartLowPoly: undefined, confirmCost: true });
    expect(out).toMatch(/linterna-1/);
  });

  it('sin clave de Tripo lo dice sin enseñar el coste @spec REQ-15, RNF-IA.4', async () => {
    const confirmCost = vi.fn();
    const noKey = async () => ({ ok: true, value: { spent: 0, budget: 300, hasKey: { tripo: false }, tripoTextToModel: 30 } });
    const [generate] = createGenerationTools({ status: noKey, startTripo: vi.fn(), waitJob: vi.fn(), confirmCost, listLibrary: vi.fn(), send: vi.fn(), askApproval: vi.fn() });
    expect(await generate.run({ prompt: 'x' })).toMatch(/no Tripo API key/);
    expect(confirmCost).not.toHaveBeenCalled();
  });

  it('enviar a Unreal desde la IA pide permiso @spec REQ-12, REQ-19', async () => {
    const send = vi.fn(async () => ({ ok: true, assetPath: '/Game/DBV/Generated/X.X', actor: 'X' }));
    const listLibrary = async () => ({ ok: true, value: [ENTRY] });
    const denyTools = createGenerationTools({ status, startTripo: vi.fn(), waitJob: vi.fn(), confirmCost: vi.fn(), listLibrary, send, askApproval: async () => 'deny' });
    expect(await denyTools[2].run({ id: 'linterna-1' })).toMatch(/DENIED/);
    expect(send).not.toHaveBeenCalled();
    const allowTools = createGenerationTools({ status, startTripo: vi.fn(), waitJob: vi.fn(), confirmCost: vi.fn(), listLibrary, send, askApproval: async () => 'once' });
    expect(await allowTools[2].run({ id: 'linterna-1' })).toMatch(/Imported as \/Game\/DBV\/Generated\/X\.X/);
    expect(await allowTools[1].run({ filter: 'lantern' })).toMatch(/linterna-1/);
    expect(await allowTools[2].run({ id: 'nope' })).toMatch(/no library asset/);
  });
});
