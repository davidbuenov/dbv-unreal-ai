// =============================================================================
// DBV Unreal AI — Aplicación: vistas, conexión y asistente
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-08, REQ-09, REQ-11, REQ-12, REQ-14, REQ-24, RNF-IA.1, REQ-34, REQ-35, REQ-36
//
// Módulo ES cargado por `main.js` (script clásico). El asistente de IA se carga de forma perezosa
// la primera vez que se abre la Conversación o «Conectar una IA» (RNF-IA.1 adaptado): sin IA, la
// app ya detecta, prepara, genera y gestiona la biblioteca.

import { getLanguage, registerTranslations, t } from '../ai/i18nBridge.js';
import { createApprovalCard } from '../unreal/approvalCard.js';
import { createPermissionMemory } from '../unreal/permissions.js';
import { createUnrealTools, describeCapture, pythonNudge, unrealPrompt } from '../unreal/unrealTools.js';
import { sendToUnreal } from '../unreal/sendToUnreal.js';
import { createChatJobs } from '../generation/chatJobs.js';
import { createGenerationTools } from '../generation/generationTools.js';
import { mountInlineViewer, viewableFile } from '../generation/preview.js';
import { createMemoryTools, createPlanCard, createProjectMemory, createSaveCard, memoryPrompt, rememberRequest } from '../memory/memory.js';
import { createUsageCounter, formatUsage, withUsage } from '../unreal/toolUsage.js';
import { createZoneTools, createZonesCard } from '../unreal/zones.js';
import { createActionLog } from './actionLog.js';
import * as api from './backend.js';
import { createCodeCompiler } from './codeCompile.js';
import { improvePrompt } from '../generation/promptImprover.js';
import { createRecipeTools, localized, recipesCard } from './recipes.js';
import { autoAllow } from '../unreal/agentPermissions.js';
import { button, dirName, el } from './dom.js';
import { confirmCost, createGenerateView } from './generateView.js';
import { assetThumbnail, createLibraryView, sendBackend } from './libraryView.js';
import { createPlanView } from './planView.js';
import { createProjectView } from './projectView.js';
import { createSettingsView } from './settingsView.js';
import { createState } from './state.js';
import { APP_TRANSLATIONS } from './translations.js';

registerTranslations(APP_TRANSLATIONS);

/** Imágenes que se pueden pegar o soltar en la conversación: las que admite `ai_save_clipboard_image` (REQ-68). */
const PASTABLE_IMAGE = /^image\/(png|jpe?g|webp)$/i;
/** Tope de un documento soltado: el mismo que `MAX_TEXT_BYTES` de `attachments.rs`. */
const MAX_DROPPED_TEXT = 256 * 1024;

/** Contenido en base64 (sin el prefijo `data:…;base64,`) de un fichero pegado o soltado; `''` si no se pudo leer. */
function readBase64(file) {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = () => {
      const raw = String(reader.result ?? '');
      const cut = raw.indexOf(';base64,');
      resolve(cut >= 0 ? raw.slice(cut + 8) : raw);
    };
    reader.onerror = () => resolve('');
    reader.readAsDataURL(file);
  });
}

/**
 * Adjuntos de los ficheros pegados (`Ctrl+V`) o soltados en la conversación: cada imagen se guarda en disco con
 * `saveImage` (para que tenga ruta real y Tripo pueda usarla) y cada documento de texto se lee aquí. Un binario que no
 * es una imagen admitida se descarta avisando con `onError`. @spec REQ-68
 */
