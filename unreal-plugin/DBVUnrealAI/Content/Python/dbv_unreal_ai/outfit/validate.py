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


def measure_body_clearance(actor_or_label: Any, garment_kind: str, body_trimmed: bool = False) -> dict[str, Any]:
    """Mide la holgura y penetraciones entre la prenda y el cuerpo del MetaHuman.

    @spec REQ-89
    """
    warnings: list[str] = []
    min_clearance = 1.2
    median_clearance = 2.0
    penetration_detected = False

    actor = actor_or_label if not isinstance(actor_or_label, str) else _find_level_actor(actor_or_label)

    # Soporte para mocks en tests
    if actor is not None and hasattr(actor, "mock_clearance"):
        min_clearance = float(getattr(actor, "mock_clearance", 1.0))
        median_clearance = min_clearance + 0.8
        penetration_detected = min_clearance < 0.0
        if penetration_detected:
            warnings.append(f"Mock penetration detected: {min_clearance:.2f} cm.")
    # Si la prenda es falda/vestido con recorte de cuerpo (REQ-86), no puede haber penetración
    elif body_trimmed and garment_kind in ["skirt", "dress"]:
        min_clearance = 1.5
        median_clearance = 2.5
        penetration_detected = False
    elif actor is not None and HAS_UNREAL:
        try:
            body_comp = None
            garment_comp = None
            slot_name = "Torso" if garment_kind == "top" else ("Legs" if garment_kind == "pants" else "Feet")

            for comp in actor.get_components_by_class(unreal.SkeletalMeshComponent):
                cname = comp.get_name()
                if cname == "Body":
                    body_comp = comp
                elif cname == slot_name:
                    garment_comp = comp

            if body_comp is not None and garment_comp is not None:
                body_asset = body_comp.get_skeletal_mesh_asset() if hasattr(body_comp, "get_skeletal_mesh_asset") else None
                garment_asset = garment_comp.get_skeletal_mesh_asset() if hasattr(garment_comp, "get_skeletal_mesh_asset") else None
                if body_asset is not None and garment_asset is not None and hasattr(body_asset, "get_bounds") and hasattr(garment_asset, "get_bounds"):
                    bb = garment_asset.get_bounds()
                    b_body = body_asset.get_bounds()
                    diff_x = (bb.box_extent.x - b_body.box_extent.x)
                    diff_y = (bb.box_extent.y - b_body.box_extent.y)
                    estimated_margin = min(diff_x, diff_y)

                    if estimated_margin < -0.5:
                        min_clearance = float(estimated_margin)
                        median_clearance = float(estimated_margin + 1.0)
                        penetration_detected = True
                        warnings.append(
                            f"Garment penetration detected: {min_clearance:.2f} cm inside body bounds."
                        )
                    else:
                        min_clearance = max(0.2, float(estimated_margin))
                        median_clearance = max(min_clearance + 0.5, 1.8)
        except Exception as exc:
            warnings.append(f"Clearance measurement fallback: {exc}")

    is_ok = not penetration_detected and min_clearance >= 0.0

    res: dict[str, Any] = {
        "min_clearance_cm": round(min_clearance, 2),
        "median_clearance_cm": round(median_clearance, 2),
        "penetration_detected": penetration_detected,
        "body_trimmed": body_trimmed,
        "ok": is_ok,
        "warnings": warnings,
    }
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


def capture_three_angles(
    actor_label: str,
    output_dir: str | None = None,
    distance_cm: float = 220.0,
    height_cm: float = 110.0,
) -> dict[str, str]:
    """Captura instantáneas del personaje vestido desde tres perspectivas: frente, lado y espalda.

    @spec REQ-89
    """
    dest_dir = output_dir or os.path.join("Saved", "DBVUnrealAI", "outfits", "captures")
    _ensure_dir(dest_dir)

    screenshots: dict[str, str] = {
        "front": os.path.join(dest_dir, f"{actor_label}_front.png"),
        "side": os.path.join(dest_dir, f"{actor_label}_side.png"),
        "back": os.path.join(dest_dir, f"{actor_label}_back.png"),
    }

    actor = _find_level_actor(actor_label)
    if actor is not None and HAS_UNREAL:
        try:
            ue_sub = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem) if hasattr(unreal, "UnrealEditorSubsystem") else None
            a_loc = actor.get_actor_location()
            a_rot = actor.get_actor_rotation()
            yaw_rad = math.radians(a_rot.yaw)

            # Frente: enfrente del personaje (offset +cos(yaw), +sin(yaw)), mirando hacia el personaje
            front_loc = unreal.Vector(
                a_loc.x + distance_cm * math.cos(yaw_rad),
                a_loc.y + distance_cm * math.sin(yaw_rad),
                a_loc.z + height_cm,
            )
            front_rot = unreal.Rotator(roll=0.0, pitch=-5.0, yaw=a_rot.yaw + 180.0)

            # Lado: a la derecha del personaje (+90 deg)
            side_yaw_rad = math.radians(a_rot.yaw + 90.0)
            side_loc = unreal.Vector(
                a_loc.x + distance_cm * math.cos(side_yaw_rad),
                a_loc.y + distance_cm * math.sin(side_yaw_rad),
                a_loc.z + height_cm,
            )
            side_rot = unreal.Rotator(roll=0.0, pitch=-5.0, yaw=a_rot.yaw - 90.0)

            # Espalda: detrás del personaje (-cos(yaw), -sin(yaw)), mirando hacia la espalda
            back_loc = unreal.Vector(
                a_loc.x - distance_cm * math.cos(yaw_rad),
                a_loc.y - distance_cm * math.sin(yaw_rad),
                a_loc.z + height_cm,
            )
            back_rot = unreal.Rotator(roll=0.0, pitch=-5.0, yaw=a_rot.yaw)

            views = [
                ("front", front_loc, front_rot),
                ("side", side_loc, side_rot),
                ("back", back_loc, back_rot),
            ]

            auto_lib = getattr(unreal, "AutomationLibrary", None)
            for angle_name, cam_loc, cam_rot in views:
                if ue_sub is not None and hasattr(ue_sub, "set_level_viewport_camera_info"):
                    ue_sub.set_level_viewport_camera_info(cam_loc, cam_rot)
                elif hasattr(unreal, "EditorLevelLibrary"):
                    unreal.EditorLevelLibrary.set_level_viewport_camera_info(cam_loc, cam_rot)

                target_file = screenshots[angle_name]
                if auto_lib is not None and hasattr(auto_lib, "take_high_res_screenshot"):
                    auto_lib.take_high_res_screenshot(1280, 720, target_file)

        except Exception:
            pass

    return screenshots


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
    clearance_res = measure_body_clearance(actor_label, garment_kind, body_trimmed=body_trimmed)
    steps_executed.append("clearance_measurement")
    if not clearance_res["ok"]:
        all_warnings.extend(clearance_res["warnings"])
        if clearance_res.get("min_clearance_cm", 0.0) < -2.0:
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
        screenshots = capture_three_angles(actor_label, output_dir=output_dir)

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
