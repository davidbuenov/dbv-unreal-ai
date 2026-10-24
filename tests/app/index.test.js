// =============================================================================
// DBV Unreal AI — Tests de arranque del HTML principal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const src = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..', 'src');

describe('index.html (RNF-01)', () => {
  /**
   * Tauri decide el tipo de contenido mirando los PRIMEROS BYTES del fichero. Un BOM UTF-8 o un comentario
   * antes del `<!doctype html>` hacen que lo trate como un fichero cualquiera: el WebView lo DESCARGA en vez de
   * pintarlo y la ventana se queda en negro (ocurrió en `tauri dev`, y habría ocurrido en el instalador).
   * Los bancos con Edge no lo ven porque asignan el tipo por la extensión.
   * @spec RNF-01
   */
  it('empieza por el doctype, sin BOM ni nada delante, para que Tauri lo reconozca como HTML', () => {
    const bytes = readFileSync(path.join(src, 'index.html'));
    expect([...bytes.subarray(0, 3)]).not.toEqual([0xef, 0xbb, 0xbf]);
    expect(bytes.subarray(0, 15).toString('utf8').toLowerCase()).toBe('<!doctype html>');
  });

  it('el doctype va seguido directamente de la etiqueta html @spec RNF-01', () => {
    const text = readFileSync(path.join(src, 'index.html'), 'utf8');
    expect(text).toMatch(/^<!doctype html>\s*<html\b/i);
  });
});
