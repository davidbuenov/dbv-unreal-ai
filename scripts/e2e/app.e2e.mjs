// =============================================================================
// DBV Unreal AI — Banco de interfaz en el motor de WebView2 (Edge headless)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// La ventana real con el backend de Rust simulado: conexión automática con el editor, vistas,
// y una conversación en la que el modelo pide Python → la app lo valida → tarjeta de aprobación →
// «Permitir» → se ejecuta y queda en el registro de acciones. @spec REQ-01, REQ-02, REQ-08, REQ-12, REQ-14, REQ-17, REQ-20, REQ-26, RNF-IA.1
//
// Uso: npm run test:e2e   (sin Edge/Chromium se omite). Captura en `$E2E_SCREENSHOT_DIR` si se define.

import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { findEdge, launch } from './harness.mjs';

if (!findEdge()) {
  console.log('Sin Edge/Chromium: se omite este banco (define EDGE_PATH para indicarlo).');
  process.exit(0);
}

const appDir = path.join(path.dirname(path.dirname(path.dirname(fileURLToPath(import.meta.url)))), 'src').replace(/\\/g, '/');
const shots = process.env.E2E_SCREENSHOT_DIR;
const PROJECT = 'D:/Unreal/DBVMCPFirst/DBVMCPFirst.uproject';
const DBV = 'PythonTypes.DBVUnrealAIToolset_0x3D7D6EDD';
const uproject = { path: PROJECT, name: 'DBVMCPFirst', engineAssociation: '5.8', modules: ['DBVMCPFirst'], plugins: [], hasCpp: true };
const mcp = { port: 8000, path: '/mcp', autoStart: true };
const connection = { id: 'c1', name: 'Ollama · qwen3:8b', provider: 'ollama', baseUrl: 'http://localhost:11434', model: 'qwen3:8b', hasKey: false, contextTokens: 8192, supportsTools: true, supportsImages: false };
const entry = { id: 'linterna-1', name: 'Linterna', provider: 'tripo', kind: 'model', prompt: 'lantern', parameters: {}, taskId: 't', createdAt: 1761300000, credits: 30, files: ['model.glb'], thumbnail: null, sentTo: [] };

function extra(connections, hasKey = false) {
  return JSON.stringify({
    ai_connections: { connections, active: connections[0]?.id ?? null, showAi: true },
    ai_providers: [{ provider: 'ollama', baseUrl: 'http://localhost:11434', cloud: false, contextTokens: 8192, maxOutputTokens: 4096 }],
    ai_model_info: null,
    ai_list_files: [],
    ai_project_state_load: null,
    ai_project_state_save: null,
    unreal_editors: [{ editor: { pid: 1, executable: 'UnrealEditor.exe', project: PROJECT }, project: uproject, mcp, listening: true }],
    unreal_recent_projects: [],
    unreal_remember_project: [PROJECT],
    unreal_readiness: { project: uproject, engine: { version: '5.8.2' }, mcp, checks: [{ id: 'engine', ok: true, detail: '5.8.2' }, { id: 'plugin:ModelContextProtocol', ok: true, detail: '' }, { id: 'autoStart', ok: true, detail: '8000' }, { id: 'toolset', ok: true, detail: '0.1.0' }], toolsetInstalled: '0.1.0', toolsetBundled: '0.1.0', ready: true },
    unreal_connect: [{ name: DBV, description: 'DBV' }, { name: 'editor_toolset.toolsets.scene.SceneTools', description: 'scene' }],
    unreal_list_toolsets: [{ name: DBV, description: 'DBV' }, { name: 'editor_toolset.toolsets.scene.SceneTools', description: 'scene' }],
    generation_status: { spent: 0, budget: 300, hasKey: { tripo: hasKey, worldlabs: false } },
    // Las herramientas de Tripo llegan de Rust (`generation/tools.rs`), las mismas que ve un agente.
    generation_tool_specs: ['generate_3d_model', 'image_to_3d_model', 'rig_model', 'animate_model', 'tripo_run', 'tripo_operations', 'tripo_balance'].map((name) => ({ name, description: name, inputSchema: { type: 'object', properties: {}, required: [] } })),
    generation_jobs: [],
    // El visor 3D carga este GLB de prueba (lo sirve el banco desde tests/fixtures).
    library_file_path: '/__fixtures/models/cubo.glb',
    library_location: { current: 'C:/Users/u/AppData/Roaming/com.davidbuenov.dbv-unreal-ai/library', default: 'C:/Users/u/AppData/Roaming/com.davidbuenov.dbv-unreal-ai/library', isDefault: true },
    library_list: [entry],
    app_mcp_command: { command: 'C:/Program Files/DBV Unreal AI/dbv-unreal-ai.exe', args: ['--mcp', '--port', '8000'] },
  });
}

