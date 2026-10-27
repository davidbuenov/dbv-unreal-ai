// =============================================================================
// DBV Unreal AI — Script para generar capturas de pantalla de alta fidelidad
// para la web de presentación (GitHub Pages / docs/)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License.
// =============================================================================

import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { existsSync, mkdirSync } from 'node:fs';
import { findEdge, launch } from './e2e/harness.mjs';

if (!findEdge()) {
  console.error('Error: Se requiere Microsoft Edge o Chromium para generar las capturas.');
  process.exit(1);
}

const rootDir = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const appDir = path.join(rootDir, 'src').replace(/\\/g, '/');
const outDir = path.join(rootDir, 'docs', 'screenshots');
if (!existsSync(outDir)) mkdirSync(outDir, { recursive: true });

const PROJECT = 'D:/Unreal/PirateAdventure/PirateAdventure.uproject';
const DBV = 'PythonTypes.DBVUnrealAIToolset_0x3D7D6EDD';
const uproject = {
  path: PROJECT,
  name: 'PirateAdventure',
  engineAssociation: '5.8',
  modules: ['PirateAdventure'],
  plugins: [],
  hasCpp: true,
};
const mcp = { port: 8000, path: '/mcp', autoStart: true };

const connection = {
  id: 'c-local',
  name: 'Claude 3.5 Sonnet · Anthropic',
  provider: 'anthropic',
  baseUrl: 'https://api.anthropic.com',
  model: 'claude-3-5-sonnet-20241022',
  hasKey: true,
  contextTokens: 200000,
  supportsTools: true,
  supportsImages: true,
};

const libraryList = [
  {
    id: 'island-1',
    name: 'Isla Pirata (Mundo 3DGS)',
    provider: 'worldlabs',
    kind: 'world',
    prompt: 'Abandoned pirate fortress island with cliffs and palms',
    parameters: { model: 'marble-1.1', metric_scale_factor: 100.0, ground_plane_offset: -2.5 },
    taskId: 'wl-1',
    createdAt: 1761400000,
    credits: 230,
    files: ['world.spz', 'collision.glb'],
    thumbnail: null,
    sentTo: ['/Game/DBV/Generated/IslaPirata'],
  },
  {
    id: 'pirate-rig-1',
    name: 'Pirata S18 (Rig)',
    provider: 'tripo',
    kind: 'rig',
    prompt: 'pirate captain with tricorn hat and coat',
    parameters: {},
    taskId: 't-2',
    createdAt: 1761350000,
    credits: 25,
    files: ['model.glb'],
    thumbnail: null,
    sentTo: ['/Game/DBV/Generated/PirataS18'],
  },
  {
    id: 'pirate-anim-1',
    name: 'Baile Pirata (Animación)',
    provider: 'tripo',
    kind: 'animation',
    prompt: 'pirate victory dance',
    parameters: {},
    taskId: 't-3',
    createdAt: 1761360000,
    credits: 10,
    files: ['clip.glb'],
    parent: 'pirate-rig-1',
    thumbnail: null,
    sentTo: [],
  },
  {
    id: 'lantern-1',
    name: 'Linterna Medieval',
    provider: 'tripo',
    kind: 'model',
    prompt: 'medieval wrought iron lantern with candle',
    parameters: {},
    taskId: 't-4',
    createdAt: 1761300000,
    credits: 20,
    files: ['model.glb'],
    thumbnail: null,
    sentTo: ['/Game/DBV/Generated/Linterna'],
  },
];

const tasksList = {
  goal: 'Configurar escenario de la bahía pirata con iluminación cinemática y actores interactivos',
  tasks: [
    { id: '1', text: 'Importar y spawnear Gaussian Splat con collider mesh en el nivel', done: true, check: 'actor exists ADbvGaussianSplatActor', cost: 230 },
    { id: '2', text: 'Alinear DirectionalLight con el ángulo del sol en el horizonte', done: true, check: 'actor exists DirectionalLight', cost: 0 },
    { id: '3', text: 'Colocar 4 linternas medievales a lo largo del muelle de madera', done: true, check: 'actor exists Linterna count >= 4', cost: 20 },
    { id: '4', text: 'Añadir personaje Pirata S18 reproduciendo su animación de victoria', done: false, check: 'actor exists PirataS18', cost: 35 },
  ],
};

