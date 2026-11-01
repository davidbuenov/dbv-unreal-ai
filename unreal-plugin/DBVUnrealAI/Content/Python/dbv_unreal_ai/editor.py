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
from pathlib import Path
import sys
from typing import Any, Callable

import unreal

from dbv_unreal_ai import analysis

#: Versión del contrato de herramientas (la app la compara con la que trae).
TOOLSET_VERSION = "0.3.2"


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


def load_gaussian_splat(source_file: str, actor_label: str = "DBV_GaussianSplat") -> str:
    """Carga un Gaussian Splat (.spz o .ply) en un ADbvGaussianSplatActor nativo.

    Aplica la orientación validada para World Labs (Roll 90.0, Escala 100x). Un fichero muy grande se carga entero
    pero el visor nativo dibuja solo sus splats más visibles: el informe lo dice (``splats``, ``drawn``, ``note``).
    Si la carga falla, no deja un actor vacío y explica el motivo. @spec REQ-54, REQ-63
    """
    result: dict[str, Any] = {"success": False}
    source_path = Path(source_file)
    editor_subsystem = getattr(unreal, "UnrealEditorSubsystem", None)
    playing = bool(editor_subsystem and unreal.get_editor_subsystem(editor_subsystem).get_game_world())
    if not source_path.exists():
        result["error"] = f"the Gaussian Splat file does not exist: {source_file}. Tell the user."
    elif playing:
        # Durante Play no se crean actores en el nivel del editor (visto en vivo el 2026-10-31: fallaba sin motivo).
        result["error"] = "Play is running: the splat can only be added to the level outside Play. Ask the user to stop Play and try again."
    else:
        actor_subsystem = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
        target_actor = None
        spawned = False
        for actor in actor_subsystem.get_all_level_actors():
            if actor.get_actor_label() == actor_label:
                target_actor = actor
                break

        if target_actor is None:
            splat_class = unreal.load_class(None, "/Script/DBVUnrealAICpp.DbvGaussianSplatActor")
            if splat_class is None:
                splat_class = getattr(unreal, "DbvGaussianSplatActor", None)

            if splat_class is not None:
                spawn_loc = unreal.Vector(0.0, 0.0, 0.0)
                spawn_rot = unreal.Rotator(pitch=0.0, yaw=0.0, roll=90.0)
                target_actor = actor_subsystem.spawn_actor_from_class(splat_class, spawn_loc, spawn_rot)
                if target_actor:
                    spawned = True
                    target_actor.set_actor_label(actor_label)
                    target_actor.set_actor_scale3d(unreal.Vector(100.0, 100.0, 100.0))
                    target_actor.set_folder_path(unreal.Name("DBV/Splats"))
                else:
                    result["error"] = "the splat actor could not be created in the level. Tell the user."
            else:
                result["error"] = "the DbvGaussianSplatActor class is not in this project: prepare the project again in the app (C++ toolset)."

        if target_actor is not None:
            component = getattr(target_actor, "splat_component", None)
            loaded = bool(component.load_file(str(source_path))) if component else False
            result["success"] = loaded
            result["actor"] = target_actor.get_actor_label()
            result["source_file"] = str(source_path)
            if loaded:
                result.update(_splat_counts(component))
            else:
                reason = str(component.get_editor_property("last_load_error")) if component else "the actor has no splat component"
                # Un actor vacío en el nivel solo confunde (en la prueba del 2026-10-31 la IA lo borró a mano).
                if spawned:
                    actor_subsystem.destroy_actor(target_actor)
                result["error"] = f"the Gaussian Splat could not be loaded: {reason}. Tell the user; do not try to load it with your own code."

    return _json(result)


