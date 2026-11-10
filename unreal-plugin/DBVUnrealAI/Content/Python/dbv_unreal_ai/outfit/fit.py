# =============================================================================
# DBV Unreal AI — S99: Ajuste de falda y vestido en cualquier cuerpo
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
#
# @spec REQ-87
# Port de skirt_shell y dress_fit sin constantes de Victoria:
# - Alturas de cintura, cadera, dobladillo y cuello calculadas desde huesos reales.
# - Detección y validación de PhysicsAsset por complexión (f_med_nrw, m_tal_nrw, etc.).
# - Reconstrucción de falda como tubo de una capa (REJECTED-10, REJECTED-11).
# - Malla de simulación simplificada para Chaos Cloth solver (D59).
# - Aislamiento de prendas de Tripo con figura completa (P-21).

from __future__ import annotations

import math
from typing import Any

try:
    import unreal

    HAS_UNREAL = True
except ImportError:
    unreal = None  # type: ignore[assignment]
    HAS_UNREAL = False


ARCHETYPE_PHYSICS_ASSETS: dict[str, str] = {
    "f_med_nrw": "/Game/MetaHumans/Common/Female/Medium/NormalWeight/Body/f_med_nrw_body_Physics.f_med_nrw_body_Physics",
    "f_med_ovw": "/Game/MetaHumans/Common/Female/Medium/Overweight/Body/f_med_ovw_body_Physics.f_med_ovw_body_Physics",
    "f_med_unw": "/Game/MetaHumans/Common/Female/Medium/Underweight/Body/f_med_unw_body_Physics.f_med_unw_body_Physics",
    "f_tal_nrw": "/Game/MetaHumans/Common/Female/Tall/NormalWeight/Body/f_tal_nrw_body_Physics.f_tal_nrw_body_Physics",
    "f_sht_nrw": "/Game/MetaHumans/Common/Female/Short/NormalWeight/Body/f_sht_nrw_body_Physics.f_sht_nrw_body_Physics",
    "m_med_nrw": "/Game/MetaHumans/Common/Male/Medium/NormalWeight/Body/m_med_nrw_body_Physics.m_med_nrw_body_Physics",
    "m_med_ovw": "/Game/MetaHumans/Common/Male/Medium/Overweight/Body/m_med_ovw_body_Physics.m_med_ovw_body_Physics",
    "m_med_unw": "/Game/MetaHumans/Common/Male/Medium/Underweight/Body/m_med_unw_body_Physics.m_med_unw_body_Physics",
    "m_tal_nrw": "/Game/MetaHumans/Common/Male/Tall/NormalWeight/Body/m_tal_nrw_body_Physics.m_tal_nrw_body_Physics",
    "m_tal_ovw": "/Game/MetaHumans/Common/Male/Tall/Overweight/Body/m_tal_ovw_body_Physics.m_tal_ovw_body_Physics",
    "m_sht_nrw": "/Game/MetaHumans/Common/Male/Short/NormalWeight/Body/m_sht_nrw_body_Physics.m_sht_nrw_body_Physics",
}


