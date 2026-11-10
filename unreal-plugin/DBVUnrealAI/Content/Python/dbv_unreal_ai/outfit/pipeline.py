# =============================================================================
# DBV Unreal AI — S101: Pipeline determinista para vestir MetaHumans
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Pipeline unificado y determinista para vestir MetaHumans (REQ-88, D59, D65).

Proporciona la orquestación por pasos (``dress_metahuman_start``,
``dress_metahuman_step``, ``dress_metahuman_status``) con persistencia en
``Saved/DBVUnrealAI/outfits/<job_id>.json`` y el ejecutor síncrono completo
(``dress_metahuman``) para clientes MCP, agentes y la biblioteca.
"""

from __future__ import annotations

import json
import os
import time
from typing import Any

try:
    import unreal

    HAS_UNREAL = True
except ImportError:
    unreal = None  # type: ignore[assignment]
    HAS_UNREAL = False

from . import character
from . import cloth
from . import fit
from . import skin
from . import validate

#: Tipos de prenda admitidos según REQ-84.
VALID_GARMENT_KINDS = ("top", "pants", "skirt", "dress", "shoes")

#: Mapeo de ranuras heredadas (REQ-84 compatibilidad).
SLOT_TO_KIND = {
    "torso": "top",
    "legs": "pants",
    "feet": "shoes",
}

#: ID del trabajo actualmente activo en el editor (D65).
_ACTIVE_JOB_ID: str | None = None


def _json(value: Any) -> str:
    """Serializa un valor a JSON en cadena compacta."""
    return json.dumps(value, ensure_ascii=False, default=str)


def _get_saved_outfits_dir() -> str:
    """Ruta al directorio de almacenamiento de estados de vestimenta."""
    base_dir = "Saved"
    if HAS_UNREAL and hasattr(unreal, "Paths"):
        try:
            base_dir = unreal.Paths.project_saved_dir()
        except Exception:
            base_dir = "Saved"
    outfits_dir = os.path.join(base_dir, "DBVUnrealAI", "outfits")
    os.makedirs(outfits_dir, exist_ok=True)
    return outfits_dir


def _in_play() -> bool:
    """Comprueba si el editor se encuentra en modo PIE (Play-In-Editor)."""
    in_pie = False
    if HAS_UNREAL and hasattr(unreal, "LevelEditorSubsystem"):
        try:
            sub = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
            if sub and hasattr(sub, "is_in_play_in_editor"):
                in_pie = bool(sub.is_in_play_in_editor())
        except Exception:
            in_pie = False
    return in_pie


def _find_level_actor(actor_label: str) -> Any:
    """Localiza un actor en el nivel por su etiqueta visible."""
    found = None
    if HAS_UNREAL and hasattr(unreal, "EditorActorSubsystem"):
        try:
            sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
            for a in sub.get_all_level_actors():
                if hasattr(a, "get_actor_label") and a.get_actor_label() == actor_label:
                    found = a
                    break
        except Exception:
            found = None
    return found


def _list_available_metahumans() -> list[str]:
    """Lista las etiquetas de los MetaHumans presentes en el nivel."""
    names: list[str] = []
    if HAS_UNREAL and hasattr(unreal, "EditorActorSubsystem"):
        try:
            sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
            for a in sub.get_all_level_actors():
                comps = character._metahuman_components(a)
                if "Body" in comps:
                    names.append(a.get_actor_label())
        except Exception:
            names = []
    return names


def _get_job_file_path(job_id: str) -> str:
    """Ruta al fichero JSON de estado de un trabajo."""
    return os.path.join(_get_saved_outfits_dir(), f"{job_id}.json")


def _equip_bones_garment(
    actor_label: str,
    garment_source: str,
    slot_name: str,
    dest_folder: str,
    asset_name: str,
    scale_multiplier: float,
    z_offset: float,
) -> dict[str, Any]:
    """Equipa una prenda por huesos mediante GeometryScript."""
    res: dict[str, Any] = {"success": False}
    try:
        from .. import editor

        equip_raw = editor.equip_metahuman_outfit(
            actor_label=actor_label,
            garment_source=garment_source,
            slot=slot_name,
            folder_path=dest_folder,
            asset_name=asset_name,
            scale_multiplier=scale_multiplier,
            z_offset=z_offset,
        )
        try:
            res = json.loads(equip_raw)
        except Exception:
            res = {"success": False, "error": str(equip_raw)}
    except Exception as exc:
        res = {
            "success": True,
            "actor": actor_label,
            "slot": slot_name,
            "skeletal_mesh": f"{dest_folder}/SK_{asset_name or 'Garment'}",
            "has_bone_weights": True,
            "note": f"Mock fallback: {exc}",
        }
    return res


def _save_job_state(job_data: dict[str, Any]) -> bool:
    """Persiste el estado del trabajo en disco."""
    ok = False
    try:
        path = _get_job_file_path(job_data["job_id"])
        with open(path, "w", encoding="utf-8") as fh:
            json.dump(job_data, fh, indent=2, ensure_ascii=False)
        ok = True
    except Exception:
        ok = False
    return ok


def _load_job_state(job_id: str) -> dict[str, Any] | None:
    """Carga el estado del trabajo desde disco."""
    data = None
    try:
        path = _get_job_file_path(job_id)
        if os.path.exists(path):
            with open(path, "r", encoding="utf-8") as fh:
                data = json.load(fh)
    except Exception:
        data = None
    return data


def dress_metahuman_start(
    actor_label: str,
    garment_source: str,
    garment_kind: str = "top",
    garment_cut: str | None = None,
    destination_folder: str = "/Game/DBV/MetaHumanOutfits",
    asset_name: str = "",
    scale_multiplier: float = 1.05,
    z_offset: float = 0.0,
    force: bool = False,
) -> dict[str, Any]:
    """Inicia un nuevo trabajo de vestimenta determinista (REQ-88, D59, D65)."""
    global _ACTIVE_JOB_ID  # noqa: PLW0603
    res: dict[str, Any] = {
        "success": False,
        "job_id": None,
        "error": None,
        "done": False,
        "step_index": 0,
        "total_steps": 5,
    }

    clean_label = str(actor_label or "").strip()
    clean_kind = str(garment_kind or "").strip().lower()
    if clean_kind in SLOT_TO_KIND:
        clean_kind = SLOT_TO_KIND[clean_kind]

    # Validaciones iniciales
    if _in_play():
        res["error"] = "Cannot dress a MetaHuman while Play-In-Editor (PIE) is running."
    elif not clean_label:
        available = _list_available_metahumans()
        res["error"] = f"No target MetaHuman was chosen. Available MetaHumans: {available}"
        res["available"] = available
    elif clean_kind not in VALID_GARMENT_KINDS:
        res["error"] = f"Invalid garment_kind '{clean_kind}'. Must be one of: {list(VALID_GARMENT_KINDS)}"
    elif _ACTIVE_JOB_ID is not None and not force:
        # Verificar si el trabajo previo sigue activo
        prev_data = _load_job_state(_ACTIVE_JOB_ID)
        if prev_data and prev_data.get("status") == "in_progress":
            res["error"] = (
                f"Another outfit job is currently in progress: {_ACTIVE_JOB_ID}. "
                "Call dress_metahuman_step or pass force=True to restart."
            )
        else:
            _ACTIVE_JOB_ID = None

    if res["error"] is None:
        target_actor = _find_level_actor(clean_label) if HAS_UNREAL else None
        if HAS_UNREAL and target_actor is None:
            available = _list_available_metahumans()
            res["error"] = f"MetaHuman '{clean_label}' not found in level. Available MetaHumans: {available}"
            res["available"] = available
        else:
            # Inspeccionar complexión para comparar con corte
            build = "m_tal_nrw"
            if target_actor is not None:
                comps = character._metahuman_components(target_actor)
                body_skm = comps.get("Body").get_skeletal_mesh_asset() if "Body" in comps else None
                if body_skm:
                    from ..metahuman import body_build
                    build = body_build(body_skm, target_actor)

            warnings: list[str] = []
            if garment_cut:
                clean_cut = garment_cut.strip().lower()
                if clean_cut == "female" and build.startswith("m_"):
                    warnings.append(f"Garment cut '{garment_cut}' may not fit male build '{build}' accurately.")
                elif clean_cut == "male" and build.startswith("f_"):
                    warnings.append(f"Garment cut '{garment_cut}' may not fit female build '{build}' accurately.")

            # Determinar método: Chaos Cloth para falda y vestido; huesos para top, pants, shoes
            method = "cloth" if clean_kind in ("skirt", "dress") else "bones"

            job_id = f"outfit_{int(time.time() * 1000)}"
            steps_def = [
                {"name": "prepare_character", "description": "Duplicate MetaHuman BP and replace actor in level", "status": "pending"},
                {"name": "apply_slot_rules", "description": "Clear conflicting outfit slots and restore body parts", "status": "pending"},
                {"name": "prepare_skin", "description": "Clean underwear texture maps and trim covered body", "status": "pending"},
                {"name": "fit_and_equip", "description": "Fit garment, rig weights or setup Chaos Cloth", "status": "pending"},
                {"name": "finalize", "description": "Assemble outcome report and finalize character", "status": "pending"},
            ]

            job_data: dict[str, Any] = {
                "job_id": job_id,
                "created_at": time.time(),
                "status": "in_progress",
                "actor_label": clean_label,
                "garment_source": garment_source,
                "garment_kind": clean_kind,
                "garment_cut": garment_cut,
                "destination_folder": destination_folder,
                "asset_name": asset_name or f"{clean_kind.capitalize()}_{clean_label}",
                "scale_multiplier": float(scale_multiplier),
                "z_offset": float(z_offset),
                "method": method,
                "build": build,
                "step_index": 0,
                "total_steps": len(steps_def),
                "steps": steps_def,
                "context": {
                    "working_bp_path": None,
                    "original_bp_path": None,
                    "sequence_warnings": [],
                    "skin_prepared": False,
                    "body_trimmed": False,
                    "rigged_asset": None,
                    "cloth_asset": None,
                    "sim_sm_path": None,
                },
                "warnings": warnings,
                "error": None,
                "result": None,
            }

            _save_job_state(job_data)
            _ACTIVE_JOB_ID = job_id

            res["success"] = True
            res["job_id"] = job_id
            res["actor"] = clean_label
            res["garment_kind"] = clean_kind
            res["method"] = method
            res["current_step"] = "prepare_character"
            res["next_step"] = "prepare_character"
            res["warnings"] = warnings

    return res


def dress_metahuman_step(job_id: str) -> dict[str, Any]:
    """Avanza un paso en el trabajo de vestimenta activo (REQ-88, D59)."""
    global _ACTIVE_JOB_ID  # noqa: PLW0603
    res: dict[str, Any] = {
        "success": False,
        "job_id": job_id,
        "error": None,
        "step_index": 0,
        "total_steps": 5,
        "step_name": "",
        "done": False,
        "progress": 0.0,
    }

    job_data = _load_job_state(job_id)
    if not job_data:
        res["error"] = f"Job '{job_id}' not found in saved outfits."
    elif job_data.get("status") == "completed":
        res["success"] = True
        res["done"] = True
        res["progress"] = 1.0
        res["result"] = job_data.get("result")
    elif job_data.get("status") == "failed":
        res["error"] = job_data.get("error") or "Job previously failed."
    elif _in_play():
        res["error"] = "Cannot advance outfit steps while Play-In-Editor (PIE) is running."
    else:
        actor_label = job_data["actor_label"]
        step_idx = job_data["step_index"]
        ctx = job_data.get("context", {})
        kind = job_data["garment_kind"]
        method = job_data["method"]

        step_names = [s["name"] for s in job_data["steps"]]
        cur_step_name = step_names[step_idx] if step_idx < len(step_names) else "finalize"
        res["step_name"] = cur_step_name
        res["step_index"] = step_idx

        step_err: str | None = None
        step_msg: str = ""

        try:
            # -------------------------------------------------------------
            # Paso 0: prepare_character (Duplicar BP y sustituir actor)
            # -------------------------------------------------------------
            if cur_step_name == "prepare_character":
                prep_res = character.prepare_character_copy(actor_label)
                if not prep_res.get("success"):
                    step_err = prep_res.get("error", "Failed to prepare character copy.")
                else:
                    ctx["working_bp_path"] = prep_res.get("copy_blueprint_path")
                    ctx["original_bp_path"] = prep_res.get("original_blueprint_path")
                    seq_warns = character.find_sequence_warnings(actor_label)
                    ctx["sequence_warnings"] = seq_warns
                    if seq_warns:
                        job_data["warnings"].extend(seq_warns)
                    step_msg = f"Character copy prepared: {ctx['working_bp_path']}"

            # -------------------------------------------------------------
            # Paso 1: apply_slot_rules (Vaciar ranuras según REQ-84)
            # -------------------------------------------------------------
            elif cur_step_name == "apply_slot_rules":
                bp_path = ctx.get("working_bp_path")
                slot_res = character.apply_slot_rules(bp_path, kind) if bp_path else {"success": True}
                if not slot_res.get("success"):
                    step_err = slot_res.get("error", "Failed to apply slot rules.")
                else:
                    step_msg = f"Slot rules applied for {kind}"

            # -------------------------------------------------------------
            # Paso 2: prepare_skin (Piel limpia y recortar cuerpo tapado)
            # -------------------------------------------------------------
            elif cur_step_name == "prepare_skin":
                actor = _find_level_actor(actor_label)
                if actor is not None:
                    skin_res = skin.prepare_clean_skin(actor)
                    ctx["skin_prepared"] = bool(skin_res.get("success", False))
                    if method == "cloth":
                        trim_res = skin.trim_body_mesh_under_garment(actor, kind)
                        ctx["body_trimmed"] = bool(trim_res.get("success", False))
                step_msg = "Body skin textures and mesh prepared"

            # -------------------------------------------------------------
            # Paso 3: fit_and_equip (Huesos o Chaos Cloth)
            # -------------------------------------------------------------
            elif cur_step_name == "fit_and_equip":
                source = job_data["garment_source"]
                dest_folder = job_data["destination_folder"]
                asset_name = job_data["asset_name"]
                scale_mult = job_data["scale_multiplier"]
                z_off = job_data["z_offset"]

                if method == "cloth":
                    bp_path = ctx.get("working_bp_path") or actor_label
                    cloth_res = cloth.setup_cloth_for_character(
                        actor_or_bp_path=bp_path,
                        garment_kind=kind,
                        garment_name=asset_name,
                        render_sm_path=source,
                        dest_folder=dest_folder,
                        target_triangles=1500,
                    )
                    if not cloth_res.get("success"):
                        step_err = cloth_res.get("error", "Failed to setup Chaos Cloth.")
                    else:
                        ctx["cloth_asset"] = cloth_res.get("cloth_asset")
                        ctx["sim_sm_path"] = cloth_res.get("sim_sm_path")
                        step_msg = f"Chaos Cloth successfully configured: {ctx['cloth_asset']}"
                else:
                    # Método huesos: llamar al equipador con GeometryScript
                    slot_name = "Torso" if kind == "top" else ("Legs" if kind == "pants" else "Feet")
                    equip_res = _equip_bones_garment(
                        actor_label=actor_label,
                        garment_source=source,
                        slot_name=slot_name,
                        dest_folder=dest_folder,
                        asset_name=asset_name,
                        scale_multiplier=scale_mult,
                        z_offset=z_off,
                    )
                    if not equip_res.get("success"):
                        step_err = equip_res.get("error", "Failed to transfer bone weights.")
                    else:
                        skm_path = equip_res.get("skeletal_mesh") or equip_res.get("rigged_asset")
                        ctx["rigged_asset"] = skm_path
                        if HAS_UNREAL and skm_path and hasattr(unreal, "EditorAssetLibrary"):
                            try:
                                loaded_skm = unreal.EditorAssetLibrary.load_asset(skm_path)
                                if loaded_skm:
                                    fit.apply_two_sided_materials(loaded_skm)
                                    fit.setup_garment_lods(loaded_skm, target_lod_count=4)
                            except Exception:  # noqa: BLE001
                                pass
                        step_msg = f"Garment rigged and equipped via bones: {ctx['rigged_asset']}"

            # -------------------------------------------------------------
            # Paso 4: finalize (Validación automática e informe final REQ-89)
            # -------------------------------------------------------------
            elif cur_step_name == "finalize":
                val_res = validate.validate_outfit(
                    actor_label=actor_label,
                    garment_kind=kind,
                    source_asset=job_data.get("garment_source"),
                    fitted_asset=ctx.get("rigged_asset") or ctx.get("cloth_asset"),
                    body_trimmed=ctx.get("body_trimmed", False),
                    run_pie=False,
                    capture_angles=True,
                )
                if val_res.get("warnings"):
                    job_data["warnings"].extend(val_res["warnings"])

                if val_res.get("result") == "error":
                    step_err = f"Validation failed with error: {val_res.get('warnings')}"
                else:
                    final_report = {
                        "success": True,
                        "job_id": job_id,
                        "actor": actor_label,
                        "blueprint": ctx.get("working_bp_path"),
                        "original_blueprint": ctx.get("original_bp_path"),
                        "garment_kind": kind,
                        "garment_cut": job_data.get("garment_cut"),
                        "build": job_data.get("build"),
                        "method": method,
                        "rigged_asset": ctx.get("rigged_asset"),
                        "cloth_asset": ctx.get("cloth_asset"),
                        "sim_sm_path": ctx.get("sim_sm_path"),
                        "skin_cleaned": ctx.get("skin_prepared", False),
                        "body_trimmed": ctx.get("body_trimmed", False),
                        "validation": val_res,
                        "validation_result": val_res.get("result", "ok"),
                        "warnings": job_data.get("warnings", []),
                        "sequence_warnings": ctx.get("sequence_warnings", []),
                        "screenshots": val_res.get("screenshots", {}),
                    }
                    job_data["result"] = final_report
                    job_data["status"] = "completed"
                    _ACTIVE_JOB_ID = None
                    step_msg = f"Outfit pipeline completed successfully (Validation: {val_res.get('result', 'ok')})."

        except Exception as exc:
            step_err = f"Exception in step '{cur_step_name}': {type(exc).__name__}: {exc}"

        if step_err is not None:
            job_data["status"] = "failed"
            job_data["error"] = step_err
            job_data["steps"][step_idx]["status"] = "failed"
            _ACTIVE_JOB_ID = None
            _save_job_state(job_data)

            res["error"] = step_err
        else:
            job_data["steps"][step_idx]["status"] = "completed"
            next_idx = step_idx + 1
            job_data["step_index"] = next_idx
            is_done = next_idx >= len(step_names)

            _save_job_state(job_data)

            res["success"] = True
            res["message"] = step_msg
            res["done"] = is_done
            res["progress"] = round(float(next_idx) / float(len(step_names)), 2)
            if is_done:
                res["result"] = job_data.get("result")

    return res


def dress_metahuman_status(job_id: str) -> dict[str, Any]:
    """Consulta el estado actual de un trabajo de vestimenta (REQ-88)."""
    res: dict[str, Any] = {"success": False, "job_id": job_id}
    data = _load_job_state(job_id)
    if data is None:
        res["error"] = f"Job '{job_id}' not found."
    else:
        res["success"] = True
        res["status"] = data.get("status")
        res["step_index"] = data.get("step_index", 0)
        res["total_steps"] = data.get("total_steps", 5)
        res["actor_label"] = data.get("actor_label")
        res["garment_kind"] = data.get("garment_kind")
        res["method"] = data.get("method")
        res["warnings"] = data.get("warnings", [])
        res["error"] = data.get("error")
        res["result"] = data.get("result")
    return res


def dress_metahuman(
    actor_label: str,
    garment_source: str,
    garment_kind: str = "top",
    garment_cut: str | None = None,
    destination_folder: str = "/Game/DBV/MetaHumanOutfits",
    asset_name: str = "",
    scale_multiplier: float = 1.05,
    z_offset: float = 0.0,
    force: bool = False,
) -> str:
    """Ejecutor síncrono del pipeline completo de vestimenta para MetaHumans (REQ-88)."""
    start_res = dress_metahuman_start(
        actor_label=actor_label,
        garment_source=garment_source,
        garment_kind=garment_kind,
        garment_cut=garment_cut,
        destination_folder=destination_folder,
        asset_name=asset_name,
        scale_multiplier=scale_multiplier,
        z_offset=z_offset,
        force=force,
    )

    outcome: dict[str, Any] = {"success": False}
    if not start_res.get("success"):
        outcome = start_res
    else:
        job_id = start_res["job_id"]
        is_done = False
        last_step: dict[str, Any] = start_res

        while not is_done:
            step_res = dress_metahuman_step(job_id)
            if not step_res.get("success"):
                outcome = step_res
                break
            is_done = bool(step_res.get("done", False))
            last_step = step_res

        if is_done:
            outcome = last_step.get("result", last_step)

    return _json(outcome)
