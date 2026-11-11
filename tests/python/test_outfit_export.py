# =============================================================================
# DBV Unreal AI — Tests de exportación y devolución de prendas a la Biblioteca
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de exportación de prendas optimizadas desde MetaHumans hacia la Biblioteca de la app.

@spec REQ-100
"""

from __future__ import annotations

import json
import os
import shutil
import sys
import tempfile
import types
import unittest
from pathlib import Path
from typing import Any
from unittest.mock import patch

# Añadir ruta del plugin Python
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

import unreal

from dbv_unreal_ai import editor, metahuman


class MockSkeletalMesh:
    """Malla esquelética simulada."""

    def __init__(self, name: str = "SK_TestGarment", path_name: str = "/Game/DBV/MetaHumanOutfits/SK_TestGarment") -> None:
        self._name = name
        self._path_name = path_name

    def get_name(self) -> str:
        return self._name

    def get_path_name(self) -> str:
        return self._path_name


class MockComponent:
    """Componente simulado de MetaHuman."""

    def __init__(self, name: str, skm: MockSkeletalMesh | None = None) -> None:
        self.name = name
        self._skm = skm

    def get_name(self) -> str:
        return self.name

    def get_skeletal_mesh_asset(self) -> MockSkeletalMesh | None:
        return self._skm

    def get_material(self, index: int) -> Any:  # noqa: ANN401
        return None


class MockMetaHumanActor:
    """Actor MetaHuman simulado."""

    def __init__(self, label: str, body_skm: MockSkeletalMesh | None = None) -> None:
        self._label = label
        self.comps: dict[str, MockComponent] = {
            "Body": MockComponent("Body", body_skm or MockSkeletalMesh("BodyMesh", "/Game/MetaHumans/Common/m_tal_nrw_body")),
            "Torso": MockComponent("Torso"),
            "Legs": MockComponent("Legs"),
            "Feet": MockComponent("Feet"),
        }

    def get_actor_label(self) -> str:
        return self._label

    def get_class(self) -> Any:  # noqa: ANN401
        return types.SimpleNamespace(get_name=lambda: "BP_MockMetaHuman_C")

    def get_components_by_class(self, cls: Any) -> list[MockComponent]:  # noqa: ANN401
        return list(self.comps.values())


class OutfitExportTests(unittest.TestCase):
    """Pruebas unitarias de export_tailored_outfit. @spec REQ-100"""

    def setUp(self) -> None:
        self.temp_dir = tempfile.mkdtemp(prefix="dbv_test_export_")
        self.actors: list[MockMetaHumanActor] = []
        # Parchear EditorActorSubsystem en unreal
        self.orig_get_editor_subsystem = getattr(unreal, "get_editor_subsystem", None)
        subsys = types.SimpleNamespace(get_all_level_actors=lambda: self.actors)
        unreal.get_editor_subsystem = lambda cls: subsys

    def tearDown(self) -> None:
        if self.orig_get_editor_subsystem is not None:
            unreal.get_editor_subsystem = self.orig_get_editor_subsystem
        if os.path.isdir(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_export_sin_destino_enumera_candidatos(self) -> None:
        """@spec REQ-100 — Sin destino explícito se enumeran los MetaHumans disponibles (REQ-83)."""
        self.actors.append(MockMetaHumanActor("BP_Pablo"))
        self.actors.append(MockMetaHumanActor("BP_Victoria"))

        report_raw = editor.export_tailored_outfit(actor_label="", slot="Torso")
        report = json.loads(report_raw)

        self.assertFalse(report["success"])
        self.assertIn("available", report)
        self.assertEqual(report["available"], ["BP_Pablo", "BP_Victoria"])
        self.assertIn("No target MetaHuman was chosen", report["error"])

    def test_export_actor_inexistente_falla_con_lista(self) -> None:
        """@spec REQ-100 — Un actor no encontrado lista las opciones válidas."""
        self.actors.append(MockMetaHumanActor("BP_Pablo"))

        report = json.loads(editor.export_tailored_outfit(actor_label="BP_Inexistente", slot="Torso"))

        self.assertFalse(report["success"])
        self.assertEqual(report["available"], ["BP_Pablo"])
        self.assertIn("BP_Inexistente", report["error"])

    def test_export_ranura_vacia_falla_con_mensaje_claro(self) -> None:
        """@spec REQ-100 — Un slot sin prenda equipada devuelve error descriptivo."""
        actor = MockMetaHumanActor("BP_Pablo")
        self.actors.append(actor)

        report = json.loads(editor.export_tailored_outfit(actor_label="BP_Pablo", slot="Torso"))

        self.assertFalse(report["success"])
        self.assertIn("No garment equipped on slot 'Torso'", report["error"])

    def _dressed_pablo(self) -> MockMetaHumanActor:
        actor = MockMetaHumanActor("BP_Pablo")
        actor.comps["Torso"]._skm = MockSkeletalMesh("SK_Tirantes_Pablo", "/Game/DBV/MetaHumanOutfits/Pablo/SK_Tirantes_Pablo")
        self.actors.append(actor)
        return actor

    @staticmethod
    def _fake_exporter(writes_file: bool) -> Any:  # noqa: ANN401
        """Sustituto de ``unreal.Exporter``: escribe (o no) el FBX que pide la tarea."""

        def run(task: Any) -> bool:  # noqa: ANN401
            if writes_file:
                with open(task.filename, "wb") as fh:
                    fh.write(b"Kaydara FBX Binary  \x00")
            return writes_file

        return types.SimpleNamespace(run_asset_export_task=run)

    def test_export_tailored_outfit_exitoso_genera_fbx_y_metadatos(self) -> None:
        """@spec REQ-100 — La exportación deja el FBX de la prenda y el manifiesto con la complexión."""
        self._dressed_pablo()

        with (
            patch.object(unreal, "AssetExportTask", types.SimpleNamespace, create=True),
            patch.object(unreal, "Exporter", self._fake_exporter(writes_file=True), create=True),
        ):
            report = json.loads(
                editor.export_tailored_outfit(
                    actor_label="BP_Pablo",
                    slot="Torso",
                    asset_name="Tirantes Deportivos/../x",
                    destination_dir=self.temp_dir,
                )
            )

        self.assertTrue(report["success"], report)
        self.assertEqual(report["actor"], "BP_Pablo")
        self.assertEqual(report["slot"], "Torso")
        self.assertEqual(report["body_archetype"], "m_tal_nrw")
        self.assertEqual(report["export_dir"], self.temp_dir)
        # El nombre no puede sacar el fichero de la carpeta de exportación.
        self.assertEqual(report["files"], ["Tirantes_Deportivos_x.fbx", "outfit_metadata.json"])
        self.assertTrue(os.path.isfile(os.path.join(self.temp_dir, "Tirantes_Deportivos_x.fbx")))

        with open(os.path.join(self.temp_dir, "outfit_metadata.json"), encoding="utf-8") as f:
            meta = json.load(f)

        self.assertEqual(meta["name"], "Tirantes Deportivos/../x")
        self.assertEqual(meta["slot"], "Torso")
        self.assertEqual(meta["garment_kind"], "top")
        self.assertEqual(meta["body_archetype"], "m_tal_nrw")
        self.assertEqual(meta["actor_source"], "BP_Pablo")
        self.assertEqual(meta["skeletal_mesh"], "/Game/DBV/MetaHumanOutfits/Pablo/SK_Tirantes_Pablo")
        self.assertEqual(meta["files"], ["Tirantes_Deportivos_x.fbx"])

    def test_export_falla_sin_fbx_y_no_deja_prenda_de_relleno(self) -> None:
        """@spec REQ-100 — Si el FBX no se exporta, la llamada falla: ni malla de relleno ni manifiesto."""
        self._dressed_pablo()

        with (
            patch.object(unreal, "AssetExportTask", types.SimpleNamespace, create=True),
            patch.object(unreal, "Exporter", self._fake_exporter(writes_file=False), create=True),
        ):
            report = json.loads(editor.export_tailored_outfit(actor_label="BP_Pablo", slot="Torso", destination_dir=self.temp_dir))

        self.assertFalse(report["success"])
        self.assertIn("Could not export 'SK_Tirantes_Pablo' to FBX", report["error"])
        self.assertEqual(os.listdir(self.temp_dir), [])

    def test_export_zapatos_se_registran_como_shoes(self) -> None:
        """@spec REQ-100, REQ-84 — La ranura Feet se exporta con garment_kind «shoes»."""
        actor = self._dressed_pablo()
        actor.comps["Feet"]._skm = MockSkeletalMesh("SK_Zapatillas", "/Game/DBV/MetaHumanOutfits/Pablo/SK_Zapatillas")

        with (
            patch.object(unreal, "AssetExportTask", types.SimpleNamespace, create=True),
            patch.object(unreal, "Exporter", self._fake_exporter(writes_file=True), create=True),
        ):
            report = json.loads(editor.export_tailored_outfit(actor_label="BP_Pablo", slot="Feet", destination_dir=self.temp_dir))

        self.assertTrue(report["success"], report)
        self.assertEqual(report["metadata"]["garment_kind"], "shoes")


if __name__ == "__main__":
    unittest.main()
