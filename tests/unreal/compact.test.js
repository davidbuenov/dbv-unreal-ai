// =============================================================================
// DBV Unreal AI — Tests: lo que se devuelve a la IA, acotado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// El asistente integrado y el MCP propio (Rust, `unreal/compact.rs`) deben devolver EXACTAMENTE el mismo texto
// compacto de un toolset (D22): los dos se comparan con los ficheros de referencia de `tests/fixtures/compact/`,
// hechos con las respuestas del editor real grabadas el 2026-10-31.

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { clip, compactToolset, firstSentence, LIST_SUMMARY_LIMIT, LIST_SUMMARY_MIN, listSummaryLimit, paramsLine, RESULT_LIMIT, toolDetail, toolsetList } from '../../src/unreal/unrealTools.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const fixtures = path.join(root, 'tests/fixtures');

/** `describe_toolset` grabado, convertido como lo hace `parse_toolset_info` de Rust (nombres cortos de herramienta). */
function recorded(toolset) {
  const response = JSON.parse(readFileSync(path.join(fixtures, `unreal-mcp/ue-5.8.4/describe_toolset.${toolset}.response.json`), 'utf8'));
  const raw = JSON.parse(response.result.content[0].text);
  return { name: raw.name ?? '', description: raw.description ?? '', tools: (raw.tools ?? []).map((t) => ({ name: String(t.name).split('.').pop(), description: t.description ?? '', inputSchema: t.inputSchema ?? null })) };
}

describe('Formato compacto de un toolset', () => {
  for (const toolset of ['ConvaiToolset.ConvaiSetupToolset', 'ConvaiToolset.ConvaiActionToolset', 'EditorToolset.EditorAppToolset', 'NiagaraToolsets.NiagaraToolset_System']) {
    it(`${toolset} coincide con la referencia común de Rust @spec REQ-51`, () => {
      const expected = readFileSync(path.join(fixtures, `compact/${toolset}.txt`), 'utf8').replace(/\r\n/g, '\n').replace(/\n+$/, '');
      expect(compactToolset(recorded(toolset))).toBe(expected);
    });
  }

  it('Niagara (306 KB en la v0.2.4) cabe en el tope y su herramienta se pide aparte @spec REQ-51', () => {
    const niagara = recorded('NiagaraToolsets.NiagaraToolset_System');
    expect(readFileSync(path.join(fixtures, 'unreal-mcp/ue-5.8.4/describe_toolset.NiagaraToolsets.NiagaraToolset_System.response.json'), 'utf8').length).toBeGreaterThan(200_000);
    expect(compactToolset(niagara).length).toBeLessThan(RESULT_LIMIT);
    expect(toolDetail(niagara, niagara.tools[0].name.toLowerCase())).toContain('input: {');
    expect(toolDetail(niagara, 'NoSuchTool')).toContain('its tools are');
  });

  it('recorta diciendo cómo pedir menos; frases y parámetros como en Rust @spec REQ-51', () => {
    expect(clip('hola')).toBe('hola');
    const clipped = clip('é'.repeat(RESULT_LIMIT + 5));
    expect(clipped).toContain(`truncated: ${RESULT_LIMIT + 5} characters; ask for less`);
    expect(firstSentence('Wires a\nMetaHuman up.\n\nAll paths are object paths.')).toBe('Wires a MetaHuman up.');
    expect(paramsLine({ type: 'object', properties: { a: { type: 'string' }, b: { type: 'array', items: { type: 'number' } }, c: { type: 'string', enum: ['x'] } }, required: ['a'] })).toBe('a*: string, b: array<number>, c: enum');
  });
});

/** `list_toolsets` grabado, interpretado como `parse_toolset_list` de Rust (`- Nombre: descripción`, en varias líneas). */
function recordedToolsets(text) {
  const toolsets = [];
  for (const line of text.split('\n')) {
    if (line.startsWith('- ')) {
      const rest = line.slice(2);
      const colon = rest.indexOf(':');
      toolsets.push({ name: (colon >= 0 ? rest.slice(0, colon) : rest).trim(), description: (colon >= 0 ? rest.slice(colon + 1) : '').trim() });
    } else if (toolsets.length && line.trim()) {
      const last = toolsets[toolsets.length - 1];
      last.description = last.description ? `${last.description}\n${line.trimEnd()}` : line.trimEnd();
    }
  }
  return toolsets;
}

describe('Lista de toolsets de UE 5.9', () => {
  /** Los 139 toolsets de `PruebaUnreal59`, con la misma referencia que Rust y dentro del tope. @spec REQ-51, REQ-112, REQ-116 */
  it('coincide con la referencia común de Rust y cabe en el tope', () => {
    const response = JSON.parse(readFileSync(path.join(fixtures, 'unreal-mcp/ue-5.9/list_toolsets.response.json'), 'utf8'));
    const toolsets = recordedToolsets(response.result.content[0].text);
    expect(toolsets).toHaveLength(139);
    const list = toolsetList(toolsets);
    expect(Array.from(list).length).toBeLessThanOrEqual(RESULT_LIMIT);
    const expected = readFileSync(path.join(fixtures, 'compact/list_toolsets.ue-5.9.txt'), 'utf8').replace(/\r\n/g, '\n').replace(/\n+$/, '');
    expect(list).toBe(expected);
  });

  /** Con más toolsets de los que caben a 40 caracteres, las descripciones se acortan igual que en Rust. @spec REQ-51, REQ-113 */
  it('con muchos toolsets acorta las descripciones y la lista cabe sin recortar', () => {
    const toolsets = Array.from({ length: 160 }, (_, i) => ({ name: `SequencerScriptingEditor.SomeLongToolsetName${String(i).padStart(3, '0')}`, description: 'This is a set of helper functions to access various parts of the API.' }));
    const limit = listSummaryLimit(toolsets);
    expect(limit).toBeLessThan(LIST_SUMMARY_LIMIT);
    expect(limit).toBeGreaterThanOrEqual(LIST_SUMMARY_MIN);
    const list = toolsetList(toolsets);
    expect(Array.from(list).length).toBeLessThanOrEqual(RESULT_LIMIT);
    expect(list).not.toContain('(truncated');
    expect(list).toContain('SomeLongToolsetName159');
    expect(listSummaryLimit(toolsets.slice(0, 10))).toBe(LIST_SUMMARY_LIMIT);
  });
});
