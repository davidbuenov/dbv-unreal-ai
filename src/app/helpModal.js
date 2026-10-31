// =============================================================================
// DBV Unreal AI — Centro de Ayuda y Guías integradas
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-46, REQ-48, REQ-59
//
// La pestaña «Guía de IAs» pinta la guía común de todas las apps DBV (`src/ai/guide.js`, texto
// generado desde dbv-specs-ops), no un texto propio: así la ayuda y `docs/IA.md` no se desvían.

import { getLanguage, t } from '../ai/i18nBridge.js';

/** La guía usa los estilos de la IA, que normalmente se cargan con ella: se piden aquí si faltan. */
function ensureAiStyles() {
  if (document.getElementById('ai-styles')) return;
  const link = document.createElement('link');
  link.id = 'ai-styles';
  link.rel = 'stylesheet';
  link.href = 'ai/ai.css';
  document.head.appendChild(link);
}

/** Pinta la guía común en `container` (se carga solo al abrir la pestaña) si la pestaña sigue abierta. */
async function renderAiGuide(container, stillOpen) {
  ensureAiStyles();
  const [{ renderGuide }, backend] = await Promise.all([import('../ai/guide.js'), import('../ai/backend.js')]);
  if (stillOpen()) renderGuide(container, { openExternalUrl: (url) => backend.openExternalUrl(url) });
}