def place_world(splat_label: str, collider_asset: str, name: str, metric_scale: float, ground_offset: float) -> str:
    """Coloca un mundo de World Labs con su escala métrica y el suelo en Z = 0. @spec REQ-18

    Verificado en vivo con el desierto (2026-10-31): el splat va a escala 100·m (metros → cm por el factor métrico) y
    la malla de colisión (.glb, en la misma convención RDF) necesita escala (m, m, −m), porque al importarla como
    glTF queda reflejada en Z. Los dos se suben 100·m·g para que el suelo quede en Z = 0. La colisión va oculta en el
    juego y usa su colisión compleja como simple: el movimiento de un personaje solo mira la simple, y sin esto
    caminaba sobre la caja de la malla.
    """
    result: dict[str, Any] = {"success": False}
    actor_subsystem = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
    actors = {actor.get_actor_label(): actor for actor in actor_subsystem.get_all_level_actors()}
    splat = actors.get(splat_label)
    if splat is None:
        result["error"] = f"no splat actor labelled {splat_label} in the level"
    else:
        scale = float(metric_scale) if metric_scale and metric_scale > 0 else 1.0
        lift = 100.0 * scale * float(ground_offset or 0.0)
        folder = unreal.Name(f"DBV/Worlds/{name}")
        splat.set_actor_scale3d(unreal.Vector(100.0 * scale, 100.0 * scale, 100.0 * scale))
        splat.set_actor_location(unreal.Vector(0.0, 0.0, lift), False, False)
        splat.set_folder_path(folder)
        result.update({"success": True, "splat": splat_label, "metric_scale": scale, "ground_lift_cm": round(lift, 1)})
        if not metric_scale or not ground_offset:
            result["note"] = "this world has no scale or ground metadata: it was placed as it comes. Tell the user."
        mesh = unreal.load_asset(collider_asset) if collider_asset else None
        if mesh is not None:
            body = mesh.get_editor_property("body_setup")
            if body is not None:
                body.set_editor_property("collision_trace_flag", unreal.CollisionTraceFlag.CTF_USE_COMPLEX_AS_SIMPLE)
                mesh.modify()
                unreal.EditorAssetLibrary.save_loaded_asset(mesh, False)
            label = f"{name}_Collider"
            collider = actors.get(label) or actor_subsystem.spawn_actor_from_object(mesh, unreal.Vector(0.0, 0.0, lift))
            collider.set_actor_label(label)
            collider.set_actor_location(unreal.Vector(0.0, 0.0, lift), False, False)
            collider.set_actor_rotation(unreal.Rotator(0.0, 0.0, 0.0), False)
            collider.set_actor_scale3d(unreal.Vector(scale, scale, -scale))
            collider.set_actor_hidden_in_game(True)
            # Invisible también en el editor (encima del splat lo ensuciaba); la colisión no depende de la visibilidad.
            component = collider.get_component_by_class(unreal.StaticMeshComponent)
            if component is not None:
                component.set_visibility(False, True)
            collider.set_folder_path(folder)
            result["collider"] = label
            starts = [actor for actor in actors.values() if isinstance(actor, unreal.PlayerStart)]
            lifted = _lift_player_starts(starts, _collider_ground(collider))
            if lifted:
                result["player_starts_lifted"] = lifted
    return _json(result)


# Altura del centro de la cápsula del jugador sobre el suelo (la de un PlayerStart recién puesto es 92 cm).
PLAYER_START_CLEARANCE = 100.0


def _collider_ground(collider: Any) -> Callable[[float, float], float | None]:
    """Altura del terreno del mundo en (x, y): solo cuenta un rayo que choque con su colisión. @spec REQ-18"""
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    label = collider.get_actor_label()

    def ground_at(x: float, y: float) -> float | None:
        hit = unreal.SystemLibrary.line_trace_single(
            world, unreal.Vector(x, y, 100000.0), unreal.Vector(x, y, -100000.0), unreal.TraceTypeQuery.ECC_VISIBILITY,
            False, [], unreal.DrawDebugTrace.NONE, True,
        ).to_tuple()
        return hit[5].z if hit[0] and hit[9] is not None and hit[9].get_actor_label() == label else None

    return ground_at


