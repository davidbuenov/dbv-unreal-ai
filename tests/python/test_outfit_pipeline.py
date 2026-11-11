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

import contextlib
import json
import shutil
import sys
import types
import unittest
from collections.abc import Iterator
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
            patch.object(pipeline.character, "prepare_character_copy", return_value={"success": True, "copy_bp_path": "/Game/BP_Pablo_DBV", "original_bp_path": "/Game/BP_Pablo"}),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline.skin, "prepare_clean_skin", return_value={"success": True}),
            patch.object(pipeline.skin, "trim_body_mesh_under_garment", return_value={"success": True}),
            patch.object(pipeline, "_equip_bones_garment", return_value={"success": True, "skeletal_mesh": "/Game/SK_Chaqueta_Rigged"}),
            patch.object(pipeline.validate, "validate_outfit", return_value={"success": True, "result": "ok", "warnings": [], "screenshots": {}}),
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
            patch.object(pipeline.character, "prepare_character_copy", return_value={"success": True, "copy_bp_path": "/Game/BP_Victoria_DBV", "original_bp_path": "/Game/BP_Victoria"}),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline.skin, "prepare_clean_skin", return_value={"success": True}),
            patch.object(pipeline.skin, "trim_body_mesh_under_garment", return_value={"success": True}),
            patch.object(pipeline.cloth, "setup_cloth_for_character", return_value={"success": True, "cloth_asset": "/Game/CA_Falda", "sim_sm_path": "/Game/SM_Sim"}),
            patch.object(pipeline.validate, "validate_outfit", return_value={"success": True, "result": "ok", "warnings": [], "screenshots": {}}),
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
            patch.object(pipeline.character, "prepare_character_copy", return_value={"success": True, "copy_bp_path": "/Game/BP_Pablo_DBV", "original_bp_path": "/Game/BP_Pablo"}),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline.skin, "prepare_clean_skin", return_value={"success": True}),
            patch.object(pipeline.skin, "trim_body_mesh_under_garment", return_value={"success": True}),
            patch.object(pipeline, "_equip_bones_garment", return_value={"success": True, "skeletal_mesh": "/Game/SK_Pants_Rigged"}),
            patch.object(pipeline.validate, "validate_outfit", return_value={"success": True, "result": "ok", "warnings": [], "screenshots": {}}),
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
            patch.object(pipeline.character, "prepare_character_copy", return_value={"success": True, "copy_bp_path": "/Game/BP_Pablo_DBV", "original_bp_path": "/Game/BP_Pablo"}),
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

    def test_dress_metahuman_step_revierte_al_original_si_paso_falla(self) -> None:
        """@spec REQ-85, REQ-88 — verifica rollback al Blueprint original si un paso de vestimenta falla."""
        mock_actor = MagicMock()
        mock_actor.get_actor_label.return_value = "BP_David3_MH"

        with (
            patch.object(pipeline, "_find_level_actor", return_value=mock_actor),
            patch.object(pipeline.character, "prepare_character_copy", return_value={"success": True, "copy_bp_path": "/Game/BP_David3_MH_DBV", "original_bp_path": "/Game/BP_David3_MH"}),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline.skin, "prepare_clean_skin", return_value={"success": True}),
            patch.object(pipeline, "_equip_bones_garment", return_value={"success": False, "error": "GeometryScript error"}),
            patch.object(pipeline.character, "is_character_copy", return_value=True),
            patch.object(pipeline.character, "revert_to_original") as mock_revert,
        ):
            start = pipeline.dress_metahuman_start(
                actor_label="BP_David3_MH",
                garment_source="/Game/DBV/Generated/Camiseta",
                garment_kind="top",
            )
            job_id = start["job_id"]
            # Paso 0: prepare_character
            pipeline.dress_metahuman_step(job_id)
            # Paso 1: apply_slot_rules
            pipeline.dress_metahuman_step(job_id)
            # Paso 2: prepare_skin
            pipeline.dress_metahuman_step(job_id)
            # Paso 3: fit_and_equip (falla)
            s3 = pipeline.dress_metahuman_step(job_id)
            self.assertFalse(s3["success"])
            self.assertIn("GeometryScript error", s3["error"])
            mock_revert.assert_called_once_with("BP_David3_MH")

        return None

    @staticmethod
    @contextlib.contextmanager
    def _fake_editor(**functions: MagicMock) -> Iterator[types.SimpleNamespace]:
        """Sustituye ``dbv_unreal_ai.editor`` (que necesita el motor) por un módulo con ``functions``."""
        import dbv_unreal_ai

        fake = types.SimpleNamespace(**functions)
        with patch.dict(sys.modules, {"dbv_unreal_ai.editor": fake}), patch.object(dbv_unreal_ai, "editor", fake, create=True):
            yield fake

    def _run_until_fit(self, kind: str, source: str = "/Game/DBV/Generated/Prenda") -> str:
        """Arranca un trabajo y ejecuta los tres pasos previos a ``fit_and_equip``."""
        start = pipeline.dress_metahuman_start(actor_label="BP_Victoria", garment_source=source, garment_kind=kind)
        self.assertTrue(start["success"], start)
        for _ in range(3):
            self.assertTrue(pipeline.dress_metahuman_step(start["job_id"])["success"])
        return start["job_id"]

    def test_reglas_de_ranura_reciben_el_actor_y_sus_mallas_previas(self) -> None:
        """@spec REQ-84, REQ-88 — apply_slot_rules trabaja sobre el actor del nivel, no sobre una ruta de Blueprint."""
        mock_actor = MagicMock()
        mock_actor.get_actor_label.return_value = "BP_Victoria"
        slots = {"Torso": "/Game/SK_Top.SK_Top", "Legs": "/Game/SK_Jeans.SK_Jeans"}
        prep = {"success": True, "reused": True, "copy_bp_path": "/Game/BP_Victoria_DBV", "original_bp_path": "/Game/BP_Victoria", "slots": slots}

        with (
            patch.object(pipeline, "_find_level_actor", return_value=mock_actor),
            patch.object(pipeline.character, "prepare_character_copy", return_value=prep),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}) as mock_rules,
        ):
            start = pipeline.dress_metahuman_start(actor_label="BP_Victoria", garment_source="/Game/SM_Vestido", garment_kind="dress")
            pipeline.dress_metahuman_step(start["job_id"])
            self.assertTrue(pipeline.dress_metahuman_step(start["job_id"])["success"])

        mock_rules.assert_called_once_with(mock_actor, "dress", slots)

    def test_paso_de_tela_llama_al_orquestador_real_con_el_blueprint_de_la_copia(self) -> None:
        """@spec REQ-87, REQ-88 — fit_and_equip encaja con la firma real de setup_cloth_for_character.

        Regresión de 2026-11-11: el paso pasaba ``dest_folder`` a una función que no lo admitía y, además,
        leía una clave que ``prepare_character_copy`` no devuelve, así que enviaba la etiqueta del actor
        como ruta de Blueprint. Ninguna prenda ``skirt`` o ``dress`` se podía equipar.
        """
        mock_actor = MagicMock()
        mock_actor.get_actor_label.return_value = "BP_Victoria"
        prep = {"success": True, "reused": False, "copy_bp_path": "/Game/DBV/BP_Victoria_DBV", "original_bp_path": "/Game/BP_Victoria"}

        for kind in ("skirt", "dress"):
            with (
                patch.object(pipeline, "_find_level_actor", return_value=mock_actor),
                patch.object(pipeline.character, "prepare_character_copy", return_value=prep),
                patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
                patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
                patch.object(pipeline, "_cloth_body_assets", return_value=("/Game/MH/f_med_nrw_body", "/Game/MH/f_med_nrw_body_Physics")),
                patch.object(pipeline.cloth, "setup_cloth_for_character", wraps=pipeline.cloth.setup_cloth_for_character) as spy,
            ):
                job_id = self._run_until_fit(kind, source="/Game/DBV/Generated/SM_Prenda.SM_Prenda")
                step = pipeline.dress_metahuman_step(job_id)

            self.assertTrue(step["success"], step)
            kwargs = spy.call_args.kwargs
            self.assertEqual(kwargs["actor_or_bp_path"], "/Game/DBV/BP_Victoria_DBV")
            self.assertEqual(kwargs["render_sm_path"], "/Game/DBV/Generated/SM_Prenda")
            self.assertEqual(kwargs["skeletal_mesh_path"], "/Game/MH/f_med_nrw_body")
            self.assertEqual(kwargs["physics_asset_path"], "/Game/MH/f_med_nrw_body_Physics")
            self.assertEqual(kwargs["dest_folder"], "/Game/DBV/MetaHumanOutfits")
            pipeline._ACTIVE_JOB_ID = None

    def test_paso_de_tela_importa_el_fichero_y_exige_malla_estatica(self) -> None:
        """@spec REQ-87, REQ-88 — una prenda en disco se importa antes; si no da un StaticMesh, el paso falla con el motivo."""
        ok = json.dumps({"success": True, "primary": "/Game/DBV/MetaHumanOutfits/Vestido.Vestido", "kind": "static"})
        rigged = json.dumps({"success": True, "primary": "/Game/DBV/MetaHumanOutfits/SK_Vestido", "kind": "skeletal"})
        failed = json.dumps({"success": False, "error": "the import produced no assets"})

        with patch("os.path.isfile", return_value=True), self._fake_editor(import_model=MagicMock(return_value=ok)) as fake:
            self.assertEqual(
                pipeline._resolve_cloth_render_mesh("C:/lib/vestido.glb", "/Game/DBV/MetaHumanOutfits", "Vestido"),
                ("/Game/DBV/MetaHumanOutfits/Vestido", None),
            )
            fake.import_model.assert_called_once_with("C:/lib/vestido.glb", "/Game/DBV/MetaHumanOutfits", "Vestido")
        with patch("os.path.isfile", return_value=True), self._fake_editor(import_model=MagicMock(return_value=rigged)):
            mesh, err = pipeline._resolve_cloth_render_mesh("C:/lib/vestido.fbx", "/Game/X", "Vestido")
            self.assertIsNone(mesh)
            self.assertIn("needs a StaticMesh", err)
        with patch("os.path.isfile", return_value=True), self._fake_editor(import_model=MagicMock(return_value=failed)):
            mesh, err = pipeline._resolve_cloth_render_mesh("C:/lib/vestido.glb", "/Game/X", "Vestido")
            self.assertIsNone(mesh)
            self.assertIn("the import produced no assets", err)
        mesh, err = pipeline._resolve_cloth_render_mesh("vestido_azul_marino-1794400441413", "/Game/X", "Vestido")
        self.assertIsNone(mesh)
        self.assertIn("local file or an Unreal asset path", err)

    def test_fallo_en_copia_ya_vestida_no_revierte_al_original(self) -> None:
        """@spec REQ-85, REQ-88 — si el personaje ya era una copia vestida, un fallo no destruye sus prendas anteriores."""
        mock_actor = MagicMock()
        mock_actor.get_actor_label.return_value = "BP_David3_MH"
        slots = {"Torso": "/Game/Fable/SK_Tirantes.SK_Tirantes", "Legs": "/Game/Fable/SK_Shorts.SK_Shorts"}
        prep = {"success": True, "reused": True, "copy_bp_path": "/Game/BP_David3_MH_DBV", "original_bp_path": "/Game/BP_David3_MH", "slots": slots}

        with (
            patch.object(pipeline, "_find_level_actor", return_value=mock_actor),
            patch.object(pipeline.character, "prepare_character_copy", return_value=prep),
            patch.object(pipeline.character, "find_sequence_warnings", return_value=[]),
            patch.object(pipeline.character, "apply_slot_rules", return_value={"success": True}),
            patch.object(pipeline, "_equip_bones_garment", return_value={"success": False, "error": "GeometryScript error"}),
            patch.object(pipeline, "_restore_slot_meshes", return_value=["Legs"]) as mock_restore,
            patch.object(pipeline.character, "revert_to_original") as mock_revert,
        ):
            start = pipeline.dress_metahuman_start(actor_label="BP_David3_MH", garment_source="/Game/SM_Pantalon", garment_kind="pants")
            for _ in range(3):
                pipeline.dress_metahuman_step(start["job_id"])
            failed = pipeline.dress_metahuman_step(start["job_id"])

        self.assertFalse(failed["success"])
        mock_revert.assert_not_called()
        mock_restore.assert_called_once_with(mock_actor, slots)
        self.assertEqual(failed["rollback"], {"action": "restored_slots", "success": True, "restored_slots": ["Legs"]})

    def test_excepcion_al_equipar_por_huesos_es_un_fallo_no_un_exito_simulado(self) -> None:
        """@spec REQ-88 — si equipar lanza una excepción, el paso falla; antes devolvía éxito con una ruta inventada."""
        with self._fake_editor(equip_metahuman_outfit=MagicMock(side_effect=RuntimeError("boom"))):
            res = pipeline._equip_bones_garment("BP_Pablo", "/Game/SM_Top", "Torso", "/Game/DBV", "Top", 1.05, 0.0)

        self.assertFalse(res["success"])
        self.assertIn("RuntimeError: boom", res["error"])
        self.assertNotIn("skeletal_mesh", res)


if __name__ == "__main__":
    unittest.main()
