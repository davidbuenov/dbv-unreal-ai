# =============================================================================
# DBV Unreal AI — Tests del análisis y la ejecución de Python del toolset
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests fuera de Unreal con un módulo ``unreal`` simulado.

Ejecutar: ``python -I -m unittest discover -s tests/python -v`` desde la raíz del repo.
"""

from __future__ import annotations

import contextlib
import sys
import types
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai import analysis  # noqa: E402


class EditorAssetLibrary:
    """Asset library."""

    @staticmethod
    def load_asset(path):  # noqa: ANN001, ANN205 — imita la API de Unreal
        """X.load_asset(asset_path) -> Object"""
        return path

    @staticmethod
    def does_directory_exist(path):  # noqa: ANN001, ANN205
        """X.does_directory_exist(directory_path) -> bool"""
        return True


class EditorLevelLibrary:
    """Old library."""

    @staticmethod
    def get_editor_world():  # noqa: ANN205
        """deprecated: The Editor Scripting Utilities Plugin is deprecated - Use the function in Unreal Editor Subsystem"""
        return None


class UnrealEditorSubsystem:
    """Subsistema del editor que sustituye a EditorLevelLibrary."""

    def get_editor_world(self):  # noqa: ANN201
        """X.get_editor_world() -> World"""
        return None


def fake_unreal() -> types.ModuleType:
    """Módulo ``unreal`` mínimo para los tests."""
    module = types.ModuleType("unreal")
    module.EditorAssetLibrary = EditorAssetLibrary
    module.EditorLevelLibrary = EditorLevelLibrary
    module.UnrealEditorSubsystem = UnrealEditorSubsystem
    module.log = lambda message: print(f"LOG {message}")
    return module


class ValidateCodeTests(unittest.TestCase):
    """@spec REQ-26"""

    def setUp(self) -> None:
        self.unreal = fake_unreal()

    def test_codigo_correcto_pasa(self) -> None:
        """@spec REQ-26"""
        report = analysis.validate_code("import unreal\nunreal.EditorAssetLibrary.load_asset('/Game/X')\n", self.unreal)
        self.assertTrue(report["ok"])
        self.assertEqual(report["findings"], [])
        self.assertEqual(report["checked"], 1)

    def test_detecta_errata_con_sugerencias(self) -> None:
        """@spec REQ-26"""
        report = analysis.validate_code("import unreal\nunreal.EditorAsetLibrary.load_asset('/Game/X')\n", self.unreal)
        self.assertFalse(report["ok"])
        finding = report["findings"][0]
        self.assertEqual((finding["kind"], finding["line"], finding["name"]), ("unknown", 2, "unreal.EditorAsetLibrary"))
        self.assertIn("EditorAssetLibrary", finding["suggestions"])

    def test_detecta_miembro_inexistente(self) -> None:
        """@spec REQ-26"""
        report = analysis.validate_code("import unreal\nunreal.EditorAssetLibrary.load_assets('/x')\n", self.unreal)
        self.assertEqual(report["findings"][0]["name"], "unreal.EditorAssetLibrary.load_assets")
        self.assertIn("load_asset", report["findings"][0]["suggestions"])

    def test_obsoleto_avisa_pero_no_bloquea(self) -> None:
        """@spec REQ-26 — visto en vivo con UE 5.8.2."""
        report = analysis.validate_code("import unreal\nw = unreal.EditorLevelLibrary.get_editor_world()\n", self.unreal)
        self.assertTrue(report["ok"])
        self.assertEqual(report["findings"][0]["kind"], "deprecated")
        self.assertIn("Unreal Editor Subsystem", report["findings"][0]["message"])
        # La alternativa concreta, no solo «usa un subsistema» (eval con qwen3:8b del 2026-10-24).
        self.assertEqual(report["findings"][0]["suggestions"], ["unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()"])

    def test_sigue_los_alias_y_los_from_import(self) -> None:
        """@spec REQ-26"""
        code = "import unreal as ue\nfrom unreal import EditorAssetLibrary as EAL, NoExiste\nue.EditorAssetLibrary.load_asset('/x')\nEAL.nada()\n"
        names = {f["name"] for f in analysis.validate_code(code, self.unreal)["findings"]}
        self.assertEqual(names, {"unreal.NoExiste", "unreal.EditorAssetLibrary.nada"})

    def test_error_de_sintaxis_y_codigo_vacio(self) -> None:
        """@spec REQ-26"""
        report = analysis.validate_code("def x(:\n  pass\n", self.unreal)
        self.assertFalse(report["ok"])
        self.assertEqual(report["syntax_error"]["line"], 1)
        self.assertFalse(analysis.validate_code("   ", self.unreal)["ok"])

    def test_no_ejecuta_el_codigo_que_analiza(self) -> None:
        """@spec REQ-26 — el código es un dato."""
        marker = []
        self.unreal.marker = marker
        analysis.validate_code("import unreal\nunreal.marker.append(1)\n", self.unreal)
        self.assertEqual(marker, [])


class DescribeNameTests(unittest.TestCase):
    """@spec REQ-25"""

    def test_describe_un_metodo_y_una_clase(self) -> None:
        """@spec REQ-25"""
        unreal = fake_unreal()
        method = analysis.describe_name("unreal.EditorAssetLibrary.load_asset", unreal)
        self.assertTrue(method["exists"])
        self.assertEqual(method["kind"], "callable")
        self.assertIn("load_asset(asset_path)", method["doc"])
        cls = analysis.describe_name("EditorAssetLibrary", unreal)
        self.assertEqual(cls["kind"], "class")
        self.assertIn("does_directory_exist", cls["members"])
        old = analysis.describe_name("EditorLevelLibrary.get_editor_world", unreal)
        self.assertTrue(old["deprecated"])
        self.assertEqual(old["alternatives"], ["unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()"])
        self.assertEqual(analysis.describe_name("EditorAssetLibrary.load_asset", unreal)["alternatives"], [])

    def test_un_nombre_que_no_existe_trae_sugerencias(self) -> None:
        """@spec REQ-25"""
        missing = analysis.describe_name("EditorAssetLibrary.load_asets", fake_unreal())
        self.assertFalse(missing["exists"])
        self.assertIn("load_asset", missing["suggestions"])
        self.assertFalse(analysis.describe_name("", fake_unreal())["exists"])


class RunCodeTests(unittest.TestCase):
    """@spec REQ-06"""

    def test_captura_salida_resultado_y_avisos(self) -> None:
        """@spec REQ-06"""
        code = "import warnings\nprint('hola')\nwarnings.warn('vieja', DeprecationWarning)\nresult = {'n': 2}\n"
        report = analysis.run_code(code, fake_unreal())
        self.assertTrue(report["success"])
        self.assertEqual(report["stdout"], "hola\n")
        self.assertEqual(report["result"], {"n": 2})
        self.assertTrue(report["warnings"][0].startswith("DeprecationWarning: vieja"))

    def test_un_fallo_vuelve_como_dato_con_su_traza(self) -> None:
        """@spec REQ-06"""
        report = analysis.run_code("x = 1 / 0\n", fake_unreal())
        self.assertFalse(report["success"])
        self.assertIn("ZeroDivisionError", report["error"])

    def test_usa_la_transaccion_del_editor(self) -> None:
        """@spec REQ-06 — todo lo que ejecuta va dentro de un deshacer."""
        events = []

        @contextlib.contextmanager
        def transaction():  # noqa: ANN202
            events.append("open")
            yield
            events.append("close")

        analysis.run_code("result = 1\n", fake_unreal(), transaction)
        self.assertEqual(events, ["open", "close"])

    def test_recorta_salidas_enormes_y_rechaza_codigo_vacio(self) -> None:
        """@spec REQ-06"""
        report = analysis.run_code("print('x' * 30000)\n", fake_unreal())
        self.assertIn("truncated", report["stdout"])
        self.assertFalse(analysis.run_code("", fake_unreal())["success"])


if __name__ == "__main__":
    unittest.main()
