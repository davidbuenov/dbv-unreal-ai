# =============================================================================
# DBV Unreal AI — Tests de importar y colocar modelos (también con esqueleto)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de ``editor.import_model`` y ``editor.play_animation`` con un módulo ``unreal`` simulado.

Lo que se protege: un rig de Tripo se importa como Skeletal Mesh (no se toma su esqueleto ni una animación por
la malla principal) y sus animaciones vuelven para reproducirlas. La verificación con el motor real está pendiente
(``task.md``). Ejecutar: ``python -I -m unittest discover -s tests/python -v``.
"""

from __future__ import annotations

import contextlib
import json
import sys
import tempfile
import types
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))


class StaticMesh:
    """Malla estática."""


class Skeleton:
    """Esqueleto: guarda los esqueletos que se le declaran compatibles."""

    def __init__(self) -> None:
        self.compatible: list[Skeleton] = []

    def modify(self) -> bool:
        """Marca el asset para la transacción."""
        return True

    def add_compatible_skeleton(self, other):  # noqa: ANN001, ANN201 — imita la API de Unreal
        """Declara un esqueleto compatible."""
        self.compatible.append(other)


class SkeletalMesh:
    """Malla con esqueleto."""

    def __init__(self, skeleton: Skeleton | None = None) -> None:
        self.skeleton = skeleton or Skeleton()

    def get_editor_property(self, name):  # noqa: ANN001, ANN201
        """Propiedad de la malla."""
        return self.skeleton if name == "skeleton" else None


class AnimSequence:
    """Animación con su esqueleto y sus pistas (huesos)."""

    def __init__(self, skeleton: Skeleton | None = None, tracks: tuple[str, ...] = ("root", "hip")) -> None:
        self.skeleton = skeleton
        self.tracks = tracks

    def get_editor_property(self, name):  # noqa: ANN001, ANN201
        """Propiedad de la animación."""
        return self.skeleton if name == "skeleton" else None


ASSETS: dict[str, object] = {}
IMPORTED: list[str] = []


class AssetImportTask:
    """Tarea de importación: devuelve lo que el test haya puesto en ``IMPORTED``."""

    def get_editor_property(self, name):  # noqa: ANN001, ANN201 — imita la API de Unreal
        """Propiedad de la tarea."""
        return list(IMPORTED) if name == "imported_object_paths" else None


class AssetTools:
    """Herramientas de assets."""

    def import_asset_tasks(self, tasks):  # noqa: ANN001, ANN201
        """Importa (nada que hacer en el simulado)."""
        return tasks


class SkeletalMeshComponent:
    """Componente que guarda lo que se le pide."""

    def __init__(self, mesh: SkeletalMesh | None = None, bones: tuple[str, ...] = ("root", "hip", "spine")) -> None:
        self.mode = None
        self.data = None
        self.mesh = mesh or SkeletalMesh()
        self.bones = bones

    def get_skeletal_mesh_asset(self):  # noqa: ANN201
        """Malla del componente."""
        return self.mesh

    def get_num_bones(self) -> int:
        """Huesos de la malla."""
        return len(self.bones)

    def get_bone_name(self, index: int) -> str:
        """Nombre del hueso ``index``."""
        return self.bones[index]

    def set_animation_mode(self, mode):  # noqa: ANN001, ANN201
        """Modo de animación."""
        self.mode = mode

    def override_animation_data(self, animation, looping, playing, position, rate):  # noqa: ANN001, ANN201, PLR0913
        """Animación del editor."""
        self.data = (animation, looping, playing, position, rate)

    def play_animation(self, animation, looping):  # noqa: ANN001, ANN201
        """Lo que se ve ya (no solo lo que se guarda)."""
        self.playing = (animation, looping)

    def set_update_animation_in_editor(self, value):  # noqa: ANN001, ANN201
        """Que el viewport la mueva fuera de una partida."""
        self.in_editor = value


class Actor:
    """Actor del nivel."""

    def __init__(self, label: str, component: SkeletalMeshComponent | None) -> None:
        self.label = label
        self.component = component

    def get_actor_label(self) -> str:
        """Etiqueta."""
        return self.label

    def get_component_by_class(self, cls):  # noqa: ANN001, ANN201
        """Componente de esa clase."""
        return self.component


ACTORS: list[Actor] = []


class EditorActorSubsystem:
    """Subsistema de actores."""

    def get_all_level_actors(self):  # noqa: ANN201
        """Actores del nivel."""
        return ACTORS


def fake_unreal() -> types.ModuleType:
    """Módulo ``unreal`` con lo que usan ``import_model`` y ``play_animation``."""
    module = types.ModuleType("unreal")
    module.StaticMesh, module.SkeletalMesh, module.Skeleton, module.AnimSequence = StaticMesh, SkeletalMesh, Skeleton, AnimSequence
    module.SkeletalMeshComponent = SkeletalMeshComponent
    module.EditorActorSubsystem = EditorActorSubsystem
    module.AssetImportTask = AssetImportTask
    module.AssetToolsHelpers = types.SimpleNamespace(get_asset_tools=AssetTools)
    module.EditorAssetLibrary = types.SimpleNamespace(does_directory_exist=lambda path: True, make_directory=lambda path: True, does_asset_exist=lambda path: False)
    module.load_asset = ASSETS.get
    module.get_editor_subsystem = lambda cls: cls()
    module.AnimationMode = types.SimpleNamespace(ANIMATION_SINGLE_NODE="single-node")
    module.AnimationLibrary = types.SimpleNamespace(get_animation_track_names=lambda animation: list(animation.tracks))
    module.ScopedEditorTransaction = lambda description: contextlib.nullcontext()
    return module


sys.modules["unreal"] = fake_unreal()

from dbv_unreal_ai import editor  # noqa: E402


class ImportModelTests(unittest.TestCase):
    """Importar modelos estáticos y riggeados."""

    def setUp(self) -> None:
        self.file = tempfile.NamedTemporaryFile(suffix=".glb", delete=False)  # noqa: SIM115 — se borra en tearDown
        self.file.close()
        ASSETS.clear()
        IMPORTED.clear()

    def tearDown(self) -> None:
        Path(self.file.name).unlink(missing_ok=True)

    def run_import(self) -> dict:
        """``import_model`` del fichero temporal, como diccionario."""
        return json.loads(editor.import_model(self.file.name, "/Game/DBV/Generated", "Pirata"))

    def test_un_rig_con_animaciones_da_el_skeletal_mesh_y_sus_clips(self) -> None:
        """@spec REQ-16, REQ-19 — el esqueleto y la animación no se toman por la malla principal."""
        found = {"/Game/DBV/Generated/Pirata_Skeleton": Skeleton(), "/Game/DBV/Generated/Anim_dance_01": AnimSequence(), "/Game/DBV/Generated/Pirata": SkeletalMesh(), "/Game/DBV/Generated/M_Pirata": object()}
        ASSETS.update(found)
        IMPORTED.extend(found)
        report = self.run_import()
        self.assertEqual(report["primary"], "/Game/DBV/Generated/Pirata")
        self.assertEqual(report["kind"], "skeletal")
        self.assertEqual(report["skeleton"], "/Game/DBV/Generated/Pirata_Skeleton")
        self.assertEqual(report["animations"], ["/Game/DBV/Generated/Anim_dance_01"])

    def test_un_modelo_estatico_sigue_como_antes(self) -> None:
        """@spec REQ-19"""
        ASSETS.update({"/Game/DBV/Generated/M_Linterna": object(), "/Game/DBV/Generated/Linterna": StaticMesh()})
        IMPORTED.extend(["/Game/DBV/Generated/M_Linterna", "/Game/DBV/Generated/Linterna"])
        report = self.run_import()
        self.assertEqual((report["primary"], report["kind"], report["animations"]), ("/Game/DBV/Generated/Linterna", "static", []))

    def test_sin_nada_importado_es_un_error(self) -> None:
        """@spec REQ-19"""
        report = self.run_import()
        self.assertFalse(report["success"])
        self.assertIn("no assets", report["error"])


class PlayAnimationTests(unittest.TestCase):
    """Reproducir una animación en el viewport del editor."""

    def setUp(self) -> None:
        ASSETS.clear()
        ACTORS.clear()

    def test_pone_el_componente_en_una_sola_animacion_en_bucle(self) -> None:
        """@spec REQ-16 — sin Animation Blueprint, como hace falta para revisar un baile de Tripo."""
        component = SkeletalMeshComponent()
        dance = AnimSequence(component.mesh.skeleton)
        ASSETS["/Game/DBV/Generated/Anim_dance_01"] = dance
        ACTORS.append(Actor("Pirata", component))
        report = json.loads(editor.play_animation("Pirata", "/Game/DBV/Generated/Anim_dance_01"))
        self.assertTrue(report["success"])
        self.assertFalse(report["compatible_skeleton_added"])
        self.assertEqual(component.mode, "single-node")
        self.assertEqual(component.data, (dance, True, True, 0.0, 1.0))
        self.assertEqual(component.playing, (dance, True), "solo con override_animation_data la instancia se quedaba sin clip (en vivo)")
        self.assertTrue(component.in_editor)

    def test_un_clip_de_tripo_con_su_propio_esqueleto_se_declara_compatible(self) -> None:
        """@spec REQ-16 — visto en vivo (S18): el GLB del baile trae su copia del rig y Unreal no lo reproducía."""
        component = SkeletalMeshComponent()
        # Mayúsculas distintas, como en vivo (malla `Root`, clip `root`): en Unreal es el mismo hueso.
        dance = AnimSequence(Skeleton(), tracks=("ROOT", "Hip"))
        ASSETS["/Game/dance"] = dance
        ACTORS.append(Actor("Pirata", component))
        report = json.loads(editor.play_animation("Pirata", "/Game/dance"))
        self.assertTrue(report["compatible_skeleton_added"])
        self.assertEqual(component.mesh.skeleton.compatible, [dance.skeleton])
        self.assertEqual(component.data[0], dance)

    def test_un_clip_de_otro_rig_se_explica_en_vez_de_mentir(self) -> None:
        """@spec REQ-16 — antes respondía «success» sin que se moviera nada."""
        component = SkeletalMeshComponent()
        ASSETS["/Game/other"] = AnimSequence(Skeleton(), tracks=("root", "tail_01", "tail_02"))
        ACTORS.append(Actor("Pirata", component))
        report = json.loads(editor.play_animation("Pirata", "/Game/other"))
        self.assertFalse(report["success"])
        self.assertIn("tail_01", report["error"])
        self.assertIsNone(component.data)

    def test_explica_lo_que_falta(self) -> None:
        """@spec REQ-16"""
        ASSETS["/Game/x"] = StaticMesh()
        self.assertIn("not an animation", json.loads(editor.play_animation("Pirata", "/Game/x"))["error"])
        ASSETS["/Game/anim"] = AnimSequence()
        ACTORS.append(Actor("Linterna", None))
        self.assertIn("no skeletal mesh actor", json.loads(editor.play_animation("Linterna", "/Game/anim"))["error"])


if __name__ == "__main__":
    unittest.main()
