# =============================================================================
# DBV Unreal AI — Tests de la holgura real entre prenda y cuerpo
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de ``dbv_unreal_ai.outfit.clearance`` con geometría analítica (esferas).

@spec REQ-89
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.outfit import clearance


class SphereBody:
    """Cuerpo esférico de radio ``r`` centrado en el origen."""

    def __init__(self, radius: float, visible=lambda p: True) -> None:
        self.radius = radius
        self.visible = visible

    def signed_distance(self, point: clearance.Point) -> tuple[float, bool] | None:
        return (math.sqrt(sum(c * c for c in point)) - self.radius, self.visible(point))

    def ray_hit_distance(self, origin: clearance.Point, direction: clearance.Point) -> float | None:
        return None

    def unsigned_distance(self, point: clearance.Point) -> float | None:
        return abs(math.sqrt(sum(c * c for c in point)) - self.radius)


class SphereGarment:
    """Prenda esférica de radio ``r`` que solo existe donde ``covers(punto)`` es verdad."""

    def __init__(self, radius: float, covers=lambda p: True) -> None:
        self.radius = radius
        self.covers = covers

    def signed_distance(self, point: clearance.Point) -> tuple[float, bool] | None:
        return None

    def unsigned_distance(self, point: clearance.Point) -> float | None:
        # Distancia a la parte de la esfera que existe (aproximada por la radial si la cubre).
        length = math.sqrt(sum(c * c for c in point)) or 1.0
        on_sphere = tuple(c * self.radius / length for c in point)
        return abs(length - self.radius) if self.covers(on_sphere) else None

    def ray_hit_distance(self, origin: clearance.Point, direction: clearance.Point) -> float | None:
        # Corte del rayo origen + t·dirección con la esfera, por fuera del origen.
        b = sum(o * d for o, d in zip(origin, direction))
        c = sum(o * o for o in origin) - self.radius * self.radius
        disc = b * b - c
        hit = None
        if disc >= 0.0:
            t = -b + math.sqrt(disc)
            point = tuple(o + t * d for o, d in zip(origin, direction))
            hit = t if t >= 0.0 and self.covers(point) else None
        return hit


def sphere_points(radius: float, steps: int = 24) -> list[clearance.Point]:
    """Puntos repartidos sobre una esfera."""
    points = []
    for i in range(1, steps):
        theta = math.pi * i / steps
        for j in range(steps):
            phi = 2.0 * math.pi * j / steps
            points.append((radius * math.sin(theta) * math.cos(phi), radius * math.sin(theta) * math.sin(phi), radius * math.cos(theta)))
    return points


def sphere_zone(radius: float) -> list[tuple[clearance.Point, clearance.Point]]:
    """Zona de piel: puntos de la esfera del cuerpo con su normal hacia fuera."""
    zone = []
    for p in sphere_points(radius):
        length = math.sqrt(sum(c * c for c in p))
        zone.append((p, tuple(c / length for c in p)))
    return zone


