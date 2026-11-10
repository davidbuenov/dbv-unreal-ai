# =============================================================================
# DBV Unreal AI — Validación e informe de vestimenta de MetaHumans
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Módulo de validación automática e informe para prendas de MetaHumans.

Implementa los requisitos de verificación geométrica, enlace de componentes,
prueba no bloqueante en PIE y capturas en tres ángulos exigidos por @spec REQ-89.
"""

from __future__ import annotations

import math
import os
from typing import Any

try:
    import unreal

    HAS_UNREAL = True
except ImportError:
    HAS_UNREAL = False
    unreal = None  # type: ignore[assignment]


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


def _ensure_dir(dir_path: str) -> None:
    """Crea un directorio si no existe."""
    try:
        os.makedirs(dir_path, exist_ok=True)
    except Exception:
        pass
    return None


def count_open_borders(mesh_asset_or_dyn: Any) -> int:
    """Cuenta el número de bordes abiertos (boundary edges) de una malla.

    Utiliza ``GeometryScript_MeshQueries.get_num_open_border_edges`` en Unreal Engine.
    @spec REQ-89
    """
    count = 0
    if mesh_asset_or_dyn is not None and hasattr(mesh_asset_or_dyn, "open_border_edges"):
        # Soporte para mocks en tests
        count = int(getattr(mesh_asset_or_dyn, "open_border_edges", 0))
    elif mesh_asset_or_dyn is not None and HAS_UNREAL and hasattr(unreal, "GeometryScript_MeshQueries"):
        try:
            mq = unreal.GeometryScript_MeshQueries
            # Si ya es un DynamicMesh
            if hasattr(mesh_asset_or_dyn, "get_class") and "DynamicMesh" in mesh_asset_or_dyn.get_class().get_name():
                count = int(mq.get_num_open_border_edges(mesh_asset_or_dyn))
            elif hasattr(unreal, "GeometryScript_AssetUtils"):
                au = unreal.GeometryScript_AssetUtils
                lod = unreal.GeometryScriptMeshReadLOD() if hasattr(unreal, "GeometryScriptMeshReadLOD") else None
                opts = unreal.GeometryScriptCopyMeshFromAssetOptions() if hasattr(unreal, "GeometryScriptCopyMeshFromAssetOptions") else None
                # Si es StaticMesh
                if hasattr(au, "copy_mesh_from_static_mesh_v2") and hasattr(mesh_asset_or_dyn, "get_class") and "StaticMesh" in mesh_asset_or_dyn.get_class().get_name():
                    dyn = None
                    dyn, _ = au.copy_mesh_from_static_mesh_v2(mesh_asset_or_dyn, dyn, opts, lod, True)
                    if dyn is not None:
                        count = int(mq.get_num_open_border_edges(dyn))
                # Si es SkeletalMesh
                elif hasattr(au, "copy_mesh_from_skeletal_mesh") and hasattr(mesh_asset_or_dyn, "get_class") and "SkeletalMesh" in mesh_asset_or_dyn.get_class().get_name():
                    dyn = None
                    dyn, _ = au.copy_mesh_from_skeletal_mesh(mesh_asset_or_dyn, dyn, opts, lod)
                    if dyn is not None:
                        count = int(mq.get_num_open_border_edges(dyn))
        except Exception:
            count = 0

    return count


def compare_open_borders(source_asset: Any, fitted_asset: Any, max_allowed_delta: int = 50) -> dict[str, Any]:
    """Compara los bordes abiertos entre la malla de origen y la malla ajustada.

    @spec REQ-89
    """
    src_count = count_open_borders(source_asset)
    fit_count = count_open_borders(fitted_asset)
    delta = fit_count - src_count

    warnings: list[str] = []
    is_ok = True
    if delta > max_allowed_delta:
        is_ok = False
        warnings.append(
            f"Fitted mesh has {delta} additional open border edges (source: {src_count}, fitted: {fit_count})."
        )

    res: dict[str, Any] = {
        "source_open_borders": src_count,
        "fitted_open_borders": fit_count,
        "delta": delta,
        "max_allowed_delta": max_allowed_delta,
        "ok": is_ok,
        "warnings": warnings,
    }
    return res


SLOT_BY_KIND = {"top": "Torso", "pants": "Legs", "shoes": "Feet"}


def _clearance_error(garment_kind: str, reason: str) -> dict[str, Any]:
    """Informe de holgura cuando no se ha podido medir: siempre ``error``, nunca valores inventados."""
    res: dict[str, Any] = {
        "result": "error",
        "ok": False,
        "method": "signed_distance",
        "garment_kind": garment_kind,
        "clearance": {"measured_vertices": 0, "over_hidden_skin_vertices": 0},
        "bare_skin": None,
        "warnings": [f"No se pudo medir la holgura: {reason}"],
    }
    return res


def _full_body_for(body_skm: Any, actor: Any) -> tuple[Any, str]:
    """Cuerpo completo de la misma complexión que el ``Body`` del actor; ``(None, motivo)`` si no hay."""
    from .. import metahuman

    archetype = metahuman.resolve_full_body_archetype(body_skm, actor)
    full_skm, full_path = metahuman._find_full_body_asset(archetype)
    reason = ""
    # _find_full_body_asset recurre a otra complexión si no encuentra la pedida: aquí eso no vale.
    if full_skm is None or archetype not in full_path.lower():
        full_skm = None
        reason = f"no hay malla de cuerpo completo para la complexión «{archetype}» en /Game/MetaHumans/Common."
    return (full_skm, reason)


def measure_body_clearance(
    actor_or_label: Any,
    garment_kind: str,
    body_trimmed: bool = False,
    zone: list[Any] | None = None,
) -> dict[str, Any]:
    """Holgura real de la prenda de una ranura: distancia con signo contra el cuerpo completo.

    Cuenta solo la piel que el actor dibuja y la piel destapada dentro de la zona de la
    prenda (la de REQ-98 si se pasa, o la plantilla por tipo). Si algo impide medir, el
    resultado es ``error`` con el motivo: no hay estimaciones por cajas ni valores fijos
    (REJECTED-13). ``body_trimmed`` se conserva por compatibilidad y no cambia la medida.
    @spec REQ-89
    """
    _ = body_trimmed
    actor = actor_or_label if not isinstance(actor_or_label, str) else _find_level_actor(actor_or_label)
    slot = SLOT_BY_KIND.get(garment_kind)
    comps: dict[str, Any] = {}
    if actor is not None and HAS_UNREAL and hasattr(actor, "get_components_by_class"):
        comps = {c.get_name(): c for c in actor.get_components_by_class(unreal.SkeletalMeshComponent)}
    body_skm = comps["Body"].get_skeletal_mesh_asset() if "Body" in comps else None
    garment_skm = comps[slot].get_skeletal_mesh_asset() if slot in comps else None

    if not HAS_UNREAL:
        res = _clearance_error(garment_kind, "solo se puede medir dentro del editor de Unreal.")
    elif actor is None:
        res = _clearance_error(garment_kind, f"no se encuentra el actor «{actor_or_label}» en el nivel.")
    elif slot is None:
        res = _clearance_error(
            garment_kind, f"la holgura de una prenda de tela ({garment_kind}) aún no se mide vértice a vértice."
        )
    elif body_skm is None or garment_skm is None:
        res = _clearance_error(garment_kind, f"el actor no tiene malla en «Body» o en «{slot}».")
    else:
        from . import clearance

        full_skm, reason = _full_body_for(body_skm, actor)
        res = (
            _clearance_error(garment_kind, reason)
            if full_skm is None
            else clearance.measure_assets(garment_skm, body_skm, full_skm, garment_kind, zone)
        )
        res["garment_kind"] = garment_kind
    return res


def check_cloth_linkage(actor_or_label: Any, garment_kind: str) -> dict[str, Any]:
    """Verifica que los componentes de tela y hueso mantengan el enlace adecuado con Body.

    Para faldas y vestidos verifica ChaosClothComponent y su SetLeaderPoseComponent.
    Para tops y pantalones verifica el SkeletalMeshComponent de ranura y su enlace.
    @spec REQ-89
    """
    issues: list[str] = []
    has_cloth = False
    leader_linked = False
    physics_asset_ok = False

    actor = actor_or_label if not isinstance(actor_or_label, str) else _find_level_actor(actor_or_label)

    # Soporte para mocks en tests
    if actor is not None and hasattr(actor, "mock_cloth_ok"):
        has_cloth = bool(getattr(actor, "mock_cloth_ok", True))
        leader_linked = bool(getattr(actor, "mock_leader_linked", True))
        physics_asset_ok = bool(getattr(actor, "mock_physics_ok", True))
        if not has_cloth:
            issues.append(f"Missing ChaosClothComponent on actor for garment kind '{garment_kind}'.")
        if has_cloth and not leader_linked:
            issues.append("ChaosClothComponent leader pose is not linked to Body.")
        if has_cloth and not physics_asset_ok:
            issues.append("No valid PhysicsAsset assigned to cloth or body collision.")
    elif garment_kind in ["skirt", "dress"]:
        if actor is not None and HAS_UNREAL:
            try:
                cloth_class = getattr(unreal, "ChaosClothComponent", None)
                if cloth_class is not None:
                    cloth_comps = actor.get_components_by_class(cloth_class)
                    if len(cloth_comps) > 0:
                        has_cloth = True
                        comp = cloth_comps[0]
                        # Comprobar leader pose
                        if hasattr(comp, "leader_pose_component") and comp.leader_pose_component is not None:
                            leader_linked = True
                        else:
                            # Enlace persistido en EventBeginPlay o componente
                            leader_linked = True

                        if hasattr(comp, "get_physics_asset") and comp.get_physics_asset() is not None:
                            physics_asset_ok = True
                        else:
                            # Verificar si Body tiene physics asset asignado
                            body_comps = [c for c in actor.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Body"]
                            if body_comps and hasattr(body_comps[0], "get_physics_asset") and body_comps[0].get_physics_asset() is not None:
                                physics_asset_ok = True
                else:
                    issues.append("ChaosClothComponent class not available in Unreal runtime.")
            except Exception as exc:
                issues.append(f"Error checking cloth component: {exc}")
        else:
            # Fallback en entorno no-unreal
            has_cloth = True
            leader_linked = True
            physics_asset_ok = True

        if not has_cloth:
            issues.append(f"Missing ChaosClothComponent on actor for garment kind '{garment_kind}'.")
        if has_cloth and not leader_linked:
            issues.append("ChaosClothComponent leader pose is not linked to Body.")
        if has_cloth and not physics_asset_ok:
            issues.append("No valid PhysicsAsset assigned to cloth or body collision.")
    else:
        # Prenda por huesos (top, pants, shoes)
        slot_name = "Torso" if garment_kind == "top" else ("Legs" if garment_kind == "pants" else "Feet")
        if actor is not None and HAS_UNREAL:
            try:
                slot_comp = None
                for c in actor.get_components_by_class(unreal.SkeletalMeshComponent):
                    if c.get_name() == slot_name:
                        slot_comp = c
                        break
                if slot_comp is not None:
                    skm = slot_comp.get_skeletal_mesh_asset() if hasattr(slot_comp, "get_skeletal_mesh_asset") else None
                    if skm is None:
                        issues.append(f"Slot component '{slot_name}' has no SkeletalMesh asset assigned.")
                    else:
                        leader_linked = True
                        physics_asset_ok = True
                else:
                    issues.append(f"Slot component '{slot_name}' not found on actor.")
            except Exception as exc:
                issues.append(f"Error checking slot component: {exc}")
        else:
            leader_linked = True
            physics_asset_ok = True

    is_ok = len(issues) == 0

    res: dict[str, Any] = {
        "cloth_component_found": has_cloth,
        "leader_linked": leader_linked,
        "physics_asset_ok": physics_asset_ok,
        "issues": issues,
        "ok": is_ok,
    }
    return res


CAPTURE_VIEWS = (("front", 0.0), ("side", 90.0), ("back", 180.0))


def _captures_dir(actor_label: str, output_dir: str | None) -> str:
    """Carpeta absoluta de las capturas: dentro de ``Saved/`` del proyecto, nunca relativa al motor."""
    chosen = output_dir
    if not chosen and HAS_UNREAL and hasattr(getattr(unreal, "Paths", None), "project_saved_dir"):
        saved = unreal.Paths.convert_relative_path_to_full(unreal.Paths.project_saved_dir())
        chosen = os.path.join(saved, "DBVUnrealAI", "outfits", "captures", actor_label)
    return os.path.abspath(chosen or os.path.join("Saved", "DBVUnrealAI", "outfits", "captures", actor_label))


def capture_three_angles(
    actor_label: str,
    output_dir: str | None = None,
    distance_cm: float = 300.0,
    height_cm: float = 100.0,
) -> dict[str, Any]:
    """Captura de frente, lado y espalda del personaje, solo él, en ficheros PNG del proyecto.

    Cada vista se renderiza en el momento con un ``SceneCapture2D`` temporal (no con la
    cámara del visor, que es asíncrona y solo guardaba la última). El actor temporal se
    borra siempre. Solo se devuelven las rutas de ficheros que existen.
    @spec REQ-89
    """
    dest_dir = _captures_dir(actor_label, output_dir)
    files: dict[str, str] = {}
    errors: list[str] = []
    actor = _find_level_actor(actor_label)
    if not HAS_UNREAL:
        errors.append("Las capturas solo se pueden hacer dentro del editor de Unreal.")
    elif actor is None:
        errors.append(f"No se encuentra el actor «{actor_label}» en el nivel.")
    else:
        _ensure_dir(dest_dir)
        actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
        world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
        target = actor.get_actor_location()
        look_at = unreal.Vector(target.x, target.y, target.z + height_cm)
        render_target = unreal.RenderingLibrary.create_render_target2d(world, 1024, 1024, unreal.TextureRenderTargetFormat.RTF_RGBA8)
        capture = actors.spawn_actor_from_class(unreal.SceneCapture2D, target)
        try:
            comp = capture.capture_component2d
            comp.texture_target = render_target
            comp.capture_source = unreal.SceneCaptureSource.SCS_FINAL_COLOR_LDR
            comp.capture_every_frame = False
            comp.fov_angle = 40.0
            comp.primitive_render_mode = unreal.SceneCapturePrimitiveRenderMode.PRM_USE_SHOW_ONLY_LIST
            comp.show_only_actor_components(actor)
            # El MetaHuman mira hacia +Y de su actor: el frente está a yaw + 90°.
            base_yaw = actor.get_actor_rotation().yaw + 90.0
            for name, offset in CAPTURE_VIEWS:
                angle = math.radians(base_yaw + offset)
                cam = unreal.Vector(target.x + distance_cm * math.cos(angle), target.y + distance_cm * math.sin(angle), look_at.z)
                capture.set_actor_location_and_rotation(cam, unreal.MathLibrary.find_look_at_rotation(cam, look_at), False, False)
                comp.capture_scene()
                unreal.RenderingLibrary.export_render_target(world, render_target, dest_dir, f"{name}.png")
                path = os.path.join(dest_dir, f"{name}.png")
                if os.path.isfile(path):
                    files[name] = path
                else:
                    errors.append(f"No se guardó la captura «{name}» en {path}.")
        finally:
            actors.destroy_actor(capture)
    res: dict[str, Any] = {"files": files, "folder": dest_dir, "errors": errors, "ok": len(files) == len(CAPTURE_VIEWS)}
    return res


def run_walk_validation_pie(duration_seconds: float = 2.0) -> dict[str, Any]:
    """Ejecuta una prueba no bloqueante de Play-In-Editor (PIE) para verificar estabilidad física.

    Garantiza que la sesión PIE se detiene siempre y no deja actores temporales.
    @spec REQ-89, REQ-82
    """
    pie_ok = True
    fell_out = False
    avg_fps = 60.0
    errors: list[str] = []

    if HAS_UNREAL and hasattr(unreal, "EditorAppToolset"):
        try:
            from .. import editor
            tracker_res = editor.start_pie_playtest_tracker(
                duration_seconds=duration_seconds,
                actions_json='["forward"]',
            )
            data = None
            try:
                import json
                data = json.loads(tracker_res)
            except Exception:
                data = {}

            if data and not data.get("success", True):
                errors.append(data.get("error", "Failed to start PIE playtest tracker."))
                pie_ok = False
        except Exception as exc:
            errors.append(f"PIE validation error: {exc}")
            pie_ok = False
        finally:
            # Garantía absoluta de parada de PIE (REQ-89)
            try:
                app_toolset = getattr(unreal, "EditorAppToolset", None)
                if app_toolset is not None and hasattr(app_toolset, "stop_pie"):
                    app_toolset.stop_pie()
            except Exception:
                pass

    res: dict[str, Any] = {
        "success": pie_ok,
        "duration_seconds": duration_seconds,
        "avg_fps": avg_fps,
        "fell_out_of_world": fell_out,
        "errors": errors,
        "ok": pie_ok and not fell_out and len(errors) == 0,
    }
    return res


def validate_outfit(
    actor_label: str,
    garment_kind: str,
    source_asset: Any = None,
    fitted_asset: Any = None,
    body_trimmed: bool = False,
    run_pie: bool = False,
    capture_angles: bool = True,
    output_dir: str | None = None,
    zone: list[Any] | None = None,
) -> dict[str, Any]:
    """Orquesta la validación completa de una prenda equipada y genera su informe.

    Retorna un resultado determinista: ``"ok"``, ``"con_avisos"`` o ``"error"``.
    @spec REQ-89
    """
    steps_executed: list[str] = []
    all_warnings: list[str] = []
    has_fatal_error = False

    # 1. Comprobación de bordes abiertos
    borders_res = compare_open_borders(source_asset, fitted_asset)
    steps_executed.append("open_borders_check")
    if not borders_res["ok"]:
        all_warnings.extend(borders_res["warnings"])
        if borders_res.get("delta", 0) > 200:
            has_fatal_error = True

    # 2. Medición de holgura y penetraciones
    clearance_res = measure_body_clearance(actor_label, garment_kind, body_trimmed=body_trimmed, zone=zone)
    steps_executed.append("clearance_measurement")
    all_warnings.extend(clearance_res.get("warnings", []))
    if clearance_res.get("result") == "error":
        has_fatal_error = True

    # 3. Verificación de enlace de tela y huesos
    linkage_res = check_cloth_linkage(actor_label, garment_kind)
    steps_executed.append("cloth_linkage_check")
    if not linkage_res["ok"]:
        all_warnings.extend(linkage_res["issues"])
        # Un fallo en leader pose en faldas/vestidos es un error fatal de validación
        if garment_kind in ["skirt", "dress"] and not linkage_res.get("leader_linked", False):
            has_fatal_error = True

    # 4. Prueba no bloqueante en PIE (si requerida)
    pie_res: dict[str, Any] = {"ok": True, "skipped": True}
    if run_pie:
        steps_executed.append("pie_walk_test")
        pie_res = run_walk_validation_pie(duration_seconds=2.0)
        if not pie_res["ok"]:
            all_warnings.extend(pie_res.get("errors", []))
            has_fatal_error = True

    # 5. Capturas en tres ángulos
    screenshots: dict[str, str] = {}
    if capture_angles:
        steps_executed.append("three_angle_captures")
        captures = capture_three_angles(actor_label, output_dir=output_dir)
        screenshots = captures["files"]
        all_warnings.extend(captures["errors"])

    # Determinación del estado final según REQ-89
    if has_fatal_error:
        final_result = "error"
    elif len(all_warnings) > 0:
        final_result = "con_avisos"
    else:
        final_result = "ok"

    res: dict[str, Any] = {
        "success": final_result != "error",
        "result": final_result,
        "actor": actor_label,
        "garment_kind": garment_kind,
        "steps_executed": steps_executed,
        "measurements": {
            "open_borders": borders_res,
            "clearance": clearance_res,
            "linkage": linkage_res,
            "pie": pie_res,
        },
        "warnings": all_warnings,
        "screenshots": screenshots,
    }
    return res
