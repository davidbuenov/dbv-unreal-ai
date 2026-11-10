// =============================================================================
// DBV Unreal AI — Enviar un asset de la biblioteca al proyecto de Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-19, REQ-16, REQ-81, REQ-83, REQ-84
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
/** Carpeta del Content Browser para las prendas de MetaHuman: la misma en la que `equip_metahuman_outfit` crea el `SK_`. @spec REQ-81 */
export const OUTFITS_FOLDER = '/Game/DBV/MetaHumanOutfits';
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

/** Código fijo para riggear y equipar una prenda en un MetaHuman del nivel. @spec REQ-81 */
export function equipMetahumanCode(actorLabel, garmentSource, slot, scaleMultiplier = 1.05, zOffset = 0) {
  const hex = Array.from(
    new TextEncoder().encode(
      JSON.stringify({
        actor_label: String(actorLabel ?? ''),
        garment_source: String(garmentSource ?? ''),
        slot: String(slot ?? 'Torso'),
        scale_multiplier: Number(scaleMultiplier) || 1.05,
        z_offset: Number(zOffset) || 0,
      }),
    ),
    (b) => b.toString(16).padStart(2, '0'),
  ).join('');
  return `import json\nimport dbv_unreal_ai.editor as _e\n_p = json.loads(bytes.fromhex("${hex}").decode("utf-8"))\nresult = _e.equip_metahuman_outfit(actor_label=_p["actor_label"], garment_source=_p["garment_source"], slot=_p["slot"], scale_multiplier=float(_p["scale_multiplier"]), z_offset=float(_p["z_offset"]))\n`;
}

/** Código fijo para vestir a un MetaHuman mediante el pipeline determinista unificado. @spec REQ-88 */
export function dressMetahumanCode(actorLabel, garmentSource, garmentKind = 'top', garmentCut = null, scaleMultiplier = 1.05, zOffset = 0) {
  const hex = Array.from(
    new TextEncoder().encode(
      JSON.stringify({
        actor_label: String(actorLabel ?? ''),
        garment_source: String(garmentSource ?? ''),
        garment_kind: String(garmentKind ?? 'top'),
        garment_cut: garmentCut ? String(garmentCut) : null,
        scale_multiplier: Number(scaleMultiplier) || 1.05,
        z_offset: Number(zOffset) || 0,
      }),
    ),
    (b) => b.toString(16).padStart(2, '0'),
  ).join('');
  return `import json\nimport dbv_unreal_ai.editor as _e\n_p = json.loads(bytes.fromhex("${hex}").decode("utf-8"))\nresult = _e.dress_metahuman(actor_label=_p["actor_label"], garment_source=_p["garment_source"], garment_kind=_p.get("garment_kind", "top"), garment_cut=_p.get("garment_cut"), scale_multiplier=float(_p.get("scale_multiplier", 1.05)), z_offset=float(_p.get("z_offset", 0.0)))\n`;
}

/** Código fijo para validar una prenda equipada en un MetaHuman. @spec REQ-89 */
export function validateMetahumanOutfitCode(actorLabel, garmentKind = 'top', runPie = false, captureAngles = true) {
  const hex = Array.from(
    new TextEncoder().encode(
      JSON.stringify({
        actor_label: String(actorLabel ?? ''),
        garment_kind: String(garmentKind ?? 'top'),
        run_pie: Boolean(runPie),
        capture_angles: Boolean(captureAngles),
      }),
    ),
    (b) => b.toString(16).padStart(2, '0'),
  ).join('');
  return `import json\nimport dbv_unreal_ai.editor as _e\n_p = json.loads(bytes.fromhex("${hex}").decode("utf-8"))\nresult = _e.validate_metahuman_outfit(actor_label=_p["actor_label"], garment_kind=_p.get("garment_kind", "top"), run_pie=_p.get("run_pie", False), capture_angles=_p.get("capture_angles", True))\n`;
}

const LIST_METAHUMANS_CODE = 'import dbv_unreal_ai.editor as _e\nresult = _e.list_metahumans()\n';