def measure_body_landmarks(body_comp: Any, actor: Any | None = None) -> dict[str, Any]:
    """Mide alturas de huesos anatómicos y localiza PhysicsAsset en cualquier cuerpo MetaHuman.

    Lee los huesos del componente visible en actor local space, validando que no estén a cero
    y que sigan un orden anatómico descendente. @spec REQ-87
    """
    res: dict[str, Any] = {
        "success": False,
        "error": None,
        "landmarks": None,
        "physics_asset": None,
    }
    if body_comp is None:
        res["error"] = "body_comp no puede ser None"
        return res

    owner = actor or (body_comp.get_owner() if hasattr(body_comp, "get_owner") else None)
    actor_z = 0.0
    if owner and hasattr(owner, "get_actor_location"):
        actor_z = float(owner.get_actor_location().z)

    bone_names = [
        "head",
        "neck_01",
        "clavicle_l",
        "spine_03",
        "spine_02",
        "pelvis",
        "thigh_l",
        "calf_l",
        "foot_l",
    ]
    heights: dict[str, float] = {}
    missing_bone: str | None = None

    for name in bone_names:
        hz = _read_bone_height(body_comp, name, actor_z)
        if hz is None or hz <= 0.0:
            missing_bone = name
            break
        heights[name] = round(hz, 2)

    if missing_bone:
        res["error"] = (
            f"Hueso «{missing_bone}» no encontrado o con altura 0. "
            "Un actor oculto devuelve huesos a 0; haz visible el actor y espera un tick del editor."
        )
    elif not (heights["head"] > heights["neck_01"] > heights["spine_02"] > heights["thigh_l"] > heights["foot_l"]):
        res["error"] = (
            f"Alturas de huesos anómalas: head={heights['head']}, neck={heights['neck_01']}, "
            f"spine_02={heights['spine_02']}, thigh={heights['thigh_l']}, foot={heights['foot_l']}"
        )
    else:
        phys_asset = _resolve_physics_asset(body_comp)
        if not phys_asset:
            res["error"] = "No se encontró PhysicsAsset para el cuerpo del MetaHuman. REQ-87 exige física válida."
        else:
            waist_z = heights["spine_02"]
            hip_z = heights["thigh_l"]
            knee_z = heights["calf_l"]
            ankle_z = heights["foot_l"]
            default_hem_z = round((knee_z + ankle_z) * 0.5, 2)
            landmarks = {
                "head_z": heights["head"],
                "neck_z": heights["neck_01"],
                "shoulder_z": heights["clavicle_l"],
                "chest_z": heights["spine_03"],
                "waist_z": waist_z,
                "hip_z": hip_z,
                "knee_z": knee_z,
                "ankle_z": ankle_z,
                "default_hem_z": default_hem_z,
                "all_bones": heights,
            }
            res["success"] = True
            res["landmarks"] = landmarks
            res["physics_asset"] = phys_asset

    return res


def _read_bone_height(body_comp: Any, bone_name: str, actor_z: float) -> float | None:
    """Lee la altura Z relativa al actor de un hueso en body_comp."""
    hz: float | None = None
    if hasattr(body_comp, "get_bone_index") and body_comp.get_bone_index(bone_name) != -1:
        if hasattr(body_comp, "get_socket_location"):
            loc = body_comp.get_socket_location(bone_name)
            hz = float(loc.z) - actor_z
        elif hasattr(body_comp, "get_socket_transform"):
            rel_space = getattr(unreal, "RelativeTransformSpace", None)
            space_val = rel_space.RTS_WORLD if rel_space else 0
            xf = body_comp.get_socket_transform(bone_name, space_val)
            hz = float(xf.translation.z) - actor_z
    return hz


def _resolve_physics_asset(body_comp: Any) -> Any:
    """Localiza el PhysicsAsset adecuado en el SkeletalMesh o por catálogo de complexión."""
    phys: Any = None
    skm = body_comp.get_skeletal_mesh_asset() if hasattr(body_comp, "get_skeletal_mesh_asset") else None
    if skm and hasattr(skm, "get_editor_property"):
        try:
            phys = skm.get_editor_property("physics_asset")
        except Exception:
            phys = None

    if not phys and skm:
        name = skm.get_name()
        for key, path in ARCHETYPE_PHYSICS_ASSETS.items():
            if key in name:
                phys = unreal.load_asset(path) if unreal and hasattr(unreal, "load_asset") else path
                break

    return phys


def sample_body_radii_at_height(
    body_points: list[tuple[float, float, float]],
    z_min: float,
    z_max: float,
    num_sectors: int = 36,
) -> list[float]:
    """Calcula el radio exterior máximo del cuerpo por sectores angulares en un intervalo Z.

    Divide 360° en num_sectors y encuentra la distancia radial máxima (X, Y) desde el eje Z.
    """
    radii = [0.0] * num_sectors
    slice_pts = [p for p in body_points if z_min <= p[2] <= z_max]

    for x, y, _ in slice_pts:
        angle = math.atan2(y, x)
        if angle < 0:
            angle += 2.0 * math.pi
        sector = int(angle / (2.0 * math.pi / num_sectors)) % num_sectors
        r = math.hypot(x, y)
        if r > radii[sector]:
            radii[sector] = r

    # Rellenar sectores vacíos con la media vecina
    avg_r = sum(radii) / max(len([r for r in radii if r > 0]), 1)
    for i in range(num_sectors):
        if radii[i] <= 0.0:
            radii[i] = avg_r

    return radii


