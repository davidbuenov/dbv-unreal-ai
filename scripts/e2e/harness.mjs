// =============================================================================
// DBV Unreal AI — Plantilla Tauri v2 + dbv-specs-ops
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Banco de pruebas de la interfaz: Edge/Chromium headless dirigido por CDP, con
// `window.__TAURI__` simulado. Es el mismo motor que WebView2, así que cubre lo
// que los tests de Node no ven (DOM, selección, scroll, menús, eventos reales de
// ratón y teclado). `src/` se sirve por HTTP, como lo sirve la app real: desde
// file:// Chromium bloquea el `import()` de módulos.
//
// Uso: ver `scripts/e2e/*.e2e.mjs` y `npm run test:e2e`.
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import http from 'node:http';
import { readFile } from 'node:fs/promises';

const EDGE_CANDIDATES = [
  process.env.EDGE_PATH,
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
  '/usr/bin/microsoft-edge',
  '/usr/bin/microsoft-edge-stable',
  '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
].filter(Boolean);
/** Ruta de Edge/Chromium (el mismo motor que WebView2), o `null` si no hay. */
export const findEdge = () => EDGE_CANDIDATES.find((candidate) => existsSync(candidate)) ?? null;
const EDGE = findEdge();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export async function launch({ appDir, docPath, docContent, extraInvoke = '{}', customInvoke = '(() => undefined)', width = 1400, height = 900, port = 9333 }) {
  // Se sirve src/ por HTTP, como lo sirve la app real (tauri://localhost): desde file:// Chromium bloquea import() de módulos.
  const MIME = { '.glb': 'model/gltf-binary', '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.md': 'text/markdown', '.svg': 'image/svg+xml', '.png': 'image/png' };
  const server = http.createServer(async (req, res) => {
    try {
      // `/__fixtures/…` sirve los ficheros de prueba (un GLB para el visor) sin meterlos en `src/`, que va en la app.
      const pathname = decodeURIComponent(new URL(req.url, 'http://x').pathname);
      const fixtures = path.join(appDir, '..', 'tests', 'fixtures');
      const file = pathname.startsWith('/__fixtures/') ? path.join(fixtures, pathname.slice('/__fixtures/'.length)) : path.join(appDir, pathname);
      const body = await readFile(file);
      res.writeHead(200, { 'content-type': MIME[path.extname(file)] || 'application/octet-stream', 'cache-control': 'no-store' });
      res.end(body);
    } catch { res.writeHead(404); res.end('no'); }
  });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  const appUrl = `http://127.0.0.1:${server.address().port}/index.html`;
  // Un navegador de una ejecución anterior que siga escuchando en `port` haría que el banco midiera SU página vieja
  // (pasó: cientos de Edge sin cerrar daban errores y aciertos de otra ejecución). Mejor fallar claro.
  const busy = await fetch(`http://127.0.0.1:${port}/json/version`).then(() => true, () => false);
  if (busy) {
    server.close();
    throw new Error(`el puerto de depuración ${port} ya está en uso: cierra los Edge del banco que quedaron abiertos (perfil dbv-harness-*)`);
  }
  const profile = mkdtempSync(path.join(tmpdir(), 'dbv-harness-'));
  const proc = spawn(EDGE, [
    '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`,
    `--window-size=${width},${height}`, '--no-first-run', '--disable-gpu', 'about:blank',
  ], { stdio: 'ignore' });
  let targets;
  for (let i = 0; i < 60; i++) {
    try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); if (targets.some((t) => t.type === 'page')) break; } catch {}
    await sleep(250);
  }
  const page = targets.find((t) => t.type === 'page');
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  let id = 0;
  const pending = new Map();
  const events = [];
  ws.onmessage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); }
    else if (msg.method) events.push(msg);
  };
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const i = ++id;
    pending.set(i, (msg) => (msg.error ? reject(new Error(`${method}: ${msg.error.message}`)) : resolve(msg.result)));
    ws.send(JSON.stringify({ id: i, method, params }));
  });
  await send('Page.enable');
  await send('Runtime.enable');
  const errors = [];
  const poll = () => { for (const e of events.splice(0)) {
    if (e.method === 'Runtime.exceptionThrown') errors.push(e.params.exceptionDetails.exception?.description || e.params.exceptionDetails.text);
    if (e.method === 'Runtime.consoleAPICalled' && ['error'].includes(e.params.type)) errors.push('console.error: ' + e.params.args.map((a) => a.value ?? a.description).join(' '));
  } return errors; };

  const mock = `
    (function () {
      const deep = () => new Proxy(function () {}, { get: (t, k) => (k === 'then' ? undefined : (k in t ? t[k] : deep())), apply: () => Promise.resolve(null) });
      const doc = ${JSON.stringify({ path: docPath, content: docContent, dir_path: path.posix.dirname(docPath), file_name: path.posix.basename(docPath) })};
      const calls = [];
      window.__calls = calls;
      const extra = ${extraInvoke};
      const custom = ${customInvoke};
      const listeners = {};
      window.__emit = (name, payload) => (listeners[name] || []).forEach((cb) => cb({ payload }));
      window.__chat = [];
      const T = deep();
      T.core = {
        invoke: (cmd, args) => {
          calls.push([cmd, args]);
          const custom_result = custom(cmd, args);
          if (custom_result !== undefined) return Promise.resolve(custom_result);
          if (cmd in extra) return Promise.resolve(extra[cmd]);
          if (cmd === 'get_cli_argument') return Promise.resolve(doc.path);
          if (cmd === 'read_file') return Promise.resolve(doc);
          if (cmd === 'list_directory') return Promise.resolve([]);
          if (cmd === 'get_app_version') return Promise.resolve('0.18.0');
          if (cmd === 'get_recent_files') return Promise.resolve([]);
          if (cmd === 'write_file') return Promise.resolve(null);
          if (cmd === 'ai_connections') return Promise.resolve({ connections: [], active: null, showAi: true });

          return Promise.resolve(null);
        },
        convertFileSrc: (p) => p,
      };
      T.os = { platform: () => 'windows' };
      T.event = { listen: (name, cb) => { (listeners[name] = listeners[name] || []).push(cb); return Promise.resolve(() => {}); } };
      T.window = { getCurrentWindow: () => deep() };
      window.__TAURI__ = T;
    })();`;
  await send('Page.addScriptToEvaluateOnNewDocument', { source: mock });
  await send('Page.navigate', { url: appUrl });
  await sleep(1500);

  const api = {
    errors: poll,
    async eval(expr) {
      const r = await send('Runtime.evaluate', { expression: `(async()=>{ ${expr} })()`, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text);
      return r.result.value;
    },
    async mouse(type, x, y, { button = 'left', clickCount = 1 } = {}) {
      await send('Input.dispatchMouseEvent', { type, x, y, button, clickCount, buttons: button === 'left' ? 1 : 2 });
    },
    async click(x, y, count = 1, button = 'left') {
      await this.mouse('mouseMoved', x, y);
      for (let i = 1; i <= count; i++) {
        await this.mouse('mousePressed', x, y, { button, clickCount: i });
        await this.mouse('mouseReleased', x, y, { button, clickCount: i });
      }
    },
    async key(key, code, vk, modifiers = 0) {
      await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code, windowsVirtualKeyCode: vk, modifiers });
      await send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: vk, modifiers });
    },
    sleep,
    async screenshot(file) {
      const r = await send('Page.captureScreenshot', { format: 'png' });
      writeFileSync(file, Buffer.from(r.data, 'base64'));
    },
    async close() {
      try { ws.close(); } catch {}
      // `msedge.exe` es un lanzador: arranca el navegador como OTRO proceso y termina, así que `proc.kill()` no cerraba
      // nada (llegó a haber 453 Edge del banco abiertos). Se le pide al navegador que se cierre por CDP y, por si no
      // responde, se cierran en Windows los procesos que usan este perfil temporal.
      try {
        const { webSocketDebuggerUrl } = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
        const browser = new WebSocket(webSocketDebuggerUrl);
        await new Promise((resolve) => {
          browser.onopen = () => browser.send(JSON.stringify({ id: 1, method: 'Browser.close' }));
          browser.onclose = resolve;
          browser.onerror = resolve;
          setTimeout(resolve, 3000);
        });
      } catch {}
      if (process.platform === 'win32') {
        const script = `Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -like '*${path.basename(profile)}*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }`;
        spawnSync('powershell', ['-NoProfile', '-Command', script], { stdio: 'ignore' });
      } else proc.kill();
      server.close();
    },
  };
  return api;
}

export { readFileSync, sleep };
