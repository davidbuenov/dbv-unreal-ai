// =============================================================================
// DBV Unreal AI — Centro de Ayuda y Guías integradas
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-46, REQ-48
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
