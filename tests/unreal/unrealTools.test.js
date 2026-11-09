// =============================================================================
// DBV Unreal AI — Tests de las herramientas de Unreal y los permisos
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { describe, expect, it, vi } from 'vitest';
import { classifyTool, createPermissionMemory, firstWord } from '../../src/unreal/permissions.js';
import { createUnrealTools, describeCapture, formatResult, isDbvToolset, pythonNudge, resolveToolset, unrealPrompt } from '../../src/unreal/unrealTools.js';
import { sendToUnreal } from '../../src/unreal/sendToUnreal.js';
import { formArgs } from '../../src/app/generateView.js';

const TOOLSETS = [
  { name: 'ToolsetRegistry.AgentSkillToolset', description: 'skills' },
  { name: 'EditorToolset.EditorAppToolset', description: 'editor' },
  { name: 'editor_toolset.toolsets.scene.SceneTools', description: 'scene\nmore' },
  { name: 'editor_toolset.toolsets.static_mesh.StaticMeshTools', description: 'meshes' },
  { name: 'PythonTypes.DBVUnrealAIToolset_0x3D7D6EDD', description: 'DBV Unreal AI' },
];

/** Backend simulado: registra las llamadas y responde con `responses[tool]`. */
function fakeBackend({ toolsets = TOOLSETS, responses = {}, described = {} } = {}) {
  const calls = [];
  return {
    calls,
    listToolsets: vi.fn(async () => ({ ok: true, value: toolsets })),
    describeToolset: vi.fn(async (name) => ({ ok: true, value: described[name] ?? { name, tools: [{ name: 'find_actors', description: 'find', inputSchema: { type: 'object' } }, { name: 'add_to_scene_from_asset', description: 'add', inputSchema: { type: 'object' } }] } })),
    callTool: vi.fn(async (toolset, tool, args, timeout) => {
      calls.push({ toolset, tool, args, timeout });
      const response = typeof responses[tool] === 'function' ? responses[tool](args) : responses[tool];
      return { ok: true, value: response ?? { text: 'ok', isError: false, value: { ok: true } } };
    }),
  };
}

const tool = (tools, name) => tools.find((t) => t.name === name);

describe('permisos (REQ-12)', () => {
  it('las consultas son de lectura y lo desconocido pide permiso @spec REQ-12', () => {
    for (const name of ['find_actors', 'GetActors', 'list_folders', 'CaptureViewport', 'IsPIERunning', 'describe_python_api', 'validate_python', 'trace_world', 'load_asset']) {
      expect(classifyTool(name), name).toBe('read');
    }
    for (const name of ['add_to_scene_from_asset', 'delete', 'import_file', 'SetCameraTransform', 'compile_blueprint', 'frobnicate', '']) {
      expect(classifyTool(name), name).toBe('modify');
    }
  });

  it('ejecutar código o leer ficheros del disco siempre pide permiso @spec REQ-12, RNF-IA.3', () => {
    for (const name of ['run_python', 'RunPython', 'execute_tool_script', 'read_file', 'write_file']) expect(classifyTool(name)).toBe('modify');
    expect(firstWord('GetSelectedActors')).toBe('get');
    expect(firstWord('editor_toolset.toolsets.scene.SceneTools.find_actors')).toBe('editor');
  });

  it('«siempre en esta conversación» se recuerda por toolset y herramienta @spec REQ-12', () => {
    const memory = createPermissionMemory();
    memory.allow('A', 'Delete');
    expect(memory.isAllowed('A', 'delete')).toBe(true);
    expect(memory.isAllowed('B', 'delete')).toBe(false);
    memory.clear();
    expect(memory.isAllowed('A', 'delete')).toBe(false);
  });
});

