# =============================================================================
# DBV Unreal AI — Auto-rigging y equipado de ropa en MetaHumans
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Descubrimiento de MetaHumans en el nivel y transferencia de pesos de ropa con GeometryScript.

Cada función pública devuelve una cadena JSON compacta. @spec REQ-81
"""

from __future__ import annotations

import json
import math
import os
import re
from typing import Any, Callable

import unreal

try:
    from .outfit import fit
except ImportError:
    fit = None  # type: ignore[assignment]

METAHUMAN_SLOTS = ("Torso", "Legs", "Feet")
NECK_HIDE_PARAM = "NeckHideScale"
SKIRT_CLEARANCE_CM = 0.7
SKIRT_BODY_X_LIMIT_CM = 24.0
SKIRT_EDGE_BAND = 0.15
SKIRT_OUTER_LAYER_CM = 0.5
SKIRT_HIDDEN_DEPTH_CM = 0.8
SKIRT_LEG_INFLUENCE = 0.85
SKIRT_SIDE_BAND_CM = 9.0
SKIRT_HIP_MARGIN_CM = 0.0
SKIRT_BLEND_CM = 12.0


def _json(value: Any) -> str:
    """Serializa el resultado de una herramienta."""
    return json.dumps(value, ensure_ascii=False, default=str)


def _round(vector: Any) -> list[float]:
    """Vector de Unreal → lista con un decimal."""
    return [round(float(vector.x), 1), round(float(vector.y), 1), round(float(vector.z), 1)]


def metahuman_components(actor: Any) -> dict[str, Any]:
    """Componentes ``SkeletalMeshComponent`` de un actor indexados por nombre."""
    comps = actor.get_components_by_class(unreal.SkeletalMeshComponent) if hasattr(actor, "get_components_by_class") else []
    return {c.get_name(): c for c in comps}


ARCHETYPE_RE = re.compile(r"(?:^|[/_])([mf]_(?:tal|med|sht)_(?:nrw|ovw))(?:[/_.]|$)", re.IGNORECASE)


def resolve_full_body_archetype(body_skm: Any, target_actor: Any = None) -> str:
    """Resuelve la complexión arquetípica de MetaHuman (``m_tal_nrw``, ``f_med_nrw``...) incluso para cuerpos derivados. @spec REQ-81, REQ-84"""
    if body_skm is None and target_actor is None:
        return "m_med_nrw"

    arch = ""
    if body_skm is not None and hasattr(unreal, "EditorAssetLibrary") and hasattr(unreal.EditorAssetLibrary, "get_metadata_tag"):
        tag = unreal.EditorAssetLibrary.get_metadata_tag(body_skm, "source_full_body") or ""
        if tag:
            m_tag = ARCHETYPE_RE.search(tag)
            if m_tag:
                arch = m_tag.group(1).lower()

    if not arch and body_skm is not None:
        path = body_skm.get_path_name() if hasattr(body_skm, "get_path_name") else ""
        m_path = ARCHETYPE_RE.search(path)
        if m_path:
            arch = m_path.group(1).lower()

    if not arch and body_skm is not None and hasattr(body_skm, "get_editor_property"):
        pa = body_skm.get_editor_property("physics_asset")
        if pa is not None and hasattr(pa, "get_path_name"):
            m_pa = ARCHETYPE_RE.search(pa.get_path_name())
            if m_pa:
                arch = m_pa.group(1).lower()

    if not arch and target_actor is not None and hasattr(target_actor, "get_components_by_class"):
        comps = target_actor.get_components_by_class(unreal.SkeletalMeshComponent) if hasattr(unreal, "SkeletalMeshComponent") else []
        for c in comps:
            skm = c.get_skeletal_mesh_asset() if hasattr(c, "get_skeletal_mesh_asset") else None
            if skm is not None and hasattr(skm, "get_path_name"):
                m_c = ARCHETYPE_RE.search(skm.get_path_name())
                if m_c:
                    arch = m_c.group(1).lower()
                    break

    if not arch and target_actor is not None and fit is not None and hasattr(target_actor, "get_components_by_class"):
        body_comp = next((c for c in target_actor.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Body"), None)
        if body_comp is not None:
            lm = fit.measure_body_landmarks(body_comp, target_actor)
            if lm.get("success") and lm.get("landmarks"):
                marks = lm["landmarks"]
                head_z = marks.get("head_z", 150.0)
                pa_name = marks.get("physics_asset") or ""
                pa_str = pa_name.get_path_name() if hasattr(pa_name, "get_path_name") else str(pa_name)
                gender = "f" if "/Female/" in pa_str or "f_" in pa_str else "m"
                height = "tal" if head_z > 158.0 else ("sht" if head_z < 145.0 else "med")
                arch = f"{gender}_{height}_nrw"

    if not arch:
        path = body_skm.get_path_name() if body_skm and hasattr(body_skm, "get_path_name") else ""
        raw = os.path.splitext(os.path.basename(path or ""))[0].replace("_body", "")
        arch = raw if raw else "m_med_nrw"

    return arch


def body_build(body_skm: Any, target_actor: Any = None) -> str:
    """Complexión del cuerpo (``f_med_nrw``, ``m_tal_nrw``…), resuelta de su arquetipo anatómico. @spec REQ-81, REQ-84"""
    arch = resolve_full_body_archetype(body_skm, target_actor)
    return arch


def list_metahumans() -> str:
    """Enumera los actores MetaHuman del nivel actual con su cuerpo base y ranuras de ropa. @spec REQ-81"""
    actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
    found: list[dict[str, Any]] = []
    for actor in actors:
        by_name = metahuman_components(actor)
        body_comp = by_name.get("Body")
        has_slots = any(slot in by_name for slot in METAHUMAN_SLOTS)
        if body_comp is not None and has_slots:
            body_skm = body_comp.get_skeletal_mesh_asset()
            body_path = body_skm.get_path_name() if body_skm else None
            build = body_build(body_skm, actor)
            slots = {}
            for slot in METAHUMAN_SLOTS:
                comp = by_name.get(slot)
                mesh = comp.get_skeletal_mesh_asset() if comp else None
                slots[slot] = mesh.get_path_name() if mesh else None
            found.append({
                "label": actor.get_actor_label(),
                "class": actor.get_class().get_name(),
                "location": _round(actor.get_actor_location()),
                "body_mesh": body_path,
                "build": build,
                "slots": slots,
            })
    return _json({"count": len(found), "metahumans": found})


def target_slot_bounds(body_skm: Any, slot_comp: Any, slot: str, scale_multiplier: float, z_offset: float) -> tuple[list[float], list[float]]:
    """Centro y dimensiones ``[x, y, z]`` de respaldo para una prenda según la complexión del MetaHuman. @spec REQ-81"""
    existing = slot_comp.get_skeletal_mesh_asset() if slot_comp else None
    scale = float(scale_multiplier or 1.05)
    offset_z = float(z_offset or 0.0)
    if existing is not None and hasattr(existing, "get_bounds"):
        bounds = existing.get_bounds()
        center = [float(bounds.origin.x), float(bounds.origin.y), float(bounds.origin.z) + offset_z]
        size = [
            float(bounds.box_extent.x) * 2.0 * scale,
            float(bounds.box_extent.y) * 2.0 * scale,
            float(bounds.box_extent.z) * 2.0 * scale,
        ]
    else:
        b = body_skm.get_bounds()
        min_z = float(b.origin.z) - float(b.box_extent.z)
        height = float(b.box_extent.z) * 2.0
        width_x = float(b.box_extent.x) * 2.0
        depth_y = float(b.box_extent.y) * 2.0
        if slot == "Legs":
            center = [float(b.origin.x), float(b.origin.y) * 0.25, min_z + height * 0.36 + offset_z]
            size = [width_x * 0.38 * scale, depth_y * 0.75 * scale, height * 0.56 * scale]
        elif slot == "Feet":
            center = [float(b.origin.x), float(b.origin.y) * 0.80, min_z + height * 0.04 + offset_z]
            size = [width_x * 0.42 * scale, depth_y * 0.85 * scale, height * 0.10 * scale]
        else:
            center = [float(b.origin.x), float(b.origin.y) * 0.30, min_z + height * 0.72 + offset_z]
            size = [width_x * 0.92 * scale, depth_y * 0.85 * scale, height * 0.38 * scale]
    return (center, size)


def _get_bone_local(body_comp: Any, name: str, fallback_z: float) -> Any:
    """Posición de un hueso en el espacio local del componente ``Body``."""
    pos = unreal.Vector(0.0, 0.0, fallback_z)
    if hasattr(body_comp, "get_bone_index") and hasattr(body_comp, "get_socket_transform"):
        if body_comp.get_bone_index(name) != -1:
            space = getattr(getattr(unreal, "RelativeTransformSpace", None), "RTS_COMPONENT", None)
            xf = body_comp.get_socket_transform(name, space) if space is not None else body_comp.get_socket_transform(name)
            pos = xf.translation
    return pos


def check_body_coverage(body_dyn: Any, slot: str, garment_dyn: Any = None) -> tuple[bool, float, str]:
    """Comprueba si ``body_dyn`` contiene geometría en la región requerida para la prenda. @spec REQ-81"""
    if body_dyn is None:
        return (False, 0.0, "body mesh is None")

    if not hasattr(body_dyn, "get_vertex_count"):
        return (True, 1.0, "mock mesh without get_vertex_count")

    vc = int(body_dyn.get_vertex_count())
    if vc == 0:
        return (False, 0.0, "body mesh is empty (0 vertices)")

    mq = getattr(unreal, "GeometryScript_MeshQueries", None)
    if mq is None or not hasattr(mq, "get_vertex_position"):
        return (True, 1.0, f"mock mesh has {vc} vertices")

    clean_slot = slot.strip().capitalize() if isinstance(slot, str) else "Torso"
    if clean_slot not in ("Torso", "Legs", "Feet") and garment_dyn is None:
        return (True, 1.0, f"slot '{slot}' does not require specific regional coverage")

    count = 0
    if garment_dyn is not None and hasattr(garment_dyn, "get_vertex_count") and hasattr(mq, "get_mesh_bounding_box"):
        g_box = mq.get_mesh_bounding_box(garment_dyn)
        z_min = float(g_box.min.z) - 3.0
        z_max = float(g_box.max.z) + 3.0
        x_min = float(g_box.min.x) - 6.0
        x_max = float(g_box.max.x) + 6.0
        y_min = float(g_box.min.y) - 6.0
        y_max = float(g_box.max.y) + 6.0
        for i in range(vc):
            v, ok = mq.get_vertex_position(body_dyn, i)
            if ok and z_min <= float(v.z) <= z_max and x_min <= float(v.x) <= x_max and y_min <= float(v.y) <= y_max:
                count += 1
        expected = 3000.0
        min_required = 150
    elif clean_slot == "Torso":
        for i in range(vc):
            v, ok = mq.get_vertex_position(body_dyn, i)
            if ok and 80.0 <= float(v.z) <= 140.0 and abs(float(v.x)) <= 28.0:
                count += 1
        expected = 5000.0
        min_required = 500
    elif clean_slot == "Legs":
        for i in range(vc):
            v, ok = mq.get_vertex_position(body_dyn, i)
            if ok and 20.0 <= float(v.z) <= 85.0 and abs(float(v.x)) <= 30.0:
                count += 1
        expected = 4000.0
        min_required = 500
    else:  # Feet
        for i in range(vc):
            v, ok = mq.get_vertex_position(body_dyn, i)
            if ok and -5.0 <= float(v.z) <= 18.0 and abs(float(v.x)) <= 25.0:
                count += 1
        expected = 1000.0
        min_required = 100

    ratio = min(1.0, count / max(expected, 1.0))
    is_ok = count >= min_required
    desc = f"{count} vertices in {clean_slot} region (ratio={ratio:.2f}, required>={min_required})"
    return (is_ok, ratio, desc)


def _find_full_body_asset(archetype: str) -> tuple[Any, str]:
    """Localiza el asset ``USkeletalMesh`` de cuerpo completo en ``/Game/MetaHumans/Common``. @spec REQ-81"""
    if not archetype:
        return (None, "")

    clean_arch = archetype.lower().strip()
    gender = "Female" if clean_arch.startswith("f_") else "Male"
    height = "Tall" if "_tal_" in clean_arch else ("Short" if "_sht_" in clean_arch else "Medium")
    weight = "Overweight" if "_ovw" in clean_arch else "NormalWeight"

    direct_candidates = [
        f"/Game/MetaHumans/Common/Common/{clean_arch}_body_preview",
        f"/Game/MetaHumans/Common/{gender}/{height}/{weight}/Body/{clean_arch}_body",
        f"/Game/MetaHumans/Common/Common/{clean_arch}_body",
    ]
    chosen_asset = None
    chosen_path = ""
    for p in direct_candidates:
        if hasattr(unreal, "load_asset"):
            cand = unreal.load_asset(p)
            if cand is not None and isinstance(cand, getattr(unreal, "SkeletalMesh", object)):
                chosen_asset = cand
                chosen_path = p
                break

    if chosen_asset is None and hasattr(unreal, "EditorAssetLibrary") and hasattr(unreal.EditorAssetLibrary, "list_assets"):
        raw = unreal.EditorAssetLibrary.list_assets("/Game/MetaHumans", recursive=True) or []
        for cand_path in raw:
            clean = str(cand_path).split(".")[0]
            if clean.lower().endswith(f"/{clean_arch}_body") or clean.lower().endswith(f"/{clean_arch}_body_preview"):
                cand = unreal.load_asset(clean) if hasattr(unreal, "load_asset") else None
                if cand is not None and isinstance(cand, getattr(unreal, "SkeletalMesh", object)):
                    chosen_asset = cand
                    chosen_path = clean
                    break

    if chosen_asset is None:
        fallback_p = "/Game/MetaHumans/Common/Common/m_tal_nrw_body_preview" if gender == "Male" else "/Game/MetaHumans/Common/Common/f_med_nrw_body_preview"
        fallback_cand = unreal.load_asset(fallback_p) if hasattr(unreal, "load_asset") else None
        if fallback_cand is not None and isinstance(fallback_cand, getattr(unreal, "SkeletalMesh", object)):
            chosen_asset = fallback_cand
            chosen_path = fallback_p

    return (chosen_asset, chosen_path)


def _ensure_full_body(
    body_comp: Any,
    body_skm: Any,
    build: str = "",
    slot: str = "Torso",
    target_actor: Any = None,
    garment_dyn: Any = None,
) -> tuple[Any, Any, bool, str]:
    """Comprueba la cobertura geométrica del cuerpo para ``slot`` y sustituye o devuelve la malla completa. @spec REQ-81"""
    if body_skm is None:
        return (None, None, False, "")

    asset_utils = getattr(unreal, "GeometryScript_AssetUtils", None)
    lod = unreal.GeometryScriptMeshReadLOD() if hasattr(unreal, "GeometryScriptMeshReadLOD") else None
    dyn = unreal.DynamicMesh() if hasattr(unreal, "DynamicMesh") else None
    if asset_utils is not None and dyn is not None and hasattr(asset_utils, "copy_mesh_from_skeletal_mesh"):
        dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(body_skm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod)

    vcount = dyn.get_vertex_count() if dyn is not None and hasattr(dyn, "get_vertex_count") else 30000
    is_covered, ratio, desc = check_body_coverage(dyn, slot, garment_dyn)

    body_path = body_skm.get_path_name() if hasattr(body_skm, "get_path_name") else ""
    is_derived = any(sub in body_path for sub in ("_spike", "SK_Body_", "Runway/Cloth"))
    needs_replacement = (not is_covered) or (vcount < 20000) or is_derived

    chosen_skm = body_skm
    chosen_path = body_path
    restored = False

    if needs_replacement:
        arch = resolve_full_body_archetype(body_skm, target_actor) if not build or build == os.path.splitext(os.path.basename(body_path))[0].replace("_body", "") else build
        cand_skm, cand_path = _find_full_body_asset(arch)
        if cand_skm is not None and asset_utils is not None and hasattr(asset_utils, "copy_mesh_from_skeletal_mesh"):
            cand_dyn = unreal.DynamicMesh() if hasattr(unreal, "DynamicMesh") else None
            if cand_dyn is not None:
                cand_dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(cand_skm, cand_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod)
                c_ok, c_ratio, _ = check_body_coverage(cand_dyn, slot, garment_dyn)
                if c_ok or (hasattr(cand_dyn, "get_vertex_count") and cand_dyn.get_vertex_count() >= 20000):
                    if not is_covered and body_comp is not None:
                        skin_mat = body_comp.get_material(0) if hasattr(body_comp, "get_material") else None
                        if hasattr(body_comp, "modify"):
                            body_comp.modify()
                        if hasattr(body_comp, "set_skeletal_mesh_asset"):
                            body_comp.set_skeletal_mesh_asset(cand_skm)
                        if skin_mat is not None and hasattr(body_comp, "set_material"):
                            body_comp.set_material(0, skin_mat)
                        restored = True
                    chosen_skm = cand_skm
                    dyn = cand_dyn
                    chosen_path = cand_path

    return (chosen_skm, dyn, restored, chosen_path)


def _smooth_1d(arr: list[float], passes: int = 4) -> list[float]:
    """Suavizado 1D ``[0.25, 0.50, 0.25]`` sobre perfiles verticales de secciones."""
    out = list(arr)
    n = len(out)
    for _ in range(passes):
        nxt = list(out)
        for i in range(1, n - 1):
            nxt[i] = 0.25 * out[i - 1] + 0.50 * out[i] + 0.25 * out[i + 1]
        out = nxt
    return out


def _smooth_2d_polar(grid: list[list[float]], nz: int, nth: int, passes: int = 5) -> list[list[float]]:
    """Suavizado 2D periódico en ángulo ``theta`` y acotado en altura ``z``."""
    out = [list(row) for row in grid]
    for _ in range(passes):
        nxt = [list(row) for row in out]
        for iz in range(nz):
            iz_lo = max(0, iz - 1)
            iz_hi = min(nz - 1, iz + 1)
            for ith in range(nth):
                ith_prev = (ith - 1) % nth
                ith_next = (ith + 1) % nth
                c = out[iz][ith]
                n_th = 0.5 * (out[iz][ith_prev] + out[iz][ith_next])
                n_z = 0.5 * (out[iz_lo][ith] + out[iz_hi][ith])
                nxt[iz][ith] = 0.40 * c + 0.35 * n_th + 0.25 * n_z
        out = nxt
    return out


def _interp_1d(arr: list[float], t: float) -> float:
    """Interpolación lineal de una lista ``arr`` para ``t`` en ``[0, 1]``."""
    ns = len(arr)
    pos = max(0.0, min(float(ns - 1), t * (ns - 1)))
    i0 = int(pos)
    i1 = min(ns - 1, i0 + 1)
    frac = pos - i0
    return arr[i0] * (1.0 - frac) + arr[i1] * frac


def _warp_t(slot: str, t: float) -> float:
    """Mapeo vertical por tramos: alinea pecho/sisa en ``Torso`` y glúteos/cintura en ``Legs``."""
    if slot == "Torso":
        warped = (t / 0.46) * 0.64 if t <= 0.46 else 0.64 + ((t - 0.46) / 0.54) * 0.36
    elif t <= 0.45:
        warped = (t / 0.45) * 0.55
    else:
        warped = 0.55 + ((t - 0.45) / 0.55) * 0.45
    return warped


def _arm_cutoff(slot: str, z: float, clav_z: float, neck_z: float, shoulder_x: float) -> float:
    """Límite lateral ``|X|`` que excluye brazos/manos y estrecha hacia el cuello en ``Torso``.

    Bajo la axila el brazo en A-pose ya está separado: el límite debe superar el ancho real de la cintura o la
    prenda se calcula sobre un cuerpo recortado y la piel la atraviesa. @spec REQ-81
    """
    if slot != "Torso":
        cutoff = shoulder_x * 1.45
    elif z < clav_z - 18.0:
        cutoff = shoulder_x * 1.35
    elif z < clav_z - 6.0:
        t_arm = (z - (clav_z - 18.0)) / 12.0
        cutoff = shoulder_x * (1.35 * (1.0 - t_arm) + 0.96 * t_arm)
    else:
        t_neck = min(1.0, max(0.0, (z - (clav_z - 6.0)) / max(neck_z - (clav_z - 6.0), 1.0)))
        cutoff = shoulder_x * (0.96 * (1.0 - t_neck) + 0.46 * t_neck)
    return cutoff


def _collect_body_points(by_name: dict[str, Any], body_dyn: Any, head_z: float) -> list[tuple[float, float, float]]:
    """Extrae los puntos ``(x, y, z)`` de ``Body`` y ``Face`` hasta ``head_z`` para cubrir torso, cuello y piernas."""
    mq = unreal.GeometryScript_MeshQueries
    pts: list[tuple[float, float, float]] = []
    for i in range(int(body_dyn.get_vertex_count())):
        v, ok = mq.get_vertex_position(body_dyn, i)
        if ok and float(v.z) <= head_z + 2.0:
            pts.append((float(v.x), float(v.y), float(v.z)))
    face_comp = by_name.get("Face")
    face_skm = face_comp.get_skeletal_mesh_asset() if face_comp else None
    if face_skm is not None:
        face_dyn = unreal.DynamicMesh()
        face_dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_skeletal_mesh(
            face_skm, face_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD()
        )
        fvc = int(face_dyn.get_vertex_count()) if hasattr(face_dyn, "get_vertex_count") else 0
        step = 2 if fvc > 15000 else 1
        for i in range(0, fvc, step):
            v, ok = mq.get_vertex_position(face_dyn, i)
            if ok and float(v.z) <= head_z + 2.0:
                pts.append((float(v.x), float(v.y), float(v.z)))
    return pts


def _clean_and_orient_garment(garment_dyn: Any, slot: str) -> Any:
    """Centra la prenda en el origen y rota si es necesario para alinear con MetaHuman (+Y frente, X ancho)."""
    _ = slot
    mesh_queries = unreal.GeometryScript_MeshQueries
    mesh_xform = unreal.GeometryScript_MeshTransforms
    g_box0 = mesh_queries.get_mesh_bounding_box(garment_dyn)
    g_center0 = (g_box0.max + g_box0.min) * 0.5
    garment_dyn = mesh_xform.translate_mesh(garment_dyn, g_center0 * -1.0)
    dx = float(g_box0.max.x - g_box0.min.x)
    dy = float(g_box0.max.y - g_box0.min.y)
    if dy > dx * 1.05 and hasattr(mesh_xform, "transform_mesh") and hasattr(unreal, "Transform") and hasattr(unreal, "Rotator"):
        rot_xf = unreal.Transform(
            location=unreal.Vector(0.0, 0.0, 0.0),
            rotation=unreal.Rotator(roll=0.0, pitch=0.0, yaw=90.0),
            scale=unreal.Vector(1.0, 1.0, 1.0),
        )
        garment_dyn = mesh_xform.transform_mesh(garment_dyn, rot_xf)
    return garment_dyn


def _compute_slice_envelopes(
    g_verts: list[tuple[float, float, float]],
    body_pts: list[tuple[float, float, float]],
    g_zmin: float,
    g_h: float,
    z_min: float,
    z_max: float,
    slot: str,
    scale_mult: float,
    clav_z: float,
    neck_z: float,
    shoulder_x: float,
) -> dict[str, list[float]]:
    """Calcula las envolventes suavizadas por franja Z de la prenda y su objetivo anatómico sobre el cuerpo."""
    ns = 20
    g_xmin, g_xmax, g_ymin, g_ymax = [], [], [], []
    b_xmin, b_xmax, b_ymin, b_ymax = [], [], [], []
    for s in range(ns):
        t0, t1 = s / float(ns), (s + 1) / float(ns)
        gz0, gz1 = g_zmin + g_h * t0, g_zmin + g_h * t1
        g_slice = [(x, y) for (x, y, z) in g_verts if gz0 <= z <= gz1]
        if g_slice:
            xs = sorted(p[0] for p in g_slice)
            ys = sorted(p[1] for p in g_slice)
            lo, hi = int(len(xs) * 0.02), max(0, int(len(xs) * 0.98) - 1)
            g_xmin.append(xs[lo])
            g_xmax.append(xs[hi])
            g_ymin.append(ys[lo])
            g_ymax.append(ys[hi])
        else:
            g_xmin.append(-15.0)
            g_xmax.append(15.0)
            g_ymin.append(-10.0)
            g_ymax.append(10.0)

        u0, u1 = _warp_t(slot, t0), _warp_t(slot, t1)
        bz0 = z_min + (z_max - z_min) * u0 - 1.2
        bz1 = z_min + (z_max - z_min) * u1 + 1.2
        b_slice = [(x, y) for (x, y, z) in body_pts if bz0 <= z <= bz1 and abs(x) <= _arm_cutoff(slot, z, clav_z, neck_z, shoulder_x)]
        if b_slice:
            xs = sorted(p[0] for p in b_slice)
            ys = sorted(p[1] for p in b_slice)
            lo, hi = int(len(xs) * 0.02), max(0, int(len(xs) * 0.98) - 1)
            b_xmin.append(xs[lo])
            b_xmax.append(xs[hi])
            b_ymin.append(ys[lo])
            b_ymax.append(ys[hi])
        else:
            b_xmin.append(b_xmin[-1] if b_xmin else -12.0)
            b_xmax.append(b_xmax[-1] if b_xmax else 12.0)
            b_ymin.append(b_ymin[-1] if b_ymin else -8.0)
            b_ymax.append(b_ymax[-1] if b_ymax else 10.0)

    g_xmin, g_xmax = _smooth_1d(g_xmin, 4), _smooth_1d(g_xmax, 4)
    g_ymin, g_ymax = _smooth_1d(g_ymin, 4), _smooth_1d(g_ymax, 4)
    b_xmin, b_xmax = _smooth_1d(b_xmin, 4), _smooth_1d(b_xmax, 4)
    b_ymin, b_ymax = _smooth_1d(b_ymin, 4), _smooth_1d(b_ymax, 4)
    b_hx = [(b_xmax[s] - b_xmin[s]) * 0.5 for s in range(ns)]
    b_cy = [(b_ymax[s] + b_ymin[s]) * 0.5 for s in range(ns)]
    g_hx = [max((g_xmax[s] - g_xmin[s]) * 0.5, 1.0) for s in range(ns)]
    g_depth = [max(g_ymax[s] - g_ymin[s], 1.0) for s in range(ns)]

    t_hx, t_ymin, t_ymax = [], [], []
    if slot == "Torso":
        bust_hx = max(b_hx[5:13])
        g_bust_hx = max(g_hx[5:12])
        for s in range(ns):
            t = (s + 0.5) / float(ns)
            if t <= 0.52:
                hx_val = max(b_hx[s], bust_hx * (g_hx[s] / g_bust_hx) * 0.98) * 1.04 * scale_mult
            else:
                w_neck = (t - 0.52) / 0.48
                hx_val = (b_hx[s] * (1.04 - 0.02 * w_neck)) * scale_mult
            ymin_val = b_ymin[s] - 0.70
            if t <= 0.78:
                ymax_val = b_ymax[s] + 0.80 * scale_mult
            else:
                w_col = (t - 0.78) / 0.22
                ymax_val = (b_ymax[s] + 0.80) * (1.0 - w_col) + (b_cy[s] + 2.2) * w_col
            t_hx.append(hx_val)
            t_ymin.append(ymin_val)
            t_ymax.append(ymax_val)
    else:
        hip_hx = max(b_hx)
        glute_ymin = min(b_ymin)
        front_ymax = max(b_ymax)
        for s in range(ns):
            t = (s + 0.5) / float(ns)
            if t >= 0.45:
                u = (t - 0.45) / 0.55
                hx_val = max(b_hx[s], hip_hx * (1.0 - 0.12 * (u**1.5))) * 1.09 * scale_mult
                ymin_val = min(b_ymin[s], glute_ymin * (1.0 - 0.18 * (u**1.6))) - 1.35
                ymax_val = max(b_ymax[s], front_ymax * (1.0 - 0.05 * u)) + 1.15
            else:
                v = (0.45 - t) / 0.45
                hx_val = hip_hx * (1.09 - 0.03 * v) * scale_mult
                ymin_val = (glute_ymin - 1.35) * (1.0 - 0.16 * v)
                ymax_val = front_ymax + 1.15 - 0.25 * v
            t_hx.append(hx_val)
            t_ymin.append(ymin_val)
            t_ymax.append(ymax_val)

    return {
        "g_xmin": g_xmin,
        "g_xmax": g_xmax,
        "g_ymin": g_ymin,
        "g_hx": g_hx,
        "g_depth": g_depth,
        "b_cy": b_cy,
        "t_hx": _smooth_1d(t_hx, 4),
        "t_ymin": _smooth_1d(t_ymin, 4),
        "t_ymax": _smooth_1d(t_ymax, 4),
    }


def _convex_hull(points: list[tuple[float, float]]) -> list[tuple[float, float]]:
    """Envolvente convexa 2D (cadena monótona), en sentido antihorario."""
    pts = sorted(set(points))
    hull = pts
    if len(pts) > 2:
        def half(seq: list[tuple[float, float]]) -> list[tuple[float, float]]:
            chain: list[tuple[float, float]] = []
            for p in seq:
                while len(chain) >= 2 and (chain[-1][0] - chain[-2][0]) * (p[1] - chain[-2][1]) - (chain[-1][1] - chain[-2][1]) * (p[0] - chain[-2][0]) <= 0.0:
                    chain.pop()
                chain.append(p)
            return chain[:-1]

        hull = half(pts) + half(list(reversed(pts)))
    return hull


def _hull_radius(hull: list[tuple[float, float]], cy: float, theta: float) -> float:
    """Distancia desde ``(0, cy)`` hasta el borde de ``hull`` en la dirección ``theta``."""
    dx, dy = math.cos(theta), math.sin(theta)
    best = 0.0
    for i, (ax, ay) in enumerate(hull):
        bx, by = hull[(i + 1) % len(hull)]
        ex, ey = bx - ax, by - ay
        denom = dx * ey - dy * ex
        if abs(denom) > 1e-9:
            t = (ax * ey - (ay - cy) * ex) / denom
            s = (ax * dy - (ay - cy) * dx) / denom
            if t > best and -1e-6 <= s <= 1.0 + 1e-6:
                best = t
    return best


def _skirt_target_radii(
    body_pts: list[tuple[float, float, float]],
    z_min: float,
    z_max: float,
    nz: int,
    nth: int,
    clearance: float,
) -> tuple[float, list[list[float]]]:
    """Radio objetivo ``[iz][ith]`` de una falda: ciñe hasta la cadera y desde ahí cae recta, sin entrar entre las piernas. @spec REQ-81"""
    rows: list[list[tuple[float, float]]] = [[] for _ in range(nz)]
    span = max(z_max - z_min, 1.0)
    for x, y, z in body_pts:
        if z_min <= z <= z_max and abs(x) <= SKIRT_BODY_X_LIMIT_CM:
            rows[min(nz - 1, int((z - z_min) / span * nz))].append((x, y))
    for iz in range(1, nz):
        rows[iz] = rows[iz] or rows[iz - 1]
    for iz in range(nz - 2, -1, -1):
        rows[iz] = rows[iz] or rows[iz + 1]
    hulls = [_convex_hull(r) for r in rows]
    widths = [max((p[0] for p in h), default=0.0) - min((p[0] for p in h), default=0.0) for h in hulls]
    hip = widths.index(max(widths))
    for iz in range(hip - 1, -1, -1):
        hulls[iz] = _convex_hull(hulls[iz] + hulls[iz + 1])
    hip_ys = [p[1] for p in hulls[hip]] or [0.0]
    cy = 0.5 * (min(hip_ys) + max(hip_ys))
    raw = [[_hull_radius(hulls[iz], cy, (ith + 0.5) / nth * 2.0 * math.pi - math.pi) + clearance for ith in range(nth)] for iz in range(nz)]
    smooth = _smooth_2d_polar(raw, nz, nth, passes=1)
    return (cy, [[max(raw[iz][ith], smooth[iz][ith]) for ith in range(nth)] for iz in range(nz)])


def _sample_polar(grid: list[list[float]], nz: int, nth: int, u: float, theta: float) -> float:
    """Interpolación bilineal de una rejilla polar, periódica en ``theta``."""
    fz = max(0.0, min(float(nz - 1), u * nz - 0.5))
    iz0 = int(fz)
    iz1 = min(nz - 1, iz0 + 1)
    wz = fz - iz0
    fth = ((theta + math.pi) / (2.0 * math.pi)) * nth - 0.5
    ith0 = int(math.floor(fth))
    wth = fth - ith0
    ith1 = (ith0 + 1) % nth
    ith0 %= nth
    low = grid[iz0][ith0] * (1.0 - wth) + grid[iz0][ith1] * wth
    high = grid[iz1][ith0] * (1.0 - wth) + grid[iz1][ith1] * wth
    return low * (1.0 - wz) + high * wz


def _triangle_cells(
    polar: list[tuple[float, float, float]],
    tri: tuple[int, int, int],
    nz: int,
    nth: int,
) -> list[tuple[int, int, float]]:
    """Celdas ``(iz, ith, radio)`` que cubre un triángulo en el mapa ``(altura, ángulo)``."""
    step = 2.0 * math.pi / nth
    ta, za, ra = polar[tri[0]]
    tb, zb, rb = polar[tri[1]]
    tc, zc, rc = polar[tri[2]]
    tb += 2.0 * math.pi * round((ta - tb) / (2.0 * math.pi))
    tc += 2.0 * math.pi * round((ta - tc) / (2.0 * math.pi))
    ua, ub, uc = (ta + math.pi) / step, (tb + math.pi) / step, (tc + math.pi) / step
    area = (ub - ua) * (zc - za) - (uc - ua) * (zb - za)
    cells: list[tuple[int, int, float]] = []
    if abs(area) > 1e-9:
        for iz in range(max(0, int(min(za, zb, zc))), min(nz - 1, int(max(za, zb, zc))) + 1):
            for iu in range(int(math.floor(min(ua, ub, uc))), int(math.floor(max(ua, ub, uc))) + 1):
                pu, pz = iu + 0.5, iz + 0.5
                w1 = ((pu - ua) * (zc - za) - (uc - ua) * (pz - za)) / area
                w2 = ((ub - ua) * (pz - za) - (pu - ua) * (zb - za)) / area
                if w1 >= -0.05 and w2 >= -0.05 and w1 + w2 <= 1.05:
                    cells.append((iz, iu % nth, ra + w1 * (rb - ra) + w2 * (rc - ra)))
    return cells


def _polar_coords(
    verts: list[tuple[float, float, float]],
    cx: float,
    cy: float,
    z_lo: float,
    z_hi: float,
    nz: int,
) -> list[tuple[float, float, float]]:
    """Vértices en ``(ángulo, fila de altura, radio)`` alrededor del eje ``(cx, cy)``."""
    span = max(z_hi - z_lo, 1.0)
    return [(math.atan2(y - cy, x - cx), (z - z_lo) / span * nz, math.hypot(x - cx, y - cy)) for x, y, z in verts]


def _outer_radius_map(
    verts: list[tuple[float, float, float]],
    tris: list[tuple[int, int, int]],
    cy: float,
    z_lo: float,
    z_hi: float,
    nz: int,
    nth: int,
) -> list[list[float]]:
    """Rasteriza los triángulos en ``(altura, ángulo)`` y guarda el radio mayor: la superficie visible desde fuera; 0 si no hay tela. @spec REQ-81"""
    grid = [[0.0 for _ in range(nth)] for _ in range(nz)]
    polar = _polar_coords(verts, 0.0, cy, z_lo, z_hi, nz)
    for tri in tris:
        for iz, ith, radius in _triangle_cells(polar, tri, nz, nth):
            if radius > grid[iz][ith]:
                grid[iz][ith] = radius
    return grid


def _hidden_triangle_ids(
    verts: list[tuple[float, float, float]],
    tris: list[tuple[int, int, int, int]],
    cx: float,
    cy: float,
    depth: float,
    nz: int = 96,
    nth: int = 216,
) -> list[int]:
    """Triángulos ``(id, a, b, c)`` tapados por otra capa en todas las celdas que cubren: no se ven desde fuera.

    Al separar las piernas la tela se abre y estos restos del maniquí asomarían; si un triángulo se ve en una sola
    celda se conserva, así que una solapa no se lleva por delante la tela real (ver REJECTED-10). @spec REQ-81
    """
    z_values = [v[2] for v in verts] or [0.0]
    z_lo, z_hi = min(z_values), max(z_values)
    polar = _polar_coords(verts, cx, cy, z_lo, z_hi, nz)
    grid = [[0.0 for _ in range(nth)] for _ in range(nz)]
    covered: list[tuple[int, list[tuple[int, int, float]]]] = []
    for tid, a, b, c in tris:
        cells = _triangle_cells(polar, (a, b, c), nz, nth)
        covered.append((tid, cells))
        for iz, ith, radius in cells:
            if radius > grid[iz][ith]:
                grid[iz][ith] = radius
    hidden: list[int] = []
    for tid, cells in covered:
        if cells and all(radius < grid[iz][ith] - depth for iz, ith, radius in cells):
            hidden.append(tid)
    return hidden


def _remove_hidden_layers(garment_dyn: Any) -> Any:
    """Borra de la malla las capas interiores que no se ven desde fuera y la compacta. @spec REQ-81"""
    mq = unreal.GeometryScript_MeshQueries
    box = mq.get_mesh_bounding_box(garment_dyn)
    verts: list[tuple[float, float, float]] = []
    for vid in range(int(garment_dyn.get_vertex_count())):
        v, ok = mq.get_vertex_position(garment_dyn, vid)
        verts.append((float(v.x), float(v.y), float(v.z)) if ok else (0.0, 0.0, 0.0))
    tris: list[tuple[int, int, int, int]] = []
    for tid in range(int(garment_dyn.get_triangle_count())):
        tri, ok = mq.get_triangle_indices(garment_dyn, tid)
        if ok:
            tris.append((tid, tri.x, tri.y, tri.z))
    center_x = 0.5 * (float(box.min.x) + float(box.max.x))
    center_y = 0.5 * (float(box.min.y) + float(box.max.y))
    for tid in _hidden_triangle_ids(verts, tris, center_x, center_y, SKIRT_HIDDEN_DEPTH_CM):
        garment_dyn, _ = unreal.GeometryScript_MeshEdits.delete_triangle_from_mesh(garment_dyn, tid, True)
    return unreal.GeometryScript_MeshRepair.compact_mesh(garment_dyn)


def _reference_bone_z(skeleton: Any, bone: str) -> float:
    """Altura de un hueso en la pose de referencia; 80 cm si el esqueleto no la expone."""
    height = 80.0
    pose_api = getattr(unreal, "AnimPoseExtensions", None)
    if pose_api is not None:
        pose = pose_api.get_reference_pose(skeleton)
        height = float(pose_api.get_bone_pose(pose, bone, unreal.AnimPoseSpaces.WORLD).translation.z)
    return height


def _is_skirt_shape(slot: str, height: float, width: float, asset_name: str = "") -> bool:
    """Una prenda de ``Legs`` poco esbelta es una falda; una muy esbelta, un pantalón."""
    name = (asset_name or "").lower()
    if any(k in name for k in ("pantalon", "short", "boxer", "calzon", "slacks", "trouser")):
        return False
    return slot == "Legs" and (height / max(width, 1.0)) <= 1.45


def _skirt_weights(x: float, z: float, hem_z: float, hip_z: float) -> tuple[float, float, float]:
    """Pesos ``(pelvis, thigh_l, thigh_r)`` de una falda: continuos, para que la tela no se parta al separar las piernas.

    Por encima de la cadera solo manda la pelvis. Hacia el dobladillo pasa a los muslos, y el reparto entre ambos es
    gradual: el centro sigue la media de las dos piernas en vez de quedarse con una. @spec REQ-81
    """
    span = max(hip_z - hem_z, 1.0)
    t = max(0.0, min(1.0, (hip_z - z) / span))
    leg = SKIRT_LEG_INFLUENCE * t * t * (3.0 - 2.0 * t)
    s = max(0.0, min(1.0, 0.5 + 0.5 * x / SKIRT_SIDE_BAND_CM))
    left = s * s * (3.0 - 2.0 * s)
    return (1.0 - leg, leg * left, leg * (1.0 - left))


def _skirt_blend(z: float, hip_z: float) -> float:
    """Cuánto pesa el reparto continuo frente a los pesos de la piel: 0 en la cadera, 1 a ``SKIRT_BLEND_CM`` por debajo.

    Arriba hay que conservar los pesos de la piel: incluyen columna y huesos correctivos, y sin ellos la cadera asoma
    por la cinturilla al andar. La rasgadura solo aparece abajo, donde la tela va de un muslo al otro. @spec REQ-81
    """
    t = max(0.0, min(1.0, (hip_z - z) / SKIRT_BLEND_CM))
    return t * t * (3.0 - 2.0 * t)


def _blend_weights(skin: dict[int, float], smooth: dict[int, float], amount: float) -> dict[int, float]:
    """Mezcla dos juegos de pesos ``{hueso: peso}`` y normaliza el resultado."""
    mixed = {b: w * (1.0 - amount) for b, w in skin.items()}
    for bone, weight in smooth.items():
        mixed[bone] = mixed.get(bone, 0.0) + weight * amount
    kept = {b: w for b, w in mixed.items() if w > 1e-3}
    total = sum(kept.values()) or 1.0
    return {b: w / total for b, w in kept.items()}


def _apply_skirt_weights(garment_dyn: Any, hip_z: float) -> Any:
    """Mezcla los pesos copiados de la piel con el reparto continuo de ``_skirt_weights`` según la altura. @spec REQ-81"""
    bw = unreal.GeometryScript_BoneWeights
    mq = unreal.GeometryScript_MeshQueries
    indices: list[int] = []
    for name in ("pelvis", "thigh_l", "thigh_r"):
        _, valid, index = bw.get_bone_index(garment_dyn, name)
        if valid:
            indices.append(int(index))
    if len(indices) == 3:
        hem_z = float(mq.get_mesh_bounding_box(garment_dyn).min.z)
        for vid in range(int(garment_dyn.get_vertex_count())):
            v, ok = mq.get_vertex_position(garment_dyn, vid)
            amount = _skirt_blend(float(v.z), hip_z) if ok else 0.0
            if amount > 0.0:
                _, current, _valid = bw.get_vertex_bone_weights(garment_dyn, vid)
                skin = {int(w.bone_index): float(w.weight) for w in current}
                smooth = dict(zip(indices, _skirt_weights(float(v.x), float(v.z), hem_z, hip_z)))
                mixed = _blend_weights(skin, smooth, amount)
                pairs = [unreal.GeometryScriptBoneWeight(bone_index=b, weight=w) for b, w in mixed.items()]
                bw.set_vertex_bone_weights(garment_dyn, vid, pairs)
    return garment_dyn


def _coarse_median_radii(fine: list[list[float]], nz: int, nth: int) -> list[list[float]]:
    """Reduce el mapa fino a la rejilla de ajuste con la mediana de cada celda, inmune a solapas y bolsillos."""
    fine_nz, fine_nth = len(fine), len(fine[0])
    cells: list[list[list[float]]] = [[[] for _ in range(nth)] for _ in range(nz)]
    for iz, row in enumerate(fine):
        for ith, radius in enumerate(row):
            if radius > 0.0:
                cells[iz * nz // fine_nz][ith * nth // fine_nth].append(radius)
    grid = [[0.0 for _ in range(nth)] for _ in range(nz)]
    for iz in range(nz):
        for ith in range(nth):
            values = sorted(cells[iz][ith])
            grid[iz][ith] = values[len(values) // 2] if values else 0.0
        filled = [r for r in grid[iz] if r > 0.0]
        mean = sum(filled) / len(filled) if filled else 10.0
        grid[iz] = [r if r > 0.0 else mean for r in grid[iz]]
    return _smooth_2d_polar(grid, nz, nth, passes=1)


def _boundary_vertex_z(garment_dyn: Any) -> list[float]:
    """Altura de los vértices en borde abierto (aristas de un solo triángulo)."""
    mq = unreal.GeometryScript_MeshQueries
    edges: dict[tuple[int, int], int] = {}
    for tid in range(int(garment_dyn.get_triangle_count())):
        tri, ok = mq.get_triangle_indices(garment_dyn, tid)
        if ok:
            for a, b in ((tri.x, tri.y), (tri.y, tri.z), (tri.z, tri.x)):
                key = (a, b) if a < b else (b, a)
                edges[key] = edges.get(key, 0) + 1
    ids = {v for e, n in edges.items() if n == 1 for v in e}
    return sorted(float(mq.get_vertex_position(garment_dyn, v)[0].z) for v in ids)


def _weld_and_trim_skirt(garment_dyn: Any) -> Any:
    """Suelda los parches de la prenda, tapa sus agujeros y corta dobladillo y cintura con un plano limpio.

    Tripo entrega islas de UV sin soldar y un recorte previo por triángulos deja el borde dentado: los dos planos
    se colocan justo dentro de los dientes, así que también eliminan las tapas que ``fill_all_mesh_holes`` crea en esos bordes. @spec REQ-81
    """
    repair = getattr(unreal, "GeometryScript_MeshRepair", None)
    booleans = getattr(unreal, "GeometryScript_MeshBooleans", None)
    result = garment_dyn
    if repair is not None and booleans is not None and hasattr(garment_dyn, "get_triangle_count"):
        weld = unreal.GeometryScriptWeldEdgesOptions()
        weld.set_editor_property("tolerance", 0.02)
        result = repair.weld_mesh_edges(result, weld)
        result = repair.compact_mesh(result)
        box = unreal.GeometryScript_MeshQueries.get_mesh_bounding_box(result)
        z_lo, z_hi = float(box.min.z), float(box.max.z)
        band = (z_hi - z_lo) * SKIRT_EDGE_BAND
        zs = _boundary_vertex_z(result)
        hem_z = max((z for z in zs if z <= z_lo + band), default=z_lo)
        top_z = min((z for z in zs if z >= z_hi - band), default=z_hi)
        filled = repair.fill_all_mesh_holes(result, unreal.GeometryScriptFillHolesOptions())
        result = filled[0] if isinstance(filled, tuple) else filled
        for plane_z, keep_above in ((hem_z + 0.01, True), (top_z - 0.01, False)):
            cut = unreal.GeometryScriptMeshPlaneCutOptions()
            cut.set_editor_property("fill_holes", False)
            cut.set_editor_property("flip_cut_side", keep_above)
            frame = unreal.Transform(location=unreal.Vector(0.0, 0.0, plane_z))
            result = booleans.apply_mesh_plane_cut(result, frame, cut)
        result = _remove_hidden_layers(repair.compact_mesh(result))
    return result


def _conform_skirt(
    garment_dyn: Any,
    body_pts: list[tuple[float, float, float]],
    z_min: float,
    z_max: float,
) -> Any:
    """Ajusta una falda en los dos sentidos a la envolvente del cuerpo, conservando pliegues y cinturilla.

    El factor solo depende de altura y ángulo, así que conserva el orden radial de las capas: lo que la prenda trae
    dentro (restos del maniquí) sigue tapado por la tela. Solo la capa exterior se empuja hasta el objetivo; empujar
    también las interiores las saca sobre la tela. @spec REQ-81
    """
    mq = unreal.GeometryScript_MeshQueries
    me = unreal.GeometryScript_MeshEdits
    box = mq.get_mesh_bounding_box(garment_dyn)
    g_lo, g_hi = float(box.min.z), float(box.max.z)
    z_scale = (z_max - z_min) / max(g_hi - g_lo, 1.0)
    nz, nth = 16, 36
    fine_nz, fine_nth = 96, 216
    cy, target = _skirt_target_radii(body_pts, z_min, z_max, nz, nth, SKIRT_CLEARANCE_CM)
    g_cy = 0.5 * (float(box.min.y) + float(box.max.y))
    g_cx = 0.5 * (float(box.min.x) + float(box.max.x))
    count = int(garment_dyn.get_vertex_count())
    mapped: list[tuple[float, float, float]] = []
    for vid in range(count):
        v, ok = mq.get_vertex_position(garment_dyn, vid)
        mapped.append((float(v.x) - g_cx, float(v.y) - g_cy + cy, z_min + (float(v.z) - g_lo) * z_scale) if ok else (0.0, cy, z_min))
    tris: list[tuple[int, int, int]] = []
    for tid in range(int(garment_dyn.get_triangle_count())):
        tri, ok = mq.get_triangle_indices(garment_dyn, tid)
        if ok:
            tris.append((tri.x, tri.y, tri.z))
    fine = _outer_radius_map(mapped, tris, cy, z_min, z_max, fine_nz, fine_nth)
    current = _coarse_median_radii(fine, nz, nth)
    span = max(z_max - z_min, 1.0)
    for vid, (x, y, z) in enumerate(mapped):
        u = max(0.0, min(1.0, (z - z_min) / span))
        dy = y - cy
        theta = math.atan2(dy, x)
        goal = _sample_polar(target, nz, nth, u, theta)
        factor = goal / max(_sample_polar(current, nz, nth, u, theta), 1.0)
        original = math.hypot(x, dy)
        radius = original * factor
        fz = min(fine_nz - 1, int(u * fine_nz))
        fth = int(((theta + math.pi) / (2.0 * math.pi)) * fine_nth) % fine_nth
        if original >= fine[fz][fth] - SKIRT_OUTER_LAYER_CM:
            radius = max(radius, goal - 0.1)
        me.set_vertex_position(garment_dyn, vid, unreal.Vector(radius * math.cos(theta), cy + radius * math.sin(theta), z), vid < count - 1)
    return garment_dyn


def _disable_neck_hide(face_comp: Any) -> list[str]:
    """Apaga ``NeckHideScale`` en los materiales de la cabeza del MetaHuman.

    Epic encoge por material (World Position Offset) el cuello y los hombros para que no atraviesen la camisa de
    fábrica; al cambiarla, la deformación queda a la vista como una rendija entre ``Face`` y ``Body``. @spec REQ-81
    """
    lib = getattr(unreal, "MaterialEditingLibrary", None)
    changed: list[str] = []
    if face_comp is None or lib is None or not hasattr(face_comp, "get_num_materials"):
        return changed
    for idx in range(int(face_comp.get_num_materials())):
        material = face_comp.get_material(idx)
        source = material
        if hasattr(material, "set_scalar_parameter_value"):
            if abs(float(material.get_scalar_parameter_value(NECK_HIDE_PARAM))) > 1e-4:
                material.set_scalar_parameter_value(NECK_HIDE_PARAM, 0.0)
            source = material.get_editor_property("parent")
        is_constant = isinstance(source, getattr(unreal, "MaterialInstanceConstant", ()))
        if is_constant and abs(float(lib.get_material_instance_scalar_parameter_value(source, NECK_HIDE_PARAM))) > 1e-4:
            lib.set_material_instance_scalar_parameter_value(source, NECK_HIDE_PARAM, 0.0)
            lib.update_material_instance(source)
            unreal.EditorAssetLibrary.save_loaded_asset(source, False)
            changed.append(source.get_path_name())
    return changed


def _deform_garment_to_slices(
    by_name: dict[str, Any],
    body_comp: Any,
    body_dyn: Any,
    garment_dyn: Any,
    slot: str,
    scale_mult: float,
    z_off: float,
    asset_name: str = "",
) -> tuple[Any, float]:
    """Adapta vértice a vértice una prenda de ``Torso`` o ``Legs`` al perfil anatómico del MetaHuman."""
    mq = unreal.GeometryScript_MeshQueries
    me = unreal.GeometryScript_MeshEdits
    g_box = mq.get_mesh_bounding_box(garment_dyn)
    g_zmin, g_zmax = float(g_box.min.z), float(g_box.max.z)
    g_h = max(g_zmax - g_zmin, 1.0)
    g_w = max(float(g_box.max.x - g_box.min.x), 1.0)
    b_box = mq.get_mesh_bounding_box(body_dyn)
    b_h = float(b_box.max.z - b_box.min.z)

    head_z = float(_get_bone_local(body_comp, "head", b_h * 0.94).z)
    neck_z = float(_get_bone_local(body_comp, "neck_01", b_h * 0.86).z)
    clav_z = float(_get_bone_local(body_comp, "clavicle_l", b_h * 0.82).z)
    waist_z = float(_get_bone_local(body_comp, "spine_02", b_h * 0.60).z)
    spine1_z = float(_get_bone_local(body_comp, "spine_01", waist_z - 7.0).z)
    calf_z = float(_get_bone_local(body_comp, "calf_l", b_h * 0.28).z)
    ankle_z = float(_get_bone_local(body_comp, "foot_l", 7.2).z)
    shoulder_x = abs(float(_get_bone_local(body_comp, "upperarm_l", 14.0).x)) or 14.0

    norm_scale = scale_mult / 1.05 if scale_mult > 0.1 else 1.0
    is_skirt = False
    if slot == "Torso":
        z_min = min(waist_z + 0.8, spine1_z + 1.0) + z_off
        z_max = min(neck_z + 0.5, clav_z + 2.5) + z_off
    else:
        z_max = waist_z + 3.0 + z_off
        is_skirt = _is_skirt_shape(slot, g_h, g_w, asset_name)
        is_pants = not is_skirt
        is_shorts = is_pants and ((g_h / max(g_w, 1.0)) <= 1.35)
        if is_shorts:
            z_min = z_max - (z_max - calf_z) * 0.60 + z_off
        elif is_pants:
            z_min = ankle_z + 2.0 + z_off
        else:
            z_min = z_max - (z_max - calf_z) * 0.56 * norm_scale

    body_pts = _collect_body_points(by_name, body_dyn, head_z)
    if is_skirt:
        garment_dyn = _conform_skirt(_weld_and_trim_skirt(garment_dyn), body_pts, z_min, z_max)
    else:
        garment_dyn = _deform_by_envelopes(garment_dyn, body_pts, g_zmin, g_h, z_min, z_max, slot, norm_scale, clav_z, neck_z, shoulder_x)

    if hasattr(unreal, "GeometryScript_Normals"):
        garment_dyn = unreal.GeometryScript_Normals.recompute_normals(garment_dyn, unreal.GeometryScriptCalculateNormalsOptions())
    avg_scale = (z_max - z_min) / g_h
    return (garment_dyn, float(avg_scale))


def _deform_by_envelopes(
    garment_dyn: Any,
    body_pts: list[tuple[float, float, float]],
    g_zmin: float,
    g_h: float,
    z_min: float,
    z_max: float,
    slot: str,
    norm_scale: float,
    clav_z: float,
    neck_z: float,
    shoulder_x: float,
) -> Any:
    """Adapta prendas de ``Torso`` y pantalones por envolventes de franjas y corrección polar hacia fuera."""
    mq = unreal.GeometryScript_MeshQueries
    me = unreal.GeometryScript_MeshEdits
    repair = getattr(unreal, "GeometryScript_MeshRepair", None)
    if repair is not None and hasattr(repair, "compact_mesh"):
        garment_dyn = repair.compact_mesh(garment_dyn)
    g_vc = int(garment_dyn.get_vertex_count())
    g_verts: list[tuple[float, float, float]] = []
    for vid in range(g_vc):
        v, ok = mq.get_vertex_position(garment_dyn, vid)
        g_verts.append((float(v.x), float(v.y), float(v.z)) if ok else (0.0, 0.0, 0.0))

    env = _compute_slice_envelopes(g_verts, body_pts, g_zmin, g_h, z_min, z_max, slot, norm_scale, clav_z, neck_z, shoulder_x)
    aligned: list[tuple[float, float, float, float]] = []
    for gx, gy, gz in g_verts:
        t = max(0.0, min(1.0, (gz - g_zmin) / g_h))
        u = _warp_t(slot, t)
        nz = z_min + u * (z_max - z_min)
        g_cx_t = 0.5 * (_interp_1d(env["g_xmin"], t) + _interp_1d(env["g_xmax"], t))
        g_hx_t = max(_interp_1d(env["g_hx"], t), 0.5)
        g_ymin_t = _interp_1d(env["g_ymin"], t)
        g_d_t = max(_interp_1d(env["g_depth"], t), 0.5)
        t_hx_t = _interp_1d(env["t_hx"], t)
        t_ymin_t = _interp_1d(env["t_ymin"], t)
        t_ymax_t = _interp_1d(env["t_ymax"], t)
        nx = (gx - g_cx_t) * (t_hx_t / g_hx_t)
        ny = t_ymin_t + (gy - g_ymin_t) * ((t_ymax_t - t_ymin_t) / g_d_t)
        aligned.append((nx, ny, nz, t))

    nz_bins, nth_bins = 14, 24
    r_body = [[4.0 for _ in range(nth_bins)] for _ in range(nz_bins)]
    for bx, by, bz in body_pts:
        if z_min - 1.5 <= bz <= z_max + 1.5 and abs(bx) <= _arm_cutoff(slot, bz, clav_z, neck_z, shoulder_x):
            u_b = max(0.0, min(1.0, (bz - z_min) / max(z_max - z_min, 1.0)))
            iz = min(nz_bins - 1, int(u_b * nz_bins))
            cy_b = _interp_1d(env["b_cy"], u_b)
            ith = int(((math.atan2(by - cy_b, bx) + math.pi) / (2.0 * math.pi)) * nth_bins) % nth_bins
            r = math.hypot(bx, by - cy_b)
            if r > r_body[iz][ith]:
                r_body[iz][ith] = r
    r_body = _smooth_2d_polar(r_body, nz_bins, nth_bins, passes=2)

    r_gmax = [[0.0 for _ in range(nth_bins)] for _ in range(nz_bins)]
    for nx, ny, _, t in aligned:
        iz = min(nz_bins - 1, int(_warp_t(slot, t) * nz_bins))
        cy_b = _interp_1d(env["b_cy"], t)
        ith = int(((math.atan2(ny - cy_b, nx) + math.pi) / (2.0 * math.pi)) * nth_bins) % nth_bins
        r = math.hypot(nx, ny - cy_b)
        if r > r_gmax[iz][ith]:
            r_gmax[iz][ith] = r

    r_scale = [[1.0 for _ in range(nth_bins)] for _ in range(nz_bins)]
    min_clear = 0.85 if slot == "Torso" else 1.05
    for iz in range(nz_bins):
        row_max = max(r_gmax[iz]) if max(r_gmax[iz]) > 1.0 else 10.0
        for ith in range(nth_bins):
            rmax = r_gmax[iz][ith]
            if rmax >= row_max * 0.55 and rmax < r_body[iz][ith] + min_clear:
                r_scale[iz][ith] = min(1.24, max(1.0, (r_body[iz][ith] + min_clear) / rmax))
    r_scale = _smooth_2d_polar(r_scale, nz_bins, nth_bins, passes=5)

    for vid, (nx, ny, nz, t) in enumerate(aligned):
        iz = min(nz_bins - 1, int(_warp_t(slot, t) * nz_bins))
        cy_b = _interp_1d(env["b_cy"], t)
        dy = ny - cy_b
        f_th = ((math.atan2(dy, nx) + math.pi) / (2.0 * math.pi)) * nth_bins
        ith0 = int(f_th) % nth_bins
        ith1 = (ith0 + 1) % nth_bins
        w_th = f_th - int(f_th)
        sc = r_scale[iz][ith0] * (1.0 - w_th) + r_scale[iz][ith1] * w_th
        me.set_vertex_position(garment_dyn, vid, unreal.Vector(nx * sc, cy_b + dy * sc, nz), vid < g_vc - 1)
    return garment_dyn


def _fit_garment_to_body(
    by_name: dict[str, Any],
    body_comp: Any,
    body_dyn: Any,
    body_skm: Any,
    slot_comp: Any,
    garment_dyn: Any,
    slot: str,
    scale_mult: float,
    z_off: float,
    asset_name: str = "",
) -> tuple[Any, list[float], list[float], float]:
    """Adapta y posiciona la prenda sobre el cuerpo (o usa ``target_slot_bounds`` en dobles de test y calzado)."""
    mesh_queries = unreal.GeometryScript_MeshQueries
    mesh_xform = unreal.GeometryScript_MeshTransforms
    g_box = mesh_queries.get_mesh_bounding_box(garment_dyn)
    g_size = g_box.max - g_box.min
    has_verts = (
        hasattr(body_dyn, "get_vertex_count")
        and hasattr(garment_dyn, "get_vertex_count")
        and hasattr(mesh_queries, "get_vertex_position")
        and hasattr(getattr(unreal, "GeometryScript_MeshEdits", None), "set_vertex_position")
    )

    if not has_verts or slot == "Feet":
        g_center = (g_box.max + g_box.min) * 0.5
        target_center, target_size = target_slot_bounds(body_skm, slot_comp, slot, scale_mult, z_off)
        sx = target_size[0] / max(float(g_size.x), 0.01)
        sy = target_size[1] / max(float(g_size.y), 0.01)
        sz = target_size[2] / max(float(g_size.z), 0.01)
        uniform_scale = (sx + sy + sz) / 3.0 if slot == "Feet" else sz
        garment_dyn = mesh_xform.scale_mesh(garment_dyn, unreal.Vector(uniform_scale, uniform_scale, uniform_scale), g_center)
        g_after = mesh_queries.get_mesh_bounding_box(garment_dyn)
        g_center_after = (g_after.max + g_after.min) * 0.5
        offset = unreal.Vector(target_center[0], target_center[1], target_center[2]) - g_center_after
        garment_dyn = mesh_xform.translate_mesh(garment_dyn, offset)
        fitted = (garment_dyn, target_center, target_size, uniform_scale)
    else:
        garment_dyn, avg_scale = _deform_garment_to_slices(
            by_name, body_comp, body_dyn, garment_dyn, slot, scale_mult, z_off, asset_name
        )
        final_box = mesh_queries.get_mesh_bounding_box(garment_dyn)
        f_center = (final_box.max + final_box.min) * 0.5
        f_size = final_box.max - final_box.min
        fitted = (
            garment_dyn,
            [float(f_center.x), float(f_center.y), float(f_center.z)],
            [float(f_size.x), float(f_size.y), float(f_size.z)],
            avg_scale,
        )
    return fitted


def resolve_garment_static_mesh(
    garment_source: str,
    folder_path: str,
    asset_name: str,
    import_model_fn: Callable[[str, str, str], str],
) -> tuple[Any, str | None]:
    """Obtiene el ``UStaticMesh`` de una ruta en disco (importándolo) o de un asset ``/Game/...``."""
    mesh = None
    err = None
    if os.path.isfile(garment_source):
        imported = json.loads(import_model_fn(garment_source, folder_path, asset_name))
        primary = imported.get("primary") if imported.get("success") else None
        mesh = unreal.load_asset(primary) if primary else None
        if mesh is None:
            err = imported.get("error") or f"failed to import garment from {garment_source}"
    elif garment_source.startswith("/"):
        mesh = unreal.load_asset(garment_source)
        if mesh is None:
            err = f"asset not found: {garment_source}"
    else:
        err = f"garment_source must be a local file or an Unreal asset path: {garment_source}"
    if mesh is not None and not isinstance(mesh, unreal.StaticMesh):
        err = f"expected a StaticMesh for rigging, got {type(mesh).__name__}"
        mesh = None
    return (mesh, err)


def _normalize_equip_args(
    folder_path: Any,
    asset_name: Any,
    scale_multiplier: Any,
    z_offset: Any,
) -> tuple[str, str, float, float, str | None]:
    """Normaliza argumentos (incluyendo llamadas posicionales de 5 argumentos) y valida tipos."""
    if isinstance(folder_path, (int, float)):
        z_offset = float(asset_name) if isinstance(asset_name, (int, float)) else 0.0
        scale_multiplier = float(folder_path)
        folder_path = "/Game/DBV/MetaHumanOutfits"
        asset_name = ""
    err: str | None = None
    scale_val = 1.05
    z_val = 0.0
    if not isinstance(folder_path, str) or not folder_path.startswith("/Game/"):
        err = f"folder_path must be a string starting with '/Game/', got {folder_path!r}"
        clean_folder = ""
    else:
        clean_folder = folder_path
        try:
            scale_val = float(scale_multiplier if scale_multiplier is not None else 1.05)
            z_val = float(z_offset if z_offset is not None else 0.0)
        except (TypeError, ValueError) as exc:
            err = f"scale_multiplier and z_offset must be numeric: {exc}"
    return (clean_folder, str(asset_name or ""), scale_val, z_val, err)


def _create_and_equip_skeletal_garment(
    target_actor: Any,
    body_comp: Any,
    slot_comp: Any,
    body_dyn: Any,
    garment_dyn: Any,
    garment_sm: Any,
    skeleton: Any,
    dest_skm_path: str,
    clean_slot: str,
    smoothing_iterations: int,
    restored_skin: bool,
    target_center: list[float],
    target_size: list[float],
    uniform_scale: float,
    skirt_hip_z: float | None = None,
    source_body_name: str = "",
) -> dict[str, Any]:
    """Transfiere pesos, crea el ``SkeletalMesh``, limpia override materials y enlaza ``LeaderPoseComponent``."""
    is_covered, ratio, desc = check_body_coverage(body_dyn, clean_slot, garment_dyn)
    if not is_covered:
        body_label = source_body_name or (body_comp.get_skeletal_mesh_asset().get_name() if (body_comp and hasattr(body_comp, "get_skeletal_mesh_asset") and body_comp.get_skeletal_mesh_asset()) else "Body")
        return {
            "success": False,
            "error": f"Body '{body_label}' no cubre la zona de {clean_slot}; no se pueden transferir pesos ({desc})",
        }

    if hasattr(unreal, "GeometryScript_BoneWeights") and hasattr(unreal.GeometryScript_BoneWeights, "copy_bones_from_mesh"):
        copy_opts = unreal.GeometryScriptCopyBonesFromMeshOptions()
        garment_dyn = unreal.GeometryScript_BoneWeights.copy_bones_from_mesh(body_dyn, garment_dyn, copy_opts)

    transfer_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
    transfer_opts.transfer_method = unreal.TransferBoneWeightsMethod.INPAINT_WEIGHTS
    if hasattr(transfer_opts, "num_smoothing_iterations"):
        transfer_opts.num_smoothing_iterations = max(0, int(smoothing_iterations))
    if hasattr(transfer_opts, "smoothing_strength"):
        transfer_opts.smoothing_strength = 0.5

    try:
        garment_dyn = unreal.GeometryScript_BoneWeights.transfer_bone_weights_from_mesh(body_dyn, garment_dyn, transfer_opts)
    except Exception:
        fallback_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
        fallback_opts.transfer_method = unreal.TransferBoneWeightsMethod.CLOSEST_POINT_ON_SURFACE
        garment_dyn = unreal.GeometryScript_BoneWeights.transfer_bone_weights_from_mesh(body_dyn, garment_dyn, fallback_opts)
    if skirt_hip_z is not None:
        garment_dyn = _apply_skirt_weights(garment_dyn, skirt_hip_z)
    _, has_weights = unreal.GeometryScript_BoneWeights.mesh_has_bone_weights(garment_dyn)

    skm_opts = unreal.GeometryScriptCreateNewSkeletalMeshAssetOptions()
    skm_opts.enable_recompute_normals = False
    skm_opts.enable_recompute_tangents = True
    sm_mats = garment_sm.get_editor_property("static_materials") if hasattr(garment_sm, "get_editor_property") else None
    if sm_mats:
        skm_opts.materials = {m.material_slot_name: m.material_interface for m in sm_mats}

    new_skm, _ = unreal.GeometryScript_NewAssetUtils.create_new_skeletal_mesh_asset_from_mesh(
        garment_dyn, skeleton, dest_skm_path, skm_opts
    )
    if new_skm is None:
        outcome: dict[str, Any] = {"success": False, "error": f"GeometryScript failed to create SkeletalMesh at {dest_skm_path}"}
    else:
        if fit is not None:
            fit.apply_two_sided_materials(new_skm)
            fit.setup_garment_lods(new_skm, target_lod_count=4)
        if source_body_name and hasattr(unreal, "EditorAssetLibrary") and hasattr(unreal.EditorAssetLibrary, "set_metadata_tag"):
            unreal.EditorAssetLibrary.set_metadata_tag(new_skm, "source_full_body", source_body_name)
        unreal.EditorAssetLibrary.save_loaded_asset(new_skm, False)
        with unreal.ScopedEditorTransaction("DBV Unreal AI: equip MetaHuman outfit"):
            target_actor.modify()
            slot_comp.modify()
            slot_comp.set_skeletal_mesh_asset(new_skm)
            if hasattr(slot_comp, "set_editor_property"):
                slot_comp.set_editor_property("visible", True)
            if hasattr(slot_comp, "empty_override_materials"):
                slot_comp.empty_override_materials()
            if sm_mats and hasattr(slot_comp, "set_material"):
                for idx, mat_slot in enumerate(sm_mats):
                    if getattr(mat_slot, "material_interface", None) is not None:
                        slot_comp.set_material(idx, mat_slot.material_interface)
            if hasattr(slot_comp, "set_leader_pose_component"):
                slot_comp.set_leader_pose_component(body_comp, False)
            neck_hide = _disable_neck_hide(metahuman_components(target_actor).get("Face")) if clean_slot == "Torso" else []
        skm_path = new_skm.get_path_name()
        outcome = {
            "success": True,
            "actor": target_actor.get_actor_label(),
            "slot": clean_slot,
            "skeletal_mesh": skm_path,
            "rigged_asset": skm_path,
            "skeleton": skeleton.get_path_name(),
            "has_bone_weights": bool(has_weights),
            "restored_full_body_skin": restored_skin,
            "neck_hide_disabled": neck_hide,
            "target_center": [round(v, 1) for v in target_center],
            "target_size": [round(v, 1) for v in target_size],
            "uniform_scale": round(uniform_scale, 3),
            "two_sided_materials": True,
            "lod_count": 4,
            "source_body": source_body_name,
        }
    return outcome


# Ranuras cuya prenda se ajusta con el algoritmo del sastre (REQ-98, ADR-V050-007). Legs solo si no es falda.
TAILOR_KIND_BY_SLOT = {"Torso": "top", "Legs": "pants"}


def _apply_tailored_slot(slot_comp: Any, skm: Any, material: Any, body_comp: Any) -> None:
    """Equipa ``skm`` en ``slot_comp`` limpiando materiales heredados y sincronizando pose con ``body_comp``. @spec REQ-81, REQ-98"""
    slot_comp.modify()
    slot_comp.set_skeletal_mesh_asset(skm)
    # La ropa de fábrica deja materiales de sustitución en la ranura (spike S116a).
    if hasattr(slot_comp, "empty_override_materials"):
        slot_comp.empty_override_materials()
    if hasattr(slot_comp, "set_editor_property"):
        slot_comp.set_editor_property("visible", True)
        slot_comp.set_editor_property("override_materials", [])
    if material is not None:
        slot_comp.set_material(0, material)
    slot_comp.set_leader_pose_component(body_comp, False)


def _reequip_top_over_pants(target_actor: Any, full_skm: Any, full_path: str, pants_skm: Any, folder: str) -> bool:
    """Reajusta el top existente por fuera del pantalón recién equipado si procede (REQ-98 paso 7). @spec REQ-98"""
    from .outfit import tailor

    rebuilt = False
    comps = metahuman_components(target_actor)
    torso_comp = comps.get("Torso")
    body_comp = comps.get("Body")
    top_skm = torso_comp.get_skeletal_mesh_asset() if torso_comp is not None else None
    get_tag = getattr(unreal.EditorAssetLibrary, "get_metadata_tag", None)
    method = str(get_tag(top_skm, "dbv_method") or "") if top_skm is not None and callable(get_tag) else ""
    source_sm_path = str(get_tag(top_skm, "dbv_source_sm") or "") if top_skm is not None and callable(get_tag) else ""
    limit_flag = str(get_tag(top_skm, "dbv_limit_top_length") or "") == "1" if top_skm is not None and callable(get_tag) else False
    top_sm = unreal.EditorAssetLibrary.load_asset(source_sm_path.split(".")[0]) if method == "tailor" and source_sm_path else None
    if top_sm is not None and torso_comp is not None and body_comp is not None:
        top_name = f"{top_skm.get_path_name().rsplit('/', 1)[-1].split('.')[0]}_layered"
        built_top = tailor.build_tailored_garment(
            full_skm, top_sm, "top", folder, top_name,
            limit_top_length=limit_flag, under_skm=pants_skm,
        )
        if built_top.get("success"):
            new_top_skm = built_top["skeletal_mesh"]
            if hasattr(unreal.EditorAssetLibrary, "set_metadata_tag"):
                unreal.EditorAssetLibrary.set_metadata_tag(new_top_skm, "source_full_body", full_path)
                unreal.EditorAssetLibrary.set_metadata_tag(new_top_skm, "dbv_method", "tailor")
                unreal.EditorAssetLibrary.set_metadata_tag(new_top_skm, "dbv_source_sm", source_sm_path)
                unreal.EditorAssetLibrary.set_metadata_tag(new_top_skm, "dbv_limit_top_length", "1" if limit_flag else "0")
            unreal.EditorAssetLibrary.save_loaded_asset(new_top_skm, False)
            _apply_tailored_slot(torso_comp, new_top_skm, built_top.get("material"), body_comp)
            rebuilt = True
    return rebuilt


def _equip_tailored_garment(
    target_actor: Any,
    body_comp: Any,
    slot_comp: Any,
    garment_sm: Any,
    kind: str,
    clean_slot: str,
    folder: str,
    dest_name: str,
    build: str,
    restored_skin: bool,
    clearance_cm: float | None,
    limit_top_length: bool = False,
) -> dict[str, Any]:
    """Ajusta la prenda al cuerpo con el algoritmo del sastre (REQ-98), la equipa y la mide (REQ-89). @spec REQ-81, REQ-98"""
    from .outfit import clearance, tailor

    full_skm, full_path = _find_full_body_asset(build)
    outcome: dict[str, Any] = {"success": False}
    if full_skm is None or build not in full_path.lower():
        outcome["error"] = f"No hay malla de cuerpo completo para la complexión «{build}» en /Game/MetaHumans/Common: no se puede construir la prenda."
    else:
        legs_comp = metahuman_components(target_actor).get("Legs") if kind == "top" else None
        under_skm = legs_comp.get_skeletal_mesh_asset() if legs_comp is not None else None
        built = tailor.build_tailored_garment(
            full_skm, garment_sm, kind, folder, dest_name, clearance_cm,
            limit_top_length=limit_top_length, under_skm=under_skm,
        )
        if not built.get("success"):
            outcome["error"] = built.get("error", "No se pudo ajustar la prenda con el sastre.")
        else:
            skm = built["skeletal_mesh"]
            if fit is not None:
                fit.setup_garment_lods(skm, target_lod_count=4)
            sm_path = garment_sm.get_path_name() if hasattr(garment_sm, "get_path_name") else ""
            if hasattr(unreal.EditorAssetLibrary, "set_metadata_tag"):
                unreal.EditorAssetLibrary.set_metadata_tag(skm, "source_full_body", full_path)
                unreal.EditorAssetLibrary.set_metadata_tag(skm, "dbv_method", "tailor")
                unreal.EditorAssetLibrary.set_metadata_tag(skm, "dbv_source_sm", sm_path)
                unreal.EditorAssetLibrary.set_metadata_tag(skm, "dbv_limit_top_length", "1" if limit_top_length else "0")
            unreal.EditorAssetLibrary.save_loaded_asset(skm, False)
            with unreal.ScopedEditorTransaction("DBV Unreal AI: equip MetaHuman outfit"):
                target_actor.modify()
                if body_comp.get_skeletal_mesh_asset().get_path_name() != full_skm.get_path_name():
                    skin_material = body_comp.get_material(0)
                    body_comp.modify()
                    body_comp.set_skeletal_mesh_asset(full_skm)
                    if skin_material is not None:
                        body_comp.set_material(0, skin_material)
                    restored_skin = True
                _apply_tailored_slot(slot_comp, skm, built.get("material"), body_comp)
                neck_hide = _disable_neck_hide(metahuman_components(target_actor).get("Face")) if clean_slot == "Torso" else []
                rebuilt_top = _reequip_top_over_pants(target_actor, full_skm, full_path, skm, folder) if kind == "pants" else False
            drawn_body = metahuman_components(target_actor)["Body"].get_skeletal_mesh_asset()
            measured = clearance.measure_assets(skm, drawn_body, full_skm, kind)
            skm_path = skm.get_path_name()
            mat = built.get("material")
            mat_path = mat.get_path_name() if mat is not None and hasattr(mat, "get_path_name") else ""
            outcome = {
                "success": True,
                "method": "tailor",
                "actor": target_actor.get_actor_label(),
                "slot": clean_slot,
                "garment_kind": kind,
                "skeletal_mesh": skm_path,
                "rigged_asset": skm_path,
                "material": mat_path,
                "textures": built.get("textures", {}),
                "offset_cm": built["offset_cm"],
                "placement": built["placement"],
                "zone": built["zone"],
                "limit_top_length": bool(limit_top_length),
                "rebuilt_top_over_pants": rebuilt_top,
                "has_bone_weights": True,
                "restored_full_body_skin": restored_skin,
                "neck_hide_disabled": neck_hide,
                "two_sided_materials": True,
                "lod_count": 4,
                "source_body": full_path,
                "validation": measured,
                "warnings": built.get("warnings", []) + measured.get("warnings", []),
            }
    return outcome


def equip_metahuman_outfit(
    actor_label: str,
    garment_source: str,
    slot: str = "Torso",
    folder_path: str = "/Game/DBV/MetaHumanOutfits",
    asset_name: str = "",
    scale_multiplier: float = 1.05,
    z_offset: float = 0.0,
    smoothing_iterations: int = 5,
    *,
    import_model_fn: Callable[[str, str, str], str],
    unique_name_fn: Callable[[str, str], str],
    clearance_cm: float | None = None,
    limit_top_length: bool = False,
) -> str:
    """Ajusta y equipa una prenda en un MetaHuman con el sastre (REQ-98) o transferencia de pesos (REQ-81). @spec REQ-81, REQ-83, REQ-98"""
    result: dict[str, Any] = {"success": False}
    clean_folder, clean_asset, clean_scale, clean_z, arg_err = _normalize_equip_args(
        folder_path, asset_name, scale_multiplier, z_offset
    )
    if arg_err is not None:
        result["error"] = arg_err
    else:
        try:
            clean_slot = slot if slot in METAHUMAN_SLOTS else "Torso"
            actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
            candidates = [a for a in actors if "Body" in metahuman_components(a)]
            # REQ-83: nunca se elige un destino por cuenta propia; sin etiqueta válida se enumeran los disponibles.
            target_actor = next((a for a in candidates if actor_label and a.get_actor_label() == actor_label), None)

            if target_actor is None:
                available = [a.get_actor_label() for a in candidates]
                if actor_label:
                    result["error"] = f"MetaHuman '{actor_label}' not found in level. Available MetaHumans: {available}"
                else:
                    result["error"] = f"No target MetaHuman was chosen. Ask the user which one to dress. Available MetaHumans: {available}"
                result["available"] = available
            elif not hasattr(unreal, "GeometryScript_AssetUtils") or not hasattr(unreal, "GeometryScript_BoneWeights"):
                result["error"] = "GeometryScripting plugin is not active in the Unreal Editor. Enable it or use Prepare Project."
            else:
                by_name = metahuman_components(target_actor)
                body_comp = by_name.get("Body")
                slot_comp = by_name.get(clean_slot)
                body_skm = body_comp.get_skeletal_mesh_asset() if body_comp else None
                garment_sm, mesh_err = resolve_garment_static_mesh(garment_source, clean_folder, clean_asset, import_model_fn)
                if body_skm is None or slot_comp is None:
                    result["error"] = f"MetaHuman '{target_actor.get_actor_label()}' is missing Body mesh or slot '{clean_slot}'"
                elif mesh_err is not None or garment_sm is None:
                    result["error"] = mesh_err or "garment StaticMesh could not be loaded"
                else:
                    build = body_build(body_skm, target_actor)
                    full_skm, body_dyn, restored_skin, source_body_path = _ensure_full_body(
                        body_comp, body_skm, build, clean_slot, target_actor
                    )
                    by_name = metahuman_components(target_actor)
                    body_comp = by_name.get("Body")
                    slot_comp = by_name.get(clean_slot)
                    skeleton = full_skm.get_editor_property("skeleton") if full_skm else None
                    if skeleton is None or body_comp is None or slot_comp is None:
                        result["error"] = f"MetaHuman '{target_actor.get_actor_label()}' has no valid skeleton on Body"
                    else:
                        if not unreal.EditorAssetLibrary.does_directory_exist(clean_folder):
                            unreal.EditorAssetLibrary.make_directory(clean_folder)
                        base_name = clean_asset.strip() or os.path.splitext(os.path.basename(garment_source))[0]
                        if not base_name.startswith("SK_"):
                            base_name = f"SK_{base_name}"
                        dest_skm_path = f"{clean_folder}/{unique_name_fn(clean_folder, base_name)}"
                        lod = unreal.GeometryScriptMeshReadLOD()
                        garment_dyn = unreal.DynamicMesh()
                        garment_dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(
                            garment_sm, garment_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod, True
                        )
                        garment_dyn = _clean_and_orient_garment(garment_dyn, clean_slot)
                        if clean_slot in ("Torso", "Legs") and fit is not None:
                            garment_dyn = fit.weld_top_dynamic_mesh(garment_dyn)
                        oriented = unreal.GeometryScript_MeshQueries.get_mesh_bounding_box(garment_dyn)
                        oriented_size = oriented.max - oriented.min
                        is_skirt = _is_skirt_shape(clean_slot, float(oriented_size.z), float(oriented_size.x), clean_asset or garment_source)
                        tailor_kind = TAILOR_KIND_BY_SLOT.get(clean_slot) if not is_skirt else None
                        if tailor_kind is not None:
                            # REQ-98 / ADR-V050-007: tops y pantalones se ajustan con el algoritmo del sastre.
                            result = _equip_tailored_garment(
                                target_actor, body_comp, slot_comp, garment_sm, tailor_kind, clean_slot,
                                clean_folder, dest_skm_path.rsplit("/", 1)[-1], build, restored_skin,
                                clearance_cm, limit_top_length=limit_top_length,
                            )
                        else:
                            skirt_hip_z = _reference_bone_z(skeleton, "thigh_l") + SKIRT_HIP_MARGIN_CM if is_skirt else None
                            garment_dyn, target_center, target_size, uniform_scale = _fit_garment_to_body(
                                by_name, body_comp, body_dyn, full_skm, slot_comp, garment_dyn, clean_slot, clean_scale, clean_z,
                                asset_name=clean_asset or garment_source,
                            )
                            result = _create_and_equip_skeletal_garment(
                                target_actor, body_comp, slot_comp, body_dyn, garment_dyn, garment_sm,
                                skeleton, dest_skm_path, clean_slot, smoothing_iterations,
                                restored_skin, target_center, target_size, uniform_scale,
                                skirt_hip_z,
                                source_body_name=source_body_path or (full_skm.get_path_name() if full_skm else ""),
                            )
                        if result.get("success"):
                            result["build"] = build  # REQ-84: la app compara el corte de la prenda con este cuerpo
                            result["source_body"] = source_body_path or (full_skm.get_path_name() if full_skm else "")
        except Exception as exc:  # noqa: BLE001 — devolvemos error corto y legible sin volcar traceback entero
            result["error"] = f"{type(exc).__name__}: {exc}"
    return _json(result)
