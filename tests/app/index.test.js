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

describe('barra superior', () => {
  const html = readFileSync(path.join(src, 'index.html'), 'utf8');
  const css = readFileSync(path.join(src, 'styles.css'), 'utf8');
  const root = path.join(src, '..');

  /** Los tres temas y el idioma se eligen directamente; la chincheta mantiene la ventana encima. */
  it('ofrece Claro/Oscuro/Sepia, ES/EN y la chincheta de «siempre encima» @spec REQ-01', () => {
    for (const theme of ['light', 'dark', 'sepia']) expect(html).toContain(`data-theme-choice="${theme}"`);
    for (const lang of ['es', 'en']) expect(html).toContain(`data-lang-choice="${lang}"`);
    expect(html).toContain('id="btn-pin"');
    expect(css).toContain(":root[data-theme='sepia']");
  });

  it('la ventana tiene permiso para fijarse encima, o el botón fallaría en silencio', () => {
    const capability = readFileSync(path.join(root, 'src-tauri', 'capabilities', 'default.json'), 'utf8');
    expect(capability).toContain('core:window:allow-set-always-on-top');
  });

  it('los textos del tema y de la chincheta existen en español e inglés', () => {
    const dictionary = readFileSync(path.join(src, 'i18n.js'), 'utf8');
    for (const key of ['theme.light', 'theme.dark', 'theme.sepia', 'action.alwaysOnTop', 'action.alwaysOnTopActive', 'action.alwaysOnTopFailed']) {
      expect(dictionary.split(`"${key}"`).length - 1, key).toBe(2);
    }
  });
});
