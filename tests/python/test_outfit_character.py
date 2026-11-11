# =============================================================================
# DBV Unreal AI — Tests de gestión de copia del personaje y ranuras de ropa
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests unitarios de ``dbv_unreal_ai.outfit.character``.

@spec REQ-85, REQ-84
"""

from __future__ import annotations

import os
import sys
import types
import unittest
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))


# Mocks para simular la API de Unreal Engine
class MockVector:
    def __init__(self, x: float = 0.0, y: float = 0.0, z: float = 0.0) -> None:
        self.x = float(x)
        self.y = float(y)
        self.z = float(z)


class MockRotator:
    def __init__(self, pitch: float = 0.0, yaw: float = 0.0, roll: float = 0.0) -> None:
        self.pitch = float(pitch)
        self.yaw = float(yaw)
        self.roll = float(roll)


class MockSkeletalMesh:
    def __init__(self, path: str) -> None:
        self._path = path

    def get_path_name(self) -> str:
        return self._path


class MockSkeletalMeshComponent:
    def __init__(self, name: str, mesh: MockSkeletalMesh | None = None) -> None:
        self._name = name
        self._mesh = mesh
        self.leader: list[Any] = []
        self.cleared_overrides = False

    def get_name(self) -> str:
        return self._name

    def get_skeletal_mesh_asset(self) -> MockSkeletalMesh | None:
        return self._mesh

    def set_skeletal_mesh_asset(self, mesh: MockSkeletalMesh | None) -> None:
        self._mesh = mesh

    def set_leader_pose_component(self, leader: Any, b_follow_visible: bool = False) -> None:
        self.leader = [leader]

    def empty_override_materials(self) -> None:
        self.cleared_overrides = True


class MockClass:
    def __init__(self, name: str, path: str) -> None:
        self._name = name
        self._path = path

    def get_name(self) -> str:
        return self._name

    def get_path_name(self) -> str:
        return self._path


class MockActor:
    def __init__(self, label: str, class_name: str, class_path: str, comps: dict[str, MockSkeletalMeshComponent] | None = None) -> None:
        self._label = label
        self._class = MockClass(class_name, class_path)
        self._comps = comps or {}
        self._location = MockVector(0, 0, 0)
        self._rotation = MockRotator(0, 0, 0)
        self._scale = MockVector(1, 1, 1)
        self._folder = ""
        self.tags: list[str] = []

    def get_actor_label(self) -> str:
        return self._label

    def set_actor_label(self, label: str) -> None:
        self._label = label

    def get_class(self) -> MockClass:
        return self._class

    def get_components_by_class(self, cls: Any) -> list[Any]:
        return list(self._comps.values())

    def get_actor_location(self) -> MockVector:
        return self._location

    def get_actor_rotation(self) -> MockRotator:
        return self._rotation

    def get_actor_scale3d(self) -> MockVector:
        return self._scale

    def set_actor_scale3d(self, scale: MockVector) -> None:
        self._scale = scale

    def get_folder_path(self) -> str:
        return self._folder

    def set_folder_path(self, folder: str) -> None:
        self._folder = folder

    def modify(self) -> bool:
        return True


class MockSequenceBinding:
    def __init__(self, name: str) -> None:
        self._name = name

    def get_name(self) -> str:
        return self._name


class MockLevelSequence:
    def __init__(self, name: str, bindings: list[str]) -> None:
        self._name = name
        self._bindings = [MockSequenceBinding(b) for b in bindings]

    def get_name(self) -> str:
        return self._name

    def get_bindings(self) -> list[MockSequenceBinding]:
        return self._bindings


class MockLevelSequenceActor:
    def __init__(self, label: str, sequence: MockLevelSequence) -> None:
        self._label = label
        self._class = MockClass("LevelSequenceActor", "/Script/LevelSequence.LevelSequenceActor")
        self._seq = sequence

    def get_actor_label(self) -> str:
        return self._label

    def get_class(self) -> MockClass:
        return self._class

    def get_sequence(self) -> MockLevelSequence:
        return self._seq


class MockEditorActorSubsystem:
    def __init__(self, actors: list[Any]) -> None:
        self.actors = actors
        self.spawned: list[Any] = []
        self.destroyed: list[Any] = []

    def get_all_level_actors(self) -> list[Any]:
        return self.actors

    def spawn_actor_from_class(self, cls: Any, loc: Any, rot: Any) -> Any:
        label = f"Spawned_{getattr(cls, '_name', 'Actor')}"
        comps = {
            "Body": MockSkeletalMeshComponent("Body"),
            "Torso": MockSkeletalMeshComponent("Torso"),
            "Legs": MockSkeletalMeshComponent("Legs"),
            "Feet": MockSkeletalMeshComponent("Feet"),
        }
        actor = MockActor(label, getattr(cls, "_name", "Actor_C"), getattr(cls, "_path", "/Game/Spawned_C"), comps)
        actor._location = loc
        actor._rotation = rot
        self.spawned.append(actor)
        self.actors.append(actor)
        return actor

    def destroy_actor(self, actor: Any) -> bool:
        self.destroyed.append(actor)
        if actor in self.actors:
            self.actors.remove(actor)
        return True


class MockScopedTransaction:
    def __init__(self, title: str) -> None:
        self.title = title

    def __enter__(self) -> MockScopedTransaction:
        return self

    def __exit__(self, *args: Any) -> None:
        pass


class MockEditorAssetLibrary:
    def __init__(self) -> None:
        self.directories: set[str] = set()
        self.assets: dict[str, Any] = {}
        self.duplicated: list[tuple[str, str]] = []

    def does_directory_exist(self, path: str) -> bool:
        return path in self.directories

    def make_directory(self, path: str) -> bool:
        self.directories.add(path)
        return True

    def does_asset_exist(self, path: str) -> bool:
        return path in self.assets

    def duplicate_asset(self, source: str, dest: str) -> bool:
        self.duplicated.append((source, dest))
        self.assets[dest] = MockClass(os.path.basename(dest) + "_C", dest + "_C")
        return True

    def load_blueprint_class(self, path: str) -> MockClass:
        return self.assets.get(path, MockClass(os.path.basename(path) + "_C", path + "_C"))

    def load_asset(self, path: str) -> MockSkeletalMesh | None:
        return MockSkeletalMesh(path)


class LevelSubsystemMock:
    def __init__(self, is_pie: bool = False) -> None:
        self.is_pie = is_pie

    def is_in_play_in_editor(self) -> bool:
        return self.is_pie


def setup_mock_unreal(actors: list[Any]) -> types.SimpleNamespace:
    eal = MockEditorAssetLibrary()
    subsystem = MockEditorActorSubsystem(actors)
    level_subsystem = LevelSubsystemMock(is_pie=False)

    unreal_mod = sys.modules.get("unreal")
    if unreal_mod is None:
        unreal_mod = types.ModuleType("unreal")
        sys.modules["unreal"] = unreal_mod

    unreal_mod.Vector = MockVector  # type: ignore[attr-defined]
    unreal_mod.Rotator = MockRotator  # type: ignore[attr-defined]
    unreal_mod.SkeletalMeshComponent = MockSkeletalMeshComponent  # type: ignore[attr-defined]
    unreal_mod.EditorActorSubsystem = MockEditorActorSubsystem  # type: ignore[attr-defined]
    unreal_mod.LevelEditorSubsystem = LevelSubsystemMock  # type: ignore[attr-defined]
    unreal_mod.EditorAssetLibrary = eal  # type: ignore[attr-defined]
    unreal_mod.ScopedEditorTransaction = MockScopedTransaction  # type: ignore[attr-defined]
    unreal_mod.get_editor_subsystem = lambda cls: level_subsystem if cls == LevelSubsystemMock else subsystem  # type: ignore[attr-defined]

    from dbv_unreal_ai.outfit import character
    character.unreal = unreal_mod
    character.HAS_UNREAL = True

    return types.SimpleNamespace(eal=eal, subsystem=subsystem, level_subsystem=level_subsystem, unreal=unreal_mod)


class TestOutfitCharacter(unittest.TestCase):
    def setUp(self) -> None:
        self.body_comp = MockSkeletalMeshComponent("Body", MockSkeletalMesh("/Game/MetaHumans/Common/Body/f_med_nrw_body.f_med_nrw_body"))
        self.torso_comp = MockSkeletalMeshComponent("Torso", MockSkeletalMesh("/Game/MetaHumans/Tops/Shirt.Shirt"))
        self.legs_comp = MockSkeletalMeshComponent("Legs", MockSkeletalMesh("/Game/MetaHumans/Bottoms/Jeans.Jeans"))
        self.feet_comp = MockSkeletalMeshComponent("Feet", MockSkeletalMesh("/Game/MetaHumans/Shoes/Flats.Flats"))

        self.pablo = MockActor(
            "BP_Pablo",
            "BP_Pablo_C",
            "/Game/MetaHumans/Pablo/BP_Pablo.BP_Pablo_C",
            {
                "Body": self.body_comp,
                "Torso": self.torso_comp,
                "Legs": self.legs_comp,
                "Feet": self.feet_comp,
            },
        )
        self.actors = [self.pablo]
        self.env = setup_mock_unreal(self.actors)

    def test_is_character_copy_identifica_copias_por_tag_y_clase(self) -> None:
        """@spec REQ-85"""
        from dbv_unreal_ai.outfit import character

        self.assertFalse(character.is_character_copy(None))
        self.assertFalse(character.is_character_copy(self.pablo))

        copy_actor = MockActor("BP_Pablo_DBV", "BP_Pablo_DBV_C", "/Game/DBV/MetaHumanOutfits/Pablo/BP_Pablo_DBV.BP_Pablo_DBV_C")
        self.assertTrue(character.is_character_copy(copy_actor))

        self.pablo.tags.append(character.TAG_DBV_COPY)
        self.assertTrue(character.is_character_copy(self.pablo))

    def test_get_original_bp_path_resuelve_ruta_original(self) -> None:
        """@spec REQ-85"""
        from dbv_unreal_ai.outfit import character

        self.assertEqual(character.get_original_bp_path(None), None)
        self.assertEqual(character.get_original_bp_path(self.pablo), "/Game/MetaHumans/Pablo/BP_Pablo")

        copy_actor = MockActor("BP_Pablo", "BP_Pablo_DBV_C", "/Game/DBV/BP_Pablo_DBV.BP_Pablo_DBV_C")
        copy_actor.tags.append(f"{character.TAG_ORIGINAL_BP_PREFIX}/Game/MetaHumans/Pablo/BP_Pablo")
        self.assertEqual(character.get_original_bp_path(copy_actor), "/Game/MetaHumans/Pablo/BP_Pablo")

    def test_find_sequence_warnings_detecta_secuencias_enlazadas(self) -> None:
        """@spec REQ-85"""
        from dbv_unreal_ai.outfit import character

        seq = MockLevelSequence("LS_Pasarela", ["BP_Pablo", "Camara"])
        seq_actor = MockLevelSequenceActor("Pasarela_Actor", seq)
        self.actors.append(seq_actor)

        warnings = character.find_sequence_warnings("BP_Pablo")
        self.assertEqual(warnings, ["LS_Pasarela"])

        no_warnings = character.find_sequence_warnings("BP_Inexistente")
        self.assertEqual(no_warnings, [])

    def test_prepare_character_copy_rechaza_pie_o_actor_invalido(self) -> None:
        """@spec REQ-85"""
        from dbv_unreal_ai.outfit import character

        # Etiqueta vacía
        res1 = character.prepare_character_copy("")
        self.assertFalse(res1["success"])
        self.assertIn("empty", res1["error"])

        # Actor inexistente lista disponibles
        res2 = character.prepare_character_copy("BP_Desconocido")
        self.assertFalse(res2["success"])
        self.assertIn("BP_Pablo", res2["available"])

        # En Play in Editor (PIE) se rechaza
        self.env.level_subsystem.is_pie = True
        res_pie = character.prepare_character_copy("BP_Pablo")
        self.assertFalse(res_pie["success"])
        self.assertIn("PIE", res_pie["error"])

    def test_prepare_character_copy_duplica_sustituye_y_reutiliza(self) -> None:
        """@spec REQ-85"""
        from dbv_unreal_ai.outfit import character

        # 1. Primera preparación: crea copia y sustituye el actor original
        res = character.prepare_character_copy("BP_Pablo")
        self.assertTrue(res["success"], res)
        self.assertFalse(res["reused"])
        self.assertEqual(res["copy_bp_path"], "/Game/DBV/MetaHumanOutfits/Pablo/BP_Pablo_DBV")
        self.assertEqual(res["original_bp_path"], "/Game/MetaHumans/Pablo/BP_Pablo")

        # El actor original fue destruido y el nuevo actor tiene el label BP_Pablo y el tag de copia
        self.assertIn(self.pablo, self.env.subsystem.destroyed)
        active_actor = next(a for a in self.env.subsystem.actors if a.get_actor_label() == "BP_Pablo")
        self.assertTrue(character.is_character_copy(active_actor))

        # 2. Segunda preparación: reutiliza la copia existente sin encadenar copias de copias
        res2 = character.prepare_character_copy("BP_Pablo")
        self.assertTrue(res2["success"])
        self.assertTrue(res2["reused"])
        self.assertEqual(res2["copy_bp_path"], res["copy_bp_path"])

    def test_apply_slot_rules_segun_tabla_req84(self) -> None:
        """@spec REQ-84"""
        from dbv_unreal_ai.outfit import character

        # Top sobre vestido: vacía Torso y restaura Legs si estaba vacía
        self.legs_comp.set_skeletal_mesh_asset(None)
        orig_slots = {"Legs": "/Game/MetaHumans/Bottoms/Jeans.Jeans", "Torso": "/Game/MetaHumans/Tops/Shirt.Shirt"}

        res_top = character.apply_slot_rules(self.pablo, "top", orig_slots)
        self.assertTrue(res_top["success"])
        self.assertIn("Legs", res_top["restored_slots"])
        self.assertEqual(self.legs_comp.get_skeletal_mesh_asset().get_path_name(), "/Game/MetaHumans/Bottoms/Jeans.Jeans")

        # Dress: retira Torso y Legs
        res_dress = character.apply_slot_rules(self.pablo, "dress", orig_slots)
        self.assertTrue(res_dress["success"])
        self.assertIn("Torso", res_dress["cleared_slots"])
        self.assertIn("Legs", res_dress["cleared_slots"])
        self.assertIsNone(self.torso_comp.get_skeletal_mesh_asset())
        self.assertIsNone(self.legs_comp.get_skeletal_mesh_asset())

        # Skirt: retira Legs y restaura Torso
        res_skirt = character.apply_slot_rules(self.pablo, "skirt", orig_slots)
        self.assertTrue(res_skirt["success"])
        self.assertIn("Torso", res_skirt["restored_slots"])
        self.assertIn("Legs", res_skirt["cleared_slots"])
        self.assertEqual(self.torso_comp.get_skeletal_mesh_asset().get_path_name(), "/Game/MetaHumans/Tops/Shirt.Shirt")

    def test_revert_to_original_restaura_blueprint_de_partida(self) -> None:
        """@spec REQ-85"""
        from dbv_unreal_ai.outfit import character

        # Intentar revertir un actor original falla
        fail_res = character.revert_to_original("BP_Pablo")
        self.assertFalse(fail_res["success"])
        self.assertIn("nothing to revert", fail_res["error"])

        # Tras preparar la copia, revert_to_original restaura el Blueprint original
        character.prepare_character_copy("BP_Pablo")
        rev_res = character.revert_to_original("BP_Pablo")
        self.assertTrue(rev_res["success"], rev_res)
        self.assertEqual(rev_res["restored_bp_path"], "/Game/MetaHumans/Pablo/BP_Pablo")

    def test_ensure_blueprint_leader_pose_valida_ruta(self) -> None:
        """@spec REQ-85 — valida que ensure_blueprint_leader_pose maneje rutas inexistentes sin error."""
        from dbv_unreal_ai.outfit import character

        self.assertFalse(character.ensure_blueprint_leader_pose("/Game/Inexistente"))


if __name__ == "__main__":
    unittest.main()
