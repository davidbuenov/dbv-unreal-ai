# =============================================================================
# DBV Unreal AI — Tests del camino de tela de un vestido: sujeción, validación y pipeline
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de lo que rodea al ajuste de un vestido: mapa de MaxDistance, holgura medida y pasos del pipeline.

@spec REQ-87, REQ-88, REQ-89
"""

from __future__ import annotations

import contextlib
import shutil
import sys
import unittest
from collections.abc import Iterator
from pathlib import Path
from unittest.mock import MagicMock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.outfit import cloth, pipeline, validate

FIT_OK = {"vertices": 15232, "inside_pct": 0.0, "min_cm": 1.0, "median_cm": 4.74, "p95_cm": 11.59}


@contextlib.contextmanager
def _saved_dir() -> Iterator[Path]:
    """Carpeta temporal para los estados de trabajo, borrada al terminar."""
    folder = Path(__file__).resolve().parent / "Saved" / "_cloth_dress_tests"
    folder.mkdir(parents=True, exist_ok=True)
    try:
        with patch.object(pipeline, "_get_saved_outfits_dir", return_value=str(folder)):
            yield folder
    finally:
        shutil.rmtree(folder, ignore_errors=True)


class TestClothMaxDistance(unittest.TestCase):
    """El mapa de sujeción de la plantilla se sustituye por el de la prenda (REQ-87)."""

    def _create(self, accepted: bool) -> tuple[dict, list[tuple[str, str, str]]]:
        calls: list[tuple[str, str, str]] = []

        def set_prop(_df: object, node: str, prop: str, value: str) -> bool:
            calls.append((node, prop, value))
            return accepted or node not in (cloth.MAX_DISTANCE_MAP_NODE, cloth.MAX_DISTANCE_NODE)

        fake = MagicMock()
        fake.DataflowEditorBlueprintLibrary.set_dataflow_node_property.side_effect = set_prop
        with patch.object(cloth, "HAS_UNREAL", True), patch.object(cloth, "unreal", fake):
            res = cloth.create_cloth_asset_and_dataflow(
                dest_folder="/Game/Outfits/Vestido",
                garment_name="Vestido",
                render_sm_path="/Game/Outfits/Vestido/SM_Vestido_Render",
                sim_sm_path="/Game/Outfits/Vestido/SM_Vestido_Sim",
                skeletal_mesh_path="/Game/Body",
                physics_asset_path="/Game/Body_Physics",
                max_distance_weights=[0.0, 0.5, 1.0],
                max_distance_cm=9.0,
            )
        return res, calls

    def test_escribe_pesos_por_vertice_y_recorrido(self) -> None:
        """@spec REQ-87"""
        res, calls = self._create(accepted=True)
        self.assertTrue(res["success"], res)
        self.assertIn((cloth.MAX_DISTANCE_MAP_NODE, "VertexWeights", "(0.0000,0.5000,1.0000)"), calls)
        self.assertIn((cloth.MAX_DISTANCE_NODE, "MaxDistance", "(Low=0.0,High=9.00)"), calls)

    def test_si_la_plantilla_no_admite_el_mapa_falla_en_vez_de_dejar_la_tela_suelta(self) -> None:
        """@spec REQ-87"""
        res, _ = self._create(accepted=False)
        self.assertFalse(res["success"])
        self.assertIn("MaxDistance", res["error"])


class TestClothValidation(unittest.TestCase):
    """La holgura de un vestido sale de las distancias que midió su ajuste (REQ-89)."""

    def test_informe_de_holgura_ok(self) -> None:
        """@spec REQ-89"""
        rep = validate.cloth_clearance_report("dress", FIT_OK)
        self.assertEqual(rep["result"], "ok")
        self.assertEqual(rep["clearance"]["measured_vertices"], 15232)
        self.assertEqual(rep["zone_source"], "cloth_fit")

    def test_un_vestido_que_el_cuerpo_atraviesa_es_error(self) -> None:
        """@spec REQ-89"""
        rep = validate.cloth_clearance_report("dress", {**FIT_OK, "inside_pct": 5.0, "min_cm": -2.0})
        self.assertEqual(rep["result"], "error")
        self.assertFalse(rep["ok"])

    def test_sin_medidas_del_ajuste_no_se_da_por_bueno(self) -> None:
        """@spec REQ-89"""
        self.assertEqual(validate.cloth_clearance_report("dress", {})["result"], "error")
        with (
            patch.object(validate, "HAS_UNREAL", True),
            patch.object(validate, "unreal", MagicMock()),
            patch.object(validate, "_find_level_actor", return_value=MagicMock()),
        ):
            sin_ajuste = validate.measure_body_clearance("Victoria", "dress")
            con_ajuste = validate.measure_body_clearance("Victoria", "dress", cloth_fit=FIT_OK)
        self.assertEqual(sin_ajuste["result"], "error")
        self.assertEqual(con_ajuste["result"], "ok")


class TestDressPipeline(unittest.TestCase):
    """El paso fit_and_equip de un vestido: ajuste, tela con sujeción y ranuras retiradas (REQ-84, REQ-87, REQ-88)."""

    def _run(self, fitted: dict, flip: bool | None = None) -> tuple[dict, MagicMock, MagicMock, MagicMock, MagicMock]:
        actor = MagicMock()
        actor.get_actor_label.return_value = "Victoria"
        prep = {"success": True, "copy_bp_path": "/Game/BP_Victoria_DBV", "original_bp_path": "/Game/BP_Victoria", "original_slots": {"Torso": "/Game/Top"}}
        cloth_ok = {"success": True, "cloth_asset": "/Game/Outfits/V/CA_V", "sim_sm_path": "/Game/Outfits/V/SM_V_Sim"}
        last: dict = {}
        with (
            _saved_dir(),
            patch.object(pipeline, "_find_level_actor", return_value=actor),
            patch.object(pipeline.character, "prepare_character_copy", return_value=prep),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}) as slots,
            patch.object(pipeline.tailor, "build_cloth_garment", return_value=fitted) as build,
            patch.object(pipeline.cloth, "setup_cloth_for_character", return_value=cloth_ok) as setup,
            patch.object(pipeline.validate, "validate_outfit", return_value={"success": True, "result": "ok", "warnings": [], "screenshots": {}}) as val,
        ):
            start = pipeline.dress_metahuman_start("Victoria", "/Game/DBV/Vestido", "dress", force=True, flip_orientation=flip)
            for _ in range(5):
                last = pipeline.dress_metahuman_step(start["job_id"])
                if not last.get("success") or last.get("done"):
                    break
        return last, slots, build, setup, val

    def test_vestido_se_ajusta_y_la_tela_recibe_sujecion_material_y_medidas(self) -> None:
        """@spec REQ-87, REQ-88, REQ-89"""
        fitted = {
            "success": True,
            "render_sm_path": "/Game/Outfits/V/SM_V_Render",
            "sim_sm_path": "/Game/Outfits/V/SM_V_Sim",
            "material_path": "/Game/Outfits/V/M_V",
            "max_distance_weights": [0.0, 1.0],
            "max_distance_cm": 9.0,
            "fit_info": FIT_OK,
        }
        last, slots, build, setup, val = self._run(fitted, flip=True)
        self.assertTrue(last.get("done"), last)
        self.assertTrue(build.call_args.kwargs["flip_orientation"])
        sent = setup.call_args.kwargs
        self.assertEqual(sent["render_sm_path"], "/Game/Outfits/V/SM_V_Render")
        self.assertEqual(sent["sim_sm_path"], "/Game/Outfits/V/SM_V_Sim")
        self.assertEqual(sent["material_path"], "/Game/Outfits/V/M_V")
        self.assertEqual(sent["max_distance_weights"], [0.0, 1.0])
        self.assertEqual(sent["actor_or_bp_path"], "/Game/BP_Victoria_DBV")
        self.assertEqual(slots.call_count, 2, "compilar el Blueprint devuelve la ropa a las ranuras: se retira otra vez")
        self.assertEqual(val.call_args.kwargs["cloth_fit"], FIT_OK)

    def test_si_la_tela_falla_se_borran_las_mallas_del_ajuste(self) -> None:
        """@spec REQ-87, REQ-88 — un intento fallido no deja SM_*_Render ni SM_*_Sim sueltos."""
        fitted = {"success": True, "render_sm_path": "/Game/V/SM_V_Render", "sim_sm_path": "/Game/V/SM_V_Sim", "fit_info": FIT_OK}
        actor = MagicMock()
        prep = {"success": True, "copy_bp_path": "/Game/BP_Victoria_DBV", "original_bp_path": "/Game/BP_Victoria"}
        with (
            _saved_dir(),
            patch.object(pipeline, "_find_level_actor", return_value=actor),
            patch.object(pipeline.character, "prepare_character_copy", return_value=prep),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline.tailor, "build_cloth_garment", return_value=fitted),
            patch.object(pipeline.cloth, "setup_cloth_for_character", return_value={"success": False, "error": "boom"}),
            patch.object(pipeline.cloth, "_discard_assets", return_value=["/Game/V/SM_V_Sim", "/Game/V/SM_V_Render"]) as discard,
            patch.object(pipeline, "_rollback_failed_job", return_value={"action": "none"}),
        ):
            start = pipeline.dress_metahuman_start("Victoria", "/Game/DBV/Vestido", "dress", force=True)
            for _ in range(4):
                last = pipeline.dress_metahuman_step(start["job_id"])
                if not last.get("success"):
                    break
        self.assertFalse(last["success"])
        discard.assert_called_once_with(["/Game/V/SM_V_Sim", "/Game/V/SM_V_Render"])

    def test_si_el_ajuste_falla_no_se_monta_la_tela(self) -> None:
        """@spec REQ-87, REQ-88"""
        last, _, _, setup, _ = self._run({"success": False, "error": "numpy no está disponible"})
        self.assertFalse(last.get("success"))
        self.assertIn("numpy", last["error"])
        setup.assert_not_called()


if __name__ == "__main__":
    unittest.main()