// El editor y el modelo simulados.
const customInvoke = `((cmd, args) => {
  if (cmd === 'unreal_call_tool') {
    if (args.tool === 'validate_python') return { text: '', isError: false, value: { ok: true, syntax_error: null, findings: [], checked: 1 } };
    if (args.tool === 'run_python') return { text: '', isError: false, value: { success: true, stdout: '42\\n', stderr: '', warnings: [], error: null, result: 42 } };
    return { text: '{}', isError: false, value: {} };
  }
  if (cmd !== 'ai_chat') return undefined;
  window.__chat.push(args);
  const n = window.__chat.length;
  const call = { type: 'toolCall', id: 't1', name: 'unreal_run_python', arguments: JSON.stringify({ code: 'import unreal\\nresult = 42', purpose: 'Calcular la respuesta' }) };
  const script = n === 1 ? [call, { type: 'done', stopReason: 'tool_use' }] : [{ type: 'text', text: 'Hecho: 42.' }, { type: 'done', stopReason: 'end_turn' }];
  script.forEach((ev, i) => setTimeout(() => window.__emit('ai-stream', { requestId: args.requestId, ...ev }), 15 + i * 8));
  return null;
})`;

let failed = 0;
const ok = (name, cond, detail = '') => {
  console.log(`${cond ? '  ok ' : '  FALLA'} ${name}${detail ? ' — ' + detail : ''}`);
  if (!cond) failed++;
};
const aiRequests = `performance.getEntriesByType('resource').map((r) => r.name).filter((n) => /\\/ai\\/(aiApp|entry)/.test(n))`;
const shot = async (h, name) => shots && (await h.screenshot(path.join(shots, name)));

// ── Arranque, conexión automática y vistas, sin IA ────────────────────────────────────────────────
{
  const h = await launch({ appDir, docPath: 'C:/x.md', docContent: '', extraInvoke: extra([]), customInvoke });
  try {
    await h.sleep(1200);
    const boot = await h.eval(`return { pill: document.getElementById('unreal-pill').textContent, cls: document.getElementById('unreal-pill').className, ai: ${aiRequests}, ready: document.querySelector('.check-ok')?.textContent ?? '' }`);
    ok('se conecta solo al editor con el servidor MCP activo', boot.pill === 'DBVMCPFirst · UE 5.8 · :8000' && boot.cls.includes('pill--online'), JSON.stringify(boot));
    ok('el diagnóstico dice que el proyecto está preparado', boot.ready.length > 0);
    ok('al arrancar no se carga el asistente de IA (RNF-IA.1)', boot.ai.length === 0, JSON.stringify(boot.ai));
    await shot(h, 'project.png');

    await h.eval(`document.querySelector('[data-theme-choice="sepia"]').click()`);
    const sepia = await h.eval(`return { theme: document.documentElement.dataset.theme, bg: getComputedStyle(document.body).backgroundColor, pressed: document.querySelector('[data-theme-choice="sepia"]').getAttribute('aria-pressed') }`);
    ok('el tema Sepia se aplica con su paleta cálida', sepia.theme === 'sepia' && sepia.bg === 'rgb(250, 243, 224)' && sepia.pressed === 'true', JSON.stringify(sepia));
    await shot(h, 'sepia.png');
    await h.eval(`document.querySelector('#btn-pin').click()`);
    await h.sleep(200);
    ok('la chincheta fija la ventana encima y lo indica', (await h.eval(`return document.querySelector('#btn-pin').getAttribute('aria-pressed')`)) === 'true');
    await h.eval(`document.querySelector('[data-theme-choice="dark"]').click(); document.querySelector('#btn-pin').click()`);

    await h.eval(`document.querySelector('[data-view="library"]').click()`);
    await h.sleep(500);
    ok('la biblioteca enseña el asset', (await h.eval(`return document.querySelector('.asset-name')?.textContent`)) === 'Linterna');
    await shot(h, 'library.png');
    await h.eval(`[...document.querySelectorAll('.asset-card button')].find((b) => b.textContent === 'Ver en 3D').click()`);
    await h.sleep(2500);
    const viewer = await h.eval(`return { canvas: Boolean(document.querySelector('#app-modal-card .viewer-stage canvas')), error: document.querySelector('#app-modal-card .check-fail')?.textContent ?? '', open: !document.getElementById('app-modal').classList.contains('hidden'), text: document.getElementById('app-modal-card').textContent }`);
    ok('«Ver en 3D» pinta el modelo con WebGL (REQ-31)', viewer.open && viewer.canvas && !viewer.error && !viewer.text.includes('null'), JSON.stringify(viewer));
    await shot(h, 'viewer.png');
    await h.eval(`document.getElementById('app-modal').click()`);
    ok('al cerrar el visor se libera el lienzo', (await h.eval(`return document.querySelectorAll('.viewer-stage canvas').length`)) === 0);

    await h.eval(`document.querySelector('[data-view="generate"]').click()`);
    await h.sleep(400);
    const gen = await h.eval(`return { disabled: document.querySelector('#view-generate .btn-primary').disabled, text: document.getElementById('view-generate').textContent }`);
    ok('sin clave de Tripo no se puede generar y se explica', gen.disabled && /clave de Tripo|Tripo key/.test(gen.text));
    await shot(h, 'generate.png');

    await h.eval(`document.querySelector('[data-view="settings"]').click()`);
    await h.sleep(400);
    ok('Ajustes da la configuración MCP para otros agentes', (await h.eval(`return document.querySelector('#view-settings pre')?.textContent ?? ''`)).includes('--mcp'));
    await shot(h, 'settings.png');
    const errs = h.errors();
    ok('sin excepciones ni console.error', errs.length === 0, JSON.stringify(errs.slice(0, 3)));
  } finally {
    await h.close();
  }
}