def generate_skirt_tube_geometry(
    body_points: list[tuple[float, float, float]],
    waist_z: float,
    hem_z: float,
    hip_z: float,
    clearance_cm: float = 1.5,
    num_rings: int = 24,
    num_sectors: int = 36,
) -> dict[str, Any]:
    """Genera vértices, normales, UVs e índices de triángulo para un tubo de falda ceñido al cuerpo.

    Resuelve REJECTED-10 y REJECTED-11: malla limpia de una sola capa sin agujeros ni manchas.
    @spec REQ-87
    """
    vertices: list[tuple[float, float, float]] = []
    uvs: list[tuple[float, float]] = []
    triangles: list[tuple[int, int, int]] = []

    z_step = (waist_z - hem_z) / float(num_rings - 1)
    dz_sample = max(z_step * 0.75, 1.5)

    last_valid_radii = [15.0] * num_sectors

    for r in range(num_rings):
        ring_z = waist_z - float(r) * z_step
        v_coord = float(r) / float(num_rings - 1)

        # Muestrear radios del cuerpo a esta altura
        body_radii = sample_body_radii_at_height(body_points, ring_z - dz_sample, ring_z + dz_sample, num_sectors)
        if any(rad > 0.0 for rad in body_radii):
            last_valid_radii = list(body_radii)
        else:
            body_radii = list(last_valid_radii)

        # Holgura progresiva: ceñida en cintura (clearance_cm), ampliada suavemente hacia el dobladillo
        flare = 1.0
        if ring_z < hip_z:
            progress = (hip_z - ring_z) / max(hip_z - hem_z, 1.0)
            flare = 1.0 + 0.35 * progress

        for s in range(num_sectors):
            angle = float(s) * (2.0 * math.pi / float(num_sectors))
            radius = (body_radii[s] + clearance_cm) * flare
            vx = radius * math.cos(angle)
            vy = radius * math.sin(angle)
            vertices.append((round(vx, 3), round(vy, 3), round(ring_z, 3)))
            u_coord = float(s) / float(num_sectors)
            uvs.append((round(u_coord, 4), round(v_coord, 4)))

    # Generar caras triangulares uniendo anillos contiguos
    for r in range(num_rings - 1):
        r0 = r * num_sectors
        r1 = (r + 1) * num_sectors
        for s in range(num_sectors):
            s_next = (s + 1) % num_sectors
            v00 = r0 + s
            v01 = r0 + s_next
            v10 = r1 + s
            v11 = r1 + s_next
            # Cara 1: v00, v10, v01
            triangles.append((v00, v10, v01))
            # Cara 2: v01, v10, v11
            triangles.append((v01, v10, v11))

    return {
        "vertices": vertices,
        "uvs": uvs,
        "triangles": triangles,
        "num_vertices": len(vertices),
        "num_triangles": len(triangles),
    }


def isolate_garment_triangles(
    vertices: list[tuple[float, float, float]],
    triangles: list[tuple[int, int, int]],
    kind: str,
    waist_z: float,
    hem_z: float,
) -> list[tuple[int, int, int]]:
    """Aísla los triángulos de la prenda según su tipo, descartando extremidades o ropa extra (P-21).

    - 'top': conserva vértices con Z >= waist_z - 6.0 cm (descarta piernas/botas).
    - 'skirt': conserva vértices entre hem_z - 3.0 cm y waist_z + 4.0 cm (descarta torso y pies).
    - 'pants': conserva vértices con Z <= waist_z + 4.0 cm.
    """
    retained_tris: list[tuple[int, int, int]] = []

    for i0, i1, i2 in triangles:
        if i0 >= len(vertices) or i1 >= len(vertices) or i2 >= len(vertices):
            continue
        v0, v1, v2 = vertices[i0], vertices[i1], vertices[i2]
        cz = (v0[2] + v1[2] + v2[2]) / 3.0

        keep = True
        if kind == "top":
            keep = cz >= (waist_z - 6.0)
        elif kind == "skirt":
            keep = (hem_z - 3.0) <= cz <= (waist_z + 4.0)
        elif kind == "pants":
            keep = cz <= (waist_z + 4.0)

        if keep:
            retained_tris.append((i0, i1, i2))

    return retained_tris