class TestClearance(unittest.TestCase):
    """Holgura con signo, piel destapada y nota final."""

    def test_prenda_holgada_por_fuera_es_ok(self) -> None:
        body = SphereBody(10.0)
        report = clearance.measure_signed_clearance(sphere_points(11.0), body, sphere_zone(10.0), SphereGarment(11.0))
        self.assertEqual(report["result"], "ok")
        self.assertAlmostEqual(report["clearance"]["median_cm"], 1.0, places=2)
        self.assertEqual(report["clearance"]["inside_vertices"], 0)
        self.assertEqual(report["bare_skin"]["bare_vertices"], 0)

    def test_prenda_dentro_de_la_piel_es_error_y_lo_explica(self) -> None:
        body = SphereBody(10.0)
        report = clearance.measure_signed_clearance(sphere_points(9.0), body)
        self.assertEqual(report["result"], "error")
        self.assertEqual(report["clearance"]["inside_pct"], 100.0)
        self.assertLess(report["clearance"]["min_cm"], -0.9)
        self.assertIn("dentro de la piel", " ".join(report["warnings"]))

    def test_unos_pocos_vertices_dentro_dan_aviso(self) -> None:
        body = SphereBody(10.0)
        points = sphere_points(11.0)
        points[0] = (0.0, 0.0, 9.5)  # dos de 552 dentro (0,36 %): entre el umbral de aviso y el de error
        points[1] = (0.0, 0.0, -9.5)
        report = clearance.measure_signed_clearance(points, body)
        self.assertEqual(report["result"], "con_avisos")

    def test_hueco_en_la_entrepierna_es_error_y_dice_donde(self) -> None:
        body = SphereBody(10.0)
        # La prenda no existe por debajo de z = -6: deja un casquete de piel al aire.
        garment = SphereGarment(11.0, covers=lambda p: p[2] > -6.0)
        report = clearance.measure_signed_clearance(sphere_points(11.0), body, sphere_zone(10.0), garment)
        self.assertEqual(report["result"], "error")
        self.assertGreater(report["bare_skin"]["bare_pct"], clearance.BARE_ERROR_PCT)
        self.assertTrue(all(r["z_cm"] < -5 for r in report["bare_skin"]["bare_regions"]))
        self.assertIn("piel al descubierto", " ".join(report["warnings"]))

    def test_prenda_demasiado_lejos_cuenta_como_destapada(self) -> None:
        body = SphereBody(10.0)
        far = clearance.MAX_GAP_CM + 3.0
        report = clearance.measure_signed_clearance(sphere_points(10.0 + far), body, sphere_zone(10.0), SphereGarment(10.0 + far))
        self.assertEqual(report["bare_skin"]["bare_pct"], 100.0)
        self.assertEqual(report["result"], "error")

    def test_sin_medida_es_error_nunca_valores_fijos(self) -> None:
        class Blind:
            def signed_distance(self, point):
                return None

            def ray_hit_distance(self, origin, direction):
                return None

            def unsigned_distance(self, point):
                return None

        report = clearance.measure_signed_clearance(sphere_points(11.0), Blind())
        self.assertEqual(report["result"], "error")
        self.assertEqual(report["clearance"], {"measured_vertices": 0, "over_hidden_skin_vertices": 0})
        self.assertNotIn("min_cm", report["clearance"])

    def test_prenda_metida_en_piel_que_no_se_dibuja_no_cuenta(self) -> None:
        # Como la camisa de fábrica de MetaHuman: entra en una piel que el cuerpo recortado no tiene.
        body = SphereBody(10.0, visible=lambda p: False)
        report = clearance.measure_signed_clearance(sphere_points(9.0), body)
        self.assertEqual(report["result"], "ok")
        self.assertEqual(report["clearance"]["measured_vertices"], 0)
        self.assertEqual(report["clearance"]["over_hidden_skin_vertices"], len(sphere_points(9.0)))

    def test_tela_solo_cerca_no_tapa_la_piel(self) -> None:
        # Bordes de un hueco a 1 cm: si el rayo no corta, la piel sigue al aire.
        class NearOnly:
            def ray_hit_distance(self, origin, direction):
                return None

            def unsigned_distance(self, point):
                return 1.0

        bare = clearance.bare_skin(sphere_zone(10.0), NearOnly())
        self.assertEqual(bare["bare_pct"], 100.0)

    def test_familia_de_hueso(self) -> None:
        self.assertEqual(clearance.bone_family("spine_04"), "spine")
        self.assertEqual(clearance.bone_family("thigh_twistCor_02_l"), "thigh")
        self.assertEqual(clearance.bone_family("index_03_l"), "other")

    def test_plantilla_de_top_deja_fuera_escote_y_brazos(self) -> None:
        n = (0.0, 1.0, 0.0)
        samples = [
            ((0.0, 10.0, 120.0), n, "spine"),
            ((0.0, 10.0, 141.0), n, "spine"),  # franja del escote: fuera
            ((30.0, 0.0, 120.0), n, "upperarm"),  # brazo: fuera
            ((0.0, 10.0, 90.0), n, "pelvis"),  # otra familia: fuera
        ]
        zone = clearance.template_zone(samples, "top", (96.0, 144.0))
        self.assertEqual([p for p, _ in zone], [(0.0, 10.0, 120.0)])

    def test_plantilla_de_pantalon_incluye_pelvis_y_muslos(self) -> None:
        n = (0.0, 1.0, 0.0)
        samples = [((0.0, 0.0, 85.0), n, "pelvis"), ((8.0, 0.0, 80.0), n, "thigh"), ((8.0, 0.0, 50.0), n, "calf")]
        zone = clearance.template_zone(samples, "pants", (74.0, 109.0))
        self.assertEqual(len(zone), 2)


if __name__ == "__main__":
    unittest.main()