function buildExtraInvoke() {
  return JSON.stringify({
    ai_connections: { connections: [connection], active: connection.id, showAi: true },
    ai_providers: [{ provider: 'anthropic', baseUrl: 'https://api.anthropic.com', cloud: true, contextTokens: 200000, maxOutputTokens: 8192 }],
    ai_model_info: null,
    ai_list_files: [],
    ai_project_state_load: null,
    ai_project_state_save: null,
    unreal_editors: [{ editor: { pid: 1420, executable: 'UnrealEditor.exe', project: PROJECT }, project: uproject, mcp, listening: true }],
    unreal_recent_projects: [],
    unreal_remember_project: [PROJECT],
    unreal_readiness: {
      project: uproject,
      engine: { version: '5.8.4' },
      mcp,
      checks: [
        { id: 'engine', ok: true, detail: '5.8.4 Instalada' },
        { id: 'plugin:ModelContextProtocol', ok: true, detail: 'Habilitado' },
        { id: 'plugin:ToolsetRegistry', ok: true, detail: 'Habilitado' },
        { id: 'plugin:PythonScriptPlugin', ok: true, detail: 'Habilitado' },
        { id: 'autoStart', ok: true, detail: 'Puerto 8000' },
        { id: 'toolset', ok: true, detail: 'v0.3.0 (Python + C++)' },
      ],
      toolsetInstalled: '0.3.0',
      toolsetBundled: '0.3.0',
      ready: true,
    },
    unreal_connect: [{ name: DBV, description: 'DBV Unreal AI Toolset' }, { name: 'editor_toolset.toolsets.scene.SceneTools', description: 'SceneTools' }],
    unreal_list_toolsets: [{ name: DBV, description: 'DBV' }, { name: 'editor_toolset.toolsets.scene.SceneTools', description: 'SceneTools' }],
    generation_status: { spent: 105, budget: 500, hasKey: { tripo: true, worldlabs: true }, worldlabsBalance: { credits: 1850 } },
    generation_tool_specs: ['generate_3d_model', 'image_to_3d_model', 'rig_model', 'animate_model', 'tripo_run', 'tripo_operations', 'tripo_balance', 'generate_world', 'export_world_mesh'].map((name) => ({ name, description: name, inputSchema: { type: 'object', properties: {}, required: [] } })),
    generation_jobs: [],
    library_file_path: '/__fixtures/models/cubo.glb',
    library_location: { current: 'C:/Users/david/AppData/Roaming/com.davidbuenov.dbv-unreal-ai/library', default: 'C:/Users/david/AppData/Roaming/com.davidbuenov.dbv-unreal-ai/library', isDefault: true },
    library_list: libraryList,
    app_mcp_command: { command: 'C:/Program Files/DBV Unreal AI/dbv-unreal-ai.exe', args: ['--mcp', '--port', '8000'] },
    project_tasks_list: tasksList,
    project_memory_read: '# Project Memory — PirateAdventure\n\n## Conventions\n- Generated assets are placed under `/Game/DBV/Generated`.\n- World scale is 100x Unreal Units.\n- Photorealistic environments use `ADbvGaussianSplatActor` paired with physical collision hulls.',
  });
}

const customInvoke = `((cmd, args) => {
  if (cmd === 'unreal_call_tool') {
    if (args.tool === 'validate_python') return { text: '', isError: false, value: { ok: true, syntax_error: null, findings: [], checked: 1 } };
    if (args.tool === 'run_python') return { text: '', isError: false, value: { success: true, stdout: 'Created ADbvGaussianSplatActor with aligned collision mesh\\n', stderr: '', warnings: [], error: null, result: true } };
    return { text: '{}', isError: false, value: {} };
  }
  if (cmd === 'unreal_get_undo_count') return { ok: true, value: 14 };
  if (cmd === 'project_tasks_list') return { ok: true, value: ${JSON.stringify(tasksList)} };
  if (cmd === 'project_memory_read') return { ok: true, value: '# Project Memory — PirateAdventure\\n\\n## Conventions\\n- Generated assets are placed under /Game/DBV/Generated.' };
  return undefined;
})`;