def _lift_player_starts(starts: list[Any], ground_at: Callable[[float, float], float | None]) -> list[str]:
    """Sube sobre el terreno del mundo los PlayerStart que quedan por debajo o a ras. @spec REQ-18

    Visto en vivo el 2026-10-31: el PlayerStart del nivel quedó a 92 cm con la duna a 104; en Play el personaje
    aparecía bajo la colisión, caía al suelo del nivel y veía la isla por encima. Los que ya están más altos se dejan.
    """
    lifted = []
    for start in starts:
        location = start.get_actor_location()
        ground = ground_at(location.x, location.y)
        if ground is not None and location.z < ground + PLAYER_START_CLEARANCE:
            start.set_actor_location(unreal.Vector(location.x, location.y, ground + PLAYER_START_CLEARANCE), False, False)
            lifted.append(start.get_actor_label())
    return lifted


def _splat_counts(component: Any) -> dict[str, Any]:
    """Splats del fichero, los que se dibujan y el grado de SH, con una nota si no son todos. @spec REQ-63"""
    asset = component.get_editor_property("splat_asset")
    drawn = int(asset.get_num_splats()) if asset else 0
    total = int(asset.get_total_splats()) if asset and hasattr(asset, "get_total_splats") else drawn
    sh_degree = int(asset.get_sh_degree()) if asset and hasattr(asset, "get_sh_degree") else 0
    counts: dict[str, Any] = {"splats": total, "drawn": drawn, "sh_degree": sh_degree}
    file_sh = int(asset.get_file_sh_degree()) if asset and hasattr(asset, "get_file_sh_degree") else sh_degree
    notes = []
    if drawn < total:
        notes.append(f"the video memory budget keeps the {drawn} most visible splats of {total}: the whole scene is there, less dense")
    if sh_degree < file_sh:
        notes.append(f"the view-dependent colour (spherical harmonics) was reduced from degree {file_sh} to {sh_degree} to fit the video memory")
    if notes:
        counts["note"] = "; ".join(notes) + ". Tell the user (dbv.Splat.VRAMBudgetMB raises the budget)."
    return counts


def get_undo_count() -> str:
    """Número de transacciones activas en el historial de deshacer de Unreal. @spec REQ-40"""
    count = 0
    if hasattr(unreal, "ToolsetLibrary") and hasattr(unreal.ToolsetLibrary, "get_active_undo_count"):
        count = int(unreal.ToolsetLibrary.get_active_undo_count())
    result = {"active_undo_count": count}
    return _json(result)


def undo_turn(count: int, expected_current: int = 0) -> str:
    """Deshace ``count`` transacciones en Unreal mediante callback post-tick de Slate. @spec REQ-40"""
    result = {"success": False, "undone": 0, "error": None}
    if count <= 0:
        result["error"] = "count must be positive"
    elif not hasattr(unreal, "ToolsetLibrary") or not hasattr(unreal.ToolsetLibrary, "undo_transaction"):
        result["error"] = "ToolsetLibrary.undo_transaction no disponible en este motor"
    else:
        current = int(unreal.ToolsetLibrary.get_active_undo_count())
        if expected_current > 0 and current not in (expected_current, expected_current + 1):
            result["error"] = f"El historial ha cambiado (esperado ~{expected_current}, encontrado {current})"
        else:
            handle_ref = [None]

            def _deferred_undo(delta_time):
                try:
                    for _ in range(count):
                        unreal.ToolsetLibrary.undo_transaction(False)
                finally:
                    if handle_ref[0] is not None:
                        unreal.unregister_slate_post_tick_callback(handle_ref[0])

            handle_ref[0] = unreal.register_slate_post_tick_callback(_deferred_undo)
            result["success"] = True
            result["undone"] = count
    return _json(result)

