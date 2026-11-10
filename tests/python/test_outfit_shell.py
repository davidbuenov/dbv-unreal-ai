# =============================================================================
# DBV Unreal AI — Tests de la carcasa del cuerpo (parte pura)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de ``dbv_unreal_ai.outfit.shell`` sin Unreal: orientación, escala, zona y avisos.

La parte que habla con GeometryScript se prueba en vivo (S116i).

@spec REQ-98
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.outfit import shell


def ring(n: int) -> list[set[int]]:
    """Vecinos de un anillo de ``n`` vértices."""
    return [{(i - 1) % n, (i + 1) % n} for i in range(n)]


class TestShellPure(unittest.TestCase):
    """Reglas de REQ-98 que no dependen del motor."""

    def test_orientacion_gira_si_el_ancho_va_en_y(self) -> None:
        # Medido en la camiseta de tirantes de Tripo: 44,9 en X y 53,0 en Y.
        plan = shell.orientation(44.9, 53.0, 34.2, 44.8)
        self.assertTrue(plan["rotate_90"])
        self.assertFalse(plan["flip_180"])

    def test_orientacion_da_la_vuelta_si_el_escote_alto_mira_delante(self) -> None:
        # El lado con el borde central más alto es la espalda (escote del top, tiro del pantalón).
        plan = shell.orientation(53.0, 44.9, 44.8, 34.2)
        self.assertFalse(plan["rotate_90"])
        self.assertTrue(plan["flip_180"])

    def test_escala_deja_la_prenda_algo_mas_ancha_que_el_cuerpo(self) -> None:
        scale = shell.fit_scale(52.5, 36.4, "top")
        self.assertAlmostEqual(scale * 52.5, 36.4 * shell.WIDTH_EASE["top"], places=6)
        self.assertEqual(shell.fit_scale(0.0, 36.4, "top"), 1.0)

    def test_el_top_queda_por_fuera_del_pantalon(self) -> None:
        # REQ-98, paso 7: en cualquier orden de equipado, la cinturilla no atraviesa el top.
        self.assertGreater(shell.shell_offset("top"), shell.shell_offset("pants"))

    def test_holgura_pedida_se_acota(self) -> None:
        self.assertEqual(shell.shell_offset("top", 0.0), shell.MIN_OFFSET_CM)
        self.assertEqual(shell.shell_offset("pants", 99.0), shell.MAX_OFFSET_CM)
        self.assertEqual(shell.shell_offset("top", 1.4), 1.4)

    def test_sin_mangas_el_brazo_queda_fuera(self) -> None:
        self.assertNotIn("upperarm", shell.eligible_families("top", sleeved=False))
        self.assertIn("upperarm", shell.eligible_families("top", sleeved=True))
        self.assertNotIn("upperarm", shell.eligible_families("pants", sleeved=True))

    def test_el_pantalon_siempre_tapa_la_entrepierna(self) -> None:
        self.assertIn("pelvis", shell.FORCED_FAMILIES["pants"])
        self.assertIn("pelvis", shell.eligible_families("pants", sleeved=False))

    def test_suavizado_por_mayoria_rellena_huecos_sueltos(self) -> None:
        covered = [True] * 10
        covered[4] = False
        self.assertEqual(shell.majority_smooth(covered, ring(10)), [True] * 10)

    def test_suavizado_por_mayoria_quita_puntos_sueltos(self) -> None:
        covered = [False] * 10
        covered[4] = True
        self.assertEqual(shell.majority_smooth(covered, ring(10)), [False] * 10)

    def test_espejo_marca_el_lado_contrario(self) -> None:
        covered = [True, False, False, False]
        mirror = [(3,), (2,), (1,), (0,)]
        self.assertEqual(shell.mirror_zone(covered, mirror), [True, False, False, True])

    def test_aviso_de_prenda_holgada(self) -> None:
        self.assertFalse(shell.looseness([1.0, 2.0, 3.0])["loose"])
        loose = shell.looseness([5.0, 6.0, 7.0])
        self.assertTrue(loose["loose"])
        self.assertEqual(loose["median_cm"], 6.0)
        self.assertFalse(shell.looseness([])["loose"])

    def test_interior_deja_fuera_el_borde_de_la_zona(self) -> None:
        covered = [False, True, True, True, True, True, False, False]
        inner = shell.interior(covered, ring(8), rings=1)
        self.assertEqual(inner, [False, False, True, True, True, False, False, False])

    def test_tipos_sin_carcasa_dan_error_claro(self) -> None:
        result = shell.build_garment_shell(None, None, "skirt", "/Game/X", "SK_X")
        self.assertFalse(result["success"])
        self.assertIn("top y pants", result["error"])


if __name__ == "__main__":
    unittest.main()
