# =============================================================================
# DBV Unreal AI — Tests unitarios del ajuste de vestidos de tela (parte pura)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de ``tailor.fit_dress`` y sus piezas sin Unreal, sobre mallas sintéticas.

@spec REQ-87
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
except ImportError:  # sin numpy (CI mínima) estas pruebas se saltan, como las del sastre
    np = None  # type: ignore[assignment]

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.outfit import tailor

WAIST_Z = 105.0


def _tube(radius_x, radius_y, z_lo: float, z_hi: float, nz: int = 40, na: int = 32, centre=(0.0, 0.0)):
    """Tubo elíptico abierto; los radios pueden ser funciones de la altura."""
    zs = np.linspace(z_lo, z_hi, nz)
    angles = np.linspace(0.0, 2.0 * math.pi, na, endpoint=False)
    verts = []
    for z in zs:
        rx = radius_x(z) if callable(radius_x) else radius_x
        ry = radius_y(z) if callable(radius_y) else radius_y
        for th in angles:
            verts.append((centre[0] + rx * math.cos(th), centre[1] + ry * math.sin(th), float(z)))
    tris = []
    for iz in range(nz - 1):
        base = iz * na
        for ia in range(na):
            a, b = base + ia, base + (ia + 1) % na
            tris.append((a, b, a + na))
            tris.append((b, b + na, a + na))
    return np.asarray(verts, dtype=float), np.asarray(tris, dtype=int)


def _disc(radius_x: float, radius_y: float, z: float, na: int = 32, off_centre=(0.37, 0.21)):
    """Tapa plana: abanico de triángulos con el centro ligeramente desplazado del eje."""
    angles = np.linspace(0.0, 2.0 * math.pi, na, endpoint=False)
    verts = [(off_centre[0], off_centre[1], z)] + [(radius_x * math.cos(th), radius_y * math.sin(th), z) for th in angles]
    tris = [(0, 1 + i, 1 + (i + 1) % na) for i in range(na)]
    return np.asarray(verts, dtype=float), np.asarray(tris, dtype=int)


def _join(parts):
    """Une mallas sin soldar (como llegan las islas de UV de Tripo)."""
    pos, tris, offset = [], [], 0
    for p, t in parts:
        pos.append(p)
        tris.append(t + offset)
        offset += len(p)
    return np.concatenate(pos), np.concatenate(tris)


def _body_rx(z: float) -> float:
    return 13.0 + 3.5 * min(abs(z - WAIST_Z) / 15.0, 1.0)


def _make_body() -> tailor.Body:
    """Cuerpo sintético: tronco con cintura en z=105 y los huesos que usa el ajuste de vestidos."""
    pos, tris = _tube(_body_rx, 10.0, 75.0, 150.0)
    names = [
        "pelvis", "spine_01", "spine_05", "head",
        "upperarm_l", "lowerarm_l", "hand_l", "upperarm_r", "lowerarm_r", "hand_r",
        "thigh_l", "calf_l", "foot_l", "thigh_r", "calf_r", "foot_r",
    ]
    bpos = np.array(
        [
            [0, 0, 87], [0, 0, 89], [0, 0, 123], [0, 0, 145],
            [17, 0, 138], [31, 0, 119], [43, 6, 105], [-17, 0, 138], [-31, 0, 119], [-43, 6, 105],
            [9, 0, 84], [10, 0, 44], [11, 0, 8], [-9, 0, 84], [-10, 0, 44], [-11, 0, 8],
        ],
        dtype=float,
    )
    wi = np.zeros((len(pos), 1), dtype=int)
    ww = np.ones((len(pos), 1), dtype=float)
    return tailor.Body({"pos": pos, "tris": tris, "names": names, "wi": wi, "ww": ww, "bpos": bpos})


def _make_dress():
    """Vestido sin mangas macizo, en espacio del cuerpo: talle, falda con vuelo, tapa de cuello y suelo."""

    def rx(z: float) -> float:
        return _body_rx(z) + 2.0 + (max(0.0, 95.0 - z) * 0.35)

    def ry(z: float) -> float:
        return 12.0 + (max(0.0, 95.0 - z) * 0.3)

    wall, wall_t = _tube(rx, ry, 60.0, 138.0)
    neck, neck_t = _disc(rx(138.0), ry(138.0), 138.0)
    floor, floor_t = _disc(rx(72.0), ry(72.0), 72.0)
    return _join([(wall, wall_t), (neck, neck_t), (floor, floor_t)])


def _as_tripo(pos):
    """Lleva la prenda a como la entrega Tripo: centrada, a otra escala y girada 90°."""
    p = (pos - (pos.min(0) + pos.max(0)) / 2.0) * 0.8
    return np.stack([p[:, 1], -p[:, 0], p[:, 2]], 1)


