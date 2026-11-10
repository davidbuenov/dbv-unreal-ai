# =============================================================================
# DBV Unreal AI — Tests unitarios para el ajuste de prendas (S99, REQ-87)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

# Asegurar importación del plugin
PLUGIN_DIR = Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"
if str(PLUGIN_DIR) not in sys.path:
    sys.path.insert(0, str(PLUGIN_DIR))

from dbv_unreal_ai.outfit import fit


class MockVector:
    def __init__(self, x: float = 0.0, y: float = 0.0, z: float = 0.0) -> None:
        self.x = x
        self.y = y
        self.z = z


class MockPhysicsAsset:
    def __init__(self, name: str) -> None:
        self._name = name

    def get_name(self) -> str:
        return self._name


class MockSkeletalMesh:
    def __init__(self, name: str, physics_asset: Any = None) -> None:
        self._name = name
        self._physics = physics_asset

    def get_name(self) -> str:
        return self._name

    def get_editor_property(self, prop: str) -> Any:
        res = self._physics if prop == "physics_asset" else None
        return res


class MockBodyComponent:
    def __init__(self, heights: dict[str, float] | None = None, skm_name: str = "f_med_nrw_body") -> None:
        self._heights = heights or {}
        self._skm = MockSkeletalMesh(skm_name, MockPhysicsAsset("f_med_nrw_body_Physics"))

    def get_bone_index(self, name: str) -> int:
        idx = 1 if name in self._heights else -1
        return idx

    def get_socket_location(self, name: str) -> MockVector:
        loc = MockVector(z=self._heights.get(name, 0.0))
        return loc

    def get_skeletal_mesh_asset(self) -> Any:
        return self._skm


class MockActor:
    def __init__(self, z: float = 0.0) -> None:
        self._z = z

    def get_actor_location(self) -> MockVector:
        return MockVector(z=self._z)


def test_measure_landmarks_none_component() -> None:
    res = fit.measure_body_landmarks(None)
    assert not res["success"]
    assert "no puede ser None" in res["error"]


def test_measure_landmarks_missing_bone() -> None:
    # Falta el hueso foot_l
    heights = {
        "head": 143.0,
        "neck_01": 133.0,
        "clavicle_l": 127.0,
        "spine_03": 100.0,
        "spine_02": 93.0,
        "pelvis": 87.0,
        "thigh_l": 84.0,
        "calf_l": 44.0,
    }
    body = MockBodyComponent(heights)
    res = fit.measure_body_landmarks(body)
    assert not res["success"]
    assert "foot_l" in res["error"]


def test_measure_landmarks_invalid_ordering() -> None:
    # Alturas invertidas (head más bajo que foot)
    heights = {
        "head": 10.0,
        "neck_01": 20.0,
        "clavicle_l": 30.0,
        "spine_03": 40.0,
        "spine_02": 50.0,
        "pelvis": 60.0,
        "thigh_l": 70.0,
        "calf_l": 80.0,
        "foot_l": 90.0,
    }
    body = MockBodyComponent(heights)
    res = fit.measure_body_landmarks(body)
    assert not res["success"]
    assert "anómalas" in res["error"]


def test_measure_landmarks_success_victoria() -> None:
    heights = {
        "head": 143.36,
        "neck_01": 132.99,
        "clavicle_l": 127.44,
        "spine_03": 99.98,
        "spine_02": 93.26,
        "pelvis": 87.07,
        "thigh_l": 84.02,
        "calf_l": 43.83,
        "foot_l": 7.15,
    }
    body = MockBodyComponent(heights, "f_med_nrw_body")
    actor = MockActor(0.0)
    res = fit.measure_body_landmarks(body, actor)
    assert res["success"]
    landmarks = res["landmarks"]
    assert landmarks["waist_z"] == 93.26
    assert landmarks["hip_z"] == 84.02
    assert landmarks["default_hem_z"] == round((43.83 + 7.15) * 0.5, 2)
    assert res["physics_asset"] is not None


def test_sample_body_radii() -> None:
    # Crear un cilindro sintético de radio 15.0 a Z=80.0
    body_points = [
        (15.0, 0.0, 80.0),
        (-15.0, 0.0, 80.0),
        (0.0, 15.0, 80.0),
        (0.0, -15.0, 80.0),
    ]
    radii = fit.sample_body_radii_at_height(body_points, 75.0, 85.0, num_sectors=8)
    assert len(radii) == 8
    # Los sectores cardinales deben tener radio cercano a 15.0
    assert any(abs(r - 15.0) < 0.1 for r in radii)


