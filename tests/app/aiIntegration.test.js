// =============================================================================
// DBV Unreal AI — Tests de lo que la app añade a la conexión a IAs común
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Los tests de `tests/ai/` de la conexión a IAs son CANÓNICOS (vienen de dbv-tauri-starter con
// `scripts/sync-ai.mjs` y no citan requisitos de ninguna app). La trazabilidad de los requisitos
// de DBV Unreal AI sobre esa parte vive aquí, probando lo que esta app pone encima.

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createHelpModal } from '../../src/app/helpModal.js';
import { createAcpSession } from '../../src/ai/acpSession.js';
import { createConnectWizard } from '../../src/ai/connectWizard.js';
import { registerTranslations } from '../../src/ai/i18nBridge.js';
import { AI_TRANSLATIONS } from '../../src/ai/translations.js';

registerTranslations(AI_TRANSLATIONS);

beforeEach(() => {
  document.body.innerHTML = '';
  Element.prototype.scrollIntoView = vi.fn();
});

describe('Centro de Ayuda: la pestaña de IA es la guía común @spec REQ-46, REQ-48', () => {
  function helpModal() {
    document.body.innerHTML = `
      <div id="help-modal" class="modal-overlay hidden">
        <div class="modal-card help-card">
          <button id="help-close">✕</button>
          <nav><button data-help-tab="ue"></button><button data-help-tab="ai"></button></nav>
          <div id="help-content" class="help-body"></div>
        </div>
      </div>`;
    return createHelpModal(document.querySelector('#help-modal'), document.querySelector('#help-content'));
  }

  it('abre la guía «Usar una IA con DBV Unreal AI», ancha y con sus secciones', async () => {
    const help = helpModal();
    help.open('ai');
    const content = document.querySelector('#help-content');
    await vi.waitFor(() => expect(content.querySelector('.ai-guide__body')).not.toBeNull());
    expect(content.textContent).toContain('DBV Unreal AI');
    const anchors = [...content.querySelectorAll('[data-anchor]')].map((node) => node.dataset.anchor);
    expect(anchors.some((anchor) => anchor.startsWith('google-antigravity'))).toBe(true);
    expect(document.querySelector('.help-card').classList.contains('help-card--wide')).toBe(true);
    expect(document.getElementById('ai-styles')).not.toBeNull();
  });

  it('cambiar de pestaña mientras carga no pinta la guía encima de la otra', async () => {
    const help = helpModal();
    help.open('ai');
    document.querySelector('[data-help-tab="ue"]').click();
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(document.querySelector('#help-content .ai-guide__body')).toBeNull();
    expect(document.querySelector('.help-card').classList.contains('help-card--wide')).toBe(false);
  });
});

describe('el agente ACP llega conectado a Unreal @spec REQ-11, REQ-20, RF-IA-02', () => {
  it('session/new recibe el servidor MCP propio de DBV Unreal AI', async () => {
    const sent = [];
    const backend = {
      on: vi.fn(async () => () => {}),
      acpStop: vi.fn(async () => ({ ok: true })),
      acpStart: vi.fn(async () => ({ ok: true })),
      acpRequest: vi.fn(async (method, params) => {
        sent.push({ method, params });
        if (method === 'initialize') return { ok: true, value: { agentInfo: { name: 'agente' } } };
        if (method === 'session/new') return { ok: true, value: { sessionId: 's1' } };
        return { ok: true, value: {} };
      }),
    };
    const server = { name: 'dbv-unreal-ai', command: 'C:/app/dbv-unreal-ai.exe', args: ['--mcp', '--port', '8000'], env: [] };
    const session = createAcpSession({ backend, getRoot: () => 'D:/Proyecto', readText: async () => null, askPermission: async () => null, stageWrite: async () => {}, onUpdate: () => {}, mcpServers: async () => [server] });
    await session.ensureSession({ id: 'claude' });
    expect(sent.find((m) => m.method === 'session/new').params.mcpServers).toEqual([server]);
  });
});

describe('«Conectar una IA» con los agentes por suscripción @spec REQ-09, REQ-43, REQ-44, REQ-47, REQ-49, RF-IA-01', () => {
  function wizardWith(tools, connections = []) {
    document.body.innerHTML = '<div id="host"></div>';
    const backend = {
      aiDetect: () => Promise.resolve({ ok: true, value: { servers: [], tools } }),
      aiProviders: () => Promise.resolve({ ok: true, value: [] }),
      aiConnections: () => Promise.resolve({ ok: true, value: { connections, active: connections[0]?.id ?? null, showAi: true } }),
      aiListAgentModels: vi.fn(() => Promise.resolve({ ok: true, value: [{ id: 'gemini-4-argon', label: 'Gemini 4 Argon', efforts: ['low', 'medium', 'high'], defaultEffort: 'medium' }] })),
      aiSetPreferences: vi.fn(),
      openExternalUrl: vi.fn(),
    };
    const host = document.getElementById('host');
    const wizard = createConnectWizard({ host, backend, onChanged: vi.fn(), onAgent: vi.fn(), notify: vi.fn() });
    return { host, backend, wizard };
  }
  const agentItem = (host, name) => [...host.querySelectorAll('.ai-cat-card--agent .ai-detect__item')].find((li) => li.textContent.includes(name));

  it('Antigravity solo con la CLI agy no se ofrece para conectar y lleva a la guía', async () => {
    const { host, backend, wizard } = wizardWith([{ name: 'agy', path: 'C:/agy.exe' }, { name: 'node', path: 'C:/node.exe' }]);
    await wizard.open();
    await vi.waitFor(() => expect(agentItem(host, 'Antigravity')).toBeTruthy());
    // Las tres categorías de REQ-43: locales, por suscripción y en la nube con clave.
    for (const kind of ['local', 'agent', 'cloud']) expect(host.querySelector('.ai-cat-card--' + kind)).not.toBeNull();
    expect(agentItem(host, 'Antigravity').textContent).toContain('agy_acp_server');
    agentItem(host, 'Antigravity').querySelector('button').click();
    await vi.waitFor(() => expect(document.querySelector('.ai-guide')).not.toBeNull());
    expect(backend.openExternalUrl).not.toHaveBeenCalled();
  });

  it('Antigravity con su servidor ACP, conectado: ⚙️ consulta los modelos al propio CLI', async () => {
    const connection = { id: 'c1', name: 'Antigravity', provider: 'agent', baseUrl: 'acp:antigravity', model: 'antigravity' };
    const { host, backend, wizard } = wizardWith([{ name: 'agy_acp_server', path: 'C:/agy_acp_server.exe' }, { name: 'node', path: 'C:/node.exe' }], [connection]);
    await wizard.open();
    await vi.waitFor(() => expect(agentItem(host, 'Antigravity')?.querySelector('.ai-agent-gear')).toBeTruthy());
    agentItem(host, 'Antigravity').querySelector('.ai-agent-gear').click();
    await vi.waitFor(() => expect(backend.aiListAgentModels).toHaveBeenCalledWith('antigravity'));
  });
});
