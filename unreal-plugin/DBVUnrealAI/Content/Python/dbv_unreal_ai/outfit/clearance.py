# =============================================================================
# DBV Unreal AI — Holgura real entre prenda y cuerpo (distancia con signo)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Mide la holgura de una prenda contra la piel, vértice a vértice.

Sustituye a la estimación por cajas envolventes (REJECTED-13): la distancia con
signo de cada vértice de la prenda al punto más cercano del cuerpo dice cuánto
se separa (positivo) o cuánto se mete en la piel (negativo), y un rayo desde la
piel hacia fuera dice si la prenda tapa la zona que debería cubrir.

Se mide siempre contra el cuerpo **completo** de la complexión: contra el
recortado que se dibuja, el punto más cercano puede caer en un brazo y el signo
no significa nada (medido el 2026-11-10 con la camisa de fábrica de Pablo). Una
penetración solo cuenta si la piel de ese punto se dibuja: la ropa de fábrica de
MetaHuman se mete a propósito en la piel que el cuerpo recortado ya no tiene.

La parte de cálculo es Python puro y trabaja con «consultas» (``signed_distance``,
``ray_hit_distance`` y ``unsigned_distance``), para poder probarla con geometría
analítica. El adaptador de Unreal envuelve el BVH de GeometryScript.

