# =============================================================================
# DBV Unreal AI — Tests unitarios del pipeline determinista de vestimenta
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests unitarios de ``dbv_unreal_ai.outfit.pipeline``.

@spec REQ-88, REQ-83, REQ-84
"""

from __future__ import annotations

import json
import shutil
import sys
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.outfit import pipeline


class TestOutfitPipeline(unittest.TestCase):
    """Batería de pruebas para el pipeline determinista unificado de vestimenta."""

    def setUp(self) -> None:
        """Limpia el estado y directorio temporal de pruebas."""
        pipeline._ACTIVE_JOB_ID = None
        saved_dir = Path("Saved") / "DBVUnrealAI" / "outfits"
        if saved_dir.exists():
            shutil.rmtree(str(saved_dir), ignore_errors=True)
        return None

    def tearDown(self) -> None:
        """Limpia tras cada prueba."""
        pipeline._ACTIVE_JOB_ID = None
        saved_dir = Path("Saved") / "DBVUnrealAI" / "outfits"
        if saved_dir.exists():
            shutil.rmtree(str(saved_dir), ignore_errors=True)
        return None

    def test_dress_metahuman_valida_actor_requerido(self) -> None:
        """Verifica que no se puede vestir sin un actor explícito (REQ-83)."""
        res = pipeline.dress_metahuman_start(
            actor_label="",
            garment_source="/Game/DBV/Generated/Top",
            garment_kind="top",
        )
        self.assertFalse(res["success"])
        self.assertIn("No target MetaHuman was chosen", res["error"])
        self.assertIn("available", res)
        return None

    def test_dress_metahuman_valida_tipo_de_prenda(self) -> None:
        """Verifica el rechazo de tipos de prenda no reconocidos (REQ-84)."""
        res = pipeline.dress_metahuman_start(
            actor_label="BP_Pablo",
            garment_source="/Game/DBV/Generated/Sombrero",
            garment_kind="hat",
        )
        self.assertFalse(res["success"])
        self.assertIn("Invalid garment_kind", res["error"])
        return None

    def test_dress_metahuman_mapea_slot_heredado(self) -> None:
        """Verifica la compatibilidad hacia atrás con garment_slot (REQ-84)."""
        with patch.object(pipeline, "_find_level_actor", return_value=MagicMock()):
            res = pipeline.dress_metahuman_start(
                actor_label="BP_Pablo",
                garment_source="/Game/DBV/Generated/Pantalones",
                garment_kind="Legs",  # Slot heredado
            )
            self.assertTrue(res["success"])
            self.assertEqual(res["garment_kind"], "pants")
            self.assertEqual(res["method"], "bones")
        return None

    def test_dress_metahuman_regla_trabajo_unico(self) -> None:
        """Verifica la restricción D65: un solo trabajo activo por editor a la vez."""
        with patch.object(pipeline, "_find_level_actor", return_value=MagicMock()):
            res1 = pipeline.dress_metahuman_start(
                actor_label="BP_Pablo",
                garment_source="/Game/DBV/Generated/Top",
                garment_kind="top",
            )
            self.assertTrue(res1["success"])
            self.assertIsNotNone(res1["job_id"])

            # Segundo intento concurrente sin force debe fallar
            res2 = pipeline.dress_metahuman_start(
                actor_label="BP_Victoria",
                garment_source="/Game/DBV/Generated/Falda",
                garment_kind="skirt",
                force=False,
            )
            self.assertFalse(res2["success"])
            self.assertIn("Another outfit job is currently in progress", res2["error"])

            # Con force=True debe permitirse
            res3 = pipeline.dress_metahuman_start(
                actor_label="BP_Victoria",
                garment_source="/Game/DBV/Generated/Falda",
                garment_kind="skirt",
                force=True,
            )
            self.assertTrue(res3["success"])
            self.assertNotEqual(res1["job_id"], res3["job_id"])
        return None

    def test_dress_metahuman_rechaza_durante_pie(self) -> None:
        """Verifica que el pipeline rechaza ejecutarse durante PIE."""
        with patch.object(pipeline, "_in_play", return_value=True):
            res = pipeline.dress_metahuman_start(
                actor_label="BP_Pablo",
                garment_source="/Game/DBV/Generated/Top",
                garment_kind="top",
            )
            self.assertFalse(res["success"])
            self.assertIn("Play-In-Editor (PIE)", res["error"])
        return None

    def test_dress_metahuman_step_avanza_pasos_huesos(self) -> None:
        """Verifica la ejecución paso a paso para prenda por huesos (top)."""
        mock_actor = MagicMock()
        mock_actor.get_actor_label.return_value = "BP_Pablo"

        with (
            patch.object(pipeline, "_find_level_actor", return_value=mock_actor),
            patch.object(pipeline.character, "prepare_character_copy", return_value={"success": True, "copy_blueprint_path": "/Game/BP_Pablo_DBV", "original_blueprint_path": "/Game/BP_Pablo"}),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline.skin, "prepare_clean_skin", return_value={"success": True}),
            patch.object(pipeline.skin, "trim_body_mesh_under_garment", return_value={"success": True}),
            patch.object(pipeline, "_equip_bones_garment", return_value={"success": True, "skeletal_mesh": "/Game/SK_Chaqueta_Rigged"}),
        ):
            start = pipeline.dress_metahuman_start(
                actor_label="BP_Pablo",
                garment_source="/Game/DBV/Generated/Chaqueta",
                garment_kind="top",
            )
            self.assertTrue(start["success"])
            job_id = start["job_id"]

            # Paso 0: prepare_character
            s0 = pipeline.dress_metahuman_step(job_id)
            self.assertTrue(s0["success"])
            self.assertEqual(s0["step_name"], "prepare_character")
            self.assertFalse(s0["done"])

            # Paso 1: apply_slot_rules
            s1 = pipeline.dress_metahuman_step(job_id)
            self.assertTrue(s1["success"])
            self.assertEqual(s1["step_name"], "apply_slot_rules")

            # Paso 2: prepare_skin
            s2 = pipeline.dress_metahuman_step(job_id)
            self.assertTrue(s2["success"])
            self.assertEqual(s2["step_name"], "prepare_skin")

            # Paso 3: fit_and_equip
            s3 = pipeline.dress_metahuman_step(job_id)
            self.assertTrue(s3["success"])
            self.assertEqual(s3["step_name"], "fit_and_equip")

            # Paso 4: finalize
            s4 = pipeline.dress_metahuman_step(job_id)
            self.assertTrue(s4["success"])
            self.assertEqual(s4["step_name"], "finalize")
            self.assertTrue(s4["done"])
            self.assertEqual(s4["progress"], 1.0)
            self.assertIsNotNone(s4["result"])
            self.assertEqual(s4["result"]["method"], "bones")
            self.assertEqual(s4["result"]["rigged_asset"], "/Game/SK_Chaqueta_Rigged")

        return None

    def test_dress_metahuman_step_avanza_pasos_tela(self) -> None:
        """Verifica la ejecución paso a paso para prenda con Chaos Cloth (skirt)."""
        mock_actor = MagicMock()
        mock_actor.get_actor_label.return_value = "BP_Victoria"

        with (
            patch.object(pipeline, "_find_level_actor", return_value=mock_actor),
            patch.object(pipeline.character, "prepare_character_copy", return_value={"success": True, "copy_blueprint_path": "/Game/BP_Victoria_DBV", "original_blueprint_path": "/Game/BP_Victoria"}),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline.skin, "prepare_clean_skin", return_value={"success": True}),
            patch.object(pipeline.skin, "trim_body_mesh_under_garment", return_value={"success": True}),
            patch.object(pipeline.cloth, "setup_cloth_for_character", return_value={"success": True, "cloth_asset": "/Game/CA_Falda", "sim_sm_path": "/Game/SM_Sim"}),
        ):
            start = pipeline.dress_metahuman_start(
                actor_label="BP_Victoria",
                garment_source="/Game/DBV/Generated/Falda",
                garment_kind="skirt",
            )
            self.assertTrue(start["success"])
            job_id = start["job_id"]
            self.assertEqual(start["method"], "cloth")

            # Ejecutar todos los pasos secuencialmente
            done = False
            last_res = None
            for _ in range(5):
                last_res = pipeline.dress_metahuman_step(job_id)
                self.assertTrue(last_res["success"])
                if last_res.get("done"):
                    done = True
                    break

            self.assertTrue(done)
            self.assertIsNotNone(last_res)
            self.assertEqual(last_res["result"]["method"], "cloth")
            self.assertEqual(last_res["result"]["cloth_asset"], "/Game/CA_Falda")

        return None

    def test_dress_metahuman_sincrono_completo(self) -> None:
        """Verifica la llamada síncrona completa dress_metahuman."""
        mock_actor = MagicMock()
        mock_actor.get_actor_label.return_value = "BP_Pablo"

        with (
            patch.object(pipeline, "_find_level_actor", return_value=mock_actor),
            patch.object(pipeline.character, "prepare_character_copy", return_value={"success": True, "copy_blueprint_path": "/Game/BP_Pablo_DBV", "original_blueprint_path": "/Game/BP_Pablo"}),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline.skin, "prepare_clean_skin", return_value={"success": True}),
            patch.object(pipeline.skin, "trim_body_mesh_under_garment", return_value={"success": True}),
            patch.object(pipeline, "_equip_bones_garment", return_value={"success": True, "skeletal_mesh": "/Game/SK_Pants_Rigged"}),
        ):
            raw = pipeline.dress_metahuman(
                actor_label="BP_Pablo",
                garment_source="/Game/DBV/Generated/Vaqueros",
                garment_kind="pants",
            )
            res = json.loads(raw)
            self.assertTrue(res["success"])
            self.assertEqual(res["actor"], "BP_Pablo")
            self.assertEqual(res["garment_kind"], "pants")
            self.assertEqual(res["rigged_asset"], "/Game/SK_Pants_Rigged")

        return None

    def test_dress_metahuman_status(self) -> None:
        """Verifica la consulta de estado con dress_metahuman_status."""
        with patch.object(pipeline, "_find_level_actor", return_value=MagicMock()):
            start = pipeline.dress_metahuman_start(
                actor_label="BP_Pablo",
                garment_source="/Game/DBV/Generated/Top",
                garment_kind="top",
            )
            job_id = start["job_id"]

            st = pipeline.dress_metahuman_status(job_id)
            self.assertTrue(st["success"])
            self.assertEqual(st["actor_label"], "BP_Pablo")
            self.assertEqual(st["status"], "in_progress")
            self.assertEqual(st["step_index"], 0)

        return None

    def test_dress_metahuman_step_falla_si_validacion_error(self) -> None:
        """Verifica que si la validación final falla (REQ-89), el trabajo no se da por bueno."""
        mock_actor = MagicMock()
        mock_actor.get_actor_label.return_value = "BP_Pablo"

        with (
            patch.object(pipeline, "_find_level_actor", return_value=mock_actor),
            patch.object(pipeline.character, "prepare_character_copy", return_value={"success": True, "copy_blueprint_path": "/Game/BP_Pablo_DBV", "original_blueprint_path": "/Game/BP_Pablo"}),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline.skin, "prepare_clean_skin", return_value={"success": True}),
            patch.object(pipeline.skin, "trim_body_mesh_under_garment", return_value={"success": True}),
            patch.object(pipeline, "_equip_bones_garment", return_value={"success": True, "skeletal_mesh": "/Game/SK_Chaqueta_Rigged"}),
            patch.object(pipeline.validate, "validate_outfit", return_value={"success": False, "result": "error", "warnings": ["Severe tear in mesh"]}),
        ):
            start = pipeline.dress_metahuman_start(
                actor_label="BP_Pablo",
                garment_source="/Game/DBV/Generated/Chaqueta",
                garment_kind="top",
            )
            job_id = start["job_id"]
            for _ in range(4):
                pipeline.dress_metahuman_step(job_id)

            # Paso 4: finalize con validación que da error
            s4 = pipeline.dress_metahuman_step(job_id)
            self.assertFalse(s4["success"])
            self.assertIn("Validation failed with error", s4["error"])
            st = pipeline.dress_metahuman_status(job_id)
            self.assertEqual(st["status"], "failed")

        return None


if __name__ == "__main__":
    unittest.main()