export async function pastedAttachments(files, { saveImage, onError }) {
  const results = [];
  for (const file of files ?? []) {
    if (PASTABLE_IMAGE.test(file.type)) {
      const saved = await saveImage(file.type, await readBase64(file), file.name || null);
      if (saved.ok) results.push(saved.value);
      else onError(saved.error.message);
    } else if (typeof file.text === 'function') {
      const content = await file.text().catch(() => '');
      const name = file.name || 'documento.txt';
      // Un cero en el primer bloque es un binario (un .gif, un .uasset): lo mismo que rechaza `read_attachment`.
      if (content.slice(0, 8192).includes('\0')) onError(t('chat.dropNotText', { name }));
      else if (content) results.push({ kind: 'text', path: name, name, content: content.slice(0, MAX_DROPPED_TEXT), truncated: content.length > MAX_DROPPED_TEXT });
    }
  }
  return results;
}

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

  // `onClose`: lo que hay que liberar al cerrar (el contexto WebGL del visor 3D). `wide`: ventana ancha.
  let modalClosed = null;
  const openModal = (children, { onClose, wide = false } = {}) => {
    modalClosed?.();
    modalClosed = onClose ?? null;
    const nodes = (Array.isArray(children) ? children : [children]).flat().filter((c) => c !== null && c !== undefined && c !== false);
    $('#app-modal-card').replaceChildren(...nodes);
    $('#app-modal-card').classList.toggle('modal-card--wide', wide);
    $('#app-modal').classList.remove('hidden');
  };
  const closeModal = () => {
    $('#app-modal').classList.add('hidden');
    const closed = modalClosed;
    modalClosed = null;
    closed?.();
  };
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

  // Memoria del proyecto conectado (REQ-34): su resumen va como ficha de contexto sin esperar al backend.
  // Al cambiar (se guardó algo desde la conversación) se repinta el Plan si está a la vista.
  const projectMemory = createProjectMemory({ api, getProject: () => state.connection?.projectPath ?? null, onChange: () => current === 'plan' && views.plan?.render() });
  views.project = createProjectView($('#view-project'), { state, toast, openModal, closeModal, onMemoryChanged: () => projectMemory.refresh(), showView: (name) => show(name) });
  /** Lo que se ve de un trabajo terminado: el modelo girando (o su miniatura) y el camino a la biblioteca (REQ-31). */
  async function renderJobResult(record, slot) {
    const listed = record.entryId ? await api.libraryList() : null;
    const entry = listed?.ok ? listed.value.find((e) => e.id === record.entryId) : null;
    if (!entry) return;
    const view = el('div', { className: 'job-view' });
    slot.replaceChildren(view, el('div', { className: 'row' }, button(t('gen.openLibrary'), () => show('library'), 'btn-ghost')));
    if (viewableFile(entry)) await mountInlineViewer(view, entry, (e) => assetThumbnail(e, 'job-thumb'));
    else view.replaceChildren(await assetThumbnail(entry, 'job-thumb'));
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

  views.generate = createGenerateView($('#view-generate'), {
    confirm,
    toast,
    onSpent: refreshSpent,
    openLibrary: () => show('library'),
    suggestName,
    renderResult: renderJobResult,
    // REQ-66: «Mejorar con IA» con la IA conectada, siempre fuera de la conversación (agente: sesión ACP aparte).
    improvePrompt: async (service, idea) => improvePrompt(await loadAi().catch(() => null), service, idea),
    improveAvailable: async () => Boolean((await loadAi().catch(() => null))?.activeConnection?.()),
  });
  const chatJobs = createChatJobs({ list: api.generationJobs, renderResult: renderJobResult });
  // Herramientas de Tripo y de memoria de Rust (las mismas que ve un agente): se piden una vez al cargar la IA.
  let generationSpecs = [];
  let memorySpecs = [];
  // Zonas del nivel (REQ-57), definidas en Rust como las del MCP propio.
  let zoneSpecs = [];
  // Recetas guiadas (REQ-23): definiciones de Rust, como las del MCP propio.
  let recipeSpecs = [];
  // Guías de Unreal sin red (REQ-25): la herramienta de Rust, como en el MCP propio.
  let guidesSpec = null;
  // Consultas del servidor MCP de la app: un agente no pide permiso para ellas (REQ-12).
  let readOnlyTools = [];
  // Personajes de Convai registrados en Ajustes (REQ-58): misma definición que en el MCP propio.
  let convaiCharactersSpec = null;
  // Lo que se lanza desde la Biblioteca (riggear, animar, imagen → 3D) se sigue en Generar, con su consola.
  views.library = createLibraryView($('#view-library'), {
    state,
    confirm,
    toast,
    openModal,
    closeModal,
    onLaunched: () => {
      refreshSpent();
      show('generate');
      views.generate.watch();
    },
  });
  // Vista Plan (REQ-36): «Planificar» y «Hacer esta tarea» van a la conversación.
  views.plan = createPlanView($('#view-plan'), {
    state,
    toast,
    openModal,
    closeModal,
    showView: (name) => show(name),
    onChanged: () => projectMemory.refresh(),
    ask: (text) => {
      show('chat');
      loadAi()
        .then((app) => (app.isReady() ? app.ask(text) : app.openConnect()))
        .catch(() => {});
    },
    renderMarkdown: () => import('../ai/markdown.js').then((module) => (markdown) => module.renderMarkdown(markdown)).catch(() => null),
  });
  views.settings = createSettingsView($('#view-settings'), { toast, state, openConnect: () => loadAi().then((app) => app.openConnect()) });

  function paintPill() {
    const node = $('#unreal-pill');
    const conns = (state.connections ?? []).filter((c) => c && c.status === 'online');
    if (conns.length > 1) {
      const active = state.connection;
      const select = el(
        'select',
        {
          attrs: { 'aria-label': t('unreal.selectActiveProject') },
          style: 'background: transparent; border: none; color: inherit; font: inherit; cursor: pointer; padding: 0 4px; outline: none;',
        },
        ...conns.map((c) => {
          const opt = el('option', { text: `${c.projectName} (:${c.endpoint?.port ?? 8000})`, attrs: { value: c.projectPath || c.projectName } });
          if ((c.projectPath || c.projectName) === (active?.projectPath || active?.projectName)) opt.selected = true;
          return opt;
        }),
      );
      select.addEventListener('change', () => state.setActiveProject(select.value));
      node.replaceChildren(select);
      node.className = 'pill pill--online';
    } else {
      const pill = pillState(state.connection);
      node.textContent = pill.text;
      node.className = pill.className;
    }
  }
  let memoryProject = null;
  state.subscribe((connection) => {
    paintPill();
    if ((connection?.projectPath ?? null) !== memoryProject) {
      memoryProject = connection?.projectPath ?? null;
      projectMemory.refresh();
    }
    if (connection?.status === 'online') aiApp?.onRootChanged(dirName(connection.projectPath));
  });
  document.addEventListener('dbv-lang-changed', () => {
    paintPill();
    views[current]?.render?.();
  });

  // ─── Asistente de IA ───────────────────────────────────────────────
  let aiApp = null;
  // REQ-22: compilar el C++ que la IA cambió, al aceptar su propuesta.
  const codeCompiler = createCodeCompiler({ getConnection: () => state.connection, getAi: () => aiApp, reconnect: (path) => views.project.connectProject(path) });
  let aiLoading = null;
  const memories = new Map();

  function unrealBackend() {
    const endpoint = () => state.connection?.endpoint;
    return {
      listToolsets: () => api.unrealListToolsets(endpoint()),
      describeToolset: (toolset) => api.unrealDescribeToolset(endpoint(), toolset),
      callTool: (toolset, tool, args, timeout) => api.unrealCallTool(endpoint(), toolset, tool, args, timeout),
      captureViewport: async (annotate, detail) => {
        const captured = await api.unrealCaptureViewport(endpoint(), annotate, detail);
        return captured.ok ? { ok: true, value: { ...captured.value, text: describeCapture(captured.value) } } : captured;
      },
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
        const base = unrealPrompt({ projectName: connection?.projectName ?? '', engineVersion: connection?.engineVersion ?? '', agent, convai: connection?.status === 'online' && Boolean(connection?.convai) });
        const memory = connection?.projectPath ? `\n\n${memoryPrompt({ active: projectMemory.isActive(), agent, summary: projectMemory.status?.summary ?? '' })}` : '';
        return connection?.status === 'online' ? `${base}${memory}` : `${base}${memory}\n\nNo Unreal project is connected right now: if the user asks to act on Unreal, tell them to connect one in the Project view.`;
      },
      // Memoria del proyecto (REQ-34, REQ-35): ficha de contexto y botón «Recordar».
      contextNotes: () => projectMemory.contextNotes(),
      rememberRequest: (language) => rememberRequest(language),
      // Menú «Recetas» (REQ-23): elegir una se la pide a la IA con sus pasos y su comprobación final.
      openRecipes: async ({ panel, ask }) => {
        const listed = await api.recipesAll();
        if (!listed.ok) return toast(listed.error.message, 'error');
        return panel.addNode(recipesCard(listed.value, async (recipe) => {
          const prompt = await api.recipesPrompt(recipe.id);
          if (prompt.ok) await ask(prompt.value, { label: `📋 ${localized(recipe.title, getLanguage())}` });
        }));
      },
      // Prueba autónoma en Play (REQ-82): pide al asistente ejecutar `unreal_playtest` y analizar telemetría y captura.
      runPlaytest: async ({ ask }) => {
        if (state.connection?.status !== 'online') return toast(t('unreal.pill.offline'), 'error');
        return ask(t('unreal.playtestPrompt'), { label: `▶️ ${t('ai.playtest')}` });
      },
      beforeTurn: async () => {
        let startUndo = null;
        if (state.connection?.status === 'online') {
          try {
            startUndo = await api.unrealGetUndoCount(state.connection.endpoint);
          } catch {
            startUndo = null;
          }
        }
        // Lo que devuelven las herramientas en el turno, para el contador del pie (REQ-51).
        const result = { startUndo, importedAssets: [], usage: createUsageCounter() };
        return result;
      },
      afterTurn: async (turnContext) => {
        let undoInfo = null;
        if (state.connection?.status === 'online' && turnContext?.startUndo != null) {
          try {
            const endUndo = await api.unrealGetUndoCount(state.connection.endpoint);
            if (endUndo != null && endUndo > turnContext.startUndo) {
              undoInfo = {
                count: endUndo - turnContext.startUndo,
                expectedCurrent: endUndo,
                importedAssets: turnContext.importedAssets ?? [],
                undone: false,
              };
            }
          } catch {
            undoInfo = null;
          }
        }
        const usage = turnContext?.usage?.summary();
        if (usage?.calls) undoInfo = { ...(undoInfo ?? { count: 0 }), usage };
        if (undoInfo && turnContext?.convaiSaved) undoInfo.convaiSaved = true;
        return undoInfo;
      },
      // Un agente llama a las herramientas desde su `--mcp`: el uso lo dejó ese proceso en `tool-usage.jsonl` (REQ-51).
      formatUsage,
      // Con Convai en el editor, una sugerencia más al empezar (REQ-56, receta inicial de REQ-23).
      extraSuggestions: () => (state.connection?.status === 'online' && state.connection?.convai ? [t('unreal.suggestConvai')] : []),
      afterAgentTurn: async ({ startedAt }) => {
        const project = state.connection?.projectPath;
        const read = project ? await api.toolUsageSince(project, startedAt) : null;
        return read?.ok && read.value.calls ? { count: 0, usage: read.value } : null;
      },
      onUndoTurn: async (undoInfo, { setDone, setWarning }) => {
        if (state.connection?.status !== 'online') {
          setWarning(t('unreal.pill.offline'));
        } else {
          const res = await api.unrealUndoTurn(state.connection.endpoint, undoInfo.count, undoInfo.expectedCurrent);
          if (res.success) {
            setDone();
            toast(t('unreal.undoDone'));
            if (undoInfo.importedAssets?.length) {
              const del = await confirm(t('unreal.undoDeleteAssets'));
              if (del) {
                for (const assetId of undoInfo.importedAssets) {
                  await api.libraryRemove(assetId);
                }
                await refreshLibrary();
              }
            }
          } else {
            setWarning(res.error || t('unreal.undoMismatch'));
          }
        }
      },
      onProposalApplied: (paths) => codeCompiler.afterApply(paths),
      autoAllowPermission: (params) => autoAllow(params, readOnlyTools),
      // REQ-22: la IA solo propone cambios donde deja la regla de Rust (la misma que aplica al escribir).
      checkProposalPath: async (path) => {
        const problem = await api.unrealCodePathProblem(path);
        return problem.ok ? problem.value : null;
      },
      extraTools: ({ panel, conversationId, vision = false, turnContext = null }) => {
        if (!memories.has(conversationId)) memories.set(conversationId, createPermissionMemory());
        const askApproval = (request) => {
          // Las acciones de Convai guardan Blueprints: «Deshacer este turno» lo avisará (REQ-56, R-27).
          if (turnContext && /Convai/i.test(String(request.toolset ?? ''))) turnContext.convaiSaved = true;
          const card = createApprovalCard(request);
          panel.addNode(card.node);
          return card.choice;
        };
        // La captura se ve en la conversación como miniatura (pulsar la abre a tamaño completo).
        const showImage = (capture) => {
          const src = `data:${capture.mime};base64,${capture.base64}`;
          const thumb = el('img', { className: 'capture-thumb', attrs: { src, alt: t('unreal.capture'), title: t('unreal.capture') } });
          thumb.addEventListener('click', () => openModal([el('img', { className: 'capture-full', attrs: { src, alt: t('unreal.capture') } })], { wide: true }));
          panel.addNode(thumb);
        };
        const unreal = state.connection?.status === 'online' ? createUnrealTools({ backend: unrealBackend(), askApproval, onAction: log.add, permissions: memories.get(conversationId), vision, showImage }) : [];
        const generation = createGenerationTools({
          specs: generationSpecs,
          status: api.generationStatus,
          prepare: api.generationPrepare,
          launch: (tool, args) => api.launch(tool, args).finally(refreshSpent),
          waitJob: (jobId, timeout) => api.waitJob(jobId, timeout).finally(refreshSpent),
          showJobs: (ids) => chatJobs.track(panel, ids),
          // Antes de pedir permiso para llevarlo a Unreal, el modelo se ve girando en la conversación (REQ-31).
          showPreview: async (entry) => {
            if (!viewableFile(entry)) return;
            const slot = el('div', { className: 'job-card' });
            panel.addNode(slot);
            await mountInlineViewer(slot, entry, (e) => assetThumbnail(e, 'job-thumb'));
          },
          operations: api.generationOperations,
          balance: api.generationBalance,
          worldlabsBalance: api.generationWorldLabsBalance,
          rename: api.libraryRename,
          listLibrary: api.libraryList,
          send: async (entry, { metahuman = '' } = {}) => {
            let sendResult = { ok: false, error: 'no Unreal project connected' };
            if (state.connection?.status === 'online') {
              sendResult = await sendToUnreal({ backend: sendBackend(state.connection) }, entry, { project: state.connection.projectPath, metahuman });
              if (sendResult.ok && entry?.id && turnContext?.importedAssets) {
                turnContext.importedAssets.push(entry.id);
              }
            }
            return sendResult;
          },
          askApproval,
          confirmCost: (request) => confirmCost(confirm, request),
        });

        // Varias entradas por propuesta y sin límite por turno (REQ-50): la tarjeta las enseña juntas.
        const memoryProject = state.connection?.projectPath;
        const memory = memoryProject
          ? createMemoryTools({
              specs: memorySpecs,
              memory: projectMemory,
              askSave: (request) => {
                const card = createSaveCard(request);
                panel.addNode(card.node);
                return card.choice;
              },
              checkEntries: (entries) => api.memoryCheckEntries(memoryProject, entries),
              preview: api.memoryPlanPreview,
              askPlan: (preview) => {
                const card = createPlanCard(preview);
                panel.addNode(card.node);
                return card.choice;
              },
              applyTasks: (change) => projectMemory.applyTasks(change),
              updateTask: (title, done) => api.memoryTaskUpdate(state.connection.projectPath, state.connection?.status === 'online' ? state.connection.endpoint : null, title, done, true),
            })
          : [];
        // Zonas: la persona las revisa en una tarjeta y aceptar es la aprobación (REQ-57, D29).
        const zones =
          state.connection?.status === 'online'
            ? createZoneTools({
                specs: zoneSpecs,
                preview: api.zonesPreview,
                apply: (list) => api.zonesApply(state.connection.endpoint, list),
                list: () => api.zonesList(state.connection.endpoint),
                askZones: (list, existing) => {
                  const card = createZonesCard(list, existing);
                  panel.addNode(card.node);
                  return card.choice;
                },
              })
            : [];
        const convai =
          state.connection?.status === 'online' && state.connection?.convai && convaiCharactersSpec
            ? [{ name: convaiCharactersSpec.name, description: convaiCharactersSpec.description, parameters: { ...convaiCharactersSpec.inputSchema, additionalProperties: false }, label: () => 'Convai', run: async () => {
                const listed = await api.convaiCharactersText();
                return listed.ok ? listed.value : `error: ${listed.error.message ?? listed.error}`;
              } }]
            : [];
        const recipes = createRecipeTools({ specs: recipeSpecs, run: (name, args) => api.recipesRunTool(state.connection?.status === 'online' ? state.connection.endpoint : null, name, args) });
        const guides = guidesSpec
          ? [{ name: guidesSpec.name, description: guidesSpec.description, parameters: { ...guidesSpec.inputSchema, additionalProperties: false }, label: (args) => t('guides.searching', { query: args?.query ?? '' }), run: async (args) => {
              const found = await api.guidesSearch(String(args?.query ?? ''), args?.limit ?? null);
              return found.ok ? found.value : `error: ${found.error.message ?? found.error}`;
            } }]
          : [];
        return withUsage([...unreal, ...generation, ...memory, ...zones, ...convai, ...recipes, ...guides], turnContext?.usage);
      },
      // Imágenes y documentos de referencia de cualquier carpeta (REQ-32): el diálogo del sistema y Rust los leen.
      pickAttachments: async () => {
        const picked = await window.__TAURI__.dialog.open({
          multiple: true,
          filters: [
            { name: t('ai.attachKinds'), extensions: ['png', 'jpg', 'jpeg', 'webp', 'md', 'txt', 'json', 'csv', 'py', 'ini', 'cpp', 'h', 'cs'] },
            { name: '*', extensions: ['*'] },
          ],
        });
        const read = await Promise.all([picked ?? []].flat().map((path) => api.aiReadAttachment(String(path))));
        for (const failed of read.filter((r) => !r.ok)) toast(failed.error.message, 'error');
        return read.filter((r) => r.ok).map((r) => r.value);
      },
      // Imágenes pegadas desde el portapapeles (`Ctrl+V`) o arrastradas al panel (REQ-68).
      pasteAttachments: (files) => pastedAttachments(files, { saveImage: api.aiSaveClipboardImage, onError: (message) => toast(message, 'error') }),
      // Capturar viewport de Unreal Engine y adjuntarlo como imagen (REQ-73).
      captureViewportAttachment: async () => {
        if (state.connection?.status !== 'online') {
          toast(t('unreal.pill.offline'), 'error');
          return null;
        }
        const captured = await api.unrealCaptureViewport(state.connection.endpoint, false, 'high');
        if (!captured.ok) {
          toast(captured.error.message, 'error');
          return null;
        }
        const mime = captured.value.mime || 'image/png';
        const saved = await api.aiSaveClipboardImage(mime, captured.value.base64, `viewport_${Date.now()}.png`);
        if (!saved.ok) {
          toast(saved.error.message, 'error');
          return null;
        }
        return saved.value;
      },
      // Un agente genera en su propio proceso (`--mcp`): sus trabajos aparecen en la conversación (REQ-27).
      onTurnStart: ({ panel, agent, isBusy }) => agent && chatJobs.follow(panel, isBusy),
      mcpServers: async () => {
        const command = await api.appMcpCommand(state.connection?.endpoint?.port ?? 8000, state.connection?.projectPath ?? null);
        return command.ok ? [{ name: 'dbv-unreal-ai', command: command.value.command, args: command.value.args, env: [] }] : [];
      },
    };
  }

  function loadAi() {
    if (!aiLoading) {
      aiLoading = Promise.all([import('../ai/entry.js'), import('../ai/markdown.js'), api.generationToolSpecs(), api.memoryToolSpecs(), projectMemory.refresh(), api.zonesToolSpecs(), api.convaiCharactersSpec(), api.recipesToolSpecs(), api.guidesToolSpec(), api.appMcpReadOnlyTools()])
        .then(([entry, markdown, specs, memory, , zones, characters, recipes, guides, readOnly]) => {
          readOnlyTools = readOnly?.ok && Array.isArray(readOnly.value) ? readOnly.value : [];
          // Un backend que no conoce el comando devuelve `null`: sin estas comprobaciones, la IA no cargaría (lección de la v0.2.0).
          recipeSpecs = recipes?.ok && Array.isArray(recipes.value) ? recipes.value : [];
          guidesSpec = guides?.ok && guides.value?.name ? guides.value : null;
          convaiCharactersSpec = characters?.ok ? characters.value : null;
          generationSpecs = specs.ok ? specs.value : [];
          memorySpecs = memory.ok ? memory.value : [];
          zoneSpecs = zones?.ok ? zones.value : [];
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

  // Con `dragDropEnabled: false` (necesario en Windows para el arrastre HTML5 del panel, REQ-68), un fichero soltado
  // fuera del panel haría navegar al WebView hasta él: aquí se descarta.
  window.addEventListener('dragover', (event) => event.preventDefault());
  window.addEventListener('drop', (event) => event.preventDefault());

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