// ── Conversación: Python validado, aprobación y registro ──────────────────────────────────────────
{
  const h = await launch({ appDir, docPath: 'C:/x.md', docContent: '', extraInvoke: extra([connection], true), customInvoke, width: 1400, height: 900, port: 9334 });
  try {
    await h.sleep(1200);
    await h.eval(`document.querySelector('[data-view="chat"]').click()`);
    await h.sleep(1500);
    ok('la Conversación abre el asistente (hay una IA conectada)', await h.eval(`return !!document.querySelector('.ai-panel__input') && document.getElementById('ai-connect-modal').classList.contains('hidden')`));
    const start = await h.eval(`return { suggestions: [...document.querySelectorAll('.ai-panel__suggestion')].map((b) => b.textContent), contextHidden: document.querySelector('.ai-panel__context')?.classList.contains('hidden') ?? null, placeholder: document.querySelector('.ai-panel__input')?.placeholder }`);
    ok('las sugerencias de la conversación son de Unreal, no de un editor de texto', start.suggestions.length === 3 && start.suggestions.join(' ').includes('nivel') && !/documento|ortograf|tabla/i.test(start.suggestions.join(' ')), JSON.stringify(start.suggestions));
    ok('el campo de texto habla del proyecto de Unreal', /Unreal/.test(start.placeholder ?? ''), start.placeholder);
    ok('cargar la IA no borra el estado de la conexión de la píldora', (await h.eval(`return document.getElementById('unreal-pill').textContent`)) === 'DBVMCPFirst · UE 5.8 · :8000');
    await h.eval(`const i = document.querySelector('.ai-panel__input'); i.value = '¿Cuánto es la respuesta?'; i.dispatchEvent(new Event('input', { bubbles: true }));`);
    await h.key('Enter', 'Enter', 13);
    await h.sleep(1200);
    const card = await h.eval(`const c = document.querySelector('.unreal-approval'); return c ? { code: c.querySelector('pre').textContent, purpose: c.querySelector('.unreal-approval__purpose')?.textContent } : null`);
    ok('aparece la tarjeta de aprobación con el código entero', card?.code === 'import unreal\nresult = 42' && card.purpose === 'Calcular la respuesta', JSON.stringify(card));
    const before = await h.eval(`return window.__calls.filter((c) => c[0] === 'unreal_call_tool').map((c) => c[1].tool)`);
    ok('antes de aprobar solo se ha VALIDADO, no ejecutado', JSON.stringify(before) === '["validate_python"]', JSON.stringify(before));
    const tools = await h.eval(`return (window.__chat[0].request.tools || []).map((t) => t.name)`);
    ok('el modelo recibe las herramientas de Unreal y de generación', ['unreal_list_toolsets', 'unreal_run_python', 'generate_3d_model', 'rig_model', 'animate_model', 'tripo_run', 'library_send_to_unreal'].every((n) => tools.includes(n)), JSON.stringify(tools));
    const system = await h.eval(`return window.__chat[0].request.messages[0].content`);
    ok('el prompt de sistema es el de Unreal, con el proyecto', system.includes('Unreal Engine assistant') && system.includes('DBVMCPFirst') && system.includes('/Game/DBV'));
    await shot(h, 'approval.png');

    await h.eval(`document.querySelector('.unreal-approval .btn-primary').click()`);
    await h.sleep(1200);
    const after = await h.eval(`return { tools: window.__calls.filter((c) => c[0] === 'unreal_call_tool').map((c) => c[1].tool), run: window.__calls.find((c) => c[0] === 'unreal_call_tool' && c[1].tool === 'run_python')?.[1], text: [...document.querySelectorAll('.ai-msg--assistant')].map((n) => n.textContent).join(' '), log: document.querySelectorAll('#action-log .action-entry').length }`);
    ok('«Permitir» ejecuta el código con su tiempo largo', after.run?.arguments?.code === 'import unreal\nresult = 42' && after.run.timeoutSecs === 180, JSON.stringify(after.run));
    ok('la IA termina la respuesta', after.text.includes('Hecho: 42.'), after.text);
    ok('el registro guarda la validación y la ejecución', after.log === 2, String(after.log));
    await shot(h, 'chat.png');
    const errs = h.errors();
    ok('sin excepciones ni console.error', errs.length === 0, JSON.stringify(errs.slice(0, 3)));
  } finally {
    await h.close();
  }
}

console.log(failed ? `\n${failed} fallos` : '\nTodo correcto');
process.exit(failed ? 1 : 0);
