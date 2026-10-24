// =============================================================================
// DBV Unreal AI — Enviar un asset de la biblioteca al proyecto de Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-19
//
// Importa con `import_model` del toolset DBV (glTF por Interchange: malla, material y textura;
// `StaticMeshTools.import_file` de Epic fuerza FBX y rechaza los .glb) y, si se pide, lo coloca
// delante de la cámara con `place_asset`, en una carpeta del Outliner. Comprobado en vivo con un
// GLB real de Tripo el 2026-10-24: 100 cm de alto, con su material y textura.

import { isDbvToolset } from './unrealTools.js';

/** Carpeta del Content Browser para lo generado (convención del usuario: todo bajo /Game/DBV). */
export const GENERATED_FOLDER = '/Game/DBV/Generated';
/** Carpeta del World Outliner para lo colocado. */
export const OUTLINER_FOLDER = 'DBV/Generated';

/** Nombre de asset válido para Unreal a partir del de la biblioteca. */
export function assetName(name) {
  const clean = String(name ?? '')
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^A-Za-z0-9_]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 48);
  return clean ? (/^[0-9]/.test(clean) ? `SM_${clean}` : clean) : 'Generated';
}

const parse = (result) => {
  if (result.value !== undefined && result.value !== null) return result.value;
  try {
    return JSON.parse(result.text);
  } catch {
    return null;
  }
};

/**
 * @param {object} deps
 * @param {object} deps.backend `listToolsets()`, `callTool(toolset, tool, args, timeoutSecs)`, `libraryFilePath(id, file)`, `libraryRecordSent(id, project, assetPath)`.
 * @param {object} entry Entrada de la biblioteca.
 * @param {{project: string, place?: boolean, folder?: string}} options
 * @returns {Promise<{ok: true, assetPath: string, actor: string|null}|{ok: false, error: string}>}
 */
export async function sendToUnreal(deps, entry, { project, place = true, folder = GENERATED_FOLDER }) {
  const { backend } = deps;
  const listed = await backend.listToolsets();
  const dbv = listed.ok ? listed.value.find((t) => isDbvToolset(t.name))?.name : null;
  const file = entry.files?.find((f) => /\.(glb|gltf|fbx|obj)$/i.test(f));
  const path = dbv && file ? await backend.libraryFilePath(entry.id, file) : null;
  let outcome;
  if (!listed.ok) outcome = { ok: false, error: listed.error.message };
  else if (!dbv) outcome = { ok: false, error: 'toolsetMissing' };
  else if (!file) outcome = { ok: false, error: 'noModelFile' };
  else if (!path.ok) outcome = { ok: false, error: path.error.message };
  else {
    const imported = await backend.callTool(dbv, 'import_model', { source_file: path.value, folder_path: folder, asset_name: assetName(entry.name) }, 300);
    const report = imported.ok ? parse(imported.value) : null;
    if (!imported.ok || !report?.success || !report.primary) outcome = { ok: false, error: imported.ok ? (report?.error ?? imported.value.text) : imported.error.message };
    else {
      let actor = null;
      if (place) {
        const placed = await backend.callTool(dbv, 'place_asset', { asset_path: report.primary, distance: 300, folder: OUTLINER_FOLDER }, 60);
        actor = placed.ok ? parse(placed.value)?.actor ?? null : null;
      }
      await backend.libraryRecordSent(entry.id, project, report.primary);
      outcome = { ok: true, assetPath: report.primary, actor };
    }
  }
  return outcome;
}