/**
 * ¿El corte de la prenda no es el del cuerpo elegido? `build` es la complexión (`f_med_nrw` mujer, `m_tal_nrw` hombre).
 * Devuelve `{garment, body}` o `null` si coincide, es neutro o no se sabe. @spec REQ-84
 */
export function cutMismatch(entry, build) {
  const cut = entry?.parameters?.garment_cut;
  const text = String(build ?? '');
  let body = null;
  if (text.startsWith('f_')) body = 'female';
  else if (text.startsWith('m_')) body = 'male';
  return (cut === 'female' || cut === 'male') && body && cut !== body ? { garment: cut, body } : null;
}

/** Equipa una prenda en el MetaHuman `target` del nivel. @spec REQ-81, REQ-83 */
async function equipOnMetahuman(backend, dbv, target, primary, slot) {
  const ran = await backend.callTool(dbv, 'run_python', { code: equipMetahumanCode(target, primary, slot) }, 120);
  const res = ran.ok ? parse(ran.value)?.result ?? null : null;
  let outcome;
  if (res?.success) outcome = { ok: true, actor: res.actor ?? target, slot: res.slot ?? slot, riggedAsset: res.rigged_asset ?? primary, build: res.build ?? '' };
  else outcome = { ok: false, error: res?.error ?? (ran.ok ? ran.value?.text : ran.error.message) ?? 'equipFailed' };
  return outcome;
}

/**
 * Los MetaHumans del nivel abierto (etiqueta, complexión y ranuras) para elegir a quién vestir. @spec REQ-83
 * @returns {Promise<{ok: true, metahumans: {label: string, build: string, slots: object}[]}|{ok: false, error: string}>}
 */
export async function listMetahumans(backend) {
  const listed = await backend.listToolsets();
  const dbv = listed.ok ? listed.value.find((t) => isDbvToolset(t.name))?.name : null;
  let outcome;
  if (!listed.ok) outcome = { ok: false, error: listed.error.message };
  else if (!dbv) outcome = { ok: false, error: 'toolsetMissing' };
  else {
    const ran = await backend.callTool(dbv, 'run_python', { code: LIST_METAHUMANS_CODE }, 60);
    const report = ran.ok ? parse(ran.value)?.result ?? null : null;
    outcome = Array.isArray(report?.metahumans) ? { ok: true, metahumans: report.metahumans } : { ok: false, error: ran.ok ? 'listFailed' : ran.error.message };
  }
  return outcome;
}

/**
 * @param {object} deps
 * @param {object} deps.backend `listToolsets()`, `callTool(toolset, tool, args, timeoutSecs)`, `libraryFilePath(id, file)`, `libraryRecordSent(id, project, assetPath)` y, opcional, `libraryWorldPlaceCode(id, splatLabel, colliderAsset, name)`.
 * @param {object} entry Entrada de la biblioteca.
 * @param {{project: string, place?: boolean, folder?: string, metahuman?: string}} options `metahuman`: etiqueta del personaje a vestir si la entrada es una prenda (REQ-83).
 * @returns {Promise<{ok: true, assetPath: string, actor: string|null, kind: string, animations: string[], playing: string|null, note?: string}|{ok: false, error: string}>}
 */
/** Espera para cargar un Gaussian Splat: un .ply de 3DGS pasa con facilidad de 2 GB (igual que el MCP propio). @spec REQ-54 */
const SPLAT_TIMEOUT_SECONDS = 600;