def conform_sleeves_to_arm_pose(
    vertices: list[tuple[float, float, float]],
    shoulder_x: float = 16.5,
    blend_x: float = 21.5,
    pivot_z: float = 38.0,
    angle_down_deg: float = 53.6,
    angle_fwd_deg: float = 10.0,
    sleeve_thickness_factor: float = 0.12,
    z_cutoff_delta: float = 45.0,
) -> list[tuple[float, float, float]]:
    """Ajusta mangas en pose T o A abierta hacia la pose A exacta de MetaHuman (53.6°).

    Rota de forma suave los vértices de las mangas a partir del hombro hacia abajo y adelante,
    ampliando ligeramente el grosor de la manga para evitar que los brazos atraviesen el cuero.
    @spec REQ-87, REQ-80
    """
    out_verts: list[tuple[float, float, float]] = []

    if not vertices:
        return out_verts

    theta_down = math.radians(angle_down_deg)
    theta_fwd = math.radians(angle_fwd_deg)
    pivot_y = 0.0

    for x, y, z in vertices:
        ax = abs(x)
        if ax > shoulder_x and z > (pivot_z - z_cutoff_delta):
            t = min(1.0, max(0.0, (ax - shoulder_x) / max(blend_x - shoulder_x, 0.001)))
            w = t * t * (3.0 - 2.0 * t)

            th_d = theta_down * w
            cd, sd = math.cos(th_d), math.sin(th_d)

            th_f = theta_fwd * w
            cf, sf = math.cos(th_f), math.sin(th_f)

            sleeve_scale = 1.0 + sleeve_thickness_factor * w

            if x > 0:
                dx = (x - shoulder_x) * sleeve_scale
                dz = (z - pivot_z) * sleeve_scale
                dy = (y - pivot_y) * sleeve_scale
                nx1 = shoulder_x + dx * cd + dz * sd
                nz1 = pivot_z - dx * sd + dz * cd
                nx = shoulder_x + (nx1 - shoulder_x) * cf - dy * sf
                ny = pivot_y + (nx1 - shoulder_x) * sf + dy * cf
                nz = nz1
            else:
                dx = (x - (-shoulder_x)) * sleeve_scale
                dz = (z - pivot_z) * sleeve_scale
                dy = (y - pivot_y) * sleeve_scale
                nx1 = -shoulder_x + dx * cd - dz * sd
                nz1 = pivot_z + dx * sd + dz * cd
                nx = -shoulder_x + (nx1 - (-shoulder_x)) * cf + dy * sf
                ny = pivot_y - (nx1 - (-shoulder_x)) * sf + dy * cf
                nz = nz1
            out_verts.append((round(nx, 4), round(ny, 4), round(nz, 4)))
        else:
            out_verts.append((round(x, 4), round(y, 4), round(z, 4)))

    return out_verts


def split_suit_geometry(
    vertices: list[tuple[float, float, float]],
    triangles: list[tuple[int, int, int]],
    waist_z: float,
    overlap_cm: float = 2.0,
) -> dict[str, Any]:
    """Separa la geometría de un traje completo en chaqueta (top) y pantalón (legs) con solape en cinturilla.

    @spec REQ-80, REQ-87
    """
    top_triangles: list[tuple[int, int, int]] = []
    bottom_triangles: list[tuple[int, int, int]] = []

    if not vertices or not triangles:
        return {
            "top_triangles": top_triangles,
            "bottom_triangles": bottom_triangles,
            "waist_z": waist_z,
            "overlap_cm": overlap_cm,
        }

    z_cutoff_top = waist_z - overlap_cm
    z_cutoff_bottom = waist_z + overlap_cm

    for i0, i1, i2 in triangles:
        if i0 >= len(vertices) or i1 >= len(vertices) or i2 >= len(vertices):
            continue
        cz = (vertices[i0][2] + vertices[i1][2] + vertices[i2][2]) / 3.0
        if cz >= z_cutoff_top:
            top_triangles.append((i0, i1, i2))
        if cz <= z_cutoff_bottom:
            bottom_triangles.append((i0, i1, i2))

    result = {
        "top_triangles": top_triangles,
        "bottom_triangles": bottom_triangles,
        "top_count": len(top_triangles),
        "bottom_count": len(bottom_triangles),
        "waist_z": waist_z,
        "overlap_cm": overlap_cm,
    }
    return result


