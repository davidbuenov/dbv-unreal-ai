# =============================================================================
# DBV Unreal AI — Herramientas del toolset que necesitan el editor
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Implementación compartida por la variante Python (``toolset``) y la C++ (``cpp_bridge``).

Cada función devuelve una cadena JSON. @spec REQ-06, REQ-25, REQ-26
"""

from __future__ import annotations

import json
import os
import sys
from typing import Any

import unreal

from dbv_unreal_ai import analysis

#: Versión del contrato de herramientas (la app la compara con la que trae).
TOOLSET_VERSION = "0.2.0"


def _json(value: Any) -> str:
    """Serializa el resultado de una herramienta."""
    return json.dumps(value, ensure_ascii=False, default=str)


def _transaction() -> Any:
    """Contexto de deshacer del editor para lo que ejecuta ``run_python``."""
    return unreal.ScopedEditorTransaction("DBV Unreal AI")


def _round(vector: Any) -> list[float]:
    """Vector de Unreal → lista con un decimal."""
    return [round(vector.x, 1), round(vector.y, 1), round(vector.z, 1)]


def run_python(code: str) -> str:
    """Ejecuta ``code`` en una transacción del editor. @spec REQ-06"""
    return _json(analysis.run_code(code, unreal, _transaction))


def validate_python(code: str) -> str:
    """Valida ``code`` contra el motor en marcha sin ejecutarlo. @spec REQ-26"""
    return _json(analysis.validate_code(code, unreal))


def describe_python_api(name: str) -> str:
    """Referencia de ``unreal.<name>``. @spec REQ-25"""
    return _json(analysis.describe_name(name, unreal))


def get_project_info() -> str:
    """Versión del motor, proyecto, nivel abierto y versión del toolset. @spec REQ-06"""
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    info = {
        "engine_version": unreal.SystemLibrary.get_engine_version(),
        "project_name": unreal.SystemLibrary.get_game_name(),
        "project_dir": unreal.Paths.convert_relative_path_to_full(unreal.Paths.project_dir()),
        "content_dir": unreal.Paths.convert_relative_path_to_full(unreal.Paths.project_content_dir()),
        "level": world.get_path_name() if world else None,
        "python_version": sys.version.split()[0],
        "toolset_version": TOOLSET_VERSION,
    }
    return _json(info)


def _unique_name(folder_path: str, asset_name: str) -> str:
    """``asset_name``, o ``asset_name_2``, ``_3``… si ya existe en ``folder_path``."""
    candidate, n = asset_name, 2
    while unreal.EditorAssetLibrary.does_asset_exist(f"{folder_path}/{candidate}"):
        candidate, n = f"{asset_name}_{n}", n + 1
    return candidate


def _classify(paths: list[str]) -> dict[str, list[str]]:
    """Separa lo importado por tipo: mallas estáticas, esqueléticas, esqueletos y animaciones."""
    kinds: dict[str, list[str]] = {"static": [], "skeletal": [], "skeleton": [], "animation": []}
    for path in paths:
        asset = unreal.load_asset(path)
        for kind, cls in (("skeletal", unreal.SkeletalMesh), ("static", unreal.StaticMesh), ("skeleton", unreal.Skeleton), ("animation", unreal.AnimSequence)):
            if isinstance(asset, cls):
                kinds[kind].append(path)
                break
    return kinds


def import_model(source_file: str, folder_path: str = "/Game/DBV/Generated", asset_name: str = "") -> str:
    """Importa un modelo (glTF/GLB, FBX, OBJ) dejando que el motor elija el importador. @spec REQ-19, REQ-16

    ``StaticMeshTools.import_file`` de Epic fuerza ``FbxFactory`` y rechaza los ``.glb`` de Tripo;
    aquí no se fija fábrica, así que glTF entra por Interchange con sus materiales y texturas. Un GLB con
    esqueleto (un rig de Tripo) da un Skeletal Mesh con su Skeleton y, si trae clips, sus Animation Sequences:
    entonces el principal es el Skeletal Mesh y se devuelven las animaciones para reproducirlas al colocarlo.
    """
    if not os.path.isfile(source_file):
        return _json({"success": False, "error": f"file not found: {source_file}"})
    if not folder_path.startswith("/Game/"):
        return _json({"success": False, "error": "folder_path must be under /Game/"})
    if not unreal.EditorAssetLibrary.does_directory_exist(folder_path):
        unreal.EditorAssetLibrary.make_directory(folder_path)
    base = asset_name.strip() or os.path.splitext(os.path.basename(source_file))[0]
    name = _unique_name(folder_path, base)
    task = unreal.AssetImportTask()
    task.filename = source_file
    task.destination_path = folder_path
    task.destination_name = name
    task.replace_existing = False
    task.automated = True
    task.save = True
    unreal.AssetToolsHelpers.get_asset_tools().import_asset_tasks([task])
    paths = [str(p) for p in task.get_editor_property("imported_object_paths")]
    kinds = _classify(paths)
    primary = (kinds["skeletal"] or kinds["static"] or paths or [None])[0]
    result = {
        "success": bool(paths),
        "imported": paths,
        "primary": primary,
        "kind": "skeletal" if kinds["skeletal"] else "static" if kinds["static"] else "other",
        "skeleton": (kinds["skeleton"] or [None])[0],
        "animations": kinds["animation"],
    }
    if not paths:
        result["error"] = "the import produced no assets (unsupported or invalid file)"
    return _json(result)


def place_asset(asset_path: str, distance: float = 300.0, folder: str = "DBV/Generated") -> str:
    """Coloca ``asset_path`` delante de la cámara del editor, apoyado en el suelo, en una carpeta del Outliner. @spec REQ-19

    Delante de la cámara a ``distance`` cm suele quedar a media altura: en vivo (S18) un personaje de Tripo quedó
    flotando a 80 cm. Si un rayo hacia abajo desde ese punto da con algo, la base de los límites del actor se apoya
    en el impacto; si no (un nivel vacío), se queda donde estaba.
    """
    asset = unreal.load_asset(asset_path)
    if asset is None:
        return _json({"success": False, "error": f"asset not found: {asset_path}"})
    location, rotation = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_level_viewport_camera_info()
    target = location + rotation.get_forward_vector() * float(distance)
    grounded = False
    with unreal.ScopedEditorTransaction("DBV Unreal AI: place asset"):
        actor = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).spawn_actor_from_object(asset, target)
        if actor:
            grounded = _drop_to_ground(actor)
        if actor and folder:
            actor.set_folder_path(folder)
    placed = actor.get_actor_location() if actor else target
    result = {"success": actor is not None, "actor": actor.get_actor_label() if actor else None, "location": _round(placed), "on_ground": grounded, "folder": folder}
    return _json(result)


def _drop_to_ground(actor: Any) -> bool:
    """Baja ``actor`` hasta que la base de sus límites toca lo que haya debajo. ``True`` si encontró suelo."""
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    start = actor.get_actor_location()
    end = start - unreal.Vector(0.0, 0.0, 100000.0)
    hit = unreal.SystemLibrary.line_trace_single(world, start, end, unreal.TraceTypeQuery.ECC_VISIBILITY, False, [actor], unreal.DrawDebugTrace.NONE, True)
    # `HitResult` no publica sus campos por nombre en Python (5.8); su tupla sí: 0 = blocking_hit, 5 = impact_point.
    fields = hit.to_tuple() if hit is not None else ()
    blocking = bool(fields and fields[0])
    if blocking:
        origin, extent = actor.get_actor_bounds(False)
        floor = fields[5].z
        actor.set_actor_location(start + unreal.Vector(0.0, 0.0, floor - (origin.z - extent.z)), False, False)
    return blocking


def play_animation(actor_label: str, animation_path: str, looping: bool = True) -> str:
    """Pone a ``actor_label`` (un Skeletal Mesh del nivel) a reproducir ``animation_path`` en el editor. @spec REQ-16, REQ-19

    Modo «una sola animación» del componente: se ve en el viewport sin Animation Blueprint, que es lo que hace
    falta para revisar un baile de Tripo; para un personaje jugable se crea luego su Animation Blueprint.

    Un clip de Tripo se importa con SU esqueleto (el GLB de la animación trae su propia copia del rig), y Unreal no
    reproduce una animación de otro esqueleto. Visto en vivo (S18, 2026-10-26): antes se respondía «success» sin que
    se moviera nada. Si todas las pistas del clip existen en el esqueleto del actor, se declara compatible
    (``add_compatible_skeleton``, lo mismo que la lista «Compatible Skeletons» del editor); si no, se explica.
    """
    animation = unreal.load_asset(animation_path)
    if not isinstance(animation, unreal.AnimSequence):
        return _json({"success": False, "error": f"not an animation: {animation_path}"})
    actors = [a for a in unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors() if a.get_actor_label() == actor_label]
    component = actors[0].get_component_by_class(unreal.SkeletalMeshComponent) if actors else None
    if component is None:
        return _json({"success": False, "error": f"no skeletal mesh actor labelled {actor_label}"})
    mesh = component.get_skeletal_mesh_asset()
    mesh_skeleton = mesh.get_editor_property("skeleton") if mesh else None
    anim_skeleton = animation.get_editor_property("skeleton")
    missing = _missing_bones(component, animation) if anim_skeleton != mesh_skeleton else []
    if missing:
        return _json({"success": False, "error": f"the animation belongs to another skeleton and {len(missing)} of its bones are not in {actor_label} (e.g. {', '.join(missing[:5])}): animate the rig this mesh comes from"})
    compatible = anim_skeleton != mesh_skeleton
    with unreal.ScopedEditorTransaction("DBV Unreal AI: play animation"):
        if compatible:
            mesh_skeleton.modify()
            mesh_skeleton.add_compatible_skeleton(anim_skeleton)
        component.set_animation_mode(unreal.AnimationMode.ANIMATION_SINGLE_NODE)
        # Lo que se guarda con el nivel...
        component.override_animation_data(animation, bool(looping), True, 0.0, 1.0)
        # ...y lo que hace que se vea ya: en vivo, solo con lo anterior la instancia se quedó sin clip, y sin
        # «actualizar en el editor» el viewport no la mueve fuera de una partida (PIE).
        component.play_animation(animation, bool(looping))
        component.set_update_animation_in_editor(True)
    return _json({"success": True, "actor": actor_label, "animation": animation_path, "looping": bool(looping), "compatible_skeleton_added": compatible})


def _missing_bones(component: Any, animation: Any) -> list[str]:
    """Pistas del clip que no son huesos del componente.

    Sin distinguir mayúsculas, como los ``FName`` de Unreal: en vivo, la malla del rig de Tripo tenía ``Root``/``Hip``
    y su clip ``root``/``hip``, y son los mismos huesos.
    """
    bones = {str(component.get_bone_name(i)).lower() for i in range(component.get_num_bones())}
    return [str(name) for name in unreal.AnimationLibrary.get_animation_track_names(animation) if str(name).lower() not in bones]


def get_scene_summary(name_filter: str = "", class_filter: str = "", limit: int = 200) -> str:
    """Actores del nivel abierto en formato compacto. @spec REQ-06, REQ-13"""
    actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
    name_part, class_part = (name_filter or "").lower(), (class_filter or "").lower()
    counts: dict[str, int] = {}
    listed: list[dict[str, Any]] = []
    maximum = max(1, min(int(limit), 1000))
    for actor in actors:
        label, cls = actor.get_actor_label(), actor.get_class().get_name()
        if name_part in label.lower() and class_part in cls.lower():
            counts[cls] = counts.get(cls, 0) + 1
            if len(listed) < maximum:
                listed.append({"label": label, "class": cls, "location": _round(actor.get_actor_location()), "folder": str(actor.get_folder_path())})
    total = sum(counts.values())
    return _json({"total": total, "counts": counts, "actors": listed, "truncated": total > len(listed)})
