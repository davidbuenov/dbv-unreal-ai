// =============================================================================
// DBV Unreal AI — Puente IPC de Unreal, generación y biblioteca
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Mismo contrato que `src/ai/backend.js`: todo devuelve `{ok, value}` o `{ok: false, error}`,
// nunca una excepción. Ningún comando devuelve una clave de API (solo `hasKey`).

import { call, on } from '../ai/backend.js';

export { on };

// ─── Unreal ──────────────────────────────────────────────────────────
export const unrealEngines = () => call('unreal_engines');
export const unrealEditors = () => call('unreal_editors');
export const unrealReadProject = (path) => call('unreal_read_project', { path });
export const unrealRecentProjects = () => call('unreal_recent_projects');
export const unrealRememberProject = (path) => call('unreal_remember_project', { path });
export const unrealOpenProject = (path) => call('unreal_open_project', { path });
export const unrealConnect = (endpoint) => call('unreal_connect', { endpoint });
export const unrealListToolsets = (endpoint) => call('unreal_list_toolsets', { endpoint });
export const unrealDescribeToolset = (endpoint, toolset) => call('unreal_describe_toolset', { endpoint, toolset });
export const unrealCallTool = (endpoint, toolset, tool, args, timeoutSecs) => call('unreal_call_tool', { endpoint, toolset, tool, arguments: args ?? {}, timeoutSecs: timeoutSecs ?? null });
export const unrealReadiness = (path) => call('unreal_readiness', { path });
export const unrealPreparePlan = (path, variant, port) => call('unreal_prepare_plan', { path, variant, port: port ?? null });
export const unrealPrepareApply = (path, variant, port) => call('unreal_prepare_apply', { path, variant, port: port ?? null });
export const unrealPrepareRevert = (path, backup) => call('unreal_prepare_revert', { path, backup });
export const appMcpCommand = (port, project = null) => call('app_mcp_command', { port, project });
/** Imagen (base64) o documento que el usuario eligió para adjuntar a la conversación (REQ-32). */
export const aiReadAttachment = (path) => call('ai_read_attachment', { path });
/** Diagnóstico sin secretos para una incidencia (RNF-08). */
export const appDiagnostics = () => call('app_diagnostics');

// ─── Generación y biblioteca ─────────────────────────────────────────
export const generationStatus = () => call('generation_status');
export const generationSetKey = (service, key) => call('generation_set_key', { service, key });
export const generationDeleteKey = (service) => call('generation_delete_key', { service });
export const generationSetBudget = (credits) => call('generation_set_budget', { credits });
/** Herramientas de Tripo para el asistente integrado: las mismas que ve un agente por MCP (D10). */
export const generationToolSpecs = () => call('generation_tool_specs');
/** Qué hará una herramienta y cuánto cuesta, sin gastar (tarjeta de coste). */
export const generationPrepare = (tool, args) => call('generation_prepare', { tool, args });
export const generationLaunch = (tool, args) => call('generation_launch', { tool, args, confirmCost: true });
export const generationOperations = (operation) => call('generation_operations', { operation: operation || null });
export const generationBalance = () => call('generation_balance');
export const generationSuggestName = (prompt) => call('generation_suggest_name', { prompt });
export const generationPresets = () => call('generation_presets');
/** Trabajos de la sesión (también los de un agente), con su consola. */
export const generationJobs = () => call('generation_jobs');
export const generationCancel = (jobId) => call('generation_cancel', { jobId });
export const libraryList = () => call('library_list');
export const libraryRemove = (id) => call('library_remove', { id });
export const libraryFilePath = (id, file) => call('library_file_path', { id, file });
export const libraryRecordSent = (id, project, assetPath) => call('library_record_sent', { id, project, assetPath });
export const libraryLocation = () => call('library_location');
export const librarySetLocation = (path, copyExisting) => call('library_set_location', { path: path || null, copyExisting: Boolean(copyExisting) });
export const libraryImport = (paths) => call('library_import', { paths });
export const libraryRename = (id, name) => call('library_rename', { id, name });
export const libraryEntryDir = (id) => call('library_entry_dir', { id });
export const librarySaveThumbnail = (id, png) => call('library_save_thumbnail', { id, png });
export const generationAllowReference = (path) => call('generation_allow_reference', { path });
// Memoria y tareas del proyecto (`dbv-unreal-ai/` junto al `.uproject`, REQ-34..REQ-38).
export const memoryStatus = (project) => call('memory_status', { project });
export const memoryActivate = (project) => call('memory_activate', { project });
export const memoryApply = (project, doc, baseHash, edit) => call('memory_apply', { project, doc, baseHash, edit });
export const memoryToolSpecs = () => call('memory_tool_specs');
export const memoryPlanPreview = (args) => call('memory_plan_preview', { args });
export const memoryTaskUpdate = (project, endpoint, title, done, byAi) => call('memory_task_update', { project, endpoint: endpoint ?? null, title, done, byAi });

// ─── Progreso de los trabajos ────────────────────────────────────────
// El oyente se registra ANTES de lanzar un trabajo y guarda lo que llega: un fallo inmediato (una clave
// mala) emite «failed» en milisegundos, y si nadie escuchaba aún la interfaz se quedaba «En cola…»
// hasta el tiempo límite (hallazgo Crítico de `/code-simplify`).
const FINAL_STAGES = ['done', 'failed', 'cancelled'];
const jobHistory = new Map();
const jobWaiters = new Map();
let jobListener = null;

/** Escucha `generation-progress` una sola vez para toda la app. */
export function listenJobs() {
  jobListener ??= on('generation-progress', (progress) => {
    const history = jobHistory.get(progress.jobId) ?? [];
    history.push(progress);
    jobHistory.set(progress.jobId, history);
    for (const waiter of jobWaiters.get(progress.jobId) ?? []) waiter(progress);
  });
  return jobListener;
}

/** Lanza una herramienta de generación ya confirmada con el oyente registrado. Devuelve los ids de sus trabajos. */
export async function launch(tool, args) {
  await listenJobs();
  return generationLaunch(tool, args);
}

/** Espera el final de un trabajo (`done`, `failed` o `cancelled`), con lo que ya hubiera llegado incluido. */
export function waitJob(jobId, timeoutMs, onProgress = () => {}) {
  return new Promise((resolve) => {
    const waiters = jobWaiters.get(jobId) ?? [];
    let settled = false;
    const timer = setTimeout(() => finish({ jobId, stage: 'failed', error: 'timeout' }), timeoutMs);
    function finish(result) {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      jobWaiters.set(jobId, (jobWaiters.get(jobId) ?? []).filter((w) => w !== receive));
      resolve(result);
    }
    function receive(progress) {
      onProgress(progress);
      if (FINAL_STAGES.includes(progress.stage)) finish(progress);
    }
    jobWaiters.set(jobId, [...waiters, receive]);
    listenJobs();
    for (const past of jobHistory.get(jobId) ?? []) receive(past);
  });
}