def weld_mesh_vertices(
    vertices: list[tuple[float, float, float]],
    triangles: list[tuple[int, int, int]],
    tolerance: float = 0.05,
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int]]]:
    """Suelda vértices coincidentes o dentro de tolerance y elimina triángulos degenerados.

    @spec REQ-81
    """
    new_vertices: list[tuple[float, float, float]] = []
    new_triangles: list[tuple[int, int, int]] = []

    if vertices:
        tol = max(tolerance, 1e-6)
        inv_tol = 1.0 / tol
        tol_sq = tol * tol
        grid: dict[tuple[int, int, int], list[int]] = {}
        remap: list[int] = [0] * len(vertices)

        for orig_idx, (vx, vy, vz) in enumerate(vertices):
            gx = int(math.floor(vx * inv_tol))
            gy = int(math.floor(vy * inv_tol))
            gz = int(math.floor(vz * inv_tol))
            found_idx = -1

            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    for dz in (-1, 0, 1):
                        cell = (gx + dx, gy + dy, gz + dz)
                        if cell in grid:
                            for candidate in grid[cell]:
                                cx, cy, cz = new_vertices[candidate]
                                dist_sq = (vx - cx) ** 2 + (vy - cy) ** 2 + (vz - cz) ** 2
                                if dist_sq <= tol_sq:
                                    found_idx = candidate
                                    break
                        if found_idx >= 0:
                            break
                    if found_idx >= 0:
                        break
                if found_idx >= 0:
                    break

            if found_idx >= 0:
                remap[orig_idx] = found_idx
            else:
                new_idx = len(new_vertices)
                new_vertices.append((vx, vy, vz))
                cell_key = (gx, gy, gz)
                if cell_key not in grid:
                    grid[cell_key] = []
                grid[cell_key].append(new_idx)
                remap[orig_idx] = new_idx

        for i0, i1, i2 in triangles:
            if i0 < len(remap) and i1 < len(remap) and i2 < len(remap):
                r0 = remap[i0]
                r1 = remap[i1]
                r2 = remap[i2]
                if r0 != r1 and r1 != r2 and r0 != r2:
                    new_triangles.append((r0, r1, r2))

    return (new_vertices, new_triangles)


def weld_top_dynamic_mesh(
    dyn_mesh: Any,
    tolerance: float = 0.05,
    fill_holes: bool = False,
) -> Any:
    """Aplica soldadura de bordes a una UDynamicMesh de prenda top.

    Reduce drásticamente los bordes abiertos residuales de la importación y sella costuras (REQ-81).
    @spec REQ-81
    """
    out_mesh = dyn_mesh
    if HAS_UNREAL and dyn_mesh is not None:
        repair = getattr(unreal, "GeometryScript_MeshRepair", None)
        if repair is not None and hasattr(repair, "weld_mesh_edges"):
            try:
                weld_opts = unreal.GeometryScriptWeldEdgesOptions()
                weld_opts.tolerance = float(tolerance)
                out_mesh = repair.weld_mesh_edges(dyn_mesh, weld_opts)
                if hasattr(repair, "compact_mesh"):
                    out_mesh = repair.compact_mesh(out_mesh)
            except Exception:  # noqa: BLE001
                out_mesh = dyn_mesh

        if fill_holes and repair is not None and hasattr(repair, "fill_all_mesh_holes"):
            try:
                fill_opts = unreal.GeometryScriptFillHolesOptions()
                out_mesh, _ = repair.fill_all_mesh_holes(out_mesh, fill_opts)
                if hasattr(repair, "compact_mesh"):
                    out_mesh = repair.compact_mesh(out_mesh)
            except Exception:  # noqa: BLE001
                pass

        normals = getattr(unreal, "GeometryScript_Normals", None)
        if normals is not None and hasattr(normals, "recompute_normals"):
            try:
                out_mesh = normals.recompute_normals(out_mesh, unreal.GeometryScriptCalculateNormalsOptions())
            except Exception:  # noqa: BLE001
                pass
    return out_mesh