def test_generate_skirt_tube_geometry() -> None:
    body_points = [
        (12.0, 0.0, 90.0),
        (0.0, 12.0, 90.0),
        (-12.0, 0.0, 90.0),
        (0.0, -12.0, 90.0),
        (14.0, 0.0, 60.0),
        (0.0, 14.0, 60.0),
    ]
    geom = fit.generate_skirt_tube_geometry(
        body_points=body_points,
        waist_z=93.0,
        hem_z=40.0,
        hip_z=84.0,
        clearance_cm=1.5,
        num_rings=6,
        num_sectors=12,
    )
    assert geom["num_vertices"] == 6 * 12
    assert geom["num_triangles"] == (6 - 1) * 12 * 2
    # El radio en el dobladillo debe ser mayor por el ensanchamiento (flare)
    hem_vertices = [v for v in geom["vertices"] if abs(v[2] - 40.0) < 0.01]
    waist_vertices = [v for v in geom["vertices"] if abs(v[2] - 93.0) < 0.01]
    hem_r = max(v[0] for v in hem_vertices)
    waist_r = max(v[0] for v in waist_vertices)
    assert hem_r > waist_r


def test_isolate_garment_triangles() -> None:
    vertices = [
        (0.0, 0.0, 110.0),  # Torso alto
        (5.0, 0.0, 110.0),
        (0.0, 5.0, 110.0),
        (0.0, 0.0, 70.0),   # Zona falda / cadera
        (5.0, 0.0, 70.0),
        (0.0, 5.0, 70.0),
        (0.0, 0.0, 10.0),   # Zona piernas bajas / zapatos
        (5.0, 0.0, 10.0),
        (0.0, 5.0, 10.0),
    ]
    triangles = [
        (0, 1, 2),  # Triángulo torso
        (3, 4, 5),  # Triángulo falda
        (6, 7, 8),  # Triángulo pie
    ]

    top_tris = fit.isolate_garment_triangles(vertices, triangles, "top", waist_z=90.0, hem_z=45.0)
    assert (0, 1, 2) in top_tris
    assert (6, 7, 8) not in top_tris

    skirt_tris = fit.isolate_garment_triangles(vertices, triangles, "skirt", waist_z=90.0, hem_z=45.0)
    assert (3, 4, 5) in skirt_tris
    assert (0, 1, 2) not in skirt_tris
    assert (6, 7, 8) not in skirt_tris


def test_conform_sleeves_to_arm_pose() -> None:
    # Vértice de torso central no debe cambiar
    # Vértice de manga izquierda (X=40, Z=40) debe rotar hacia abajo (Z menor) y adelante (Y mayor)
    verts = [
        (0.0, 0.0, 40.0),    # Torso
        (10.0, 0.0, 40.0),   # Hombro interior
        (40.0, 0.0, 40.0),   # Manga izquierda abierta
        (-40.0, 0.0, 40.0),  # Manga derecha abierta
    ]
    conformed = fit.conform_sleeves_to_arm_pose(verts, shoulder_x=16.5, blend_x=21.5, pivot_z=38.0)
    assert len(conformed) == 4

    # Torso intacto
    assert conformed[0] == (0.0, 0.0, 40.0)
    assert conformed[1] == (10.0, 0.0, 40.0)

    # Manga izquierda: Z baja y Y sube (inclinación A-pose hacia adelante)
    nx_l, ny_l, nz_l = conformed[2]
    assert nz_l < 40.0
    assert ny_l > 0.0

    # Manga derecha: simétrica en X, Z baja igual
    nx_r, ny_r, nz_r = conformed[3]
    assert nz_r < 40.0
    assert abs(abs(nx_l) - abs(nx_r)) < 0.1