describe('herramientas de Unreal (REQ-10)', () => {
  it('resuelve toolsets por el final del nombre y sugiere si hay dudas @spec REQ-10', () => {
    expect(resolveToolset('SceneTools', TOOLSETS).name).toBe('editor_toolset.toolsets.scene.SceneTools');
    expect(resolveToolset('editortoolset.editorapptoolset', TOOLSETS).name).toBe('EditorToolset.EditorAppToolset');
    const vague = resolveToolset('tools', TOOLSETS);
    expect(vague.name).toBeNull();
    expect(vague.candidates.length).toBeGreaterThan(1);
    expect(isDbvToolset('PythonTypes.DBVUnrealAIToolset_0x3D7D6EDD')).toBe(true);
    expect(isDbvToolset('DBVMCPFirst.MyCustomSceneToolset')).toBe(false);
  });

  it('una herramienta de lectura se ejecuta sin preguntar y queda en el registro @spec REQ-10, REQ-14', async () => {
    const backend = fakeBackend({ responses: { find_actors: { text: '[]', isError: false, value: [{ label: 'Cube' }] } } });
    const askApproval = vi.fn();
    const onAction = vi.fn();
    const tools = createUnrealTools({ backend, askApproval, onAction });
    const out = await tool(tools, 'unreal_call_tool').run({ toolset: 'SceneTools', tool: 'find_actors', arguments: { name: 'Cube' } });
    expect(out).toBe('[{"label":"Cube"}]');
    expect(askApproval).not.toHaveBeenCalled();
    expect(onAction).toHaveBeenCalledWith(expect.objectContaining({ tool: 'find_actors', kind: 'read', outcome: 'ok' }));
  });

  it('una modificación pide permiso; denegada no se ejecuta @spec REQ-12', async () => {
    const backend = fakeBackend();
    const askApproval = vi.fn(async () => 'deny');
    const onAction = vi.fn();
    const tools = createUnrealTools({ backend, askApproval, onAction });
    const out = await tool(tools, 'unreal_call_tool').run({ toolset: 'SceneTools', tool: 'add_to_scene_from_asset', arguments: { asset_path: '/Game/DBV/X' }, purpose: 'Poner X' });
    expect(out).toMatch(/DENIED/);
    expect(backend.calls).toEqual([]);
    expect(askApproval).toHaveBeenCalledWith(expect.objectContaining({ kind: 'tool', tool: 'add_to_scene_from_asset', purpose: 'Poner X' }));
    expect(onAction).toHaveBeenCalledWith(expect.objectContaining({ outcome: 'denied' }));
  });

  it('«siempre» no vuelve a preguntar en la misma conversación @spec REQ-12', async () => {
    const backend = fakeBackend();
    const askApproval = vi.fn(async () => 'always');
    const tools = createUnrealTools({ backend, askApproval });
    const call = tool(tools, 'unreal_call_tool');
    await call.run({ toolset: 'SceneTools', tool: 'add_to_scene_from_asset', arguments: {} });
    await call.run({ toolset: 'SceneTools', tool: 'add_to_scene_from_asset', arguments: {} });
    expect(askApproval).toHaveBeenCalledTimes(1);
    expect(backend.calls).toHaveLength(2);
  });

  it('un toolset inventado vuelve al modelo con sugerencias, sin llamar al editor @spec REQ-10', async () => {
    const backend = fakeBackend();
    const tools = createUnrealTools({ backend, askApproval: vi.fn() });
    const out = await tool(tools, 'unreal_call_tool').run({ toolset: 'StaticMesh', tool: 'import_file' });
    expect(out).toMatch(/unknown toolset/);
    expect(out).toMatch(/StaticMeshTools/);
    expect(backend.calls).toEqual([]);
  });

  it('el Python no se cuela por unreal_call_tool @spec REQ-12, REQ-26', async () => {
    const backend = fakeBackend();
    const tools = createUnrealTools({ backend, askApproval: vi.fn(async () => 'once') });
    expect(await tool(tools, 'unreal_call_tool').run({ toolset: 'DBVUnrealAIToolset_0x3D7D6EDD', tool: 'run_python', arguments: { code: 'x' } })).toMatch(/unreal_run_python/);
    expect(backend.calls).toEqual([]);
  });

  it('describe el toolset en formato compacto y dice qué herramientas piden permiso @spec REQ-10, REQ-51', async () => {
    const tools = createUnrealTools({ backend: fakeBackend(), askApproval: vi.fn() });
    const out = await tool(tools, 'unreal_describe_toolset').run({ toolset: 'SceneTools' });
    expect(out).toMatch(/^- find_actors\(/m);
    expect(out).toContain('Read-only (run without asking): find_actors.');
    expect(out).not.toContain('add_to_scene_from_asset, ');
    const detail = await tool(tools, 'unreal_describe_toolset').run({ toolset: 'SceneTools', tool: 'add_to_scene_from_asset' });
    expect(detail).toMatch(/## add_to_scene_from_asset\n[\s\S]*input: /);
  });
});

describe('Python con validación previa (REQ-26)', () => {
  const unknown = { ok: false, syntax_error: null, findings: [{ kind: 'unknown', line: 2, name: 'unreal.EditorAsetLibrary', message: 'does not exist', suggestions: ['EditorAssetLibrary'] }] };
  const deprecated = { ok: true, syntax_error: null, findings: [{ kind: 'deprecated', line: 1, name: 'unreal.EditorLevelLibrary.get_editor_world', message: 'deprecated: use Unreal Editor Subsystem', suggestions: [] }] };

  it('un nombre inexistente vuelve al modelo SIN preguntar al usuario ni ejecutar @spec REQ-26', async () => {
    const backend = fakeBackend({ responses: { validate_python: { text: '', isError: false, value: unknown } } });
    const askApproval = vi.fn();
    const tools = createUnrealTools({ backend, askApproval });
    const out = await tool(tools, 'unreal_run_python').run({ code: 'import unreal\nunreal.EditorAsetLibrary.x()' });
    expect(out).toMatch(/NOT run/);
    expect(out).toMatch(/did you mean: EditorAssetLibrary/);
    expect(askApproval).not.toHaveBeenCalled();
    expect(backend.calls.map((c) => c.tool)).toEqual(['validate_python']);
  });

  it('tras dos correcciones fallidas se enseña al usuario con el aviso @spec REQ-26', async () => {
    const backend = fakeBackend({ responses: { validate_python: { text: '', isError: false, value: unknown }, run_python: { text: '', isError: false, value: { success: false, error: 'AttributeError' } } } });
    const askApproval = vi.fn(async () => 'once');
    const run = tool(createUnrealTools({ backend, askApproval }), 'unreal_run_python');
    await run.run({ code: 'a' });
    await run.run({ code: 'a' });
    const third = await run.run({ code: 'a' });
    expect(askApproval).toHaveBeenCalledTimes(1);
    expect(askApproval.mock.calls[0][0]).toMatchObject({ kind: 'python', code: 'a', findings: unknown, validated: true });
    expect(third).toMatch(/AttributeError/);
  });

  it('un aviso de obsoleto no bloquea: se pide permiso, se ejecuta con tiempo largo y se informa @spec REQ-26, REQ-06', async () => {
    const backend = fakeBackend({ responses: { validate_python: { text: '', isError: false, value: deprecated }, run_python: { text: '', isError: false, value: { success: true, stdout: 'hi\n' } } } });
    const askApproval = vi.fn(async () => 'once');
    const out = await tool(createUnrealTools({ backend, askApproval }), 'unreal_run_python').run({ code: 'print("hi")', purpose: 'Saludar' });
    expect(out).toMatch(/"stdout":"hi\\n"/);
    expect(out).toMatch(/deprecated/);
    const run = backend.calls.find((c) => c.tool === 'run_python');
    expect(run.timeout).toBe(180);
    expect(askApproval).toHaveBeenCalledWith(expect.objectContaining({ purpose: 'Saludar', toolset: 'PythonTypes.DBVUnrealAIToolset_0x3D7D6EDD' }));
  });

  it('sin el toolset DBV usa el RunPython del proyecto (DBVMCPFirst) sin validar @spec REQ-26', async () => {
    const toolsets = [{ name: 'EditorToolset.EditorAppToolset', description: '' }, { name: 'DBVMCPFirst.MyCustomSceneToolset', description: '' }];
    const described = { 'DBVMCPFirst.MyCustomSceneToolset': { name: 'DBVMCPFirst.MyCustomSceneToolset', tools: [{ name: 'RunPython', description: '', inputSchema: {} }] } };
    const backend = fakeBackend({ toolsets, described });
    const askApproval = vi.fn(async () => 'once');
    await tool(createUnrealTools({ backend, askApproval }), 'unreal_run_python').run({ code: 'print(1)' });
    expect(backend.calls).toEqual([expect.objectContaining({ toolset: 'DBVMCPFirst.MyCustomSceneToolset', tool: 'RunPython', args: { code: 'print(1)' } })]);
    expect(askApproval.mock.calls[0][0].validated).toBe(false);
    expect(backend.describeToolset).not.toHaveBeenCalledWith('EditorToolset.EditorAppToolset');
  });

  it('sin toolset DBV, la referencia de la API explica cómo instalarlo @spec REQ-25', async () => {
    const backend = fakeBackend({ toolsets: [TOOLSETS[2]] });
    const out = await tool(createUnrealTools({ backend, askApproval: vi.fn() }), 'unreal_describe_python_api').run({ name: 'EditorAssetLibrary' });
    expect(out).toMatch(/Prepare project/);
  });
});

describe('modelos que no llaman a la herramienta (REQ-10, REQ-26)', () => {
  it('recuerda ejecutar el Python que el modelo solo escribió en su respuesta @spec REQ-10, REQ-26', () => {
    const reply = "Here's the corrected code:\n```python\nimport unreal\nx = 1\n```";
    expect(pythonNudge(reply)).toMatch(/unreal_run_python/);
    expect(pythonNudge('Hay 2 luces en el nivel.')).toBeNull();
    expect(pythonNudge('```python\nprint("hola")\n```')).toBeNull();
    expect(pythonNudge(undefined)).toBeNull();
  });

  it('un callback por fotograma escrito en la respuesta lleva su aviso aparte @spec REQ-97', () => {
    const reply = '```python\nimport unreal\nh = unreal.register_slate_post_tick_callback(lambda dt: None)\n```';
    expect(pythonNudge(reply)).toMatch(/per-frame callback/);
    expect(pythonNudge('```python\nimport unreal\nx = 1\n```')).not.toMatch(/per-frame callback/);
  });

  it('el prompt manda medir antes de cambiar y prohíbe grabar con callbacks por fotograma @spec REQ-97', () => {
    for (const agent of [false, true]) {
      const prompt = unrealPrompt({ projectName: 'Demo', agent });
      expect(prompt).toContain('do not change code guided only by a hypothesis: measure first');
      expect(prompt).toContain('runtime_diagnosis');
      expect(prompt).toContain('NEVER record with per-frame callbacks');
      expect(prompt).toContain('a hypothesis, not a fact');
      expect(prompt).toContain('what you noticed / what the data says / conclusion');
    }
  });
});

describe('formato y prompt', () => {
  it('recorta resultados enormes y marca los errores @spec REQ-10', () => {
    expect(formatResult({ text: 'x'.repeat(13000), isError: false, value: null })).toMatch(/truncated: 13000/);
    expect(formatResult({ text: 'boom', isError: true, value: null })).toBe('error: boom');
  });

  it('el prompt pide usar /Game/DBV, validar el Python y poner la cabecera estándar DBV en los ficheros nuevos @spec REQ-19, REQ-26, REQ-69', async () => {
    const prompt = unrealPrompt({ projectName: 'DBVMCPFirst', engineVersion: '5.8.2' });
    expect(prompt).toMatch(/DBVMCPFirst/);
    expect(prompt).toMatch(/\/Game\/DBV/);
    expect(prompt).toMatch(/unreal_describe_python_api/);
    expect(prompt).toMatch(/EditorActorSubsystem\)\.get_all_level_actors\(\)/);
    expect(prompt).toContain('Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona');
    expect(prompt).toContain('Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops');
    expect(prompt).toContain('docs/**/*.md');

    const fs = await import('node:fs');
    const path = await import('node:path');
    const walk = (dir) =>
      fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) return entry.name === 'vendor' || entry.name === 'target' ? [] : walk(full);
        return /\.(js|rs|py|cpp|h)$/.test(entry.name) ? [full] : [];
      });
    const missing = ['src', 'src-tauri/src', 'tests', 'unreal-plugin'].flatMap(walk).filter((file) => {
      const head = fs.readFileSync(file, 'utf8').split(/\r?\n/).slice(0, 8).join('\n');
      return !head.includes('=====') || (!head.includes('David Bueno Vallejo') && !head.includes('GENERADO por scripts/sync-ai.mjs')) || !head.includes('dbv-specs-ops');
    });
    expect(missing).toEqual([]);
  });

  it('permite autoría personalizada en Copyright manteniendo de forma inmutable la atribución DBV @spec REQ-74', async () => {
    const prompt = unrealPrompt({ projectName: 'SuperGame', engineVersion: '5.8.4', copyrightHolder: 'MiEmpresa Games' });
    expect(prompt).toContain('// SuperGame — <Short file title / description>');
    expect(prompt).toContain('// Copyright (c) 2026 MiEmpresa Games');
    expect(prompt).toContain('// Generated with DBV Unreal AI · https://github.com/davidbuenov/dbv-unreal-ai');
    expect(prompt).toContain('// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops');
  });
});

