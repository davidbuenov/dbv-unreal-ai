# =============================================================================
# DBV Unreal AI — Tests de las recetas del toolset (comprobación y Agent Skills)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de ``recipes`` con un módulo ``unreal`` simulado. La verificación con el motor real se hizo en vivo el
2026-11-01 (las cuatro comprobaciones y ``ListSkills``). Ejecutar: ``python -I -m unittest discover -s tests/python``.
"""

from __future__ import annotations

import sys
import types
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))
sys.modules.setdefault("unreal", types.ModuleType("unreal"))

from dbv_unreal_ai import recipes  # noqa: E402


class Default:
    """Objeto por defecto de una clase: guarda las propiedades que se le ponen."""

    def __init__(self) -> None:
        self.props: dict[str, str] = {}

    def set_editor_property(self, name: str, value: str) -> None:
        """Como en Unreal."""
        self.props[name] = value


def fake_skills_unreal() -> types.ModuleType:
    """Lo justo para ``register_skills``: ``AgentSkill``, ``uclass`` y ``get_default_object``."""
    module = types.ModuleType("unreal")
    defaults: dict[type, Default] = {}
    created: list[str] = []

    class AgentSkill:
        """Base de las skills."""

    def uclass():  # noqa: ANN202 — imita el decorador de Unreal
        def register(cls: type) -> type:
            created.append(cls.__name__)
            return cls
        return register

    module.AgentSkill = AgentSkill
    module.uclass = uclass
    module.get_default_object = lambda cls: defaults.setdefault(cls, Default())
    module.created = created
    module.defaults = defaults
    return module


class RecipesTests(unittest.TestCase):
    """@spec REQ-23"""

    def test_las_recetas_son_las_mismas_que_lee_la_app(self) -> None:
        """La lista sale del mismo ``recipes.json`` que compila Rust. @spec REQ-23"""
        self.assertEqual([r["id"] for r in recipes.load()], ["test_level", "tripo_character", "world_walk", "convai_metahuman", "runtime_diagnosis"])
        self.assertEqual(sorted(recipes.CHECKS), sorted(r["id"] for r in recipes.load()))

    def test_una_receta_desconocida_no_comprueba_nada(self) -> None:
        """@spec REQ-23"""
        result = recipes.check("rm -rf")
        self.assertIsNone(result["passed"])
        self.assertIn("unknown recipe", result["detail"])

    def test_publica_cada_receta_como_skill_una_sola_vez(self) -> None:
        """Las clases se crean una vez y se guardan (si no, Python las libera). @spec REQ-23"""
        fake = fake_skills_unreal()
        with mock.patch.object(recipes, "unreal", fake), mock.patch.dict(recipes._SKILLS, clear=True):
            self.assertEqual(recipes.register_skills(), 5)
            self.assertEqual(recipes.register_skills(), 5)
            self.assertEqual(fake.created, ["DbvRecipeTestLevel", "DbvRecipeTripoCharacter", "DbvRecipeWorldWalk", "DbvRecipeConvaiMetahuman", "DbvRecipeRuntimeDiagnosis"])
            props = fake.defaults[recipes._SKILLS["DbvRecipeWorldWalk"]].props
        self.assertTrue(props["description"].startswith("DBV Unreal AI recipe: Walk through a World Labs world."))
        self.assertIn('recipes.check("world_walk")', props["instructions"])
        self.assertIn("recipe_check", props["instructions"])

    def test_diagnostico_en_ejecucion_exige_medir_y_prohibe_callbacks_por_fotograma(self) -> None:
        """La receta manda medir antes de cambiar y no grabar con callbacks por fotograma. @spec REQ-97"""
        recipe = next(r for r in recipes.load() if r["id"] == "runtime_diagnosis")
        self.assertIn("MEASURING before changing anything", recipe["steps"])
        self.assertIn("Never record with per-frame callbacks", recipe["steps"])
        self.assertIn("what you noticed / what the data says / conclusion", recipe["steps"])
        self.assertEqual(sorted(recipe["title"]), ["en", "es", "zh-CN"])

    def test_diagnostico_en_ejecucion_se_comprueba_fuera_de_play(self) -> None:
        """El editor es el juez: con una partida en marcha no pasa. @spec REQ-97"""
        for game_world, passed in ((None, True), (object(), False)):
            fake = types.ModuleType("unreal")
            fake.UnrealEditorSubsystem = object
            fake.get_editor_subsystem = lambda _cls, world=game_world: types.SimpleNamespace(get_game_world=lambda: world)
            with mock.patch.object(recipes, "unreal", fake), mock.patch.object(recipes, "_actors", return_value=[]):
                result = recipes.check("runtime_diagnosis")
            self.assertEqual(result["passed"], passed, result)

    def test_sin_agent_skill_no_publica_nada(self) -> None:
        """Un motor sin ``AgentSkill``. @spec REQ-23"""
        with mock.patch.object(recipes, "unreal", types.ModuleType("unreal")):
            self.assertEqual(recipes.register_skills(), 0)


if __name__ == "__main__":
    unittest.main()
