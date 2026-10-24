// =============================================================================
// DBV Unreal AI — Aplicación: vistas, conexión y asistente
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-08, REQ-09, REQ-11, REQ-12, REQ-14, REQ-24, RNF-IA.1
//
// Módulo ES cargado por `main.js` (script clásico). El asistente de IA se carga de forma perezosa
// la primera vez que se abre la Conversación o «Conectar una IA» (RNF-IA.1 adaptado): sin IA, la
// app ya detecta, prepara, genera y gestiona la biblioteca.

import { registerTranslations, t } from '../ai/i18nBridge.js';
import { createApprovalCard } from '../unreal/approvalCard.js';
import { createPermissionMemory } from '../unreal/permissions.js';
import { createUnrealTools, pythonNudge, unrealPrompt } from '../unreal/unrealTools.js';
import { sendToUnreal } from '../unreal/sendToUnreal.js';
import { createChatJobs } from '../generation/chatJobs.js';
import { createGenerationTools } from '../generation/generationTools.js';
import { createActionLog } from './actionLog.js';
import * as api from './backend.js';
import { button, dirName, el } from './dom.js';
import { confirmCost, createGenerateView } from './generateView.js';
import { assetThumbnail, createLibraryView, sendBackend } from './libraryView.js';
import { createProjectView } from './projectView.js';
import { createSettingsView } from './settingsView.js';
import { createState } from './state.js';
import { APP_TRANSLATIONS } from './translations.js';

registerTranslations(APP_TRANSLATIONS);

/** Texto de la píldora de conexión con Unreal. */
export function pillState(connection) {
  const vars = { project: connection?.projectName ?? '', version: connection?.engineVersion ?? '', port: String(connection?.endpoint?.port ?? '') };
  const status = connection?.status ?? 'offline';
  const text = status === 'offline' ? t('unreal.pill.offline') : t(`unreal.pill.${status}`, vars);
  const className = { online: 'pill pill--online', prep: 'pill pill--prep', connecting: 'pill pill--prep pill--busy' }[status] ?? 'pill';
  return { text, className };
}