export function createHelpModal(modalElement, contentElement) {
  let activeTab = 'ue';

  const TABS = {
    ue: () => {
      const isEs = getLanguage() === 'es';
      return `
        <div class="help-section">
          <h4>${isEs ? 'Requisitos de Unreal Engine 5.8' : 'Unreal Engine 5.8 Requirements'}</h4>
          <p>${isEs 
            ? 'DBV Unreal AI se comunica de forma bidireccional y nativa mediante el servidor MCP HTTP integrado en el motor (puerto 8000 por defecto).' 
            : 'DBV Unreal AI communicates natively and bidirectionally via the built-in MCP HTTP server in the engine (default port 8000).'}</p>
          <div class="help-box">
            <strong>${isEs ? 'Plugins del motor necesarios:' : 'Required Engine Plugins:'}</strong>
            <p class="mono" style="margin-top: 4px;">ModelContextProtocol, ToolsetRegistry, PythonScriptPlugin, EditorToolset</p>
          </div>
          <p>${isEs
            ? 'Para conectar un proyecto sin editar archivos manualmente, usa el botón <strong>«Preparar proyecto»</strong> en la vista <em>Proyecto</em>. La app activará los plugins, configurará el arranque automático y realizará una copia de seguridad reversible.'
            : 'To prepare a project without manual editing, click <strong>«Prepare project»</strong> in the <em>Project</em> view. The app activates plugins, sets auto-start and creates a reversible backup.'}</p>
        </div>
      `;
    },
    // La pinta `renderAiGuide` (DOM seguro, no HTML en texto): aquí solo el aviso mientras se carga.
    ai: () => `<p class="hint">${getLanguage() === 'es' ? 'Cargando la guía…' : 'Loading the guide…'}</p>`,
    // Tutorial de Convai (REQ-59): qué hace falta y cómo se usa, con los problemas vistos en la prueba del 2026-10-31.
    convai: () => {
      const isEs = getLanguage() === 'es';
      return isEs
        ? `
        <div class="help-section">
          <h4>Convai: personajes que hablan</h4>
          <p>Con Convai, un MetaHuman de tu nivel conversa contigo por voz (o por texto), mueve la boca y hace acciones: seguirte, ir a un sitio o llevarte a él. DBV Unreal AI lo prepara todo desde la conversación.</p>
          <div class="help-box">
            <strong>Qué necesitas</strong>
            <ol>
              <li><strong>Unreal Engine 5.8 o posterior</strong> y el proyecto preparado con la app (vista <em>Proyecto</em> → «Preparar proyecto»).</li>
              <li>El <strong>plugin Convai</strong> instalado desde Fab y activado en el proyecto (<em>Edit → Plugins</em>), con la <strong>sesión iniciada</strong> en su ventana dentro del editor. La app no necesita ninguna clave de Convai.</li>
              <li>Un <strong>personaje creado en el Playground</strong> de convai.com (personalidad, voz e idioma) y su <strong>Character ID</strong>.</li>
              <li>Un <strong>MetaHuman ensamblado</strong> en el nivel: se coloca su Blueprint <span class="mono">BP_&lt;Nombre&gt;</span> (en <span class="mono">/Game/MetaHumans/&lt;Nombre&gt;</span>), no el asset del personaje, que no se puede arrastrar a la escena.</li>
            </ol>
          </div>
          <h4>Pasos</h4>
          <ol>
            <li><strong>Ajustes → Convai</strong>: añade el personaje con un nombre y su Character ID. La vista <em>Proyecto</em> comprueba que el plugin, sus herramientas y la sesión están bien.</li>
            <li>En la conversación pide: <em>«Haz que BP_VictoriaNew sea Victoria con Convai»</em>. La IA lee las guías de Convai, usa el Character ID registrado y te pide permiso en cada paso (componentes, jugador, movimiento y NavMesh).</li>
            <li>Pide <em>«Define las zonas de este nivel»</em>: revisas los sitios propuestos y se crean en <span class="mono">DBV/Zones</span>. El personaje los conoce y puede ir a ellos.</li>
            <li>Guarda el nivel, pulsa <strong>Play</strong> y habla con el micrófono, o pulsa <strong>Intro</strong> para escribir. Prueba «Sígueme», «Ve al mirador norte» o «Llévame al inicio».</li>
          </ol>
          <h4>Problemas conocidos</h4>
          <ul>
            <li><strong>Brazos y piernas estirados:</strong> abre <span class="mono">metahuman_base_skel</span>, activa <em>Show Retargeting Options</em>, clic derecho en <span class="mono">root</span> → <em>Recursively Set Translation Retargeting Skeleton</em>, pon <span class="mono">pelvis</span> en <em>Animation Scaled</em> y guarda.</li>
            <li><strong>«Llévame a…» y no se mueve:</strong> pide a la IA que mejore la descripción de la acción <em>Escort</em>. Si en tu idioma sigue sin moverse, configura el idioma del personaje en el Playground; «Ve a…» usa <em>Move to</em>, que es más directa.</li>
            <li><strong>Un agente ve el editor dos veces:</strong> el asistente «Set Up AI Coding» de Convai escribe un <span class="mono">.mcp.json</span> y un <span class="mono">CLAUDE.md</span> en el proyecto. La vista <em>Proyecto</em> lo avisa; quítalos si usas la app.</li>
            <li><strong>Volver a ensamblar el MetaHuman</strong> regenera su Blueprint y borra la configuración de Convai: repite el paso 2.</li>
            <li>Las acciones de Convai <strong>guardan Blueprints</strong>: «Deshacer este turno» puede no revertirlas. Ten el proyecto en control de versiones.</li>
          </ul>
        </div>`
        : `
        <div class="help-section">
          <h4>Convai: characters that talk</h4>
          <p>With Convai, a MetaHuman in your level talks with you by voice (or text), moves its lips and performs actions: following you, going somewhere or taking you there. DBV Unreal AI sets it all up from the conversation.</p>
          <div class="help-box">
            <strong>What you need</strong>
            <ol>
              <li><strong>Unreal Engine 5.8 or later</strong> and the project prepared with the app (<em>Project</em> view → "Prepare project").</li>
              <li>The <strong>Convai plugin</strong> installed from Fab and enabled in the project (<em>Edit → Plugins</em>), <strong>signed in</strong> in its window inside the editor. The app needs no Convai key.</li>
              <li>A <strong>character created in the Playground</strong> at convai.com (personality, voice and language) and its <strong>Character ID</strong>.</li>
              <li>An <strong>assembled MetaHuman</strong> in the level: place its Blueprint <span class="mono">BP_&lt;Name&gt;</span> (in <span class="mono">/Game/MetaHumans/&lt;Name&gt;</span>), not the character asset, which cannot be dragged into the scene.</li>
            </ol>
          </div>
          <h4>Steps</h4>
          <ol>
            <li><strong>Settings → Convai</strong>: add the character with a name and its Character ID. The <em>Project</em> view checks that the plugin, its tools and the sign-in are fine.</li>
            <li>In the conversation ask: <em>"Make BP_VictoriaNew be Victoria with Convai"</em>. The AI reads the Convai guides, uses the registered Character ID and asks your permission at each step (components, player, movement and NavMesh).</li>
            <li>Ask <em>"Define the zones of this level"</em>: you review the proposed places and they are created in <span class="mono">DBV/Zones</span>. The character knows them and can walk there.</li>
            <li>Save the level, press <strong>Play</strong> and talk with the microphone, or press <strong>Enter</strong> to type. Try "Follow me", "Go to the north lookout" or "Take me to the start".</li>
          </ol>
          <h4>Known issues</h4>
          <ul>
            <li><strong>Stretched arms and legs:</strong> open <span class="mono">metahuman_base_skel</span>, enable <em>Show Retargeting Options</em>, right-click <span class="mono">root</span> → <em>Recursively Set Translation Retargeting Skeleton</em>, set <span class="mono">pelvis</span> to <em>Animation Scaled</em> and save.</li>
            <li><strong>"Take me to…" and it does not move:</strong> ask the AI to improve the description of the <em>Escort</em> action. If it still does not move in your language, set the character language in the Playground; "Go to…" uses <em>Move to</em>, which is more direct.</li>
            <li><strong>An agent sees the editor twice:</strong> the Convai "Set Up AI Coding" assistant writes a <span class="mono">.mcp.json</span> and a <span class="mono">CLAUDE.md</span> in the project. The <em>Project</em> view warns about it; remove them if you use the app.</li>
            <li><strong>Re-assembling the MetaHuman</strong> regenerates its Blueprint and deletes the Convai setup: repeat step 2.</li>
            <li>Convai actions <strong>save Blueprints</strong>: "Undo this turn" may not revert them. Keep the project under version control.</li>
          </ul>
        </div>`;
    },
    shortcuts: () => {
      const isEs = getLanguage() === 'es';
      return `
        <div class="help-section">
          <h4>${isEs ? 'Atajos de Teclado Globales' : 'Global Keyboard Shortcuts'}</h4>
          <table class="help-table">
            <thead>
              <tr><th>${isEs ? 'Atajo' : 'Shortcut'}</th><th>${isEs ? 'Acción' : 'Action'}</th></tr>
            </thead>
            <tbody>
              <tr><td><span class="help-kbd">F1</span></td><td>${isEs ? 'Abrir este Centro de Ayuda' : 'Open this Help Center'}</td></tr>
              <tr><td><span class="help-kbd">Ctrl+Mayús+I</span> / <span class="help-kbd">Cmd+Shift+I</span></td><td>${isEs ? 'Abrir / alternar panel de Conversación' : 'Open / toggle Conversation panel'}</td></tr>
              <tr><td><span class="help-kbd">Esc</span></td><td>${isEs ? 'Cerrar cualquier ventana modal activa' : 'Close any active modal dialog'}</td></tr>
              <tr><td><span class="help-kbd">Ctrl+S</span> / <span class="help-kbd">Cmd+S</span></td><td>${isEs ? 'Guardar en vistas de edición' : 'Save in edit views'}</td></tr>
            </tbody>
          </table>
        </div>
      `;
    },
    links: () => {
      const isEs = getLanguage() === 'es';
      return `
        <div class="help-section">
          <h4>${isEs ? 'Enlaces y Documentación Oficial' : 'Official Links and Documentation'}</h4>
          <p>${isEs ? 'Accede a los recursos y guías detalladas:' : 'Access detailed resources and guides:'}</p>
          <div class="help-box" style="display: grid; gap: 8px;">
            <div>🌐 <a href="https://github.com/davidbuenov/dbv-unreal-ai" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">Repositorio Oficial en GitHub</a></div>
            <div>🤖 <a href="https://antigravity.google/product/antigravity-cli" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">Google Antigravity CLI</a></div>
            <div>⚡ <a href="https://github.com/google-gemini/gemini-cli" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">Gemini CLI (GitHub)</a></div>
            <div>🎨 <a href="https://www.tripo3d.ai" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">Tripo 3D Studio</a></div>
            <div>🌍 <a href="https://www.worldlabs.ai" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">World Labs (Marble)</a></div>
            <div>🗣️ <a href="https://convai.com" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">Convai (Playground)</a></div>
          </div>
        </div>
      `;
    },
  };

  function renderContent() {
    const fn = TABS[activeTab] || TABS.ue;
    contentElement.innerHTML = fn();
    for (const btn of modalElement.querySelectorAll('[data-help-tab]')) {
      btn.classList.toggle('active', btn.dataset.helpTab === activeTab);
    }
    modalElement.querySelector('.help-card')?.classList.toggle('help-card--wide', activeTab === 'ai');
    if (activeTab === 'ai') {
      renderAiGuide(contentElement, () => activeTab === 'ai').catch((error) => {
        contentElement.textContent = String(error?.message ?? error);
      });
    }
  }

  function open(tab) {
    if (tab && TABS[tab]) activeTab = tab;
    renderContent();
    modalElement.classList.remove('hidden');
  }

  function close() {
    modalElement.classList.add('hidden');
  }

  modalElement.querySelector('#help-close')?.addEventListener('click', close);
  for (const btn of modalElement.querySelectorAll('[data-help-tab]')) {
    btn.addEventListener('click', () => {
      activeTab = btn.dataset.helpTab;
      renderContent();
    });
  }

  // Cierre al pulsar el fondo
  modalElement.addEventListener('click', (e) => {
    if (e.target === modalElement) close();
  });

  return { open, close };
}