def apply_two_sided_materials(asset_or_actor: Any) -> list[str]:
    """Configura override_two_sided y two_sided a True en todos los materiales de la prenda.

    Evita caras transparentes en cuellos, mangas, dobladillos o vistas traseras (REQ-81).
    @spec REQ-81
    """
    modified: list[str] = []
    if HAS_UNREAL and asset_or_actor is not None:
        target_mats: list[Any] = []

        if hasattr(asset_or_actor, "get_editor_property"):
            try:
                mats = asset_or_actor.get_editor_property("materials")
                if mats:
                    for m in mats:
                        mi = getattr(m, "material_interface", None)
                        if mi:
                            target_mats.append(mi)
            except Exception:  # noqa: BLE001
                pass

            try:
                sm_mats = asset_or_actor.get_editor_property("static_materials")
                if sm_mats:
                    for sm in sm_mats:
                        mi = getattr(sm, "material_interface", None)
                        if mi:
                            target_mats.append(mi)
            except Exception:  # noqa: BLE001
                pass

        if hasattr(asset_or_actor, "get_materials"):
            try:
                for mi in asset_or_actor.get_materials():
                    if mi:
                        target_mats.append(mi)
            except Exception:  # noqa: BLE001
                pass
        elif hasattr(asset_or_actor, "get_components_by_class"):
            try:
                sk_cls = getattr(unreal, "SkeletalMeshComponent", None)
                if sk_cls:
                    for comp in asset_or_actor.get_components_by_class(sk_cls):
                        num_m = comp.get_num_materials() if hasattr(comp, "get_num_materials") else 0
                        for idx in range(num_m):
                            mi = comp.get_material(idx)
                            if mi:
                                target_mats.append(mi)
            except Exception:  # noqa: BLE001
                pass

        seen: set[str] = set()
        for mat in target_mats:
            path = mat.get_path_name() if hasattr(mat, "get_path_name") else str(mat)
            if path in seen:
                continue
            seen.add(path)
            changed = False

            if hasattr(mat, "get_editor_property") and hasattr(mat, "set_editor_property"):
                try:
                    bpo = mat.get_editor_property("base_property_overrides")
                    if bpo is not None and hasattr(bpo, "set_editor_property"):
                        bpo.set_editor_property("override_two_sided", True)
                        bpo.set_editor_property("two_sided", True)
                        mat.set_editor_property("base_property_overrides", bpo)
                        changed = True
                except Exception:  # noqa: BLE001
                    pass

                if not changed:
                    try:
                        mat.set_editor_property("two_sided", True)
                        changed = True
                    except Exception:  # noqa: BLE001
                        pass

            if changed:
                eal = getattr(unreal, "EditorAssetLibrary", None)
                if eal and hasattr(eal, "save_loaded_asset"):
                    try:
                        eal.save_loaded_asset(mat, False)
                    except Exception:  # noqa: BLE001
                        pass
                modified.append(path)
    return modified


def setup_garment_lods(skm_asset: Any, target_lod_count: int = 4) -> dict[str, Any]:
    """Regenera los niveles de LOD del SkeletalMesh de la prenda para igualar los 4 LOD del cuerpo MetaHuman.

    @spec REQ-81
    """
    res: dict[str, Any] = {
        "success": False,
        "asset": skm_asset.get_path_name() if hasattr(skm_asset, "get_path_name") else str(skm_asset),
        "target_lods": target_lod_count,
        "lod_count": 1,
    }
    if HAS_UNREAL and skm_asset is not None:
        sub_cls = getattr(unreal, "SkeletalMeshEditorSubsystem", None)
        sub = unreal.get_editor_subsystem(sub_cls) if sub_cls and hasattr(unreal, "get_editor_subsystem") else None
        if sub and hasattr(sub, "get_lod_count"):
            try:
                curr_lods = int(sub.get_lod_count(skm_asset))
                res["lod_count"] = curr_lods
                if curr_lods < target_lod_count and hasattr(sub, "regenerate_lod"):
                    ok = bool(sub.regenerate_lod(skm_asset, new_lod_count=target_lod_count))
                    if ok:
                        res["lod_count"] = int(sub.get_lod_count(skm_asset))
                        res["success"] = True
                        eal = getattr(unreal, "EditorAssetLibrary", None)
                        if eal and hasattr(eal, "save_loaded_asset"):
                            eal.save_loaded_asset(skm_asset, False)
                    else:
                        res["error"] = "regenerate_lod returned false"
                else:
                    res["success"] = True
            except Exception as exc:  # noqa: BLE001
                res["error"] = str(exc)
        else:
            res["success"] = True
            res["note"] = "SkeletalMeshEditorSubsystem not available (mock/headless environment)"
    else:
        res["success"] = True
        res["note"] = "Mock environment: LOD setup simulated"
    return res


