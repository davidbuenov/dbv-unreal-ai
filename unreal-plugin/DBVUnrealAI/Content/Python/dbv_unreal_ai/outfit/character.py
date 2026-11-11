# =============================================================================
# DBV Unreal AI — Gestión de copia del personaje, ranuras y retorno al original
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Copia de Blueprint, sustitución del actor, ranuras y rollback al original.

@spec REQ-85, REQ-84
"""

from __future__ import annotations

import json
import os
import re
from typing import Any

try:
    import unreal
    HAS_UNREAL = True
except ImportError:
    unreal = None  # type: ignore[assignment]
    HAS_UNREAL = False

#: Etiqueta de actor que identifica una copia gestionada por DBV Unreal AI.
TAG_DBV_COPY = "DBV_OUTFIT_COPY"

#: Prefijo de etiqueta que almacena la ruta del Blueprint original.
TAG_ORIGINAL_BP_PREFIX = "DBV_ORIGINAL_BP:"

#: Sufijo utilizado para las copias de Blueprint de MetaHuman.
COPY_BP_SUFFIX = "_DBV"

#: Ranuras de ropa estándar de MetaHuman.
METAHUMAN_SLOTS = ("Torso", "Legs", "Feet")

#: Tipos de prenda reconocidos y su mapeo de ranuras según REQ-84.
GARMENT_KINDS = ("top", "pants", "skirt", "dress", "shoes")


def _json(value: Any) -> str:
    """Serializa a JSON con codificación UTF-8 segura."""
    return json.dumps(value, ensure_ascii=False, default=str)


def _in_play() -> bool:
    """Comprueba si el editor se encuentra actualmente en una sesión PIE."""
    subsystem_class = getattr(unreal, "LevelEditorSubsystem", None)
    subsystem = unreal.get_editor_subsystem(subsystem_class) if subsystem_class is not None else None
    return bool(subsystem is not None and subsystem.is_in_play_in_editor())


def _metahuman_components(actor: Any) -> dict[str, Any]:
    """Componentes SkeletalMeshComponent de un actor indexados por nombre."""
    comps = actor.get_components_by_class(unreal.SkeletalMeshComponent) if hasattr(actor, "get_components_by_class") else []
    return {c.get_name(): c for c in comps}


def is_character_copy(actor: Any) -> bool:
    """Indica si el actor corresponde a una copia generada por DBV Unreal AI. @spec REQ-85"""
    if actor is None:
        return False

    tags = getattr(actor, "tags", []) or []
    has_tag = any(t == TAG_DBV_COPY or str(t).startswith(TAG_ORIGINAL_BP_PREFIX) for t in tags)
    if has_tag:
        return True

    class_name = actor.get_class().get_name() if hasattr(actor, "get_class") else ""
    return bool(class_name.endswith(f"{COPY_BP_SUFFIX}_C") or class_name.endswith(COPY_BP_SUFFIX))


def get_original_bp_path(actor: Any) -> str | None:
    """Determina la ruta del asset Blueprint original de un MetaHuman. @spec REQ-85"""
    if actor is None:
        return None

    resolved_path: str | None = None
    tags = getattr(actor, "tags", []) or []
    for tag in tags:
        tag_str = str(tag)
        if tag_str.startswith(TAG_ORIGINAL_BP_PREFIX):
            resolved_path = tag_str[len(TAG_ORIGINAL_BP_PREFIX):].split(".")[0]
            break

    if resolved_path is None:
        class_path = actor.get_class().get_path_name() if hasattr(actor, "get_class") else ""
        clean_path = class_path.removesuffix("_C").split(".")[0]
        if clean_path.endswith(COPY_BP_SUFFIX):
            clean_path = clean_path[:-len(COPY_BP_SUFFIX)]
        resolved_path = clean_path if clean_path else None

    return resolved_path


def find_sequence_warnings(actor_label: str) -> list[str]:
    """Busca LevelSequences en el nivel actual que enlacen al actor especificado. @spec REQ-85"""
    warnings: list[str] = []
    editor_actor_subsystem = (
        unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
        if hasattr(unreal, "EditorActorSubsystem")
        else None
    )
    if editor_actor_subsystem is None or not actor_label:
        return warnings

    actors = editor_actor_subsystem.get_all_level_actors() if hasattr(editor_actor_subsystem, "get_all_level_actors") else []
    seq_actors = [a for a in actors if "LevelSequence" in a.get_class().get_name()]
    for sa in seq_actors:
        seq = sa.get_sequence() if hasattr(sa, "get_sequence") else None
        if seq is None and hasattr(sa, "get_editor_property"):
            seq = sa.get_editor_property("LevelSequence")
        if seq is not None and hasattr(seq, "get_bindings"):
            bindings = seq.get_bindings()
            for b in bindings:
                b_name = b.get_name() if hasattr(b, "get_name") else ""
                if b_name == actor_label:
                    seq_name = seq.get_name() if hasattr(seq, "get_name") else str(seq)
                    warnings.append(seq_name)
                    break
    return warnings


def snapshot_actor_slots(actor: Any) -> dict[str, str | None]:
    """Captura los SkeletalMesh asignados actualmente en las ranuras del actor."""
    by_name = _metahuman_components(actor)
    slots: dict[str, str | None] = {}
    for slot in ("Body", *METAHUMAN_SLOTS):
        comp = by_name.get(slot)
        mesh = comp.get_skeletal_mesh_asset() if comp and hasattr(comp, "get_skeletal_mesh_asset") else None
        slots[slot] = mesh.get_path_name() if mesh and hasattr(mesh, "get_path_name") else None
    return slots


def prepare_character_copy(actor_label: str) -> dict[str, Any]:
    """Prepara o reutiliza la copia de Blueprint para el MetaHuman de destino. @spec REQ-85"""
    # Guard clauses iniciales
    if not actor_label or not actor_label.strip():
        return {"success": False, "error": "Actor label cannot be empty"}

    if _in_play():
        return {"success": False, "error": "Operation rejected during Play in Editor (PIE)"}

    editor_actor_subsystem = (
        unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
        if hasattr(unreal, "EditorActorSubsystem")
        else None
    )
    if editor_actor_subsystem is None:
        return {"success": False, "error": "EditorActorSubsystem unavailable"}

    actors = editor_actor_subsystem.get_all_level_actors() if hasattr(editor_actor_subsystem, "get_all_level_actors") else []
    target_actor = next((a for a in actors if a.get_actor_label() == actor_label), None)
    if target_actor is None:
        available = [a.get_actor_label() for a in actors if "Body" in _metahuman_components(a)]
        return {"success": False, "error": f"MetaHuman '{actor_label}' not found", "available": available}

    by_name = _metahuman_components(target_actor)
    if "Body" not in by_name:
        return {"success": False, "error": f"Actor '{actor_label}' is missing Body component"}

    outcome: dict[str, Any] = {"success": False}
    try:
        seq_warnings = find_sequence_warnings(actor_label)

        # Si ya es una copia previa de DBV, se reutiliza sin duplicar nuevamente
        if is_character_copy(target_actor):
            copy_bp_path = target_actor.get_class().get_path_name().removesuffix("_C").split(".")[0]
            orig_bp_path = get_original_bp_path(target_actor) or copy_bp_path
            outcome = {
                "success": True,
                "reused": True,
                "actor": target_actor,
                "actor_label": actor_label,
                "copy_bp_path": copy_bp_path,
                "original_bp_path": orig_bp_path,
                "warnings": seq_warnings,
                "slots": snapshot_actor_slots(target_actor),
            }
        else:
            orig_bp_path = get_original_bp_path(target_actor)
            if not orig_bp_path:
                outcome = {"success": False, "error": f"Could not resolve original Blueprint path for '{actor_label}'"}
            else:
                orig_bp_clean = orig_bp_path.split(".")[0]
                bp_name = os.path.basename(orig_bp_clean)
                char_name = re.sub(r"^BP_", "", bp_name)
                dest_folder = f"/Game/DBV/MetaHumanOutfits/{char_name}"
                dest_bp_name = f"BP_{char_name}{COPY_BP_SUFFIX}"
                dest_bp_path = f"{dest_folder}/{dest_bp_name}"

                eal = unreal.EditorAssetLibrary
                if hasattr(eal, "does_directory_exist") and not eal.does_directory_exist(dest_folder):
                    eal.make_directory(dest_folder)

                # Duplicar Blueprint si no existe aún en disco
                dup_ok = True
                if hasattr(eal, "does_asset_exist") and not eal.does_asset_exist(dest_bp_path):
                    dup_ok = bool(eal.duplicate_asset(orig_bp_clean, dest_bp_path))
                    if dup_ok:
                        ensure_blueprint_leader_pose(dest_bp_path)

                if not dup_ok:
                    outcome = {"success": False, "error": f"Failed to duplicate Blueprint from {orig_bp_clean} to {dest_bp_path}"}
                else:
                    copy_class = eal.load_blueprint_class(dest_bp_path) if hasattr(eal, "load_blueprint_class") else None
                    if copy_class is None:
                        outcome = {"success": False, "error": f"Failed to load copied Blueprint class at {dest_bp_path}"}
                    else:
                        # Capturar estado y transform del actor original
                        loc = target_actor.get_actor_location()
                        rot = target_actor.get_actor_rotation()
                        scale = target_actor.get_actor_scale3d() if hasattr(target_actor, "get_actor_scale3d") else unreal.Vector(1, 1, 1)
                        folder = target_actor.get_folder_path() if hasattr(target_actor, "get_folder_path") else ""
                        orig_slots = snapshot_actor_slots(target_actor)

                        with unreal.ScopedEditorTransaction("DBV Unreal AI: Prepare character copy"):
                            new_actor = editor_actor_subsystem.spawn_actor_from_class(copy_class, loc, rot)
                            if new_actor is None:
                                outcome = {"success": False, "error": f"Failed to spawn actor from class {copy_class}"}
                            else:
                                if hasattr(new_actor, "set_actor_scale3d"):
                                    new_actor.set_actor_scale3d(scale)
                                if folder and hasattr(new_actor, "set_folder_path"):
                                    new_actor.set_folder_path(folder)

                                # Añadir tags de control de DBV
                                tags = list(getattr(new_actor, "tags", []) or [])
                                tags.append(TAG_DBV_COPY)
                                tags.append(f"{TAG_ORIGINAL_BP_PREFIX}{orig_bp_clean}")
                                new_actor.tags = tags

                                # Transferir mallas esqueléticas existentes del original
                                new_comps = _metahuman_components(new_actor)
                                body_comp = new_comps.get("Body")
                                for slot_name in METAHUMAN_SLOTS:
                                    comp = new_comps.get(slot_name)
                                    orig_mesh_path = orig_slots.get(slot_name)
                                    if comp and orig_mesh_path and hasattr(eal, "load_asset"):
                                        mesh_asset = eal.load_asset(orig_mesh_path)
                                        if mesh_asset and hasattr(comp, "set_skeletal_mesh_asset"):
                                            comp.set_skeletal_mesh_asset(mesh_asset)
                                            if body_comp and hasattr(comp, "set_leader_pose_component"):
                                                comp.set_leader_pose_component(body_comp, False)

                                # Destruir actor original del nivel y nombrar la copia con su misma etiqueta
                                editor_actor_subsystem.destroy_actor(target_actor)
                                new_actor.set_actor_label(actor_label)

                                outcome = {
                                    "success": True,
                                    "reused": False,
                                    "actor": new_actor,
                                    "actor_label": actor_label,
                                    "copy_bp_path": dest_bp_path,
                                    "original_bp_path": orig_bp_clean,
                                    "warnings": seq_warnings,
                                    "slots": snapshot_actor_slots(new_actor),
                                    "original_slots": orig_slots,
                                }
    except (RuntimeError, ValueError, TypeError) as exc:
        outcome = {"success": False, "error": f"{type(exc).__name__}: {exc}"}

    return outcome


def apply_slot_rules(
    actor: Any,
    garment_kind: str,
    original_slots: dict[str, str | None] | None = None,
) -> dict[str, Any]:
    """Aplica las reglas de ranuras y retiro de prendas según el tipo. @spec REQ-84"""
    # Guard clauses
    if actor is None:
        return {"success": False, "error": "Actor is required"}

    kind = str(garment_kind or "").strip().lower()
    if kind not in GARMENT_KINDS:
        return {"success": False, "error": f"Invalid garment kind '{garment_kind}'. Allowed: {list(GARMENT_KINDS)}"}

    by_name = _metahuman_components(actor)
    torso_comp = by_name.get("Torso")
    legs_comp = by_name.get("Legs")
    feet_comp = by_name.get("Feet")
    body_comp = by_name.get("Body")

    orig_slots = original_slots or {}
    eal = unreal.EditorAssetLibrary

    actions: list[str] = []
    restored: list[str] = []
    cleared: list[str] = []

    with unreal.ScopedEditorTransaction(f"DBV Unreal AI: Apply slot rules for {kind}"):
        actor.modify()

        # Tabla de REQ-84:
        # top   -> ocupa Torso, retira Torso
        # pants -> ocupa Legs, retira Legs
        # shoes -> ocupa Feet, retira Feet
        # skirt -> ocupa tela, retira Legs
        # dress -> ocupa tela, retira Torso y Legs
        if kind == "top":
            # Si venía de un vestido, Legs recupera la prenda original
            if legs_comp and not legs_comp.get_skeletal_mesh_asset():
                orig_legs = orig_slots.get("Legs")
                if orig_legs and hasattr(eal, "load_asset"):
                    mesh = eal.load_asset(orig_legs)
                    if mesh and hasattr(legs_comp, "set_skeletal_mesh_asset"):
                        legs_comp.set_skeletal_mesh_asset(mesh)
                        if body_comp and hasattr(legs_comp, "set_leader_pose_component"):
                            legs_comp.set_leader_pose_component(body_comp, False)
                        restored.append("Legs")

            if torso_comp and hasattr(torso_comp, "empty_override_materials"):
                torso_comp.empty_override_materials()
            actions.append("torso_ready")

        elif kind == "pants":
            # Si venía de un vestido, Torso recupera la prenda original
            if torso_comp and not torso_comp.get_skeletal_mesh_asset():
                orig_torso = orig_slots.get("Torso")
                if orig_torso and hasattr(eal, "load_asset"):
                    mesh = eal.load_asset(orig_torso)
                    if mesh and hasattr(torso_comp, "set_skeletal_mesh_asset"):
                        torso_comp.set_skeletal_mesh_asset(mesh)
                        if body_comp and hasattr(torso_comp, "set_leader_pose_component"):
                            torso_comp.set_leader_pose_component(body_comp, False)
                        restored.append("Torso")

            if legs_comp and hasattr(legs_comp, "empty_override_materials"):
                legs_comp.empty_override_materials()
            actions.append("legs_ready")

        elif kind == "shoes":
            if feet_comp and hasattr(feet_comp, "empty_override_materials"):
                feet_comp.empty_override_materials()
            actions.append("feet_ready")

        elif kind == "skirt":
            # Falda: retira Legs (oculta/vacía), Torso se mantiene o restaura
            if legs_comp and hasattr(legs_comp, "set_skeletal_mesh_asset"):
                legs_comp.set_skeletal_mesh_asset(None)
                cleared.append("Legs")

            if torso_comp and not torso_comp.get_skeletal_mesh_asset():
                orig_torso = orig_slots.get("Torso")
                if orig_torso and hasattr(eal, "load_asset"):
                    mesh = eal.load_asset(orig_torso)
                    if mesh and hasattr(torso_comp, "set_skeletal_mesh_asset"):
                        torso_comp.set_skeletal_mesh_asset(mesh)
                        if body_comp and hasattr(torso_comp, "set_leader_pose_component"):
                            torso_comp.set_leader_pose_component(body_comp, False)
                        restored.append("Torso")

            actions.append("skirt_cloth_slot_prepared")

        elif kind == "dress":
            # Vestido: retira Torso y Legs
            if torso_comp and hasattr(torso_comp, "set_skeletal_mesh_asset"):
                torso_comp.set_skeletal_mesh_asset(None)
                cleared.append("Torso")
            if legs_comp and hasattr(legs_comp, "set_skeletal_mesh_asset"):
                legs_comp.set_skeletal_mesh_asset(None)
                cleared.append("Legs")
            actions.append("dress_cloth_slots_prepared")

        # Asegurar que Feet siempre quede sincronizado con Body en el viewport (REQ-85)
        if feet_comp and body_comp and hasattr(feet_comp, "set_leader_pose_component"):
            feet_comp.set_leader_pose_component(body_comp, False)

    result = {
        "success": True,
        "garment_kind": kind,
        "actions": actions,
        "cleared_slots": cleared,
        "restored_slots": restored,
        "current_slots": snapshot_actor_slots(actor),
    }
    return result


def ensure_blueprint_leader_pose(blueprint_path: str) -> bool:
    """Asegura que Torso, Legs y Feet tengan SetLeaderPoseComponent(Body) en el Blueprint copiado. @spec REQ-85"""
    bound = False
    eal = getattr(unreal, "EditorAssetLibrary", None) if HAS_UNREAL else None
    if eal is not None and hasattr(eal, "does_asset_exist") and eal.does_asset_exist(blueprint_path):
        bp = eal.load_asset(blueprint_path)
        if bp is not None:
            try:
                from editor_toolset.toolsets.blueprint import BlueprintTools

                tools = BlueprintTools()
                graph = tools.get_graph(blueprint=bp, graph_name="EventGraph")
                if graph:
                    dsl = tools.read_graph_dsl(graph=graph) or ""
                    missing_slots = [
                        s for s in ("Torso", "Legs", "Feet")
                        if f"Get{s}" not in dsl or "SetLeaderPoseComponent" not in dsl
                    ]
                    if missing_slots:
                        lines = [
                            f"  (Components|SkinnedMesh|SetLeaderPoseComponent (Variables|Default|Get{s}) (Variables|Default|GetBody))"
                            for s in missing_slots
                        ]
                        lead_code = "(event EventBeginPlay\n" + "\n".join(lines) + ")\n"
                        tools.write_graph_dsl(graph=graph, code=lead_code)
                        if hasattr(unreal, "BlueprintEditorLibrary"):
                            unreal.BlueprintEditorLibrary.compile_blueprint(bp)
                        eal.save_asset(blueprint_path)
                    bound = True
                else:
                    bound = True
            except Exception:
                bound = False
    return bound


def revert_to_original(actor_label: str) -> dict[str, Any]:
    """Restaura el actor original en el nivel a partir de su Blueprint original. @spec REQ-85"""
    # Guard clauses
    if not actor_label or not actor_label.strip():
        return {"success": False, "error": "Actor label cannot be empty"}

    if _in_play():
        return {"success": False, "error": "Operation rejected during Play in Editor (PIE)"}

    editor_actor_subsystem = (
        unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
        if hasattr(unreal, "EditorActorSubsystem")
        else None
    )
    if editor_actor_subsystem is None:
        return {"success": False, "error": "EditorActorSubsystem unavailable"}

    actors = editor_actor_subsystem.get_all_level_actors() if hasattr(editor_actor_subsystem, "get_all_level_actors") else []
    target_actor = next((a for a in actors if a.get_actor_label() == actor_label), None)
    if target_actor is None:
        return {"success": False, "error": f"Actor '{actor_label}' not found in level"}

    if not is_character_copy(target_actor):
        return {"success": False, "error": f"Actor '{actor_label}' is not a DBV character copy; nothing to revert"}

    orig_bp_path = get_original_bp_path(target_actor)
    if not orig_bp_path:
        return {"success": False, "error": f"Could not determine original Blueprint path for '{actor_label}'"}

    outcome: dict[str, Any] = {"success": False}
    try:
        orig_bp_clean = orig_bp_path.split(".")[0]
        eal = unreal.EditorAssetLibrary
        orig_class = eal.load_blueprint_class(orig_bp_clean) if hasattr(eal, "load_blueprint_class") else None
        if orig_class is None:
            outcome = {"success": False, "error": f"Failed to load original Blueprint class at {orig_bp_clean}"}
        else:
            loc = target_actor.get_actor_location()
            rot = target_actor.get_actor_rotation()
            scale = target_actor.get_actor_scale3d() if hasattr(target_actor, "get_actor_scale3d") else unreal.Vector(1, 1, 1)
            folder = target_actor.get_folder_path() if hasattr(target_actor, "get_folder_path") else ""

            with unreal.ScopedEditorTransaction("DBV Unreal AI: Revert MetaHuman to original"):
                restored_actor = editor_actor_subsystem.spawn_actor_from_class(orig_class, loc, rot)
                if restored_actor is None:
                    outcome = {"success": False, "error": f"Failed to spawn original actor from class {orig_class}"}
                else:
                    if hasattr(restored_actor, "set_actor_scale3d"):
                        restored_actor.set_actor_scale3d(scale)
                    if folder and hasattr(restored_actor, "set_folder_path"):
                        restored_actor.set_folder_path(folder)

                    editor_actor_subsystem.destroy_actor(target_actor)
                    restored_actor.set_actor_label(actor_label)

                    outcome = {
                        "success": True,
                        "actor": actor_label,
                        "restored_bp_path": orig_bp_clean,
                        "slots": snapshot_actor_slots(restored_actor),
                    }
    except (RuntimeError, ValueError, TypeError) as exc:
        outcome = {"success": False, "error": f"{type(exc).__name__}: {exc}"}

    return outcome