export function start() {
  const state = createState();
  const $ = (selector) => document.querySelector(selector);

  // ─── Avisos, confirmaciones y modal genérico ───────────────────────
  let toastTimer = null;
  function toast(message, tone) {
    let node = $('#app-toast');
    if (!node) {
      node = el('div', { attrs: { id: 'app-toast', role: 'status', 'aria-live': 'polite' } });
      document.body.append(node);
    }
    node.textContent = message;
    node.className = `app-toast${tone === 'error' ? ' app-toast-error' : ''}`;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => node.classList.add('hidden'), 4200);
  }

  /** Confirmación con botones propios; resuelve con la `key` elegida. El último botón recibe el foco. */
  function confirm(options) {
    return new Promise((resolve) => {
      const modal = $('#ai-confirm-modal');
      const actions = $('#ai-confirm-actions');
      $('#ai-confirm-title').textContent = options.title || '';
      $('#ai-confirm-text').textContent = options.text || '';
      // Lo que se enviará (REQ-27): siempre como texto, nunca como HTML.
      $('#ai-confirm-detail').textContent = options.detail || '';
      $('#ai-confirm-detail').classList.toggle('hidden', !options.detail);
      actions.replaceChildren();
      const onKey = (event) => {
        if (event.key !== 'Escape') return;
        event.stopPropagation();
        finish('cancel');
      };
      function finish(key) {
        modal.classList.add('hidden');
        document.removeEventListener('keydown', onKey, true);
        resolve(key);
      }
      for (const choice of options.choices) {
        const tone = choice.tone === 'primary' ? ' btn-primary' : choice.tone === 'danger' ? ' btn-danger' : '';
        actions.append(el('button', { className: `button${tone}`, text: choice.label, attrs: { type: 'button' }, on: { click: () => finish(choice.key) } }));
      }
      $('#ai-confirm-backdrop').onclick = () => finish('cancel');
      document.addEventListener('keydown', onKey, true);
      modal.classList.remove('hidden');
      actions.lastElementChild?.focus();
    });
  }

  const openModal = (children) => {
    $('#app-modal-card').replaceChildren(...children);
    $('#app-modal').classList.remove('hidden');
  };
  const closeModal = () => $('#app-modal').classList.add('hidden');
  $('#app-modal').addEventListener('click', (event) => event.target.id === 'app-modal' && closeModal());

  // ─── Registro de acciones y gasto ──────────────────────────────────
  const log = createActionLog($('#action-log'), { toast, projectName: () => state.connection?.projectName ?? '' });
  $('#btn-log').addEventListener('click', () => $('#action-log').classList.toggle('hidden'));
  async function refreshSpent() {
    const status = await api.generationStatus();
    const pill = $('#cost-pill');
    pill.classList.toggle('hidden', !status.ok || !status.value.spent);
    if (status.ok) pill.textContent = t('gen.spent', { spent: String(status.value.spent) });
  }

  // ─── Vistas ────────────────────────────────────────────────────────
  const views = {};
  let current = 'project';
  function show(name) {
    current = name;
    for (const panel of document.querySelectorAll('[data-view-panel]')) panel.classList.toggle('hidden', panel.dataset.viewPanel !== name);
    for (const item of document.querySelectorAll('.nav-item')) {
      if (item.dataset.view === name) item.setAttribute('aria-current', 'page');
      else item.removeAttribute('aria-current');
    }
    if (name === 'chat') openAssistant();
    else views[name]?.render();
  }
  for (const item of document.querySelectorAll('.nav-item')) item.addEventListener('click', () => show(item.dataset.view));

  views.project = createProjectView($('#view-project'), { state, toast, openModal, closeModal });
  /** Lo que se ve de un trabajo terminado: la miniatura y el camino a la biblioteca. */
  async function renderJobResult(record, slot) {
    const listed = record.entryId ? await api.libraryList() : null;
    const entry = listed?.ok ? listed.value.find((e) => e.id === record.entryId) : null;
    if (entry) slot.replaceChildren(await assetThumbnail(entry, 'job-thumb'), el('div', { className: 'row' }, button(t('gen.openLibrary'), () => show('library'), 'btn-ghost')));
  }

  /** Nombre corto para un asset con la IA conectada (local o de nube); `null` si no hay o no responde. */
  async function suggestName(text) {
    if (!aiApp) return null;
    const language = t('gen.nameLanguage');
    const reply = await aiApp.complete([
      { role: 'system', content: `Reply with ONLY a short name (1 to 4 words) in ${language} for the 3D asset described by the user. No quotes, no punctuation at the end, no explanations.` },
      { role: 'user', content: text },
    ]);
    const name = String(reply ?? '').split('\n').map((line) => line.trim()).find(Boolean) ?? '';
    return name.replace(/^["'«“]+|["'»”.]+$/g, '').slice(0, 60) || null;
  }

  views.generate = createGenerateView($('#view-generate'), { confirm, toast, onSpent: refreshSpent, openLibrary: () => show('library'), suggestName, renderResult: renderJobResult });
  const chatJobs = createChatJobs({ list: api.generationJobs, renderResult: renderJobResult });
  // Herramientas de Tripo de Rust (las mismas que ve un agente): se piden una vez al cargar la IA.
  let generationSpecs = [];
  views.library = createLibraryView($('#view-library'), { state, confirm, toast });
  views.settings = createSettingsView($('#view-settings'), { toast, state, openConnect: () => loadAi().then((app) => app.openConnect()) });

  function paintPill() {
    const pill = pillState(state.connection);
    const node = $('#unreal-pill');
    node.textContent = pill.text;
    node.className = pill.className;
  }
  state.subscribe((connection) => {
    paintPill();
    if (connection?.status === 'online') aiApp?.onRootChanged(dirName(connection.projectPath));
  });
  document.addEventListener('dbv-lang-changed', () => {
    paintPill();
    views[current]?.render?.();
  });

  // ─── Asistente de IA ───────────────────────────────────────────────
  let aiApp = null;
  let aiLoading = null;
  const memories = new Map();

  function unrealBackend() {
    const endpoint = () => state.connection?.endpoint;
    return {
      listToolsets: () => api.unrealListToolsets(endpoint()),
      describeToolset: (toolset) => api.unrealDescribeToolset(endpoint(), toolset),
      callTool: (toolset, tool, args, timeout) => api.unrealCallTool(endpoint(), toolset, tool, args, timeout),
    };
  }

  function buildHost(renderMarkdownNodes) {
    return {
      elements: { panel: $('#ai-panel-container'), connectBody: $('#ai-connect-body'), editorOverlay: $('#editor-overlay') },
      toast,
      confirm,
      filesChanged() {},
      renderMarkdown(target, markdown) {
        target.replaceChildren(renderMarkdownNodes(markdown));
      },
      getFolderName: () => state.connection?.projectName ?? '',
      getActive: () => null,
      isOpenDocument: () => false,
      getOpenText: () => '',
      isDirty: () => false,
      getVisibleSection: () => null,
      openDocument() {},
      applyToEditor: () => false,
      insertAtCursor: () => false,
      getEditorSelection: () => null,
      replaceRange: () => false,
      showPanel(open) {
        if (open && current !== 'chat') show('chat');
      },
      openConnect() {
        $('#ai-connect-modal').classList.remove('hidden');
      },
      closeConnect() {
        $('#ai-connect-modal').classList.add('hidden');
      },
      onVisibilityChanged() {},
      // ── Extensiones de DBV Unreal AI ──
      followUp: (reply) => (state.connection?.status === 'online' ? pythonNudge(reply) : null),
      promptDomain: (_language, { agent = false } = {}) => {
        const connection = state.connection;
        const base = unrealPrompt({ projectName: connection?.projectName ?? '', engineVersion: connection?.engineVersion ?? '', agent });
        return connection?.status === 'online' ? base : `${base}\n\nNo Unreal project is connected right now: if the user asks to act on Unreal, tell them to connect one in the Project view.`;
      },
      extraTools: ({ panel, conversationId }) => {
        if (!memories.has(conversationId)) memories.set(conversationId, createPermissionMemory());
        const askApproval = (request) => {
          const card = createApprovalCard(request);
          panel.addNode(card.node);
          return card.choice;
        };
        const unreal = state.connection?.status === 'online' ? createUnrealTools({ backend: unrealBackend(), askApproval, onAction: log.add, permissions: memories.get(conversationId) }) : [];
        const generation = createGenerationTools({
          specs: generationSpecs,
          status: api.generationStatus,
          prepare: api.generationPrepare,
          launch: (tool, args) => api.launch(tool, args).finally(refreshSpent),
          waitJob: (jobId, timeout) => api.waitJob(jobId, timeout).finally(refreshSpent),
          showJobs: (ids) => chatJobs.track(panel, ids),
          operations: api.generationOperations,
          balance: api.generationBalance,
          rename: api.libraryRename,
          listLibrary: api.libraryList,
          send: (entry) => (state.connection?.status === 'online' ? sendToUnreal({ backend: sendBackend(state.connection) }, entry, { project: state.connection.projectPath }) : Promise.resolve({ ok: false, error: 'no Unreal project connected' })),
          askApproval,
          confirmCost: (request) => confirmCost(confirm, request),
        });
        return [...unreal, ...generation];
      },
      // Un agente genera en su propio proceso (`--mcp`): sus trabajos aparecen en la conversación (REQ-27).
      onTurnStart: ({ panel, agent, isBusy }) => agent && chatJobs.follow(panel, isBusy),
      mcpServers: async () => {
        const command = await api.appMcpCommand(state.connection?.endpoint?.port ?? 8000);
        return command.ok ? [{ name: 'dbv-unreal-ai', command: command.value.command, args: command.value.args, env: [] }] : [];
      },
    };
  }

  function loadAi() {
    if (!aiLoading) {
      aiLoading = Promise.all([import('../ai/entry.js'), import('../ai/markdown.js'), api.generationToolSpecs()])
        .then(([entry, markdown, specs]) => {
          generationSpecs = specs.ok ? specs.value : [];
          return entry.createAi(buildHost(markdown.renderMarkdown));
        })
        .then((app) => {
          aiApp = app;
          if (state.connection?.projectPath) app.onRootChanged(dirName(state.connection.projectPath));
          return app;
        })
        .catch((error) => {
          aiLoading = null;
          console.error('[ai]', error);
          toast(t('ai.loadFailed'), 'error');
          throw error;
        });
    }
    return aiLoading;
  }

  function openAssistant() {
    loadAi()
      .then((app) => {
        if (!app.isReady()) app.openConnect();
        else app.focusInput();
      })
      .catch(() => {});
  }

  $('#ai-connect-close').addEventListener('click', () => $('#ai-connect-modal').classList.add('hidden'));
  $('#ai-connect-backdrop').addEventListener('click', () => $('#ai-connect-modal').classList.add('hidden'));
  document.addEventListener('keydown', (event) => {
    const mod = event.ctrlKey || event.metaKey;
    if (event.key === 'Escape') {
      $('#ai-connect-modal').classList.add('hidden');
      closeModal();
    } else if (mod && event.shiftKey && event.key.toLowerCase() === 'i') {
      event.preventDefault();
      show('chat');
    }
  });

  refreshSpent();
  views.project.autoConnect();
  return { dispose: () => aiApp?.dispose?.(), state };
}
