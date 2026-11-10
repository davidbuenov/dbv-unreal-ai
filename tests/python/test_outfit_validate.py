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

    def test_measure_body_clearance_sin_editor_es_error_y_no_inventa_valores(self) -> None:
        """Fuera de Unreal no se puede medir: ``error`` con motivo, sin holguras fijas (REJECTED-13)."""
        res = validate.measure_body_clearance("BP_Pablo", "top")
        self.assertEqual(res["result"], "error")
        self.assertFalse(res["ok"])
        self.assertEqual(res["clearance"], {"measured_vertices": 0, "over_hidden_skin_vertices": 0})
        self.assertIn("No se pudo medir", res["warnings"][0])
        return None

    def test_measure_body_clearance_falda_no_se_da_por_buena_sin_medir(self) -> None:
        """El atajo «falda con cuerpo recortado = sin penetración» ya no existe."""
        with (
            patch.object(validate, "HAS_UNREAL", True),
            patch.object(validate, "unreal", MagicMock()),
            patch.object(validate, "_find_level_actor", return_value=MagicMock()),
        ):
            res = validate.measure_body_clearance("BP_Victoria", "skirt", body_trimmed=True)
        self.assertEqual(res["result"], "error")
        self.assertIn("tela", res["warnings"][0])
        return None

    def test_measure_body_clearance_sin_el_actor_es_error(self) -> None:
        with patch.object(validate, "HAS_UNREAL", True), patch.object(validate, "_find_level_actor", return_value=None):
            res = validate.measure_body_clearance("NoExiste", "pants")
        self.assertEqual(res["result"], "error")
        self.assertIn("NoExiste", res["warnings"][0])
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
        """Sin capturas reales no se devuelven rutas de ficheros que no existen. @spec REQ-89"""
        with patch.object(validate, "_find_level_actor", return_value=None):
            caps = validate.capture_three_angles("BP_Pablo", output_dir="Saved/Captures")
        self.assertEqual(caps["files"], {})
        self.assertFalse(caps["ok"])
        self.assertTrue(caps["errors"])
        self.assertTrue(caps["folder"].endswith("Captures"))
        self.assertEqual([name for name, _ in validate.CAPTURE_VIEWS], ["front", "side", "back"])
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
        mock_actor = MagicMock(mock_cloth_ok=True, mock_leader_linked=True, mock_physics_ok=True)
        with (
            patch.object(validate, "_find_level_actor", return_value=mock_actor),
            patch.object(validate, "compare_open_borders", return_value={"ok": True, "delta": 0, "warnings": []}),
            patch.object(validate, "measure_body_clearance", return_value={"result": "ok", "ok": True, "warnings": [], "clearance": {"measured_vertices": 100}}),
            patch.object(validate, "capture_three_angles", return_value={"files": {"front": "f.png", "side": "s.png", "back": "b.png"}, "folder": "x", "errors": [], "ok": True}),
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
        mock_actor = MagicMock(mock_cloth_ok=True, mock_leader_linked=True, mock_physics_ok=True)
        with (
            patch.object(validate, "_find_level_actor", return_value=mock_actor),
            patch.object(validate, "compare_open_borders", return_value={"ok": False, "delta": 40, "warnings": ["Minor border edge increase"]}),
            patch.object(validate, "measure_body_clearance", return_value={"result": "ok", "ok": True, "warnings": [], "clearance": {"measured_vertices": 100}}),
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
        mock_actor = MagicMock(mock_cloth_ok=True, mock_leader_linked=False, mock_physics_ok=True)
        with (
            patch.object(validate, "_find_level_actor", return_value=mock_actor),
            patch.object(validate, "compare_open_borders", return_value={"ok": True, "delta": 0, "warnings": []}),
            patch.object(validate, "measure_body_clearance", return_value={"result": "ok", "ok": True, "warnings": [], "clearance": {"measured_vertices": 100}}),
        ):
            report = validate.validate_outfit(
                actor_label="BP_Victoria",
                garment_kind="skirt",
            )
            self.assertFalse(report["success"])
            self.assertEqual(report["result"], "error")
        return None


    def test_validate_outfit_con_holgura_en_error_nunca_es_ok(self) -> None:
        """La camiseta de caja y los shorts con hueco del 2026-11-10 no pueden salir como ``ok``. @spec REQ-89"""
        bad = {
            "result": "error",
            "ok": False,
            "warnings": ["El 27.8 % de la prenda está dentro de la piel (hasta 8.7 cm): el cuerpo la atraviesa."],
            "clearance": {"measured_vertices": 7330, "inside_pct": 27.78},
        }
        with (
            patch.object(validate, "compare_open_borders", return_value={"ok": True, "delta": 0, "warnings": []}),
            patch.object(validate, "measure_body_clearance", return_value=bad),
        ):
            report = validate.validate_outfit(actor_label="Pablo_Deportista", garment_kind="top", capture_angles=False)
        self.assertEqual(report["result"], "error")
        self.assertIn("dentro de la piel", " ".join(report["warnings"]))
        return None


if __name__ == "__main__":
    unittest.main()

