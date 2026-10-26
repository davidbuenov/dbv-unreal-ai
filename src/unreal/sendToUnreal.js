// =============================================================================
// DBV Unreal AI — Enviar un asset de la biblioteca al proyecto de Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-19, REQ-16
//
// Importa con `import_model` del toolset DBV (glTF por Interchange: malla, material y textura;
// `StaticMeshTools.import_file` de Epic fuerza FBX y rechaza los .glb) y, si se pide, lo coloca
// delante de la cámara con `place_asset`, en una carpeta del Outliner. Comprobado en vivo con un
// GLB real de Tripo el 2026-10-24: 100 cm de alto, con su material y textura. Un rig o una animación de Tripo
// llega como Skeletal Mesh con sus Animation Sequences: se coloca y se pone a reproducir la primera con
// `play_animation` (toolset 0.2.0; con uno anterior se importa igual y se avisa de que hay que actualizarlo).
// Sin verificar aún contra el motor real.

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
 * @returns {Promise<{ok: true, assetPath: string, actor: string|null, kind: string, animations: string[], playing: string|null, note?: string}|{ok: false, error: string}>}
 */
export async function sendToUnreal(deps, entry, { project, place = true, folder = GENERATED_FOLDER }) {
  const { backend } = deps;
  const listed = await backend.listToolsets();
  const dbv = listed.ok ? listed.value.find((t) => isDbvToolset(t.name))?.name : null;
  const splatFile = entry.files?.find((f) => /\.(spz|ply)$/i.test(f));
  const modelFile = entry.files?.find((f) => /\.(glb|gltf|fbx|obj)$/i.test(f));
  const isWorld = entry.kind === 'world' || Boolean(splatFile);

  let outcome;
  if (!listed.ok) {
    outcome = { ok: false, error: listed.error.message };
  } else if (!dbv) {
    outcome = { ok: false, error: 'toolsetMissing' };
  } else if (isWorld && splatFile) {
    const splatPath = await backend.libraryFilePath(entry.id, splatFile);
    if (!splatPath.ok) {
      outcome = { ok: false, error: splatPath.error.message };
    } else {
      const splatActorLabel = `${assetName(entry.name)}_Splat`;
      const loadedSplat = await backend.callTool(dbv, 'load_gaussian_splat', { source_file: splatPath.value, actor_label: splatActorLabel }, 120);
      const splatReport = loadedSplat.ok ? parse(loadedSplat.value) : null;
      if (!loadedSplat.ok || !splatReport?.success) {
        outcome = { ok: false, error: loadedSplat.ok ? (splatReport?.error ?? loadedSplat.value.text) : loadedSplat.error.message };
      } else {
        let colliderActor = null;
        let colliderPath = null;
        if (modelFile) {
          const meshPath = await backend.libraryFilePath(entry.id, modelFile);
          if (meshPath.ok) {
            const importedMesh = await backend.callTool(dbv, 'import_model', { source_file: meshPath.value, folder_path: folder, asset_name: `${assetName(entry.name)}_Collider` }, 300);
            const meshReport = importedMesh.ok ? parse(importedMesh.value) : null;
            if (importedMesh.ok && meshReport?.success && meshReport.primary) {
              colliderPath = meshReport.primary;
              if (place) {
                const placed = await backend.callTool(dbv, 'place_asset', { asset_path: meshReport.primary, distance: 0, folder: OUTLINER_FOLDER }, 60);
                colliderActor = placed.ok ? parse(placed.value)?.actor ?? null : null;
              }
            }
          }
        }
        await backend.libraryRecordSent(entry.id, project, splatActorLabel);
        outcome = {
          ok: true,
          assetPath: splatActorLabel,
          actor: splatReport.actor ?? splatActorLabel,
          kind: 'world',
          animations: [],
          playing: null,
          ...(colliderActor ? { colliderActor } : {}),
          ...(colliderPath ? { colliderPath } : {}),
        };
      }
    }
  } else if (!modelFile) {
    outcome = { ok: false, error: 'noModelFile' };
  } else {
    const path = await backend.libraryFilePath(entry.id, modelFile);
    if (!path.ok) {
      outcome = { ok: false, error: path.error.message };
    } else {
      const imported = await backend.callTool(dbv, 'import_model', { source_file: path.value, folder_path: folder, asset_name: assetName(entry.name) }, 300);
      const report = imported.ok ? parse(imported.value) : null;
      if (!imported.ok || !report?.success || !report.primary) {
        outcome = { ok: false, error: imported.ok ? (report?.error ?? imported.value.text) : imported.error.message };
      } else {
        let actor = null;
        let playing = null;
        let note;
        const animations = report.animations ?? [];
        if (place) {
          const placed = await backend.callTool(dbv, 'place_asset', { asset_path: report.primary, distance: 300, folder: OUTLINER_FOLDER }, 60);
          actor = placed.ok ? parse(placed.value)?.actor ?? null : null;
        }
        if (actor && report.kind === 'skeletal' && animations.length) {
          const played = await backend.callTool(dbv, 'play_animation', { actor_label: actor, animation_path: animations[0], looping: true }, 60);
          const playedReport = played.ok && !played.value?.isError ? parse(played.value) : null;
          if (playedReport?.success) playing = animations[0];
          else note = 'toolsetOutdated';
        }
        await backend.libraryRecordSent(entry.id, project, report.primary);
        outcome = { ok: true, assetPath: report.primary, actor, kind: report.kind ?? 'static', animations, playing, ...(note ? { note } : {}) };
      }
    }
  }
  return outcome;
}
