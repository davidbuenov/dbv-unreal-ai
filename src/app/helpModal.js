// =============================================================================
// DBV Unreal AI — Centro de Ayuda y Guías integradas
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-46
//

import { getLanguage, t } from '../ai/i18nBridge.js';

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
    ai: () => {
      const isEs = getLanguage() === 'es';
      return `
        <div class="help-section">
          <h4>${isEs ? 'Modalidades de IA disponibles' : 'Available AI Modalities'}</h4>
          <p>${isEs
            ? 'Puedes conectar tres tipos de IA según tu preferencia de coste y privacidad:'
            : 'You can connect three types of AI according to your cost and privacy preferences:'}</p>
          <table class="help-table">
            <thead>
              <tr>
                <th>${isEs ? 'Tipo' : 'Type'}</th>
                <th>${isEs ? 'Ejemplos' : 'Examples'}</th>
                <th>${isEs ? 'Privacidad / Coste' : 'Privacy / Cost'}</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td><strong style="color: #a371f7;">${isEs ? 'Suscripción' : 'Subscription'}</strong></td>
                <td>Claude Code, Antigravity CLI, Gemini CLI, Codex, Copilot CLI</td>
                <td>${isEs ? 'Usa tu suscripción activa (Pro/Plus/Google One) a través de su agente CLI.' : 'Uses your active subscription (Pro/Plus/Google One) via CLI agent.'}</td>
              </tr>
              <tr>
                <td><strong style="color: #d29922;">${isEs ? 'API Cloud' : 'Cloud API'}</strong></td>
                <td>Anthropic Claude, OpenAI, Google AI Studio, OpenRouter</td>
                <td>${isEs ? 'Pago directo por tokens consumidos. Requiere API key custodiada en el sistema.' : 'Pay-per-token directly. Requires API key stored securely.'}</td>
              </tr>
              <tr>
                <td><strong style="color: #3fb950;">${isEs ? 'Local' : 'Local'}</strong></td>
                <td>Ollama (11434), LM Studio (1234)</td>
                <td>${isEs ? '100% privado y gratuito. Nada sale de tu máquina.' : '100% private and free. Nothing leaves your machine.'}</td>
              </tr>
            </tbody>
          </table>
          <p class="help-box" style="margin-top: 6px;">
            ${isEs
              ? '💡 <em>Google Antigravity CLI</em> es el agente oficial para cuentas gratuitas y Google One / Gemini Advanced. Para claves directas de API, utiliza <em>Google AI Studio</em> o <em>Gemini CLI</em>.'
              : '💡 <em>Google Antigravity CLI</em> is the official agent for free and Google One / Gemini Advanced accounts. For direct API keys, use <em>Google AI Studio</em> or <em>Gemini CLI</em>.'}
          </p>
        </div>
      `;
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
