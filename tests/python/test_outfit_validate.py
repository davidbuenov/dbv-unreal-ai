# =============================================================================
# DBV Unreal AI — Tests unitarios de validación de vestimenta
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests unitarios de ``dbv_unreal_ai.outfit.validate``.

@spec REQ-89
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.outfit import validate


class TestOutfitValidate(unittest.TestCase):
    """Batería de pruebas para la validación e informe de prendas de MetaHumans."""

    def test_count_open_borders_retorna_cero_sin_malla(self) -> None:
        """Verifica que sin malla válida retorna 0 sin excepciones."""
        self.assertEqual(validate.count_open_borders(None), 0)
        return None

    def test_compare_open_borders_detecta_aumento_de_agujeros(self) -> None:
        """Verifica que detecta roturas o bordes abiertos adicionales sobre el umbral."""
        mock_src = MagicMock(open_border_edges=10)
        mock_fit_clean = MagicMock(open_border_edges=15)
        clean_res = validate.compare_open_borders(mock_src, mock_fit_clean, max_allowed_delta=50)
        self.assertTrue(clean_res["ok"])
        self.assertEqual(clean_res["delta"], 5)
        self.assertEqual(len(clean_res["warnings"]), 0)

        # Malla con desgarro severo (+60 bordes abiertos)
        mock_fit_torn = MagicMock(open_border_edges=75)
        torn_res = validate.compare_open_borders(mock_src, mock_fit_torn, max_allowed_delta=50)
        self.assertFalse(torn_res["ok"])
        self.assertEqual(torn_res["delta"], 65)
        self.assertGreater(len(torn_res["warnings"]), 0)
        return None

    def test_measure_body_clearance_detecta_penetraciones(self) -> None:
        """Verifica que detecta penetraciones negativas y genera avisos."""
        # Actor con penetración simulada (-1.2 cm dentro del cuerpo)
        mock_actor_pen = MagicMock(mock_clearance=-1.2)
        res = validate.measure_body_clearance(mock_actor_pen, "top")
        self.assertFalse(res["ok"])
        self.assertTrue(res["penetration_detected"])
        self.assertLess(res["min_clearance_cm"], 0.0)
        self.assertGreater(len(res["warnings"]), 0)

        # Actor con holgura limpia (1.5 cm)
        mock_actor_clean = MagicMock(mock_clearance=1.5)
        res_clean = validate.measure_body_clearance(mock_actor_clean, "top")
        self.assertTrue(res_clean["ok"])
        self.assertFalse(res_clean["penetration_detected"])
        self.assertGreaterEqual(res_clean["min_clearance_cm"], 0.0)
        return None

    def test_measure_body_clearance_con_cuerpo_recortado(self) -> None:
        """Verifica que para falda/vestido con cuerpo recortado no hay penetración."""
        res = validate.measure_body_clearance(None, "skirt", body_trimmed=True)
        self.assertTrue(res["ok"])
        self.assertFalse(res["penetration_detected"])
        self.assertTrue(res["body_trimmed"])
        return None

    def test_check_cloth_linkage_valida_componente_y_leader_pose(self) -> None:
        """Verifica el enlace de ChaosClothComponent con Body y PhysicsAsset."""
        # Falda con tela correctamente configurada
        mock_actor_ok = MagicMock(mock_cloth_ok=True, mock_leader_linked=True, mock_physics_ok=True)
        res_ok = validate.check_cloth_linkage(mock_actor_ok, "skirt")
        self.assertTrue(res_ok["ok"])
        self.assertTrue(res_ok["leader_linked"])
        self.assertTrue(res_ok["physics_asset_ok"])

        # Falda sin enlace de leader pose a Body
        mock_actor_no_leader = MagicMock(mock_cloth_ok=True, mock_leader_linked=False, mock_physics_ok=True)
        res_no_leader = validate.check_cloth_linkage(mock_actor_no_leader, "skirt")
        self.assertFalse(res_no_leader["ok"])
        self.assertIn("leader pose is not linked", str(res_no_leader["issues"]))

        # Prenda por huesos
        res_bones = validate.check_cloth_linkage(MagicMock(), "top")
        self.assertTrue(res_bones["ok"])
        return None

    def test_capture_three_angles_genera_rutas_correctas(self) -> None:
        """Verifica que capture_three_angles genera los nombres para frente, lado y espalda."""
        with patch.object(validate, "_find_level_actor", return_value=None):
            caps = validate.capture_three_angles("BP_Pablo", output_dir="Saved/Captures")
            self.assertIn("front", caps)
            self.assertIn("side", caps)
            self.assertIn("back", caps)
            self.assertTrue(caps["front"].endswith("BP_Pablo_front.png"))
            self.assertTrue(caps["side"].endswith("BP_Pablo_side.png"))
            self.assertTrue(caps["back"].endswith("BP_Pablo_back.png"))
        return None

    def test_run_walk_validation_pie_limpieza_segura(self) -> None:
        """Verifica que run_walk_validation_pie retorna resultado y no lanza excepciones."""
        res = validate.run_walk_validation_pie(duration_seconds=1.0)
        self.assertTrue(res["success"])
        self.assertTrue(res["ok"])
        self.assertFalse(res["fell_out_of_world"])
        return None

    def test_validate_outfit_resultado_ok(self) -> None:
        """Verifica un informe con resultado 'ok' cuando todo está correcto."""
        mock_actor = MagicMock(mock_clearance=1.5, mock_cloth_ok=True, mock_leader_linked=True, mock_physics_ok=True)
        with (
            patch.object(validate, "_find_level_actor", return_value=mock_actor),
            patch.object(validate, "compare_open_borders", return_value={"ok": True, "delta": 0, "warnings": []}),
        ):
            report = validate.validate_outfit(
                actor_label="BP_Pablo",
                garment_kind="top",
                run_pie=False,
                capture_angles=True,
            )
            self.assertTrue(report["success"])
            self.assertEqual(report["result"], "ok")
            self.assertEqual(report["actor"], "BP_Pablo")
            self.assertEqual(len(report["warnings"]), 0)
            self.assertIn("screenshots", report)
        return None

    def test_validate_outfit_resultado_con_avisos(self) -> None:
        """Verifica resultado 'con_avisos' ante advertencias no críticas."""
        mock_actor = MagicMock(mock_clearance=0.3, mock_cloth_ok=True, mock_leader_linked=True, mock_physics_ok=True)
        with (
            patch.object(validate, "_find_level_actor", return_value=mock_actor),
            patch.object(validate, "compare_open_borders", return_value={"ok": False, "delta": 40, "warnings": ["Minor border edge increase"]}),
        ):
            report = validate.validate_outfit(
                actor_label="BP_Pablo",
                garment_kind="top",
            )
            self.assertTrue(report["success"])
            self.assertEqual(report["result"], "con_avisos")
            self.assertGreater(len(report["warnings"]), 0)
        return None

    def test_validate_outfit_resultado_error(self) -> None:
        """Verifica que fallos críticos (falta de leader pose o rotura de malla) producen 'error'."""
        mock_actor = MagicMock(mock_clearance=1.5, mock_cloth_ok=True, mock_leader_linked=False, mock_physics_ok=True)
        with (
            patch.object(validate, "_find_level_actor", return_value=mock_actor),
            patch.object(validate, "compare_open_borders", return_value={"ok": True, "delta": 0, "warnings": []}),
        ):
            report = validate.validate_outfit(
                actor_label="BP_Victoria",
                garment_kind="skirt",
            )
            self.assertFalse(report["success"])
            self.assertEqual(report["result"], "error")
        return None


if __name__ == "__main__":
    unittest.main()