export async function sendToUnreal(deps, entry, { project, place = true, folder: requestedFolder, metahuman = '' }) {
  const { backend } = deps;
  const folder = requestedFolder ?? (entry.parameters?.garment_slot ? OUTFITS_FOLDER : GENERATED_FOLDER);
  const listed = await backend.listToolsets();
  const dbv = listed.ok ? listed.value.find((t) => isDbvToolset(t.name))?.name : null;
  const splatFile = entry.files?.find((f) => /\.(spz|ply)$/i.test(f));
  const modelFile = entry.files?.find((f) => /\.(glb|gltf|fbx|obj)$/i.test(f));
  const soundFile = entry.files?.find((f) => /\.(wav|mp3|ogg)$/i.test(f));
  const isWorld = entry.kind === 'world' || Boolean(splatFile);
  const isSound = entry.kind === 'sound' || Boolean(soundFile);

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
      const loadedSplat = await backend.callTool(dbv, 'load_gaussian_splat', { source_file: splatPath.value, actor_label: splatActorLabel }, SPLAT_TIMEOUT_SECONDS);
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
            }
          }
        }
        // Un mundo de World Labs con escala y suelo lo coloca el código fijo de Rust (REQ-18, el mismo que usa el
        // --mcp): splat a escala métrica, colisión alineada y oculta, suelo en Z = 0. Sin metadatos, como antes.
        const placeCode = place && backend.libraryWorldPlaceCode ? await backend.libraryWorldPlaceCode(entry.id, splatActorLabel, colliderPath, assetName(entry.name)) : null;
        let world = null;
        if (placeCode?.ok && placeCode.value) {
          const placedWorld = await backend.callTool(dbv, 'run_python', { code: placeCode.value }, 120);
          world = placedWorld.ok ? parse(placedWorld.value)?.result ?? null : null;
          colliderActor = world?.collider ?? null;
        } else if (place && colliderPath) {
          const placed = await backend.callTool(dbv, 'place_asset', { asset_path: colliderPath, distance: 0, folder: OUTLINER_FOLDER }, 60);
          colliderActor = placed.ok ? parse(placed.value)?.actor ?? null : null;
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
          ...(world ? { world } : {}),
        };
      }
    }
  } else if (!isWorld && isSound && soundFile) {
    const soundPath = await backend.libraryFilePath(entry.id, soundFile);
    if (!soundPath.ok) {
      outcome = { ok: false, error: soundPath.error.message };
    } else {
      const imported = await backend.callTool(dbv, 'import_sound', { source_file: soundPath.value, folder_path: '/Game/DBV/Audio', asset_name: assetName(entry.name) }, 120);
      const report = imported.ok ? parse(imported.value) : null;
      if (!imported.ok || !report?.success || !report.primary) {
        outcome = { ok: false, error: imported.ok ? (report?.error ?? imported.value.text) : imported.error.message };
      } else {
        await backend.libraryRecordSent(entry.id, project, report.primary);
        outcome = { ok: true, assetPath: report.primary, actor: null, kind: 'sound', animations: [], playing: null };
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
        const garmentSlot = entry.parameters?.garment_slot ?? null;
        const target = String(metahuman ?? '').trim();
        const animations = report.animations ?? [];
        // REQ-83: una prenda solo se equipa en el MetaHuman elegido; sin destino se importa y se avisa, sin colocarla.
        const equip = place && garmentSlot && target ? await equipOnMetahuman(backend, dbv, target, report.primary, garmentSlot) : null;
        if (equip && !equip.ok) {
          outcome = { ok: false, error: equip.error };
        } else {
          let actor = equip?.actor ?? null;
          let playing = null;
          let note = place && garmentSlot && !target ? 'noMetahumanTarget' : undefined;
          if (place && !garmentSlot) {
            const placed = await backend.callTool(dbv, 'place_asset', { asset_path: report.primary, distance: 300, folder: OUTLINER_FOLDER }, 60);
            actor = placed.ok ? parse(placed.value)?.actor ?? null : null;
          }
          if (actor && !equip && report.kind === 'skeletal' && animations.length) {
            const played = await backend.callTool(dbv, 'play_animation', { actor_label: actor, animation_path: animations[0], looping: true }, 60);
            const playedReport = played.ok && !played.value?.isError ? parse(played.value) : null;
            if (playedReport?.success) playing = animations[0];
            else note = 'toolsetOutdated';
          }
          const finalAsset = equip?.riggedAsset ?? report.primary;
          await backend.libraryRecordSent(entry.id, project, finalAsset);
          outcome = {
            ok: true,
            assetPath: finalAsset,
            actor,
            kind: equip ? 'metahuman_garment' : report.kind ?? 'static',
            animations,
            playing,
            ...(equip ? { equippedSlot: equip.slot } : {}),
            ...(equip && cutMismatch(entry, equip.build) ? { cutMismatch: cutMismatch(entry, equip.build) } : {}),
            ...(note ? { note } : {}),
          };
        }
      }
    }
  }
  return outcome;
}