def test_split_suit_geometry() -> None:
    vertices = [
        (0.0, 0.0, 30.0),  # Torso alto
        (0.0, 0.0, 16.0),  # Cintura solapada alta
        (0.0, 0.0, 14.0),  # Cintura solapada baja
        (0.0, 0.0, -20.0), # Pantalón / pierna
    ]
    triangles = [
        (0, 0, 0),  # z=30 (top)
        (1, 1, 1),  # z=16 (solapado: pertenece a top y a bottom con overlap=2.0 en waist=15.0)
        (2, 2, 2),  # z=14 (solapado: pertenece a top y a bottom)
        (3, 3, 3),  # z=-20 (bottom)
    ]
    res = fit.split_suit_geometry(vertices, triangles, waist_z=15.0, overlap_cm=2.0)
    assert (0, 0, 0) in res["top_triangles"]
    assert (3, 3, 3) in res["bottom_triangles"]
    # Los triángulos solapados están en ambas prendas para evitar agujeros en la cintura
    assert (1, 1, 1) in res["top_triangles"]
    assert (1, 1, 1) in res["bottom_triangles"]
    assert (2, 2, 2) in res["top_triangles"]
    assert (2, 2, 2) in res["bottom_triangles"]


def test_weld_mesh_vertices_merges_close_vertices() -> None:
    # V0 y V1 están a distancia 0.02 (menor que tolerancia 0.05): deben fusionarse
    # V2 y V3 son vértices lejanos
    verts = [
        (0.0, 0.0, 0.0),       # V0
        (0.0, 0.0, 0.02),      # V1 (coincidente con V0)
        (10.0, 0.0, 0.0),      # V2
        (0.0, 10.0, 0.0),      # V3
    ]
    # T0 une V0, V1, V2 -> al fusionar V0 y V1, se vuelve degenerado (0, 0, 1) y debe descartarse
    # T1 une V0, V2, V3 -> triángulo válido no degenerado
    tris = [
        (0, 1, 2),
        (0, 2, 3),
    ]
    welded_v, welded_t = fit.weld_mesh_vertices(verts, tris, tolerance=0.05)
    assert len(welded_v) == 3
    assert len(welded_t) == 1
    # El triángulo restante une el vértice fusionado con V2 y V3
    assert welded_t[0] == (0, 1, 2)


def test_weld_top_dynamic_mesh_mock() -> None:
    # En entorno mock o con None, debe devolver el objeto sin error
    res_none = fit.weld_top_dynamic_mesh(None)
    assert res_none is None


class MockBasePropertyOverrides:
    def __init__(self) -> None:
        self.override_two_sided = False
        self.two_sided = False

    def set_editor_property(self, prop: str, val: Any) -> None:
        if prop == "override_two_sided":
            self.override_two_sided = bool(val)
        elif prop == "two_sided":
            self.two_sided = bool(val)

    def get_editor_property(self, prop: str) -> Any:
        res = getattr(self, prop, None)
        return res


class MockMaterialInstance:
    def __init__(self, path: str = "/Game/DBV/TestMat.TestMat") -> None:
        self._path = path
        self.bpo = MockBasePropertyOverrides()

    def get_path_name(self) -> str:
        return self._path

    def get_editor_property(self, prop: str) -> Any:
        res = self.bpo if prop == "base_property_overrides" else None
        return res

    def set_editor_property(self, prop: str, val: Any) -> None:
        if prop == "base_property_overrides":
            self.bpo = val


class MockGarmentMeshWithMats:
    def __init__(self, mats: list[Any]) -> None:
        self._mats = mats

    def get_materials(self) -> list[Any]:
        return self._mats


def test_apply_two_sided_materials_mock() -> None:
    mat = MockMaterialInstance()
    mesh = MockGarmentMeshWithMats([mat])
    modified = fit.apply_two_sided_materials(mesh)
    assert len(modified) == 1
    assert mat.bpo.override_two_sided
    assert mat.bpo.two_sided


def test_setup_garment_lods_mock() -> None:
    skm = MockSkeletalMesh("SK_Garment_Test")
    res = fit.setup_garment_lods(skm, target_lod_count=4)
    assert res["success"]
    assert res["target_lods"] == 4


def test_conform_sleeves_full_length_cuff() -> None:
    # Vértice de puño en T-pose a X=45.0, Z=38.0
    # Debe rotarse hacia abajo (Z menor a ~12.3) y adelante (Y mayor)
    verts = [
        (45.0, 0.0, 38.0),
    ]
    conformed = fit.conform_sleeves_to_arm_pose(verts, shoulder_x=16.5, blend_x=21.5, pivot_z=38.0)
    assert len(conformed) == 1
    nx, ny, nz = conformed[0]
    # En A-pose, Z baja respecto al pivote 38.0 y Y avanza hacia adelante
    assert nz < 20.0
    assert ny > 0.0


