# =============================================================================
# DBV Unreal AI — Tests del ensamblado en Blueprint de modelos de Fab por piezas
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""@spec REQ-104"""

from __future__ import annotations

import importlib
import sys
import types
import unittest
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))


class StaticMesh:
    def __init__(self, name: str) -> None:
        self._name = name

    def get_name(self) -> str:
        return self._name


class Other:
    pass


def fake_unreal(assets: dict[str, Any], existing: set[str]) -> types.ModuleType:
    """Lo justo de `unreal` para el ensamblado: biblioteca de assets, fábrica y subsistema de subobjetos."""
    mod = types.ModuleType("unreal")
    mod.StaticMesh = StaticMesh
    mod.Actor = object
    mod.Blueprint = object
    mod.StaticMeshComponent = object
    mod.created = []
    mod.components = []

    class EditorAssetLibrary:
        @staticmethod
        def list_assets(folder: str, recursive: bool = True, include_folder: bool = False) -> list[str]:
            return [p for p in assets if p.startswith(folder)]

        @staticmethod
        def load_asset(path: str) -> Any:
            return assets[path]

        @staticmethod
        def does_asset_exist(path: str) -> bool:
            return path in existing

        @staticmethod
        def save_asset(path: str, only_if_is_dirty: bool = True) -> bool:
            return True

    class Props:
        def __init__(self) -> None:
            self.props: dict[str, Any] = {}

        def set_editor_property(self, name: str, value: Any) -> None:
            self.props[name] = value

    class BlueprintFactory(Props):
        pass

    class AddNewSubobjectParams(Props):
        pass

    class Tools:
        def create_asset(self, name: str, folder: str, cls: Any, factory: Any) -> str:
            mod.created.append(f"{folder}/{name}")
            return "bp"

    class AssetToolsHelpers:
        @staticmethod
        def get_asset_tools() -> Tools:
            return Tools()

    class Sub:
        def k2_gather_subobject_data_for_blueprint(self, bp: Any) -> list[str]:
            return ["root"]

        def add_new_subobject(self, params: Any) -> tuple[str, str]:
            return (f"h{len(mod.components)}", "")

        def rename_subobject(self, handle: str, text: Any) -> None:
            pass

        def k2_find_subobject_data_from_handle(self, handle: str) -> str:
            return handle

    class Lib:
        @staticmethod
        def is_handle_valid(handle: str) -> bool:
            return True

        @staticmethod
        def get_object_for_blueprint(data: str, bp: Any) -> Props:
            component = Props()
            mod.components.append(component)
            return component

    class BlueprintEditorLibrary:
        @staticmethod
        def compile_blueprint(bp: Any) -> None:
            pass

    mod.EditorAssetLibrary = EditorAssetLibrary
    mod.BlueprintFactory = BlueprintFactory
    mod.AddNewSubobjectParams = AddNewSubobjectParams
    mod.AssetToolsHelpers = AssetToolsHelpers
    mod.SubobjectDataBlueprintFunctionLibrary = Lib
    mod.BlueprintEditorLibrary = BlueprintEditorLibrary
    mod.get_engine_subsystem = lambda cls: Sub()
    mod.SubobjectDataSubsystem = Sub
    mod.Text = str
    return mod


def load_fab(assets: dict[str, Any], existing: set[str] | None = None) -> Any:
    """Carga `dbv_unreal_ai.fab` de nuevo sobre un `unreal` falso (el módulo se enlaza a `unreal` al importarse)."""
    sys.modules["unreal"] = fake_unreal(assets, existing or set())
    import dbv_unreal_ai.fab as fab

    return importlib.reload(fab)


class AssembleBlueprintTest(unittest.TestCase):
    def setUp(self) -> None:
        self._previous = sys.modules.get("unreal")

    def tearDown(self) -> None:
        # Otros tests usan su propio `unreal` falso: se deja como estaba.
        if self._previous is None:
            sys.modules.pop("unreal", None)
        else:
            sys.modules["unreal"] = self._previous

    def test_varias_piezas_dan_un_blueprint_con_una_malla_por_componente(self) -> None:
        assets = {"/Game/Fab/Car/body": StaticMesh("car_body"), "/Game/Fab/Car/tyre": StaticMesh("tyre"), "/Game/Fab/Car/M": Other()}
        fab = load_fab(assets)
        res = fab.assemble_blueprint("/Game/Fab/Car", "Camo muscle car")
        unreal = sys.modules["unreal"]
        self.assertTrue(res["created"])
        self.assertEqual(res["blueprint"], "/Game/Fab/Car/BP_Camo_muscle_car")
        self.assertEqual(res["pieces"], 2)
        self.assertEqual(unreal.created, ["/Game/Fab/Car/BP_Camo_muscle_car"])
        self.assertEqual([c.props["static_mesh"].get_name() for c in unreal.components], ["car_body", "tyre"])

    def test_una_sola_malla_no_necesita_blueprint(self) -> None:
        fab = load_fab({"/Game/Fab/Rock/rock": StaticMesh("rock")})
        res = fab.assemble_blueprint("/Game/Fab/Rock", "Rock")
        self.assertFalse(res["created"])
        self.assertIsNone(res["blueprint"])
        self.assertEqual(sys.modules["unreal"].created, [])

    def test_si_ya_existe_se_reutiliza(self) -> None:
        assets = {"/Game/Fab/Car/a": StaticMesh("a"), "/Game/Fab/Car/b": StaticMesh("b")}
        fab = load_fab(assets, {"/Game/Fab/Car/BP_Car"})
        res = fab.assemble_blueprint("/Game/Fab/Car", "Car")
        self.assertEqual(res["blueprint"], "/Game/Fab/Car/BP_Car")
        self.assertEqual(sys.modules["unreal"].created, [])

    def test_el_nombre_del_blueprint_es_seguro(self) -> None:
        fab = load_fab({})
        self.assertEqual(fab.blueprint_name("Generic Sport Coupé Car!"), "BP_Generic_Sport_Coup_Car")
        self.assertEqual(fab.blueprint_name(""), "BP_FabModel")


if __name__ == "__main__":
    unittest.main()
