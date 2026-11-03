// =============================================================================
// DBV Unreal AI — Tests de los cambios de código del proyecto en la conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { describe, expect, it, vi } from 'vitest';
import { applyChange, createProposal } from '../../src/ai/proposal.js';
import { createTools } from '../../src/ai/tools.js';
import { permissionRefusal } from '../../src/ai/aiApp.js';

const FILES = { 'Source/Demo/DemoActor.h': 'UCLASS()\nclass ADemoActor : public AActor\n{\n\tGENERATED_BODY()\n};\n' };
const readText = async (path) => FILES[path] ?? null;
// La regla real vive en Rust (`code_policy.rs`); aquí basta con que la conversación la consulte.
const checkPath = vi.fn(async (path) => (path.startsWith('Source/') ? null : `${path}: only Source/** (C++ and *.Build.cs), Content/Python/** (.py), Config/*.ini and the *.md files at the project root can be changed`));

describe('Cambios de código del proyecto', () => {
  it('una ruta que no deja la regla de la app no entra en la propuesta y la IA recibe el motivo @spec REQ-22', async () => {
    const proposal = createProposal();
    const refused = await applyChange(proposal, { path: 'Plugins/DBVUnrealAI/Content/Python/dbv_unreal_ai/editor.py', action: 'replace_all', content: 'x' }, readText, checkPath);
    expect(refused).toEqual({ ok: false, message: expect.stringContaining('only Source/**') });
    expect(proposal.files.size).toBe(0);

    const accepted = await applyChange(proposal, { path: 'Source/Demo/DemoActor.h', action: 'edit', search: '\tGENERATED_BODY()\n', replace: '\tGENERATED_BODY()\n\n\tUPROPERTY(EditAnywhere)\n\tfloat Speed = 600.f;\n' }, readText, checkPath);
    expect(accepted.ok).toBe(true);
    expect(proposal.files.get('Source/Demo/DemoActor.h').proposed).toContain('float Speed = 600.f;');
  });

  it('propose_changes consulta la regla de la app para cada fichero @spec REQ-22', async () => {
    const proposal = createProposal();
    const tools = createTools({ listFiles: async () => [], readText, search: async () => [], outline: null, checkProposal: async () => null, getProposal: () => proposal, checkPath });
    const propose = tools.find((tool) => tool.name === 'propose_changes');
    const report = await propose.run({
      summary: 'Velocidad',
      changes: [
        { path: 'Content/Maps/Level.umap', action: 'replace_all', content: 'x' },
        { path: 'Config/Windows/WindowsEngine.ini', action: 'replace_all', content: '[x]' },
        { path: 'Source/Demo/DemoActor.h', action: 'edit', search: '\tGENERATED_BODY()\n', replace: '\tGENERATED_BODY()\n\tfloat Speed;\n' },
      ],
    });
    expect(report).toContain('error: Content/Maps/Level.umap');
    expect(report).toContain('ok: edit Source/Demo/DemoActor.h');
    expect(report).toContain('error: Config/Windows/WindowsEngine.ini');
    expect(checkPath).toHaveBeenCalledWith('Config/Windows/WindowsEngine.ini');
    expect([...proposal.files.keys()]).toEqual(['Source/Demo/DemoActor.h']);
  });

  it('sin regla de la app (otras apps del starter) todo sigue igual @spec REQ-22', async () => {
    const proposal = createProposal();
    const applied = await applyChange(proposal, { path: 'notes/readme.md', action: 'create', content: '# Hola' }, readText);
    expect(applied.ok).toBe(true);
  });

  it('el permiso de un agente para tocar un fichero prohibido se rechaza con su motivo, y lo permitido se pregunta @spec REQ-22', async () => {
    const toRelative = (absolute) => absolute.replace('D:/Unreal/Demo/', '');
    const options = [{ optionId: 'ok', kind: 'allow_once' }, { optionId: 'no', kind: 'reject_once' }];
    const forbidden = { options, toolCall: { content: [{ type: 'diff', path: 'D:/Unreal/Demo/Plugins/DBVUnrealAI/Content/Python/dbv_unreal_ai/editor.py', oldText: 'a', newText: 'b' }] } };
    expect(await permissionRefusal(forbidden, toRelative, checkPath)).toEqual({ message: expect.stringContaining('Plugins/DBVUnrealAI'), optionId: 'no' });

    const allowed = { options, toolCall: { content: [{ type: 'diff', path: 'D:/Unreal/Demo/Source/Demo/DemoActor.h', oldText: 'a', newText: 'b' }] } };
    expect(await permissionRefusal(allowed, toRelative, checkPath)).toBeNull();
    expect(await permissionRefusal(forbidden, toRelative, null)).toBeNull();
  });
});