async function run() {
  console.log('Iniciando captura de pantallas...');
  const h = await launch({
    appDir,
    docPath: PROJECT,
    docContent: '',
    extraInvoke: buildExtraInvoke(),
    customInvoke,
    width: 1440,
    height: 900,
    port: 9444,
  });

  try {
    await h.sleep(1500);

    // 1. CAPTURA: Proyecto y Diagnóstico (05-project-readiness.png)
    console.log('Capturando: 05-project-readiness.png...');
    await h.screenshot(path.join(outDir, '05-project-readiness.png'));

    // 2. CAPTURA: Conversación con tarjeta de código y botón deshacer (01-hero-conversation.png)
    console.log('Preparando Conversación...');
    await h.eval(`document.querySelector('[data-view="chat"]').click()`);
    await h.sleep(1200);

    // Inyectamos mensaje interactivo realista en el chat
    await h.eval(`
      const chatPanel = document.querySelector('.ai-panel__messages');
      if (chatPanel) {
        chatPanel.innerHTML = '';
        
        // Mensaje Usuario
        const userMsg = document.createElement('div');
        userMsg.className = 'ai-msg ai-msg--user';
        userMsg.innerHTML = '<div class="ai-msg__bubble">Inspecciona el nivel actual y coloca la Isla Pirata generada con World Labs Marble delante de la cámara principal, vinculando el Gaussian Splat con su collider físico.</div>';
        chatPanel.appendChild(userMsg);

        // Mensaje Asistente
        const assistantMsg = document.createElement('div');
        assistantMsg.className = 'ai-msg ai-msg--assistant';
        assistantMsg.innerHTML = \`
          <div class="ai-msg__bubble">
            <p>He analizado la posición de la cámara del editor en <code>X=1250, Y=-450, Z=180</code>. Voy a spawnear el actor fotorrealista <code>ADbvGaussianSplatActor</code> junto con su malla de colisión <code>StaticMeshComponent</code> con física activada.</p>
            <div class="unreal-approval" style="margin-top: 12px;">
              <div class="unreal-approval__header">
                <span class="unreal-approval__badge">Acción de Unreal Engine</span>
                <span class="unreal-approval__purpose">Spawnear y alinear Gaussian Splat + Colisión</span>
              </div>
              <pre><code>import unreal
# Crear actor nativo de Gaussian Splatting DBV
splat_actor = unreal.EditorLevelLibrary.spawn_actor_from_class(unreal.DbvGaussianSplatActor, unreal.Vector(1250, -450, 0))
splat_actor.set_actor_label("World_PirateIsland_3DGS")
splat_actor.load_splat_file("/Game/DBV/Generated/IslaPirata/world.spz")

# Añadir malla de colisión física sincronizada
collider = unreal.EditorLevelLibrary.spawn_actor_from_class(unreal.StaticMeshActor, unreal.Vector(1250, -450, 0))
collider.static_mesh_component.set_static_mesh(unreal.load_asset("/Game/DBV/Generated/IslaPirata/collision.glb"))
collider.set_actor_hidden_in_game(True)
collider.set_actor_enable_collision(True)
result = True</code></pre>
              <div class="unreal-approval__actions" style="display: flex; gap: 8px; margin-top: 8px;">
                <button class="btn btn-primary btn-sm">✓ Permitir ejecución</button>
                <button class="btn btn-ghost btn-sm">Rechazar</button>
              </div>
            </div>
            <div class="ai-turn-footer" style="margin-top: 16px; padding-top: 8px; border-top: 1px solid var(--border-subtle, #333); display: flex; align-items: center; justify-content: space-between;">
              <span class="text-muted" style="font-size: 12px;">Turno completado · 3 transacciones registradas</span>
              <button class="btn btn-ghost btn-sm btn-undo-turn" style="color: #f87171; border-color: rgba(248,113,113,0.3);">↩️ Deshacer este turno</button>
            </div>
          </div>
        \`;
        chatPanel.appendChild(assistantMsg);
      }
    `);
    await h.sleep(600);
    console.log('Capturando: 01-hero-conversation.png...');
    await h.screenshot(path.join(outDir, '01-hero-conversation.png'));

    // 3. CAPTURA: Biblioteca y Visor 3D (02-library-3d-viewer.png)
    console.log('Preparando Biblioteca...');
    await h.eval(`document.querySelector('[data-view="library"]').click()`);
    await h.sleep(800);
    // Abrir el visor 3D del pirata o cubo
    await h.eval(`
      const btn3D = [...document.querySelectorAll('.asset-card button')].find(b => b.textContent.includes('Ver en 3D'));
      if (btn3D) btn3D.click();
    `);
    await h.sleep(2500);
    console.log('Capturando: 02-library-3d-viewer.png...');
    await h.screenshot(path.join(outDir, '02-library-3d-viewer.png'));

    // Cerrar modal
    await h.eval(`const modal = document.getElementById('app-modal'); if (modal) modal.click();`);
    await h.sleep(500);

    // 4. CAPTURA: Generar con World Labs Marble (03-generate-worldlabs-tripo.png)
    console.log('Preparando Generar...');
    await h.eval(`document.querySelector('[data-view="generate"]').click()`);
    await h.sleep(800);
    // Cambiar a la pestaña de World Labs
    await h.eval(`
      const wlTab = document.querySelector('[data-gen-tab="worldlabs"]');
      if (wlTab) wlTab.click();
      const promptInput = document.querySelector('#wl-prompt');
      if (promptInput) {
        promptInput.value = 'An ancient abandoned pirate fortress island at golden hour sunset with palm trees, stone battlements, and turquoise ocean water';
      }
    `);
    await h.sleep(600);
    console.log('Capturando: 03-generate-worldlabs-tripo.png...');
    await h.screenshot(path.join(outDir, '03-generate-worldlabs-tripo.png'));

    // 5. CAPTURA: Vista Plan con tareas comprobadas (04-plan-tasks.png)
    console.log('Preparando Vista Plan...');
    await h.eval(`
      const planBtn = document.querySelector('[data-view="plan"]');
      if (planBtn) planBtn.click();
    `);
    await h.sleep(800);
    console.log('Capturando: 04-plan-tasks.png...');
    await h.screenshot(path.join(outDir, '04-plan-tasks.png'));

    console.log('¡Todas las capturas se generaron correctamente en docs/screenshots/!');
  } finally {
    await h.close();
  }
}

run().catch((err) => {
  console.error('Fallo en la captura de pantalla:', err);
  process.exit(1);
});
