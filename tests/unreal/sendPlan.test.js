// =============================================================================
// DBV Unreal AI — Tests: la ventana y el MCP propio envían lo mismo a Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// `sendToUnreal.js` (ventana) y `library::send_plan` (Rust, MCP propio) eligen el mismo fichero con los casos comunes
// de `tests/fixtures/send-plan/cases.json` (D25).

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { sendToUnreal } from '../../src/unreal/sendToUnreal.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const { cases } = JSON.parse(readFileSync(path.join(root, 'tests/fixtures/send-plan/cases.json'), 'utf8'));

/** Editor simulado que anota cada llamada y responde que todo fue bien. */
function backend() {
  const calls = [];
  return {
    calls,
    listToolsets: async () => ({ ok: true, value: [{ name: 'DBVUnrealAICpp.DBVUnrealAICppToolset' }] }),
    libraryFilePath: async (_id, file) => ({ ok: true, value: `/lib/${file}` }),
    libraryRecordSent: async () => ({ ok: true }),
    callTool: async (_toolset, tool, args) => {
      calls.push({ tool, args });
      return { ok: true, value: { text: '', isError: false, value: { success: true, primary: '/Game/DBV/Generated/X', actor: 'X', kind: 'static', animations: [] } } };
    },
  };
}

describe('Qué se envía a Unreal (REQ-54, D25)', () => {
  for (const testCase of cases) {
    it(`${testCase.name}: lo mismo que el MCP propio @spec REQ-54`, async () => {
      const fake = backend();
      const result = await sendToUnreal({ backend: fake }, { id: 'x', name: 'Isla', kind: testCase.kind, files: testCase.files }, { project: 'D:/P/Demo' });
      const { plan, file, collider } = testCase.expected;
      if (plan === 'nothing') {
        expect(result).toEqual({ ok: false, error: 'noModelFile' });
      } else if (plan === 'sound') {
        expect(fake.calls[0]).toMatchObject({ tool: 'import_sound', args: { source_file: `/lib/${file}`, folder_path: '/Game/DBV/Audio' } });
      } else if (plan === 'splat') {
        expect(fake.calls[0]).toMatchObject({ tool: 'load_gaussian_splat', args: { source_file: `/lib/${file}` } });
        const imported = fake.calls.find((c) => c.tool === 'import_model');
        expect(imported?.args.source_file ?? null).toBe(collider ? `/lib/${collider}` : null);
      } else {
        expect(fake.calls[0]).toMatchObject({ tool: 'import_model', args: { source_file: `/lib/${file}` } });
        expect(fake.calls.some((c) => c.tool === 'load_gaussian_splat')).toBe(false);
      }
    });
  }
});