describe('contexto del proyecto y del editor (REQ-13)', () => {
  it('el resumen de escena y la info del proyecto son de lectura y van al toolset DBV @spec REQ-13', async () => {
    const backend = fakeBackend({ responses: { get_scene_summary: { text: '', isError: false, value: { total: 2, counts: { DirectionalLight: 1, SkyLight: 1 } } }, get_project_info: { text: '', isError: false, value: { engine_version: '5.8.2' } } } });
    const askApproval = vi.fn();
    const tools = createUnrealTools({ backend, askApproval });
    expect(await tool(tools, 'unreal_scene_summary').run({ class_filter: 'Light', limit: 5 })).toBe('{"total":2,"counts":{"DirectionalLight":1,"SkyLight":1}}');
    expect(await tool(tools, 'unreal_project_info').run({})).toMatch(/5\.8\.2/);
    expect(backend.calls[0]).toMatchObject({ tool: 'get_scene_summary', args: { name_filter: '', class_filter: 'Light', limit: 5 } });
    expect(askApproval).not.toHaveBeenCalled();
  });

  it('sin toolset DBV, el resumen de escena usa el GetActors del proyecto @spec REQ-13', async () => {
    const toolsets = [{ name: 'DBVMCPFirst.MyCustomSceneToolset', description: '' }];
    const described = { 'DBVMCPFirst.MyCustomSceneToolset': { name: 'DBVMCPFirst.MyCustomSceneToolset', tools: [{ name: 'RunPython', description: '', inputSchema: {} }, { name: 'GetActors', description: '', inputSchema: {} }] } };
    const backend = fakeBackend({ toolsets, described, responses: { GetActors: { text: '', isError: false, value: { actors: [] } } } });
    expect(await tool(createUnrealTools({ backend, askApproval: vi.fn() }), 'unreal_scene_summary').run({})).toBe('{"actors":[]}');
  });
});