@unittest.skipIf(np is None, "numpy no está disponible")
class TestTailorDress(unittest.TestCase):
    """Ajuste de vestidos de tela (REQ-87)."""

    def test_perfil_detecta_mangas_separadas_del_talle(self) -> None:
        """@spec REQ-87"""
        torso = _tube(14.0, 10.0, 0.0, 60.0)
        left = _tube(4.0, 4.0, 20.0, 45.0, centre=(26.0, 0.0))
        right = _tube(4.0, 4.0, 20.0, 45.0, centre=(-26.0, 0.0))
        pos, tris = _join([torso, left, right])
        marks = tailor.dress_marks(pos, tris)
        self.assertTrue(marks["sleeved"])
        self.assertAlmostEqual(marks["armpit"], 45.0, delta=2.0)
        self.assertAlmostEqual(marks["chest_hw"], 14.0, delta=1.0)
        sleeveless = tailor.dress_marks(*torso)
        self.assertFalse(sleeveless["sleeved"])

    def test_tapas_que_atraviesa_un_hueso_se_quitan_y_las_paredes_no(self) -> None:
        """@spec REQ-87"""
        body = _make_body()
        pos, tris = _make_dress()
        col = tailor.body_collider(body, with_arms=True)
        kept, cap = tailor.drop_end_caps(pos, tris, tailor._body_segments(body), col)
        wall_tris = 2 * 39 * 32
        self.assertEqual(int(cap.sum()), 64, "las dos tapas (cuello y suelo de la falda), de 32 triángulos cada una")
        self.assertFalse(bool(cap[:wall_tris].any()), "ninguna pared se retira")
        self.assertEqual(len(kept), wall_tris)

    def test_piezas_sueltas_pequenas_se_marcan(self) -> None:
        """@spec REQ-87"""
        big = _tube(15.0, 12.0, 0.0, 80.0)
        stub = _tube(3.0, 3.0, 5.0, 12.0, nz=4, na=8, centre=(5.0, 0.0))
        pos, tris = _join([big, stub])
        loose = tailor.loose_parts(pos, tris)
        self.assertFalse(bool(loose[: len(big[1])].any()))
        self.assertTrue(bool(loose[len(big[1]) :].all()))

    def test_pesos_de_tela_sujetan_talle_y_mangas_y_sueltan_la_falda(self) -> None:
        """@spec REQ-87"""
        pos = np.array([[0, 0, 120.0], [0, 0, 92.0], [0, 0, 75.0], [0, 0, 50.0], [40.0, 0, 50.0]])
        sleeve = np.array([0.0, 0.0, 0.0, 0.0, 1.0])
        w = tailor.cloth_free_weights(pos, sleeve, pin_z=92.0, free_z=58.0)
        self.assertEqual(float(w[0]), 0.0)
        self.assertEqual(float(w[1]), 0.0)
        self.assertTrue(0.0 < float(w[2]) < 1.0)
        self.assertEqual(float(w[3]), 1.0)
        self.assertEqual(float(w[4]), 0.0, "una manga que cuelga por debajo del talle sigue sujeta al brazo")

    def test_fit_dress_coloca_el_vestido_sin_atravesar_el_cuerpo(self) -> None:
        """@spec REQ-87"""
        body = _make_body()
        pos, tris = _make_dress()
        fitted, kept, info = tailor.fit_dress(_as_tripo(pos), tris, body, clearance=1.0)
        self.assertTrue(info["rot90"])
        self.assertFalse(info["sleeved"])
        self.assertEqual(int(info["cap_mask"].sum()), 64)
        self.assertEqual(len(kept), len(tris) - 64)
        self.assertEqual(info["inside_pct"], 0.0)
        self.assertGreaterEqual(info["min_cm"], 0.99)
        self.assertLess(info["hem_z"], info["waist_z"])
        self.assertGreater(info["pin_z"], info["free_z"])
        used = np.unique(kept.ravel())
        self.assertGreater(float(fitted[used][:, 2].max()), 130.0, "el vestido queda a la altura del cuerpo, no en el origen")

    def test_flip_orientation_gira_el_vestido_180_grados(self) -> None:
        """@spec REQ-87, REQ-88"""
        body = _make_body()
        pos, tris = _make_dress()
        pos = pos.copy()
        mark = int(np.argmax(pos[:, 1] + (pos[:, 2] == pos[:, 2].max()) * 100.0))  # un vértice del borde superior, delante
        raw = _as_tripo(pos)
        a, _, info_a = tailor.fit_dress(raw, tris, body)
        b, _, info_b = tailor.fit_dress(raw, tris, body, flip_orientation=True)
        self.assertNotEqual(info_a["flip180"], info_b["flip180"])
        self.assertTrue(info_b["flip_requested"])
        self.assertLess(float(a[mark, 1]) * float(b[mark, 1]), 0.0, "el mismo vértice pasa de un lado al otro del cuerpo")


if __name__ == "__main__":
    unittest.main()
