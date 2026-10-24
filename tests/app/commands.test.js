// =============================================================================
// DBV Unreal AI — Los comandos que usa la interfaz existen en el backend
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Los demás tests simulan `invoke`, así que un comando sin registrar en `invoke_handler` pasaba toda la
// verificación y fallaba solo en la app real («Command generation_set_key not found»).

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = join(import.meta.dirname, '..', '..');

function sourceFiles(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? sourceFiles(path) : path.endsWith('.js') ? [path] : [];
  });
}

describe('comandos IPC', () => {
  /** @spec REQ-05, REQ-13 */
  it('todo comando que llama la interfaz está registrado en invoke_handler', () => {
    const used = new Set();
    for (const file of sourceFiles(join(ROOT, 'src'))) {
      for (const match of readFileSync(file, 'utf8').matchAll(/\b(?:call|invoke)\(\s*'([a-z_]+)'/g)) used.add(match[1]);
    }
    const lib = readFileSync(join(ROOT, 'src-tauri', 'src', 'lib.rs'), 'utf8');
    const handler = lib.slice(lib.indexOf('generate_handler!['));
    const registered = new Set([...handler.matchAll(/^\s+(?:[a-z_]+::)*([a-z_]+),$/gm)].map((match) => match[1]));

    expect(used.size).toBeGreaterThan(40);
    expect([...used].filter((name) => !registered.has(name))).toEqual([]);
  });
});