describe('la IA ve el viewport (REQ-13)', () => {
  const capture = { mime: 'image/jpeg', base64: 'AAAA', width: 1568, height: 498, info: { cameraFOV: 90, labeledActors: [{ label: 'Pirata' }] }, text: 'Viewport capture (1568×498 px).' };

  it('solo se ofrece a un modelo con visión @spec REQ-13', () => {
    const backend = { ...fakeBackend(), captureViewport: vi.fn() };
    expect(tool(createUnrealTools({ backend, askApproval: vi.fn() }), 'unreal_capture_viewport')).toBeUndefined();
    expect(tool(createUnrealTools({ backend, askApproval: vi.fn(), vision: true }), 'unreal_capture_viewport')).toBeDefined();
  });

  it('devuelve la imagen aparte del texto, la enseña y no pide permiso @spec REQ-13', async () => {
    const backend = { ...fakeBackend(), captureViewport: vi.fn(async () => ({ ok: true, value: capture })) };
    const askApproval = vi.fn();
    const showImage = vi.fn();
    const result = await tool(createUnrealTools({ backend, askApproval, vision: true, showImage }), 'unreal_capture_viewport').run({ annotate: true });
    expect(backend.captureViewport).toHaveBeenCalledWith(true, 'normal');
    expect(result).toEqual({ text: capture.text, images: [{ mime: 'image/jpeg', base64: 'AAAA' }] });
    expect(showImage).toHaveBeenCalledWith(capture);
    expect(askApproval).not.toHaveBeenCalled();
    expect(describeCapture(capture)).toContain('Labeled actors: Pirata');
  });
});

