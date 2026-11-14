// =============================================================================
// DBV Unreal AI — Tests de compatibilidad con Unreal 5.9
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Lo que se protege: con un editor 5.9 la app aprueba por la marca de cada herramienta (`accessMode`) sin empeorar
// 5.8, avisa si el proyecto está abierto con otra versión del motor, marca la Preview y le dice a la IA dónde están
// las herramientas que cambiaron. Las herramientas y el toolset vienen del fixture grabado en `PruebaUnreal59`.

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, vi } from 'vitest';
import { classifyTool } from '../../src/unreal/permissions.js';
import { createUnrealTools, ue59Prompt, unrealPrompt } from '../../src/unreal/unrealTools.js';
import { isPreview, majorMinor, shortVersion, versionMismatch } from '../../src/unreal/engineVersion.js';
import { checkLabel } from '../../src/app/projectView.js';
import { pillState } from '../../src/app/app.js';

/** `EditorAppToolset` tal como lo describe UE 5.9 (fixture real), con los nombres cortos y la marca de acceso. */
function editorAppToolset59() {
  const fixtures = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../fixtures/unreal-mcp/ue-5.9');
  const recorded = JSON.parse(readFileSync(path.join(fixtures, 'describe_toolsets.EditorToolset.EditorAppToolset.response.json'), 'utf8'));
  const info = JSON.parse(recorded.result.content[0].text).toolsets['EditorToolset.EditorAppToolset'];
  return {
    name: info.name,
    description: info.description,
    tools: info.tools.map((t) => ({ name: t.name.split('.').pop(), description: t.description, inputSchema: t.inputSchema, accessMode: t.annotations?.accessMode ?? null })),
  };
}

describe('aprobaciones con la marca de UE 5.9 (REQ-112, REQ-12)', () => {
  /** @spec REQ-112, REQ-12 */
  it('la marca manda cuando está; sin ella, la regla del verbo de siempre', () => {
    expect(classifyTool('LoadLevel', 'write')).toBe('modify');
    expect(classifyTool('GetActors', 'execute')).toBe('modify');
    expect(classifyTool('SetSomething', 'read')).toBe('read');
    expect(classifyTool('ReadTexture', 'READ')).toBe('read');
    expect(classifyTool('run_python', 'read')).toBe('modify');
    expect(classifyTool('read_file', 'read')).toBe('modify');
    // 5.8: sin marca, igual que en la v0.5.1.
    expect(classifyTool('GetActors')).toBe('read');
    expect(classifyTool('LoadLevel')).toBe('read');
    expect(classifyTool('frobnicate')).toBe('modify');
    expect(classifyTool('frobnicate', 'unknown-mode')).toBe('modify');
  });

  /** @spec REQ-112 */
  it('tras describir el toolset de 5.9, una herramienta marcada «write» pide permiso aunque su verbo sea de consulta', async () => {
    const info = editorAppToolset59();
    const writeWithReadVerb = info.tools.find((t) => t.accessMode === 'write' && classifyTool(t.name) === 'read');
    const readWithWriteVerb = info.tools.find((t) => t.accessMode === 'read' && classifyTool(t.name) === 'modify');
    const backend = {
      listToolsets: vi.fn(async () => ({ ok: true, value: [{ name: 'EditorToolset.EditorAppToolset', description: 'editor' }] })),
      describeToolset: vi.fn(async () => ({ ok: true, value: info })),
      callTool: vi.fn(async () => ({ ok: true, value: { text: 'ok', isError: false, value: { ok: true } } })),
    };
    const askApproval = vi.fn(async () => 'deny');
    const tools = createUnrealTools({ backend, askApproval });
    const describeOut = await tools.find((t) => t.name === 'unreal_describe_toolset').run({ toolset: 'EditorAppToolset' });
    const readOnlyLine = describeOut.split('\n').find((l) => l.startsWith('Read-only'));
    for (const t of info.tools.filter((x) => x.accessMode === 'read')) expect(readOnlyLine).toContain(t.name);
    for (const t of info.tools.filter((x) => x.accessMode !== 'read')) expect(readOnlyLine.split(/[:,.]\s*/)).not.toContain(t.name);

    const call = tools.find((t) => t.name === 'unreal_call_tool');
    if (writeWithReadVerb) {
      expect(await call.run({ toolset: 'EditorAppToolset', tool: writeWithReadVerb.name, arguments: {} })).toMatch(/DENIED/);
      expect(askApproval).toHaveBeenCalledTimes(1);
    }
    if (readWithWriteVerb) {
      askApproval.mockClear();
      await call.run({ toolset: 'EditorAppToolset', tool: readWithWriteVerb.name, arguments: {} });
      expect(askApproval).not.toHaveBeenCalled();
    }
    expect(writeWithReadVerb || readWithWriteVerb, 'el fixture tiene al menos un caso en que la marca y el verbo no coinciden').toBeTruthy();
  });
});

describe('versión del editor conectado y Preview (REQ-109)', () => {
  /** @spec REQ-109 */
  it('lee la versión, marca la Preview y detecta un editor de otra versión que el proyecto', () => {
    expect(majorMinor('5.9.0-60351404')).toBe('5.9');
    expect(shortVersion('5.9.0-60351404')).toBe('5.9.0');
    expect(majorMinor('')).toBeNull();
    expect(isPreview('5.9.0')).toBe(true);
    expect(isPreview('5.8.4')).toBe(false);
    expect(versionMismatch('5.8', '5.9.0-60351404')).toEqual({ project: '5.8', editor: '5.9' });
    expect(versionMismatch('5.9', '5.9.0-60351404')).toBeNull();
    expect(versionMismatch('5.8', '')).toBeNull();
  });

  /** @spec REQ-109 */
  it('la píldora de conexión dice Preview con la 5.9 y no con la 5.8', () => {
    const online = (engineVersion) => pillState({ status: 'online', projectName: 'Demo', engineVersion, endpoint: { port: 8000 } }).text;
    expect(online('5.9.0')).toContain('Preview');
    expect(online('5.8.4')).not.toContain('Preview');
  });

  /** @spec REQ-110 */
  it('«Preparar» explica para qué es AllToolsets', () => {
    const { why } = checkLabel({ id: 'plugin:AllToolsets', ok: false, detail: '' });
    expect(why).toMatch(/5\.9/);
  });
});

describe('prompt según la versión (REQ-112)', () => {
  /** @spec REQ-112 */
  it('solo con 5.9 le dice a la IA dónde están la captura, el PIE y los toolsets nativos', () => {
    for (const version of ['5.9.0', '5.10.1', '6.0.0']) expect(ue59Prompt(version)).toContain('SceneCaptureToolset.CaptureEditorViewport');
    expect(ue59Prompt('5.9.0')).toContain('Preview');
    for (const version of ['5.8.4', '', '5.8']) expect(ue59Prompt(version)).toBe('');
    expect(unrealPrompt({ projectName: 'P', engineVersion: '5.9.0' })).toContain('StartPIE');
    expect(unrealPrompt({ projectName: 'P', engineVersion: '5.8.4' })).not.toContain('StartPIE');
  });
});
