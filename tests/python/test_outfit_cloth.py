# =============================================================================
# DBV Unreal AI — Tests unitarios de simulación de tela con Chaos Cloth
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests unitarios de ``dbv_unreal_ai.outfit.cloth``.

@spec REQ-87
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.outfit import cloth


class TestOutfitCloth(unittest.TestCase):
    """Batería de pruebas para la gestión y configuración procedimental de Chaos Cloth."""

    def test_archetype_constants(self) -> None:
        """Verifica que las constantes de arquetipos y plantillas están bien definidas."""
        self.assertIn("f_med_nrw", cloth.ARCHETYPE_FULL_BODIES)
        self.assertIn("m_tal_nrw", cloth.ARCHETYPE_FULL_BODIES)
        self.assertIn("f_med_nrw", cloth.ARCHETYPE_PHYSICS)
        self.assertIn("m_tal_nrw", cloth.ARCHETYPE_PHYSICS)
        self.assertTrue(cloth.DEFAULT_TEMPLATE_CA.startswith("/DBVUnrealAI/Cloth/"))
        self.assertTrue(cloth.DEFAULT_TEMPLATE_DF.startswith("/DBVUnrealAI/Cloth/"))
        self.assertTrue(cloth.FALLBACK_TEMPLATE_CA.startswith("/Game/DBV/"))
        self.assertTrue(cloth.FALLBACK_TEMPLATE_DF.startswith("/Game/DBV/"))

    def test_generate_simulation_mesh_validation(self) -> None:
        """Valida los parámetros de entrada de generate_cloth_simulation_mesh."""
        # Parámetros vacíos
        res1 = cloth.generate_cloth_simulation_mesh("", "/Game/DBV", "SM_Sim")
        self.assertFalse(res1["success"])
        self.assertIn("obligatorios", res1["error"])

        # Triángulos no positivos
        res2 = cloth.generate_cloth_simulation_mesh("/Game/SM_Render", "/Game/DBV", "SM_Sim", target_triangles=0)
        self.assertFalse(res2["success"])
        self.assertIn("positivo", res2["error"])

    def test_generate_simulation_mesh_mock(self) -> None:
        """Comprueba la generación simulada de malla de simulación en entorno sin Unreal."""
        res = cloth.generate_cloth_simulation_mesh(
            render_sm_path="/Game/DBV/Runway/Cloth/SM_Falda_Render",
            dest_folder="/Game/DBV/Runway/Cloth",
            sim_name="SM_Falda_Sim",
            target_triangles=1200,
        )
        self.assertTrue(res["success"])
        self.assertEqual(res["sim_sm_path"], "/Game/DBV/Runway/Cloth/SM_Falda_Sim")
        self.assertEqual(res["triangle_count"], 1200)

    def test_create_cloth_asset_and_dataflow_validation(self) -> None:
        """Valida que create_cloth_asset_and_dataflow exija todos los argumentos necesarios."""
        res = cloth.create_cloth_asset_and_dataflow(
            dest_folder="",
            garment_name="Falda",
            render_sm_path="/Game/SM_Render",
            sim_sm_path="/Game/SM_Sim",
            skeletal_mesh_path="/Game/SK_Body",
            physics_asset_path="/Game/PA_Body",
        )
        self.assertFalse(res["success"])
        self.assertIn("obligatorios", res["error"])

        res2 = cloth.create_cloth_asset_and_dataflow(
            dest_folder="/Game/Cloth",
            garment_name="Falda",
            render_sm_path="",
            sim_sm_path="/Game/SM_Sim",
            skeletal_mesh_path="/Game/SK_Body",
            physics_asset_path="/Game/PA_Body",
        )
        self.assertFalse(res2["success"])
        self.assertIn("obligatorios", res2["error"])

    def test_create_cloth_asset_and_dataflow_mock(self) -> None:
        """Verifica la creación procedimental simulada de Cloth Asset y Dataflow."""
        res = cloth.create_cloth_asset_and_dataflow(
            dest_folder="/Game/DBV/MetaHumanOutfits/Falda",
            garment_name="FaldaCorta",
            render_sm_path="/Game/DBV/MetaHumanOutfits/Falda/SM_Render",
            sim_sm_path="/Game/DBV/MetaHumanOutfits/Falda/SM_Sim",
            skeletal_mesh_path="/Game/MetaHumans/Victoria/f_med_nrw_body",
            physics_asset_path="/Game/MetaHumans/Victoria/f_med_nrw_body_Physics",
        )
        self.assertTrue(res["success"])
        self.assertEqual(res["cloth_asset"], "/Game/DBV/MetaHumanOutfits/Falda/CA_FaldaCorta")
        self.assertEqual(res["dataflow_asset"], "/Game/DBV/MetaHumanOutfits/Falda/DF_FaldaCorta")

    def test_setup_cloth_component_in_blueprint_validation(self) -> None:
        """Verifica la validación de setup_cloth_component_in_blueprint."""
        res = cloth.setup_cloth_component_in_blueprint("", "/Game/CA_Asset")
        self.assertFalse(res["success"])
        self.assertIn("obligatorios", res["error"])

        res_ok = cloth.setup_cloth_component_in_blueprint(
            blueprint_path="/Game/BP_Victoria_DBV",
            cloth_asset_path="/Game/Cloth/CA_Vestido",
            slot_name="SkirtCloth",
        )
        self.assertTrue(res_ok["success"])
        self.assertEqual(res_ok["slot_name"], "SkirtCloth")

    def test_setup_cloth_for_character_garment_kind_filter(self) -> None:
        """Comprueba que Chaos Cloth solo se aplique a faldas y vestidos según REQ-87."""
        # Top rechazado para Chaos Cloth (va por huesos)
        res_top = cloth.setup_cloth_for_character(
            actor_or_bp_path="/Game/BP_Victoria_DBV",
            garment_kind="top",
            garment_name="TopHalter",
            render_sm_path="/Game/SM_Top",
        )
        self.assertFalse(res_top["success"])
        self.assertIn("faldas y vestidos", res_top["error"])

        # Pants rechazado para Chaos Cloth
        res_pants = cloth.setup_cloth_for_character(
            actor_or_bp_path="/Game/BP_Pablo_DBV",
            garment_kind="pants",
            garment_name="Pantalones",
            render_sm_path="/Game/SM_Pants",
        )
        self.assertFalse(res_pants["success"])
        self.assertIn("faldas y vestidos", res_pants["error"])

    def test_setup_cloth_for_character_full_pipeline_mock(self) -> None:
        """Verifica el flujo completo de simulación, assets y componentes para falda y vestido."""
        for kind in ("skirt", "dress"):
            res = cloth.setup_cloth_for_character(
                actor_or_bp_path="/Game/DBV/Runway/Cloth/BP_Victoria_DBV",
                garment_kind=kind,
                garment_name=f"Prenda_{kind}",
                render_sm_path=f"/Game/DBV/Runway/Cloth/SM_{kind}_Render",
            )
            self.assertTrue(res["success"])
            self.assertIsNotNone(res["sim_sm_path"])
            self.assertIsNotNone(res["cloth_asset"])
            self.assertIsNotNone(res["dataflow_asset"])
            self.assertEqual(res["blueprint_path"], "/Game/DBV/Runway/Cloth/BP_Victoria_DBV")

    def test_fallo_al_crear_la_tela_no_deja_restos(self) -> None:
        """@spec REQ-87, REQ-88 — si el Cloth Asset no se crea, se borran la simulación y los assets a medias."""
        from types import SimpleNamespace
        from unittest.mock import patch

        existing = {"/Game/Outfits/Vestido/SM_Vestido_Sim", "/Game/Outfits/Vestido/DF_Vestido"}
        eal = SimpleNamespace(
            does_asset_exist=lambda path: path in existing,
            delete_asset=lambda path: existing.discard(path) is None,
        )
        sim_ok = {"success": True, "sim_sm_path": "/Game/Outfits/Vestido/SM_Vestido_Sim"}
        ca_fail = {"success": False, "error": "Excepción configurando Dataflow: boom"}

        with (
            patch.object(cloth, "HAS_UNREAL", True),
            patch.object(cloth, "unreal", SimpleNamespace(EditorAssetLibrary=eal)),
            patch.object(cloth, "generate_cloth_simulation_mesh", return_value=sim_ok),
            patch.object(cloth, "create_cloth_asset_and_dataflow", return_value=ca_fail),
        ):
            res = cloth.setup_cloth_for_character(
                actor_or_bp_path="/Game/BP_Victoria_DBV",
                garment_kind="dress",
                garment_name="Vestido",
                render_sm_path="/Game/SM_Vestido",
                dest_folder="/Game/Outfits/Vestido",
            )

        self.assertFalse(res["success"])
        self.assertIn("boom", res["error"])
        self.assertEqual(existing, set())
        self.assertIsNone(res["sim_sm_path"])
        self.assertEqual(sorted(res["discarded_assets"]), ["/Game/Outfits/Vestido/DF_Vestido", "/Game/Outfits/Vestido/SM_Vestido_Sim"])


if __name__ == "__main__":
    unittest.main()