@spec REQ-89
"""

from __future__ import annotations

from typing import Any, Callable, Protocol

try:
    import unreal

    HAS_UNREAL = True
except ImportError:
    HAS_UNREAL = False
    unreal = None  # type: ignore[assignment]

Point = tuple[float, float, float]

# Un vértice cuenta como «dentro de la piel» por debajo de esta distancia (cm).
INSIDE_TOLERANCE_CM = 0.2
# Porcentaje de vértices dentro de la piel que da aviso y que da error.
INSIDE_WARN_PCT = 0.2
INSIDE_ERROR_PCT = 2.0
# Porcentaje de piel destapada dentro de la zona que da aviso y que da error.
BARE_WARN_PCT = 1.0
BARE_ERROR_PCT = 3.0
# Un rayo desde la piel que no encuentra prenda a menos de esto, la deja destapada.
MAX_GAP_CM = 8.0
# No se da por tapada la piel que solo tiene tela «cerca»: con esa regla el hueco de la
# entrepierna de los shorts del 2026-11-10 bajaba del 10,2 % al 2,2 % y pasaba como aviso.
# Distancia a la que un punto del cuerpo completo se considera piel que se dibuja.
VISIBLE_SKIN_TOLERANCE_CM = 0.05
# El rayo empieza un poco por dentro, para ver también la prenda metida en la piel.
RAY_BACKOFF_CM = 2.0

# Familias de huesos de la zona que cubre cada tipo cuando no hay zona explícita.
TEMPLATE_FAMILIES = {
    "top": ("spine", "clavicle"),
    "pants": ("pelvis", "thigh"),
}
# En la plantilla de ``top`` se deja fuera la franja alta (escote y sisas abiertos).
TOP_TEMPLATE_NECK_BAND_CM = 8.0
BONE_FAMILIES = ("spine", "pelvis", "clavicle", "thigh", "calf", "upperarm", "lowerarm", "neck", "hand", "foot", "head")


class MeshQuery(Protocol):
    """Consultas espaciales sobre una malla."""

    def signed_distance(self, point: Point) -> tuple[float, bool] | None:
        """``(distancia con signo, piel_visible)`` al punto más cercano; ``None`` si no se puede medir."""

    def ray_hit_distance(self, origin: Point, direction: Point) -> float | None:
        """Distancia al primer corte del rayo con la malla; ``None`` si no corta."""

    def unsigned_distance(self, point: Point) -> float | None:
        """Distancia sin signo al punto más cercano; ``None`` si no se puede medir."""


def _percentile(sorted_values: list[float], fraction: float) -> float:
    """Percentil por posición en una lista ya ordenada (no vacía)."""
    index = min(len(sorted_values) - 1, max(0, int(round(fraction * (len(sorted_values) - 1)))))
    return sorted_values[index]


def clearance_stats(distances: list[float], hidden_vertices: int = 0) -> dict[str, Any]:
    """Resumen de las distancias con signo de la prenda a la piel que se dibuja. @spec REQ-89

    ``hidden_vertices`` son los vértices sobre piel que no se dibuja: no cuentan para la
    penetración, pero sí demuestran que la medida se hizo.
    """
    values = sorted(distances)
    stats: dict[str, Any] = {"measured_vertices": len(values), "over_hidden_skin_vertices": hidden_vertices}
    if values:
        inside = sum(1 for v in values if v < -INSIDE_TOLERANCE_CM)
        stats.update(
            {
                "min_cm": round(values[0], 2),
                "median_cm": round(_percentile(values, 0.5), 2),
                "p95_cm": round(_percentile(values, 0.95), 2),
                "inside_vertices": inside,
                "inside_pct": round(100.0 * inside / len(values), 2),
            }
        )
    return stats


def bare_skin(zone: list[tuple[Point, Point]], garment: MeshQuery, max_gap_cm: float = MAX_GAP_CM) -> dict[str, Any]:
    """Piel de la zona que la prenda no tapa: rayo desde cada punto de piel por su normal. @spec REQ-89"""
    bare_points: list[Point] = []
    for point, normal in zone:
        origin = (
            point[0] - normal[0] * RAY_BACKOFF_CM,
            point[1] - normal[1] * RAY_BACKOFF_CM,
            point[2] - normal[2] * RAY_BACKOFF_CM,
        )
        hit = garment.ray_hit_distance(origin, normal)
        covered = hit is not None and hit <= max_gap_cm + RAY_BACKOFF_CM
        if not covered:
            bare_points.append(point)
    total = len(zone)
    result: dict[str, Any] = {
        "zone_vertices": total,
        "bare_vertices": len(bare_points),
        "bare_pct": round(100.0 * len(bare_points) / total, 2) if total else 0.0,
        "bare_regions": _bare_regions(bare_points),
    }
    return result


def _bare_regions(points: list[Point]) -> list[dict[str, Any]]:
    """Agrupa la piel destapada por lado, cara y franja de 10 cm, para explicarla en el informe."""
    counts: dict[tuple[str, str, int], int] = {}
    for x, y, z in points:
        side = "izquierda" if x > 3.0 else ("derecha" if x < -3.0 else "centro")
        face = "delante" if y > 0.0 else "detrás"
        key = (side, face, int(z // 10) * 10)
        counts[key] = counts.get(key, 0) + 1
    ordered = sorted(counts.items(), key=lambda item: -item[1])[:6]
    regions = [{"side": k[0], "face": k[1], "z_cm": k[2], "vertices": n} for k, n in ordered]
    return regions


def grade(stats: dict[str, Any], bare: dict[str, Any] | None) -> tuple[str, list[str]]:
    """Convierte medidas en ``ok``, ``con_avisos`` o ``error`` con motivos legibles. @spec REQ-89"""
    warnings: list[str] = []
    result = "ok"
    if not stats.get("measured_vertices") and not stats.get("over_hidden_skin_vertices"):
        result = "error"
        warnings.append("No se pudo medir la holgura: la prenda no tiene vértices medibles contra el cuerpo.")
    else:
        inside_pct = float(stats.get("inside_pct", 0.0))
        if inside_pct >= INSIDE_ERROR_PCT:
            result = "error"
            warnings.append(
                f"El {inside_pct:.1f} % de la prenda está dentro de la piel (hasta {abs(stats['min_cm']):.1f} cm): el cuerpo la atraviesa."
            )
        elif inside_pct >= INSIDE_WARN_PCT:
            result = "con_avisos"
            warnings.append(f"El {inside_pct:.1f} % de la prenda roza o entra en la piel (mínimo {stats['min_cm']:.1f} cm).")
        bare_pct = float(bare.get("bare_pct", 0.0)) if bare else 0.0
        if bare and bare_pct >= BARE_ERROR_PCT:
            result = "error"
            where = ", ".join(f"{r['side']} {r['face']} a {r['z_cm']} cm" for r in bare["bare_regions"][:3])
            warnings.append(f"Queda piel al descubierto dentro de la prenda ({bare_pct:.1f} % de la zona): {where}.")
        elif bare and bare_pct >= BARE_WARN_PCT:
            result = "con_avisos" if result == "ok" else result
            warnings.append(f"Hay pequeñas zonas de piel sin tapar dentro de la prenda ({bare_pct:.1f} % de la zona).")
    return (result, warnings)


def measure_signed_clearance(
    garment_points: list[Point],
    body: MeshQuery,
    zone: list[tuple[Point, Point]] | None = None,
    garment: MeshQuery | None = None,
) -> dict[str, Any]:
    """Distancia con signo de la prenda a la piel y piel destapada en la zona. @spec REQ-89"""
    distances: list[float] = []
    hidden = 0
    for point in garment_points:
        measured = body.signed_distance(point)
        if measured is not None and measured[1]:
            distances.append(measured[0])
        elif measured is not None:
            hidden += 1
    stats = clearance_stats(distances, hidden)
    bare = bare_skin(zone, garment) if zone and garment is not None else None
    result, warnings = grade(stats, bare)
    report: dict[str, Any] = {
        "result": result,
        "ok": result == "ok",
        "method": "signed_distance",
        "clearance": stats,
        "bare_skin": bare,
        "warnings": warnings,
    }
    return report


def bone_family(bone_name: str) -> str:
    """Familia anatómica de un hueso de MetaHuman (``spine_04`` → ``spine``)."""
    lowered = bone_name.lower()
    family = next((f for f in BONE_FAMILIES if lowered.startswith(f)), "other")
    return family


def template_zone(
    samples: list[tuple[Point, Point, str]],
    garment_kind: str,
    z_range: tuple[float, float],
) -> list[tuple[Point, Point]]:
    """Zona de piel que debería tapar una prenda sin zona explícita, por familia de huesos y altura.

    ``samples`` son ``(punto, normal, familia_del_hueso_dominante)``. En ``top`` se deja
    fuera la franja del escote y las sisas, que puede estar abierta a propósito.
    @spec REQ-89
    """
    families = TEMPLATE_FAMILIES.get(garment_kind, ())
    z_low = z_range[0] + 1.0
    z_high = z_range[1] - (TOP_TEMPLATE_NECK_BAND_CM if garment_kind == "top" else 1.0)
    zone = [(p, n) for p, n, fam in samples if fam in families and z_low <= p[2] <= z_high]
    return zone


class UnrealMeshQuery:
    """Consultas sobre un ``DynamicMesh`` con el BVH de GeometryScript.

    Con ``drawn`` (otra ``UnrealMeshQuery`` del cuerpo que se dibuja), ``signed_distance``
    dice además si el triángulo más cercano es piel que se ve.
    """

    def __init__(self, dyn_mesh: Any, drawn: "UnrealMeshQuery | None" = None) -> None:
        spatial = unreal.GeometryScript_MeshSpatial
        self._mesh, self._bvh = spatial.build_bvh_for_mesh(dyn_mesh)
        self._options = unreal.GeometryScriptSpatialQueryOptions()
        self._drawn = drawn
        self._visible: dict[int, bool] = {}

    def _nearest(self, point: Point) -> Any:
        """Punto más cercano de la malla (``GeometryScriptTrianglePoint``)."""
        query = unreal.Vector(point[0], point[1], point[2])
        _, nearest, _ = unreal.GeometryScript_MeshSpatial.find_nearest_point_on_mesh(self._mesh, self._bvh, query, self._options)
        return nearest

    def _triangle_visible(self, triangle_id: int) -> bool:
        """El triángulo del cuerpo completo existe en el cuerpo que se dibuja (se mira su centro)."""
        if triangle_id not in self._visible:
            visible = True
            if self._drawn is not None:
                positions = unreal.GeometryScript_MeshQueries.get_triangle_positions(self._mesh, triangle_id)
                a, b, c = positions[1], positions[2], positions[3]
                centre = ((a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0, (a.z + b.z + c.z) / 3.0)
                gap = self._drawn.unsigned_distance(centre)
                visible = gap is not None and gap <= VISIBLE_SKIN_TOLERANCE_CM
            self._visible[triangle_id] = visible
        return self._visible[triangle_id]

    def signed_distance(self, point: Point) -> tuple[float, bool] | None:
        """Distancia con signo por la normal de la cara más cercana, y si esa piel se dibuja."""
        value: tuple[float, bool] | None = None
        nearest = self._nearest(point)
        if nearest.valid:
            normal = unreal.GeometryScript_MeshQueries.get_triangle_face_normal(self._mesh, nearest.triangle_id)
            normal = normal[0] if isinstance(normal, tuple) else normal
            dx = point[0] - nearest.position.x
            dy = point[1] - nearest.position.y
            dz = point[2] - nearest.position.z
            value = (float(dx * normal.x + dy * normal.y + dz * normal.z), self._triangle_visible(int(nearest.triangle_id)))
        return value

    def unsigned_distance(self, point: Point) -> float | None:
        """Distancia sin signo al punto más cercano."""
        nearest = self._nearest(point)
        distance = None
        if nearest.valid:
            dx = point[0] - nearest.position.x
            dy = point[1] - nearest.position.y
            dz = point[2] - nearest.position.z
            distance = float((dx * dx + dy * dy + dz * dz) ** 0.5)
        return distance

    def point_visible(self, point: Point) -> bool:
        """El punto del cuerpo completo es piel que se dibuja."""
        gap = self._drawn.unsigned_distance(point) if self._drawn is not None else 0.0
        visible = gap is not None and gap <= VISIBLE_SKIN_TOLERANCE_CM
        return visible

    def ray_hit_distance(self, origin: Point, direction: Point) -> float | None:
        """Distancia al primer corte del rayo, o ``None``."""
        spatial = unreal.GeometryScript_MeshSpatial
        _, hit, _ = spatial.find_nearest_ray_intersection_with_mesh(
            self._mesh,
            self._bvh,
            unreal.Vector(origin[0], origin[1], origin[2]),
            unreal.Vector(direction[0], direction[1], direction[2]),
            self._options,
        )
        distance = float(hit.ray_parameter) if hit.hit else None
        return distance


def dyn_from_skeletal_mesh(skm: Any) -> Any:
    """Copia LOD0 de una malla esquelética a un ``DynamicMesh``, en el espacio del asset."""
    dyn = unreal.DynamicMesh()
    dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_skeletal_mesh(
        skm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD()
    )
    return dyn


def vertex_ids(dyn: Any) -> list[int]:
    """Identificadores de vértice válidos (la malla puede tener huecos de IDs)."""
    queries = unreal.GeometryScript_MeshQueries
    ids: list[int] = list(range(int(dyn.get_vertex_count())))
    if hasattr(queries, "get_all_vertex_i_ds"):
        raw = queries.get_all_vertex_i_ds(dyn)
        raw = raw[1] if isinstance(raw, tuple) else raw
        if hasattr(unreal, "GeometryScript_List") and hasattr(unreal.GeometryScript_List, "convert_index_list_to_array"):
            ids = [int(i) for i in unreal.GeometryScript_List.convert_index_list_to_array(raw)]
    return ids


def vertex_points(dyn: Any) -> list[Point]:
    """Posiciones de todos los vértices válidos."""
    queries = unreal.GeometryScript_MeshQueries
    points: list[Point] = []
    for i in vertex_ids(dyn):
        p, ok = queries.get_vertex_position(dyn, i)
        if ok:
            points.append((float(p.x), float(p.y), float(p.z)))
    return points


def body_samples(body_dyn: Any, family_of: Callable[[str], str] = bone_family) -> list[tuple[Point, Point, str]]:
    """``(punto, normal, familia)`` de cada vértice del cuerpo, con el hueso de más peso."""
    queries = unreal.GeometryScript_MeshQueries
    bones = unreal.GeometryScript_BoneWeights
    _, info = bones.get_all_bones_info(body_dyn)
    names = [str(b.name) for b in info]
    _, normal_list, _, _ = unreal.GeometryScript_Normals.get_mesh_per_vertex_normals(body_dyn)
    normals = list(unreal.GeometryScript_List.convert_vector_list_to_array(normal_list))
    samples: list[tuple[Point, Point, str]] = []
    for i in vertex_ids(body_dyn):
        p, ok = queries.get_vertex_position(body_dyn, i)
        _, weight, has_weight = bones.get_largest_vertex_bone_weight(body_dyn, i)
        if ok and has_weight and i < len(normals) and 0 <= weight.bone_index < len(names):
            n = normals[i]
            samples.append(((float(p.x), float(p.y), float(p.z)), (float(n.x), float(n.y), float(n.z)), family_of(names[weight.bone_index])))
    return samples


def measure_assets(
    garment_skm: Any,
    drawn_body_skm: Any,
    full_body_skm: Any,
    garment_kind: str,
    zone: list[tuple[Point, Point]] | None = None,
) -> dict[str, Any]:
    """Mide una prenda equipada contra el cuerpo completo, contando solo la piel que se dibuja.

    Si no hay zona explícita (la carcasa de REQ-98 la da), se usa la plantilla por tipo.
    La zona se queda con la piel que se dibuja: lo que el cuerpo recortado no tiene no
    puede verse por un hueco.
    @spec REQ-89
    """
    garment_dyn = dyn_from_skeletal_mesh(garment_skm)
    full_dyn = dyn_from_skeletal_mesh(full_body_skm)
    same_body = drawn_body_skm is full_body_skm or (
        hasattr(drawn_body_skm, "get_path_name") and drawn_body_skm.get_path_name() == full_body_skm.get_path_name()
    )
    drawn_q = None if same_body else UnrealMeshQuery(dyn_from_skeletal_mesh(drawn_body_skm))
    garment_q = UnrealMeshQuery(garment_dyn)
    body_q = UnrealMeshQuery(full_dyn, drawn=drawn_q)
    chosen_zone = zone
    if chosen_zone is None and garment_kind in TEMPLATE_FAMILIES:
        box = unreal.GeometryScript_MeshQueries.get_mesh_bounding_box(garment_dyn)
        chosen_zone = template_zone(body_samples(full_dyn), garment_kind, (float(box.min.z), float(box.max.z)))
    if chosen_zone:
        chosen_zone = [(p, n) for p, n in chosen_zone if body_q.point_visible(p)]
    report = measure_signed_clearance(vertex_points(garment_dyn), body_q, chosen_zone, garment_q)
    report["zone_source"] = "explicit" if zone is not None else ("template" if chosen_zone else "none")
    return report
