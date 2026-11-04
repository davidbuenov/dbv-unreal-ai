// =============================================================================
// DBV Unreal AI — Centro de Ayuda y Guías integradas
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-46, REQ-48, REQ-59, REQ-64
//
// La pestaña «Guía de IAs» pinta la guía común de todas las apps DBV (`src/ai/guide.js`, texto
// generado desde dbv-specs-ops), no un texto propio: así la ayuda y `docs/IA.md` no se desvían.

import { getLanguage } from '../ai/i18nBridge.js';
import { HELP_TABS, helpTab } from './helpContent.js';

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

  // Contenido por idioma (REQ-64): `helpContent.js`.
  const TABS = Object.fromEntries(HELP_TABS.map((tab) => [tab, () => helpTab(tab, getLanguage())]));

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

/** Dispara el evento global para abrir el Centro de Ayuda en la pestaña indicada. */
export function openHelp(tab) {
  document.dispatchEvent(new CustomEvent('dbv-open-help', { detail: { tab } }));
}
