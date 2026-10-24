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
export const appMcpCommand = (port) => call('app_mcp_command', { port });
/** Diagnóstico sin secretos para una incidencia (RNF-08). */
export const appDiagnostics = () => call('app_diagnostics');

// ─── Generación y biblioteca ─────────────────────────────────────────
export const generationStatus = () => call('generation_status');
export const generationSetKey = (service, key) => call('generation_set_key', { service, key });
export const generationDeleteKey = (service) => call('generation_delete_key', { service });
export const generationSetBudget = (credits) => call('generation_set_budget', { credits });
export const generationTripoStart = (start) => call('generation_tripo_start', { start });
export const generationCancel = (jobId) => call('generation_cancel', { jobId });
export const libraryList = () => call('library_list');
export const libraryRemove = (id) => call('library_remove', { id });
export const libraryFilePath = (id, file) => call('library_file_path', { id, file });
export const libraryRecordSent = (id, project, assetPath) => call('library_record_sent', { id, project, assetPath });

/** Espera el final de un trabajo de generación (`done`, `failed` o `cancelled`). */
export function waitJob(jobId, timeoutMs, onProgress = () => {}) {
  return new Promise((resolve) => {
    let unlisten = null;
    const timer = setTimeout(() => finish({ stage: 'failed', error: 'timeout' }), timeoutMs);
    function finish(result) {
      clearTimeout(timer);
      Promise.resolve(unlisten).then((stop) => stop?.());
      resolve(result);
    }
    unlisten = on('generation-progress', (progress) => {
      if (progress.jobId !== jobId) return;
      onProgress(progress);
      if (['done', 'failed', 'cancelled'].includes(progress.stage)) finish(progress);
    });
  });
}