describe('MetaHumans y playtest autónomo (REQ-80, REQ-81, REQ-82)', () => {
  it('formArgs propaga garment_slot cuando se elige una prenda de MetaHuman @spec REQ-80', () => {
    const textGarment = formArgs({ mode: 'text', prompt: 'cyberpunk jacket', name: 'Chaqueta', variants: 1, garmentSlot: 'Torso' });
    expect(textGarment.tool).toBe('generate_3d_model');
    expect(textGarment.args.garment_slot).toBe('Torso');

    const imgGarment = formArgs({ mode: 'image', image: 'ref.png', name: 'Botas', variants: 1, garmentSlot: 'Feet' });
    expect(imgGarment.tool).toBe('image_to_3d_model');
    expect(imgGarment.args.garment_slot).toBe('Feet');
  });

  it('formArgs manda el tipo y el corte de la prenda y deja el campo antiguo para compatibilidad @spec REQ-84', () => {
    const dress = formArgs({ mode: 'text', prompt: 'red dress', name: 'Vestido', variants: 1, garmentKind: 'dress', garmentCut: 'female' });
    expect(dress.args).toMatchObject({ garment_kind: 'dress', garment_cut: 'female' });
    expect(dress.args.garment_slot).toBeUndefined();
    const plain = formArgs({ mode: 'text', prompt: 'a lamp', name: 'Lámpara', variants: 1, garmentKind: '', garmentCut: 'male' });
    expect(plain.args.garment_cut, 'sin prenda no hay corte').toBeUndefined();
    const legacy = formArgs({ mode: 'image', image: 'r.png', name: 'Botas', variants: 1, garmentSlot: 'Feet' });
    expect(legacy.args.garment_slot).toBe('Feet');
    expect(legacy.args.garment_kind).toBeUndefined();
    const both = formArgs({ mode: 'text', prompt: 'x', name: 'X', variants: 1, garmentKind: 'skirt', garmentSlot: 'Torso' });
    expect(both.args.garment_kind, 'manda el tipo').toBe('skirt');
    expect(both.args.garment_slot).toBeUndefined();
  });

  it('lista y equipa prendas de MetaHuman mediante el toolset DBV @spec REQ-81', async () => {
    const backend = fakeBackend({
      responses: {
        run_python: (args) => {
          if (args.code.includes('list_metahumans')) {
            return { text: '', isError: false, value: { success: true, result: { count: 1, metahumans: [{ label: 'BP_Victoria', build: 'f_med_nrw' }] } } };
          }
          return { text: '', isError: false, value: { success: true, result: { success: true, actor: 'BP_Victoria', slot: 'Torso', rigged_asset: '/Game/DBV/MetaHumanOutfits/Chaqueta_Rigged' } } };
        },
        import_model: { text: '', isError: false, value: { success: true, primary: '/Game/DBV/Generated/Chaqueta', kind: 'static', animations: [] } },
      },
    });
    backend.libraryFilePath = vi.fn(async () => ({ ok: true, value: 'C:/lib/model.glb' }));
    backend.libraryRecordSent = vi.fn(async () => ({ ok: true }));

    const askApproval = vi.fn(async () => 'once');
    const tools = createUnrealTools({ backend, askApproval });

    const listed = await tool(tools, 'unreal_list_metahumans').run({});
    expect(listed).toContain('BP_Victoria');
    expect(askApproval).not.toHaveBeenCalled();

    const equipped = await tool(tools, 'unreal_equip_metahuman_outfit').run({ actor_label: 'BP_Victoria', source: '/Game/DBV/Generated/Chaqueta', slot: 'Torso' });
    expect(equipped).toContain('Chaqueta_Rigged');
    expect(askApproval).toHaveBeenCalledTimes(1);

    const sent = await sendToUnreal(
      { backend },
      { id: 'g-1', name: 'Chaqueta', kind: 'model', files: ['model.glb'], parameters: { garment_slot: 'Torso' } },
      { project: 'D:/P/Demo.uproject', place: true, metahuman: 'BP_Victoria' },
    );
    expect(sent).toMatchObject({ ok: true, actor: 'BP_Victoria', kind: 'metahuman_garment', equippedSlot: 'Torso' });
  });

  it('unreal_equip_metahuman_outfit sin destino no pide permiso ni ejecuta nada y manda listar @spec REQ-83', async () => {
    const backend = fakeBackend();
    const askApproval = vi.fn(async () => 'once');
    const tools = createUnrealTools({ backend, askApproval });
    const out = await tool(tools, 'unreal_equip_metahuman_outfit').run({ source: '/Game/DBV/Generated/Chaqueta', slot: 'Torso' });
    expect(out).toContain('no target MetaHuman was chosen');
    expect(out).toContain('unreal_list_metahumans');
    expect(askApproval).not.toHaveBeenCalled();
    expect(backend.calls.filter((c) => c.tool === 'run_python')).toHaveLength(0);
    expect(tool(tools, 'unreal_equip_metahuman_outfit').parameters.required).toContain('actor_label');
  });

  it('unreal_playtest arranca PIE, mide telemetría, captura el viewport y detiene siempre PIE @spec REQ-82', async () => {
    const capture = { mime: 'image/jpeg', base64: 'BBBB', width: 1024, height: 576, info: {}, text: 'PIE capture' };
    const backend = {
      ...fakeBackend({
        responses: {
          run_python: (args) => {
            if (args.code.includes('read_pie_playtest_telemetry')) {
              return { text: '', isError: false, value: { success: true, result: { pawn_spawned: true, distance_moved_cm: 618.0, fell_out_of_world: false } } };
            }
            return { text: '', isError: false, value: { success: true, result: { started: true } } };
          },
        },
      }),
      captureViewport: vi.fn(async () => ({ ok: true, value: capture })),
    };
    const askApproval = vi.fn(async () => 'once');
    const tools = createUnrealTools({ backend, askApproval, vision: true, sleep: async () => {} });
    const res = await tool(tools, 'unreal_playtest').run({ duration_seconds: 3, actions: ['forward', 'jump'] });
    expect(res.images).toHaveLength(1);
    expect(res.text).toContain('"distance_moved_cm":618');
    expect(res.text).toContain('"stopped_pie":true');
    const calledTools = backend.calls.map((c) => c.tool);
    expect(calledTools[0]).toBe('StartPIE');
    expect(calledTools[calledTools.length - 1]).toBe('StopPIE');
  });
});
