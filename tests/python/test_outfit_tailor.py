# =============================================================================
# DBV Unreal AI — Tests unitarios del sastre de prendas (parte pura)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de ``dbv_unreal_ai.outfit.tailor`` sin Unreal: orientación, hitos, tapones, ajuste y pesos.

@spec REQ-81, REQ-98
"""

from __future__ import annotations

import math
import os
import site
import sys
import unittest
from pathlib import Path

_user_site = site.getusersitepackages()
if os.path.isdir(_user_site) and _user_site not in sys.path:
    sys.path.append(_user_site)

try:
    import numpy as np
except ImportError:  # el CI no instala numpy (en Unreal lo trae PythonFoundationPackages): se saltan
    np = None  # type: ignore[assignment]

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.outfit import tailor


def _make_cylinder(radius_x: float, radius_y: float, z_lo: float, z_hi: float, nz: int = 20, na: int = 24) -> tuple[np.ndarray, np.ndarray]:
    """Construye un tubo elíptico cerrado lateralmente entre ``z_lo`` y ``z_hi``. @spec REQ-98"""
    zs = np.linspace(z_lo, z_hi, nz)
    angles = np.linspace(0.0, 2.0 * math.pi, na, endpoint=False)
    verts: list[tuple[float, float, float]] = []
    for z in zs:
        for th in angles:
            verts.append((radius_x * math.cos(th), radius_y * math.sin(th), float(z)))
    tris: list[tuple[int, int, int]] = []
    for iz in range(nz - 1):
        base = iz * na
        for ia in range(na):
            jn = (ia + 1) % na
            a, b = base + ia, base + jn
            c, d = a + na, b + na
            tris.append((a, b, c))
            tris.append((b, d, c))
    return np.asarray(verts, dtype=float), np.asarray(tris, dtype=int)


def _make_synthetic_body() -> tailor.Body:
    """Construye un modelo ``Body`` sintético con tronco, entrepierna a z=82 y brazos separados. @spec REQ-98"""
    pos, tris = _make_cylinder(16.0, 11.0, 60.0, 148.0, nz=30, na=24)
    # Por debajo de z=82 las piernas se separan del eje central X=0
    below_crotch = (pos[:, 2] < 82.0) & (np.abs(pos[:, 0]) < 2.0)
    pos[below_crotch, 0] = np.where(pos[below_crotch, 0] >= 0.0, 2.5, -2.5)
    n_verts = len(pos)
    bone_names = ["root", "pelvis", "spine_03", "clavicle_l", "clavicle_r", "upperarm_l", "upperarm_r", "thigh_l", "thigh_r"]
    b_idx = np.zeros((n_verts, 2), dtype=int)
    b_idx[:, 0] = np.where(pos[:, 2] >= 100.0, 2, 1)
    b_w = np.zeros((n_verts, 2), dtype=float)
    b_w[:, 0] = 1.0
    bpos = np.array(
        [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 92.0],
            [0.0, 0.0, 122.0],
            [6.0, 0.0, 142.0],
            [-6.0, 0.0, 142.0],
            [17.5, 0.0, 138.0],
            [-17.5, 0.0, 138.0],
            [9.0, 0.0, 88.0],
            [-9.0, 0.0, 88.0],
        ],
        dtype=float,
    )
    return tailor.Body({"pos": pos, "tris": tris, "names": bone_names, "wi": b_idx, "ww": b_w, "bpos": bpos})


@unittest.skipIf(np is None, "numpy no está disponible")
class TestTailorPure(unittest.TestCase):
    """Pruebas unitarias puras del sastre (REQ-81, REQ-98)."""

    def test_resolve_clearance_acota_holgura_y_respeta_orden_de_capas(self) -> None:
        """@spec REQ-98"""
        self.assertGreater(tailor.resolve_clearance("top"), tailor.resolve_clearance("pants"))
        self.assertEqual(tailor.resolve_clearance("top", 0.05), tailor.MIN_CLEARANCE_CM)
        self.assertEqual(tailor.resolve_clearance("pants", 50.0), tailor.MAX_CLEARANCE_CM)
        self.assertEqual(tailor.resolve_clearance("top", 1.25), 1.25)

    def test_normalise_gira_90_y_voltea_180_segun_escote(self) -> None:
        """@spec REQ-98"""
        rng = np.random.default_rng(7)
        pts = rng.uniform([-12.0, -25.0, 0.0], [12.0, 25.0, 50.0], size=(400, 3))
        # Tras girar 90 grados: X'=-Y, Y'=X. Ponemos el borde superior más alto en X>0 (Y'>0) que en X<0 (Y'<0)
        # para que normalise detecte que el escote alto está en +Y y active flip180.
        front_mask = (np.abs(pts[:, 1]) < 4.0) & (pts[:, 0] > 2.0)
        back_mask = (np.abs(pts[:, 1]) < 4.0) & (pts[:, 0] < -2.0)
        pts[front_mask, 2] = np.maximum(pts[front_mask, 2], 49.0)
        pts[back_mask, 2] = np.minimum(pts[back_mask, 2], 32.0)
        normed, info = tailor.normalise(pts, "top")
        self.assertTrue(info["rot90"])
        self.assertTrue(info["flip180"])
        self.assertGreater(float(normed[:, 0].max() - normed[:, 0].min()), float(normed[:, 1].max() - normed[:, 1].min()))

    def test_envolvente_convexa_y_radios_polares(self) -> None:
        """@spec REQ-98"""
        pts = np.array([[-10.0, -8.0], [10.0, -8.0], [10.0, 10.0], [0.0, 6.0], [-10.0, 10.0]])
        hull = tailor.convex_hull(pts)
        self.assertEqual(len(hull), 4)
        radii = tailor.hull_radii(hull, np.array([0.0, 0.0]), nt=4)
        self.assertEqual(len(radii), 4)
        self.assertTrue(np.all(radii >= 7.9))

    def test_drop_inner_caps_elimina_tapon_horizontal_interior(self) -> None:
        """@spec REQ-98"""
        body = _make_synthetic_body()
        col = tailor.body_collider(body)
        pos, tris = _make_cylinder(18.0, 13.0, 90.0, 135.0, nz=10, na=16)
        # Añadimos un tapón horizontal profundo dentro del torso (z = 110, radio = 4 cm)
        cap_idx = len(pos)
        cap_verts = np.array([[0.0, 0.0, 110.0], [4.0, 0.0, 110.0], [0.0, 4.0, 110.0], [-4.0, 0.0, 110.0]], dtype=float)
        pos_with_cap = np.vstack([pos, cap_verts])
        cap_tris = np.array([[cap_idx, cap_idx + 1, cap_idx + 2], [cap_idx, cap_idx + 2, cap_idx + 3]], dtype=int)
        tris_with_cap = np.vstack([tris, cap_tris])
        kept_tris, cap_mask = tailor.drop_inner_caps(pos_with_cap, tris_with_cap, col, z_max=130.0)
        self.assertEqual(int(cap_mask[-2:].sum()), 2)
        self.assertEqual(len(kept_tris), len(tris_with_cap) - 2)

    def test_fit_top_respeta_limit_top_length_opcional(self) -> None:
        """Por defecto conserva el largo natural; con limit_top_length=True comprime sobre la entrepierna. @spec REQ-98"""
        body = _make_synthetic_body()
        col = tailor.body_collider(body)
        # Camiseta larga con mangas cortas/sisa para que garment_armhole detecte el estrechamiento
        pos, tris = _make_cylinder(20.0, 14.0, 0.0, 80.0, nz=25, na=24)
        # Ensanchamos por encima de z=55 para simular la sisa
        upper = pos[:, 2] > 55.0
        pos[upper, 0] *= 1.25

        q_natural, _, info_natural = tailor.fit_top(pos, tris, body, col, clearance=1.0, limit_top_length=False)
        q_short, _, info_short = tailor.fit_top(pos, tris, body, col, clearance=1.0, limit_top_length=True)

        self.assertFalse(info_natural["limit_top_length"])
        self.assertNotIn("length_squash", info_natural)
        self.assertTrue(info_short["limit_top_length"])
        self.assertIn("length_squash", info_short)
        self.assertLess(float(q_natural[:, 2].min()), float(q_short[:, 2].min()))
        self.assertLessEqual(info_natural["inside_pct"], 5.0)

    def test_fit_pants_y_pesos_suavizados(self) -> None:
        """@spec REQ-81, REQ-98"""
        body = _make_synthetic_body()
        col = tailor.body_collider(body)
        pos, tris = _make_cylinder(18.0, 13.0, 0.0, 50.0, nz=20, na=24)
        # Simulamos bifurcación de perneras por debajo de z=25 desplazando X hacia fuera de la línea central
        legs = pos[:, 2] < 25.0
        pos[legs & (np.abs(pos[:, 0]) < 3.0), 0] = np.where(pos[legs & (np.abs(pos[:, 0]) < 3.0), 0] >= 0.0, 4.0, -4.0)

        q, kept_tris, info = tailor.fit_pants(pos, tris, body, col, clearance=0.8)
        self.assertIn("scale", info)
        self.assertLessEqual(info["inside_pct"], 5.0)

        b_idx, b_w = tailor.skin_weights(body, q, kept_tris)
        self.assertEqual(b_idx.shape[0], len(q))
        self.assertTrue(np.allclose(b_w.sum(axis=1), 1.0, atol=1e-5))

        ref_nrm = np.column_stack([q[:, 0], q[:, 1], np.zeros(len(q))])
        nrm = tailor.welded_normals(q, kept_tris, ref_nrm)
        self.assertEqual(nrm.shape, q.shape)
        norms = np.linalg.norm(nrm, axis=1)
        self.assertTrue(np.allclose(norms, 1.0, atol=1e-4))

    def test_build_tailored_garment_rechaza_tipo_invalido(self) -> None:
        """@spec REQ-98"""
        res = tailor.build_tailored_garment(None, None, "skirt", "/Game/X", "SK_X")
        self.assertFalse(res["success"])
        self.assertIn("top y pants", res["error"])


if __name__ == "__main__":
    unittest.main()
