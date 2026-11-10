# =============================================================================
# DBV Unreal AI — Prendas ceñidas como carcasa del cuerpo
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Construye tops y pantalones a partir de la malla del cuerpo del personaje.

La prenda de Tripo no se deforma (REJECTED-13): se coloca sobre el cuerpo con una
sola escala y posición, decide qué piel tapa, y de esa piel sale la prenda,
separada por su normal. Al borrar triángulos de una copia del cuerpo se conservan
UV y pesos, así que la carcasa hereda topología, UV y huesos del cuerpo completo
(REJECTED-12). El color y el relieve se hornean desde el modelo de Tripo.

Lecciones del spike S116a (2026-11-10, UE 5.8):

- Las UV del cuerpo de MetaHuman están en la segunda casilla (U de 1 a 2); el
  horneado solo escribe en 0..1, así que se desplazan.
- El cuerpo trae colores de vértice de máscaras y el material glTF los multiplica:
  se pintan de blanco.
- ``bake_texture`` devuelve texturas transitorias: se guardan dibujándolas en un
  render target y creando la textura estática desde él (P-28). ``export_texture2d``
  escribe HDR y ``duplicate_loaded_asset`` no acepta transitorios.
- Color y normal se hornean en llamadas separadas.

@spec REQ-98
"""

from __future__ import annotations

import math
from typing import Any

try:
    import unreal

    HAS_UNREAL = True
except ImportError:
    HAS_UNREAL = False
    unreal = None  # type: ignore[assignment]

from .clearance import bone_family

# Holgura por defecto (cm): el top se separa más que el pantalón para quedar por fuera
# en la cintura en cualquier orden de equipado (REQ-98, paso 7).
DEFAULT_OFFSET_CM = {"top": 1.0, "pants": 0.6}
MIN_OFFSET_CM = 0.3
MAX_OFFSET_CM = 3.0
# El top queda un poco más ancho que el pecho; el pantalón, que la cadera.
WIDTH_EASE = {"top": 1.06, "pants": 1.04}
# Rayo desde la piel: empieza por dentro y busca prenda hasta esta distancia.
RAY_BACKOFF_CM = 3.0
RAY_REACH_CM = 13.0
# Por encima de esta holgura mediana la prenda era holgada y pierde volumen (REQ-98, paso 8).
LOOSE_MEDIAN_CM = 4.0
# Si la prenda tapa menos de esta parte de la parte alta del brazo, es sin mangas.
SLEEVE_MIN_RATIO = 0.3
SMOOTH_PASSES = 2
# Hasta dónde busca el horneado la prenda de Tripo desde la carcasa. Con 12 cm, la piel de la
# cadera que el pantalón tapa siempre (entrepierna) se quedaba negra (2026-11-10).
BAKE_PROJECTION_CM = 20.0
TEXTURE_SIZE = 2048
FLAT_NORMAL = "/Engine/EngineMaterials/FlatNormal"

KIND_FAMILIES = {
    # Un top largo llega a la cadera, cuya piel pesa sobre todo en pelvis.
    "top": ("spine", "clavicle", "pelvis"),
    "pants": ("pelvis", "thigh", "calf"),
}
SLEEVE_FAMILIES = ("upperarm", "lowerarm")
# La entrepierna siempre queda tapada en un pantalón (REQ-98, paso 2).
FORCED_FAMILIES = {"pants": ("pelvis",)}


# ---------------------------------------------------------------------------
# Parte pura (sin Unreal)
# ---------------------------------------------------------------------------


def orientation(band_width_x: float, band_width_y: float, centre_top_plus_y: float, centre_top_minus_y: float) -> dict[str, bool]:
    """Cómo girar la prenda para que su ancho vaya en X y su frente mire a +Y.

    El lado con el borde superior central más alto es la espalda: el escote es más
    bajo delante en un top y el tiro es más alto detrás en un pantalón. Las alturas se
    miden ya con la prenda girada si ``rotate_90`` es verdad.
    @spec REQ-98
    """
    plan = {
        "rotate_90": band_width_y > band_width_x,
        "flip_180": centre_top_plus_y > centre_top_minus_y,
    }
    return plan


def fit_scale(garment_width: float, body_width: float, kind: str) -> float:
    """Escala uniforme para que la prenda quede algo más ancha que el cuerpo. @spec REQ-98"""
    ease = WIDTH_EASE.get(kind, 1.05)
    scale = body_width * ease / garment_width if garment_width > 0.0 else 1.0
    return scale


def shell_offset(kind: str, clearance_cm: float | None = None) -> float:
    """Separación de la carcasa respecto a la piel, acotada. @spec REQ-98, REQ-88"""
    wanted = DEFAULT_OFFSET_CM.get(kind, 1.0) if clearance_cm is None else float(clearance_cm)
    offset = min(MAX_OFFSET_CM, max(MIN_OFFSET_CM, wanted))
    return offset


def eligible_families(kind: str, sleeved: bool) -> tuple[str, ...]:
    """Familias de huesos cuya piel puede tapar la prenda. @spec REQ-98"""
    families = KIND_FAMILIES.get(kind, ())
    if kind == "top" and sleeved:
        families = families + SLEEVE_FAMILIES
    return families


def majority_smooth(covered: list[bool], neighbours: list[set[int]], passes: int = SMOOTH_PASSES) -> list[bool]:
    """Suaviza la zona: un vértice queda tapado si lo están la mayoría de sus vecinos."""
    current = list(covered)
    for _ in range(passes):
        nxt = list(current)
        for i, around in enumerate(neighbours):
            if around:
                hits = sum(1 for j in around if current[j])
                nxt[i] = hits * 2 > len(around) or (current[i] and hits * 2 == len(around))
        current = nxt
    return current


def mirror_zone(covered: list[bool], mirror_of: list[tuple[int, ...]]) -> list[bool]:
    """Zona simétrica en X: tapado si lo está él o su espejo."""
    result = list(covered)
    for i, flag in enumerate(covered):
        if flag:
            for j in mirror_of[i]:
                result[j] = True
    return result


def looseness(hit_distances: list[float]) -> dict[str, Any]:
    """Holgura mediana de la prenda colocada y aviso si era holgada. @spec REQ-98"""
    ordered = sorted(hit_distances)
    median = ordered[len(ordered) // 2] if ordered else 0.0
    report: dict[str, Any] = {"median_cm": round(median, 2), "loose": median > LOOSE_MEDIAN_CM}
    return report


def interior(covered: list[bool], neighbours: list[set[int]], rings: int = 2) -> list[bool]:
    """Vértices tapados lejos del borde de la zona (para ocultar la piel sin que asome)."""
    inner = list(covered)
    for _ in range(rings):
        inner = [inner[i] and all(inner[j] for j in neighbours[i]) for i in range(len(inner))]
    return inner


# ---------------------------------------------------------------------------
# Adaptador de Unreal
# ---------------------------------------------------------------------------


def _first(value: Any) -> Any:
    """Las funciones de GeometryScript devuelven a veces ``(malla, extra)``."""
    return value[0] if isinstance(value, tuple) else value


class BodyModel:
    """Cuerpo completo en ``DynamicMesh`` con posición, normal, familia y vecinos por vértice."""

    def __init__(self, full_skm: Any) -> None:
        from .clearance import dyn_from_skeletal_mesh

        queries = unreal.GeometryScript_MeshQueries
        bones = unreal.GeometryScript_BoneWeights
        self.skm = full_skm
        self.dyn = dyn_from_skeletal_mesh(full_skm)
        _, info = bones.get_all_bones_info(self.dyn)
        names = [str(b.name) for b in info]
        count = int(self.dyn.get_vertex_count())
        _, normal_list, _, _ = unreal.GeometryScript_Normals.get_mesh_per_vertex_normals(self.dyn)
        normals = list(unreal.GeometryScript_List.convert_vector_list_to_array(normal_list))
        self.pos: list[tuple[float, float, float]] = []
        self.normal: list[tuple[float, float, float]] = []
        self.family: list[str] = []
        for i in range(count):
            p, _ = queries.get_vertex_position(self.dyn, i)
            n = normals[i]
            _, weight, ok = bones.get_largest_vertex_bone_weight(self.dyn, i)
            self.pos.append((float(p.x), float(p.y), float(p.z)))
            self.normal.append((float(n.x), float(n.y), float(n.z)))
            self.family.append(bone_family(names[weight.bone_index]) if ok and 0 <= weight.bone_index < len(names) else "other")
        self.tris: list[tuple[int, int, int]] = []
        for t in range(int(self.dyn.get_triangle_count())):
            idx, _ = queries.get_triangle_indices(self.dyn, t)
            self.tris.append((int(idx.x), int(idx.y), int(idx.z)))
        self.neighbours: list[set[int]] = [set() for _ in range(count)]
        for a, b, c in self.tris:
            self.neighbours[a].update((b, c))
            self.neighbours[b].update((a, c))
            self.neighbours[c].update((a, b))

    def landmarks(self, kind: str) -> dict[str, float]:
        """Altura de arranque, ancho de referencia y centro en Y para colocar la prenda."""
        torso = [p for p, f in zip(self.pos, self.family) if f in ("spine", "clavicle")]
        if kind == "top":
            top_z = max(p[2] for p in torso if 8.0 < abs(p[0]) < 14.0)
            ref_z = top_z - 20.0
            band = [p for p in torso if abs(p[2] - ref_z) < 2.0]
        else:
            pelvis = [p for p, f in zip(self.pos, self.family) if f == "pelvis"]
            top_z = max(p[2] for p in pelvis) - 1.0
            hips = [p for p, f in zip(self.pos, self.family) if f in ("pelvis", "thigh")]
            ref_z = max(((z, _width_at(hips, z)) for z in _frange(top_z - 25.0, top_z - 5.0, 1.0)), key=lambda zw: zw[1])[0]
            band = [p for p in hips if abs(p[2] - ref_z) < 2.0]
        marks = {
            "top_z": top_z,
            "ref_z": ref_z,
            "width": max(p[0] for p in band) - min(p[0] for p in band),
            "centre_y": (max(p[1] for p in band) + min(p[1] for p in band)) / 2.0,
        }
        return marks


def _frange(start: float, stop: float, step: float) -> list[float]:
    """Rango de floats [start, stop)."""
    count = max(0, int(math.ceil((stop - start) / step)))
    return [start + k * step for k in range(count)]


def _width_at(points: list[tuple[float, float, float]], z: float, half: float = 1.5) -> float:
    """Ancho en X de los puntos en una franja de altura."""
    xs = [p[0] for p in points if abs(p[2] - z) < half]
    width = (max(xs) - min(xs)) if xs else 0.0
    return width


def _mesh_points(dyn: Any) -> list[tuple[float, float, float]]:
    """Posiciones de los vértices válidos."""
    from .clearance import vertex_points

    return vertex_points(dyn)


def load_garment(garment_sm: Any) -> Any:
    """Copia la prenda de Tripo a un ``DynamicMesh`` centrado en el origen."""
    garment = unreal.DynamicMesh()
    garment, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(
        garment_sm, garment, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True
    )
    box = unreal.GeometryScript_MeshQueries.get_mesh_bounding_box(garment)
    garment = _first(unreal.GeometryScript_MeshTransforms.translate_mesh(garment, (box.min + box.max) * -0.5))
    return garment


def place_garment(garment: Any, body: BodyModel, kind: str) -> tuple[Any, dict[str, Any]]:
    """Gira, escala y coloca la prenda sobre el cuerpo con un solo cambio (REQ-98, paso 1). @spec REQ-98"""
    xform = unreal.GeometryScript_MeshTransforms
    points = _mesh_points(garment)
    zs = sorted(p[2] for p in points)
    band_z = zs[0] + (0.75 if kind == "top" else 0.8) * (zs[-1] - zs[0])
    band = [p for p in points if abs(p[2] - band_z) < 4.0]
    width_x = max(p[0] for p in band) - min(p[0] for p in band)
    width_y = max(p[1] for p in band) - min(p[1] for p in band)
    if width_y > width_x:
        garment = _first(xform.transform_mesh(garment, unreal.Transform(rotation=unreal.Rotator(0.0, 0.0, 90.0))))
        points = _mesh_points(garment)
    plus = max([p[2] for p in points if abs(p[0]) < 4.0 and p[1] > 0.0] or [0.0])
    minus = max([p[2] for p in points if abs(p[0]) < 4.0 and p[1] < 0.0] or [0.0])
    plan = orientation(width_x, width_y, plus, minus)
    if plan["flip_180"]:
        garment = _first(xform.transform_mesh(garment, unreal.Transform(rotation=unreal.Rotator(0.0, 0.0, 180.0))))
        points = _mesh_points(garment)
    marks = body.landmarks(kind)
    zs = sorted(p[2] for p in points)
    g_top = zs[-1]
    g_ref_z = g_top - (0.3 if kind == "top" else 0.2) * (zs[-1] - zs[0])
    g_width = _width_at(points, g_ref_z, 3.0)
    scale = fit_scale(g_width, marks["width"], kind)
    garment = _first(xform.scale_mesh(garment, unreal.Vector(scale, scale, scale), unreal.Vector(0.0, 0.0, 0.0)))
    lift = marks["top_z"] + 0.5 - g_top * scale
    garment = _first(xform.translate_mesh(garment, unreal.Vector(0.0, marks["centre_y"], lift)))
    box = unreal.GeometryScript_MeshQueries.get_mesh_bounding_box(garment)
    info: dict[str, Any] = {
        "rotated_90": plan["rotate_90"],
        "flipped_180": plan["flip_180"],
        "scale": round(scale, 4),
        "garment_z_cm": [round(float(box.min.z), 1), round(float(box.max.z), 1)],
        "landmarks": {k: round(v, 1) for k, v in marks.items()},
    }
    return (garment, info)


def covered_zone(body: BodyModel, garment: Any, kind: str) -> tuple[list[bool], dict[str, Any]]:
    """Piel que tapa la prenda colocada: rayo desde la piel por su normal (REQ-98, paso 2). @spec REQ-98"""
    spatial = unreal.GeometryScript_MeshSpatial
    garment, bvh = spatial.build_bvh_for_mesh(garment)
    options = unreal.GeometryScriptSpatialQueryOptions()
    box = unreal.GeometryScript_MeshQueries.get_mesh_bounding_box(garment)
    z_low, z_high = float(box.min.z) - 1.0, float(box.max.z) + 1.0
    candidates = set(KIND_FAMILIES.get(kind, ())) | (set(SLEEVE_FAMILIES) if kind == "top" else set())
    hit_at: list[float | None] = [None] * len(body.pos)
    for i, (p, n, fam) in enumerate(zip(body.pos, body.normal, body.family)):
        if fam in candidates and z_low <= p[2] <= z_high:
            origin = unreal.Vector(p[0] - n[0] * RAY_BACKOFF_CM, p[1] - n[1] * RAY_BACKOFF_CM, p[2] - n[2] * RAY_BACKOFF_CM)
            _, hit, _ = spatial.find_nearest_ray_intersection_with_mesh(garment, bvh, origin, unreal.Vector(n[0], n[1], n[2]), options)
            distance = float(hit.ray_parameter) - RAY_BACKOFF_CM if hit.hit else None
            hit_at[i] = distance if distance is not None and distance <= RAY_REACH_CM - RAY_BACKOFF_CM else None
    upper_arm = [i for i, f in enumerate(body.family) if f == "upperarm" and z_low <= body.pos[i][2] <= z_high]
    sleeve_ratio = (sum(1 for i in upper_arm if hit_at[i] is not None) / len(upper_arm)) if upper_arm else 0.0
    sleeved = kind == "top" and sleeve_ratio >= SLEEVE_MIN_RATIO
    families = set(eligible_families(kind, sleeved))
    forced = set(FORCED_FAMILIES.get(kind, ()))
    covered = [
        body.family[i] in families and (hit_at[i] is not None or (body.family[i] in forced and z_low <= body.pos[i][2] <= z_high))
        for i in range(len(body.pos))
    ]
    covered = mirror_zone(covered, _mirror_index(body))
    covered = majority_smooth(covered, body.neighbours)
    info: dict[str, Any] = {
        "covered_vertices": sum(covered),
        "sleeved": sleeved,
        "sleeve_ratio": round(sleeve_ratio, 2),
        "looseness": looseness([d for d in hit_at if d is not None]),
    }
    return (covered, info)


def _mirror_index(body: BodyModel) -> list[tuple[int, ...]]:
    """Para cada vértice, los del triángulo más cercano a su reflejo en X."""
    spatial = unreal.GeometryScript_MeshSpatial
    probe, bvh = spatial.build_bvh_for_mesh(body.dyn)
    options = unreal.GeometryScriptSpatialQueryOptions()
    mirror: list[tuple[int, ...]] = []
    for p in body.pos:
        _, nearest, _ = spatial.find_nearest_point_on_mesh(probe, bvh, unreal.Vector(-p[0], p[1], p[2]), options)
        mirror.append(body.tris[int(nearest.triangle_id)] if nearest.valid else ())
    return mirror


def build_shell(body: BodyModel, covered: list[bool], offset_cm: float) -> Any:
    """Copia de la piel tapada, separada por la normal, con UV en 0..1 y color de vértice blanco (pasos 3 y 5). @spec REQ-98"""
    from .clearance import dyn_from_skeletal_mesh

    shell = dyn_from_skeletal_mesh(body.skm)
    drop = [t for t, (a, b, c) in enumerate(body.tris) if not (covered[a] and covered[b] and covered[c])]
    triangle_list = unreal.GeometryScript_List.convert_array_to_index_list(drop, unreal.GeometryScriptIndexType.TRIANGLE)
    shell = _first(unreal.GeometryScript_MeshEdits.delete_triangles_from_mesh(shell, triangle_list))
    offset = unreal.GeometryScriptMeshOffsetOptions()
    offset.offset_distance = offset_cm
    shell = _first(unreal.GeometryScript_MeshModeling.apply_mesh_offset(shell, offset))
    uv_box = _first(unreal.GeometryScript_MeshQueries.get_uv_set_bounding_box(shell, 0))
    shift = unreal.Vector2D(-math.floor(uv_box.min.x), -math.floor(uv_box.min.y))
    if shift.x or shift.y:
        shell = _first(unreal.GeometryScript_UVs.translate_mesh_u_vs(shell, 0, shift, unreal.GeometryScriptMeshSelection()))
    shell = _first(
        unreal.GeometryScript_VertexColors.set_mesh_constant_vertex_color(shell, unreal.LinearColor(1.0, 1.0, 1.0, 1.0), unreal.GeometryScriptColorFlags())
    )
    return shell


def _save_texture(transient: Any, folder: str, name: str, linear: bool, normal_map: bool = False) -> Any:
    """Convierte una textura transitoria en asset: canvas -> render target -> textura (P-28).

    ``linear`` para datos que no son color (normal, metal y rugosidad); ``normal_map`` solo para el normal.
    """
    is_normal = linear
    eal = unreal.EditorAssetLibrary
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    rendering = unreal.RenderingLibrary
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    rt_name = f"RT_{name}"
    for path in (f"{folder}/{name}", f"{folder}/{rt_name}"):
        if eal.does_asset_exist(path):
            eal.delete_asset(path)
    target = tools.create_asset(rt_name, folder, unreal.TextureRenderTarget2D, unreal.TextureRenderTargetFactoryNew())
    target.set_editor_property("size_x", TEXTURE_SIZE)
    target.set_editor_property("size_y", TEXTURE_SIZE)
    target.set_editor_property(
        "render_target_format", unreal.TextureRenderTargetFormat.RTF_RGBA8 if is_normal else unreal.TextureRenderTargetFormat.RTF_RGBA8_SRGB
    )
    canvas, _, ctx = rendering.begin_draw_canvas_to_render_target(world, target)
    canvas.draw_texture(
        transient, unreal.Vector2D(0, 0), unreal.Vector2D(TEXTURE_SIZE, TEXTURE_SIZE), unreal.Vector2D(0, 0), unreal.Vector2D(1, 1),
        unreal.LinearColor(1.0, 1.0, 1.0, 1.0), unreal.BlendMode.BLEND_OPAQUE,
    )
    rendering.end_draw_canvas_to_render_target(world, ctx)
    compression = unreal.TextureCompressionSettings.TC_NORMALMAP if normal_map else (
        unreal.TextureCompressionSettings.TC_MASKS if linear else unreal.TextureCompressionSettings.TC_DEFAULT
    )
    texture = rendering.render_target_create_static_texture2d_editor_only(target, name, compression, unreal.TextureMipGenSettings.TMGS_FROM_TEXTURE_GROUP)
    if texture is not None and is_normal:
        texture.set_editor_property("srgb", False)
    eal.delete_asset(f"{folder}/{rt_name}")
    return texture


def bake_appearance(shell: Any, garment: Any, garment_sm: Any, folder: str, base: str) -> dict[str, Any]:
    """Hornea color y relieve de Tripo en las UV de la carcasa y crea su material (paso 4). @spec REQ-98"""
    bake = unreal.GeometryScript_Bake
    source_material = garment_sm.static_materials[0].material_interface if garment_sm.static_materials else None
    textures = {str(t.parameter_info.name): t.parameter_value for t in (source_material.texture_parameter_values if source_material else [])}
    options = unreal.GeometryScriptBakeTextureOptions()
    options.resolution = unreal.GeometryScriptBakeResolution.RESOLUTION2048
    options.projection_distance = BAKE_PROJECTION_CM
    options.samples_per_pixel = unreal.GeometryScriptBakeSamplesPerPixel.SAMPLE4
    source = unreal.GeometryScriptBakeSourceMeshOptions()
    source.source_normal_map = textures.get("NormalTexture")
    identity = unreal.Transform()
    saved: dict[str, Any] = {}
    kinds = [("BaseColor", bake.make_bake_type_texture(textures.get("BaseColorTexture"), 0), False)]
    # El normal horneado no se usa: al pasar por el canvas se le aplica gamma (plano = 0,88 en vez
    # de 0,5) y oscurecía la prenda (2026-11-10). La carcasa usa el normal plano del motor.
    # El mapa de metal y rugosidad de Tripo está en las UV de Tripo: leído con las de la carcasa,
    # la prenda salía metálica y casi negra (2026-11-10). Se hornea como los demás.
    if textures.get("MetallicRoughnessTexture") is not None:
        kinds.append(("MetallicRoughness", bake.make_bake_type_texture(textures.get("MetallicRoughnessTexture"), 0), True))
    for label, kind, is_normal in kinds:
        baked = bake.bake_texture(shell, identity, unreal.GeometryScriptBakeTargetMeshOptions(), garment, identity, source, [kind], options)
        saved[label] = _save_texture(baked[0], folder, f"T_{base}_{label}", is_normal, label == "Normal") if baked else None
    material = _make_material(source_material, folder, f"MI_{base}", saved)
    result: dict[str, Any] = {"material": material, "textures": {k: (v.get_path_name() if v else None) for k, v in saved.items()}}
    return result


def _make_material(source_material: Any, folder: str, name: str, textures: dict[str, Any]) -> Any:
    """Instancia de material de dos caras con las texturas horneadas."""
    eal = unreal.EditorAssetLibrary
    path = f"{folder}/{name}"
    if eal.does_asset_exist(path):
        eal.delete_asset(path)
    material = unreal.AssetToolsHelpers.get_asset_tools().duplicate_asset(name, folder, source_material)
    editing = unreal.MaterialEditingLibrary
    if textures.get("BaseColor") is not None:
        editing.set_material_instance_texture_parameter_value(material, "BaseColorTexture", textures["BaseColor"])
    # El normal de Tripo está en sus UV, no en las de la carcasa: se sustituye por el plano.
    flat_normal = unreal.load_asset(FLAT_NORMAL)
    if flat_normal is not None:
        editing.set_material_instance_texture_parameter_value(material, "NormalTexture", flat_normal)
    if textures.get("MetallicRoughness") is not None:
        editing.set_material_instance_texture_parameter_value(material, "MetallicRoughnessTexture", textures["MetallicRoughness"])
    overrides = material.get_editor_property("base_property_overrides")
    overrides.set_editor_property("override_two_sided", True)
    overrides.set_editor_property("two_sided", True)
    material.set_editor_property("base_property_overrides", overrides)
    return material


def shell_to_skeletal_mesh(shell: Any, full_skm: Any, dest_path: str, material: Any) -> Any:
    """Malla esquelética de la carcasa: copia del asset del cuerpo con la carcasa como LOD0 (paso 5). @spec REQ-98

    Se duplica el cuerpo, no se crea desde el ``USkeleton``: los índices de hueso de la carcasa
    son los del esqueleto de referencia de la malla del cuerpo, que no sigue el orden del
    ``USkeleton``. Con ``create_new_skeletal_mesh_asset_from_mesh`` cada vértice seguía a otro
    hueso y los hombros se levantaban como solapas (visto en vivo el 2026-11-10).
    """
    eal = unreal.EditorAssetLibrary
    folder, name = dest_path.rsplit("/", 1)
    if eal.does_asset_exist(dest_path):
        eal.delete_asset(dest_path)
    skm = unreal.AssetToolsHelpers.get_asset_tools().duplicate_asset(name, folder, full_skm)
    if skm is not None:
        options = unreal.GeometryScriptCopyMeshToAssetOptions()
        options.replace_materials = True
        options.new_materials = [material]
        options.enable_recompute_normals = False
        options.enable_recompute_tangents = True
        unreal.GeometryScript_AssetUtils.copy_mesh_to_skeletal_mesh(shell, skm, options, unreal.GeometryScriptMeshWriteLOD())
    return skm


def build_garment_shell(
    full_skm: Any,
    garment_sm: Any,
    kind: str,
    folder: str,
    base_name: str,
    clearance_cm: float | None = None,
) -> dict[str, Any]:
    """Todo REQ-98 salvo equipar: colocar, zona, carcasa, horneado y malla esquelética. @spec REQ-98"""
    result: dict[str, Any] = {"success": False, "method": "body_shell", "garment_kind": kind, "warnings": []}
    skeleton = full_skm.get_editor_property("skeleton") if full_skm is not None else None
    if kind not in KIND_FAMILIES:
        result["error"] = f"La carcasa del cuerpo solo vale para top y pants, no para «{kind}»."
    elif skeleton is None:
        result["error"] = "El cuerpo completo no tiene esqueleto."
    else:
        body = BodyModel(full_skm)
        garment, placed = place_garment(load_garment(garment_sm), body, kind)
        covered, zone = covered_zone(body, garment, kind)
        if zone["covered_vertices"] == 0:
            result["error"] = "La prenda colocada no tapa ninguna zona del cuerpo: no se puede construir."
            result["placement"] = placed
        else:
            offset = shell_offset(kind, clearance_cm)
            shell = build_shell(body, covered, offset)
            base = base_name[3:] if base_name.startswith("SK_") else base_name
            appearance = bake_appearance(shell, garment, garment_sm, folder, base)
            skm = shell_to_skeletal_mesh(shell, full_skm, f"{folder}/SK_{base}", appearance["material"])
            if skm is None:
                result["error"] = f"GeometryScript no pudo crear la malla esquelética en {folder}/SK_{base}."
            else:
                if zone["looseness"]["loose"]:
                    result["warnings"].append(
                        f"La prenda era holgada (holgura mediana {zone['looseness']['median_cm']:.1f} cm) y se ha ceñido al cuerpo: ha perdido volumen."
                    )
                result.update(
                    {
                        "success": True,
                        "skeletal_mesh": skm,
                        "material": appearance["material"],
                        "textures": appearance["textures"],
                        "offset_cm": offset,
                        "placement": placed,
                        "zone": zone,
                        "covered": covered,
                        "inner_covered": interior(covered, body.neighbours),
                        "body": body,
                    }
                )
    return result
