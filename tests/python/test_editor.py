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
import math
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
    module.ToolsetLibrary = types.SimpleNamespace(
        active_undo_count=5,
        undo_count_invoked=0,
        get_active_undo_count=lambda: module.ToolsetLibrary.active_undo_count,
        undo_transaction=lambda is_user=False: _mock_undo(module.ToolsetLibrary),
    )
    module.registered_post_tick = []
    module.register_slate_post_tick_callback = lambda cb: _mock_register_post_tick(module, cb)
    module.unregister_slate_post_tick_callback = lambda h: _mock_unregister_post_tick(module, h)
    return module


def _mock_undo(toolset) -> bool:
    toolset.undo_count_invoked += 1
    toolset.active_undo_count = max(0, toolset.active_undo_count - 1)
    return True


def _mock_register_post_tick(module, cb) -> int:
    module.registered_post_tick.append(cb)
    handle = len(module.registered_post_tick)
    # Ejecutamos el callback simulando el post-tick de Slate
    cb(0.016)
    return handle


def _mock_unregister_post_tick(module, handle) -> None:
    pass



sys.modules["unreal"] = fake_unreal()

from dbv_unreal_ai import editor, metahuman  # noqa: E402


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

    def test_import_sound_crea_asset_de_audio(self) -> None:
        """@spec REQ-72"""
        sound_asset = "/Game/DBV/Audio/Explosion"
        ASSETS[sound_asset] = object()
        IMPORTED.append(sound_asset)
        res = json.loads(editor.import_sound(self.file.name, "/Game/DBV/Audio", "Explosion"))
        self.assertTrue(res["success"])
        self.assertEqual(res["primary"], sound_asset)
        self.assertEqual(res["kind"], "sound")


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


class UndoTurnTests(unittest.TestCase):
    """Pruebas de deshacer transacciones de un turno de la IA."""

    def setUp(self) -> None:
        unreal_mod = sys.modules["unreal"]
        unreal_mod.ToolsetLibrary.active_undo_count = 5
        unreal_mod.ToolsetLibrary.undo_count_invoked = 0

    def test_get_undo_count_devuelve_el_conteo(self) -> None:
        """@spec REQ-40"""
        report = json.loads(editor.get_undo_count())
        self.assertEqual(report["active_undo_count"], 5)

    def test_undo_turn_ejecuta_deshacer_en_slate_post_tick(self) -> None:
        """@spec REQ-40"""
        report = json.loads(editor.undo_turn(2, 5))
        self.assertTrue(report["success"])
        self.assertEqual(report["undone"], 2)
        unreal_mod = sys.modules["unreal"]
        self.assertEqual(unreal_mod.ToolsetLibrary.undo_count_invoked, 2)
        self.assertEqual(unreal_mod.ToolsetLibrary.active_undo_count, 3)

    def test_undo_turn_rechaza_si_historial_cambio(self) -> None:
        """@spec REQ-40"""
        report = json.loads(editor.undo_turn(2, expected_current=10))
        self.assertFalse(report["success"])
        self.assertIn("El historial ha cambiado", report["error"])

    def test_undo_turn_rechaza_conteo_invalido(self) -> None:
        """@spec REQ-40"""
        report = json.loads(editor.undo_turn(0, 5))
        self.assertFalse(report["success"])
        self.assertIn("positive", report["error"])


class SplatComponent:
    """Componente de splats simulado: carga o falla con el motivo que se le dé."""

    def __init__(self, loads: bool, reason: str = "", total: int = 10, drawn: int = 10, sh_degree: int = 3) -> None:
        self.loads, self.reason = loads, reason
        self.asset = types.SimpleNamespace(get_num_splats=lambda: drawn, get_total_splats=lambda: total, get_sh_degree=lambda: sh_degree)

    def load_file(self, path: str) -> bool:
        """Carga (o no) el fichero."""
        return self.loads

    def get_editor_property(self, name):  # noqa: ANN001, ANN201 — imita la API de Unreal
        """Motivo del fallo o asset."""
        return self.reason if name == "last_load_error" else self.asset


class SplatActor:
    """Actor de splats con su componente."""

    def __init__(self, component: SplatComponent) -> None:
        self.splat_component = component
        self.label = ""

    def set_actor_label(self, label: str) -> None:
        """Etiqueta."""
        self.label = label

    def get_actor_label(self) -> str:
        """Etiqueta."""
        return self.label

    def set_actor_scale3d(self, scale) -> None:  # noqa: ANN001
        """Escala."""

    def set_folder_path(self, folder) -> None:  # noqa: ANN001
        """Carpeta del Outliner."""


class LoadSplatTests(unittest.TestCase):
    """Cargar un Gaussian Splat: informe y limpieza si falla (REQ-63)."""

    def setUp(self) -> None:
        self.file = tempfile.NamedTemporaryFile(suffix=".ply", delete=False)  # noqa: SIM115 — se borra en tearDown
        self.file.close()
        self.destroyed: list[SplatActor] = []
        unreal_mod = sys.modules["unreal"]
        self.saved = {name: getattr(unreal_mod, name, None) for name in ("load_class", "Vector", "Rotator", "Name")}
        unreal_mod.load_class = lambda outer, path: SplatActor
        unreal_mod.Vector = lambda *args: args
        unreal_mod.Rotator = lambda **kwargs: kwargs
        unreal_mod.Name = str
        ACTORS.clear()

    def tearDown(self) -> None:
        Path(self.file.name).unlink(missing_ok=True)
        unreal_mod = sys.modules["unreal"]
        for name, value in self.saved.items():
            setattr(unreal_mod, name, value)
        for name in ("spawn_actor_from_class", "destroy_actor"):
            if hasattr(EditorActorSubsystem, name):
                delattr(EditorActorSubsystem, name)

    def load(self, component: SplatComponent) -> dict:
        """``load_gaussian_splat`` con un actor recién creado que lleva ``component``."""
        EditorActorSubsystem.spawn_actor_from_class = lambda _self, cls, loc, rot: SplatActor(component)
        EditorActorSubsystem.destroy_actor = lambda _self, actor: self.destroyed.append(actor) or True
        return json.loads(editor.load_gaussian_splat(self.file.name, "Iglesia_Splat"))

    def test_el_informe_da_splats_dibujados_y_grado_de_sh(self) -> None:
        """@spec REQ-63 — la IA puede decirle al usuario qué se ve."""
        report = self.load(SplatComponent(loads=True, total=9975598, drawn=9975598, sh_degree=3))
        self.assertTrue(report["success"])
        self.assertEqual((report["splats"], report["drawn"], report["sh_degree"]), (9975598, 9975598, 3))
        self.assertNotIn("note", report)

    def test_si_no_se_dibujan_todos_hay_una_nota(self) -> None:
        """@spec REQ-63"""
        report = self.load(SplatComponent(loads=True, total=20, drawn=5))
        self.assertIn("5 most visible splats of 20", report["note"])

    def test_si_se_reducen_los_sh_la_nota_lo_dice(self) -> None:
        """@spec REQ-62, REQ-63"""
        component = SplatComponent(loads=True, sh_degree=0)
        component.asset.get_file_sh_degree = lambda: 3
        report = self.load(component)
        self.assertIn("reduced from degree 3 to 0", report["note"])

    def test_si_falla_da_el_motivo_y_no_deja_un_actor_vacio(self) -> None:
        """@spec REQ-63 — en la prueba del 2026-10-31 quedaba un actor vacío y la IA improvisó 38 minutos."""
        report = self.load(SplatComponent(loads=False, reason="PLY format 'ascii' is not supported (only binary_little_endian)"))
        self.assertFalse(report["success"])
        self.assertIn("binary_little_endian", report["error"])
        self.assertIn("do not try to load it with your own code", report["error"])
        self.assertEqual(len(self.destroyed), 1)

    def test_durante_play_se_explica_en_vez_de_fallar_en_silencio(self) -> None:
        """@spec REQ-63 — visto en vivo el 2026-10-31: devolvía success false sin motivo."""
        unreal_mod = sys.modules["unreal"]
        unreal_mod.UnrealEditorSubsystem = type("UnrealEditorSubsystem", (), {"get_game_world": lambda self: object()})
        try:
            report = self.load(SplatComponent(loads=True))
        finally:
            del unreal_mod.UnrealEditorSubsystem
        self.assertFalse(report["success"])
        self.assertIn("stop Play", report["error"])

    def test_si_no_se_puede_crear_el_actor_se_explica(self) -> None:
        """@spec REQ-63"""
        EditorActorSubsystem.spawn_actor_from_class = lambda _self, cls, loc, rot: None
        report = json.loads(editor.load_gaussian_splat(self.file.name, "X"))
        self.assertFalse(report["success"])
        self.assertIn("could not be created", report["error"])

    def test_un_fichero_que_no_existe_se_explica(self) -> None:
        """@spec REQ-63"""
        report = json.loads(editor.load_gaussian_splat("Z:/no/existe.ply", "X"))
        self.assertFalse(report["success"])
        self.assertIn("does not exist", report["error"])


class PlayerStartFake:
    """PlayerStart con su posición."""

    def __init__(self, label: str, x: float, y: float, z: float) -> None:
        self.label = label
        self.location = types.SimpleNamespace(x=x, y=y, z=z)

    def get_actor_label(self) -> str:
        """Etiqueta."""
        return self.label

    def get_actor_location(self):  # noqa: ANN201 — imita la API de Unreal
        """Posición."""
        return self.location

    def set_actor_location(self, location, sweep, teleport) -> None:  # noqa: ANN001
        """Mueve el actor (``unreal.Vector`` simulado devuelve la tupla)."""
        self.location = types.SimpleNamespace(x=location[0], y=location[1], z=location[2])


class LiftPlayerStartTests(unittest.TestCase):
    """Al colocar un mundo, el punto de salida del jugador queda sobre su terreno (REQ-18)."""

    def setUp(self) -> None:
        unreal_mod = sys.modules["unreal"]
        self.saved = getattr(unreal_mod, "Vector", None)
        unreal_mod.Vector = lambda *args: args

    def tearDown(self) -> None:
        sys.modules["unreal"].Vector = self.saved

    def test_un_punto_de_salida_bajo_la_duna_sube_sobre_ella(self) -> None:
        """@spec REQ-18 — visto en vivo: a 92 cm con la duna a 104, el personaje caía bajo el mundo."""
        start = PlayerStartFake("PlayerStart", 0.0, 0.0, 92.0)
        lifted = editor._lift_player_starts([start], lambda x, y: 104.0)
        self.assertEqual(lifted, ["PlayerStart"])
        self.assertEqual(start.location.z, 104.0 + editor.PLAYER_START_CLEARANCE)

    def test_uno_ya_alto_o_fuera_del_mundo_no_se_toca(self) -> None:
        """@spec REQ-18"""
        high = PlayerStartFake("Alto", 0.0, 0.0, 900.0)
        outside = PlayerStartFake("Fuera", 90000.0, 0.0, -50.0)
        lifted = editor._lift_player_starts([high, outside], lambda x, y: 104.0 if x < 50000 else None)
        self.assertEqual(lifted, [])
        self.assertEqual((high.location.z, outside.location.z), (900.0, -50.0))


class MetaHumanCompFake:
    """Componente esquelético de MetaHuman simulado."""

    def __init__(self, name: str, mesh: object | None = None) -> None:
        self._name = name
        self._mesh = mesh
        self.leader = None
        self.cleared_overrides = False
        self.assigned_materials: dict[int, object] = {}

    def get_name(self) -> str:
        """Nombre del componente."""
        return self._name

    def get_skeletal_mesh_asset(self):  # noqa: ANN201
        """Malla asignada."""
        return self._mesh

    def set_skeletal_mesh_asset(self, mesh) -> None:  # noqa: ANN001
        """Asigna malla."""
        self._mesh = mesh

    def empty_override_materials(self) -> None:
        """Limpia los override materials heredados del MetaHuman."""
        self.cleared_overrides = True

    def set_material(self, index: int, mat: object) -> None:
        """Asigna un material explícito a una ranura del componente."""
        self.assigned_materials[index] = mat

    def set_leader_pose_component(self, body, force_update: bool = False) -> None:  # noqa: ANN001
        """Sincroniza con Body."""
        self.leader = (body, force_update)

    def modify(self) -> bool:
        """Marca para Undo."""
        return True


class Vec3Fake:
    """Vector 3D con operaciones aritméticas básicas para GeometryScript."""

    def __init__(self, x: float = 0.0, y: float = 0.0, z: float = 0.0) -> None:
        self.x, self.y, self.z = float(x), float(y), float(z)

    def __sub__(self, other: Vec3Fake) -> Vec3Fake:
        return Vec3Fake(self.x - other.x, self.y - other.y, self.z - other.z)

    def __add__(self, other: Vec3Fake) -> Vec3Fake:
        return Vec3Fake(self.x + other.x, self.y + other.y, self.z + other.z)

    def __mul__(self, scalar: float) -> Vec3Fake:
        return Vec3Fake(self.x * scalar, self.y * scalar, self.z * scalar)


class MetaHumanTests(unittest.TestCase):
    """Listado y auto-rigging de prendas en MetaHumans (REQ-81)."""

    def setUp(self) -> None:
        ASSETS.clear()
        ACTORS.clear()
        unreal_mod = sys.modules["unreal"]
        unreal_mod.Vector = Vec3Fake
        unreal_mod.DynamicMesh = lambda: types.SimpleNamespace(scaled=None, translated=None, weighted=False)
        unreal_mod.GeometryScriptMeshReadLOD = lambda: object()
        unreal_mod.GeometryScriptCopyMeshFromAssetOptions = lambda: object()
        unreal_mod.GeometryScriptTransferBoneWeightsOptions = lambda: types.SimpleNamespace(transfer_method=None, num_smoothing_iterations=0, smoothing_strength=0.0)
        unreal_mod.TransferBoneWeightsMethod = types.SimpleNamespace(INPAINT_WEIGHTS="INPAINT_WEIGHTS")
        unreal_mod.GeometryScriptCreateNewSkeletalMeshAssetOptions = lambda: types.SimpleNamespace(enable_recompute_normals=True, enable_recompute_tangents=False, materials={})
        unreal_mod.GeometryScript_AssetUtils = types.SimpleNamespace(
            copy_mesh_from_skeletal_mesh=lambda skm, dyn, opts, lod: (dyn, "ok"),
            copy_mesh_from_static_mesh_v2=lambda sm, dyn, opts, lod, mats: (dyn, "ok"),
        )
        unreal_mod.GeometryScript_MeshQueries = types.SimpleNamespace(
            get_mesh_bounding_box=lambda dyn: types.SimpleNamespace(min=Vec3Fake(-20.0, -10.0, 0.0), max=Vec3Fake(20.0, 10.0, 50.0)),
        )
        unreal_mod.GeometryScript_MeshTransforms = types.SimpleNamespace(
            scale_mesh=lambda dyn, scale, origin: dyn,
            translate_mesh=lambda dyn, offset: dyn,
        )
        unreal_mod.GeometryScript_BoneWeights = types.SimpleNamespace(
            transfer_bone_weights_from_mesh=lambda src, dst, opts: dst,
            mesh_has_bone_weights=lambda dst: (dst, True),
        )
        unreal_mod.GeometryScript_NewAssetUtils = types.SimpleNamespace(
            create_new_skeletal_mesh_asset_from_mesh=lambda dyn, skel, path, opts: (
                types.SimpleNamespace(get_path_name=lambda: f"{path}.{path.rsplit('/', 1)[-1]}"),
                "ok",
            ),
        )
        unreal_mod.EditorAssetLibrary.save_loaded_asset = lambda asset, only_dirty: True

    def _make_mh_actor(self, label: str = "BP_Victoria") -> object:
        skel = types.SimpleNamespace(get_path_name=lambda: "/Game/MetaHumans/Common/Female/Medium/NormalWeight/Body/metahuman_base_skel")
        body_mesh = types.SimpleNamespace(
            get_path_name=lambda: "/Game/MetaHumans/Victoria/Body/f_med_nrw_body.f_med_nrw_body",
            get_bounds=lambda: types.SimpleNamespace(origin=Vec3Fake(0.0, 8.9, 49.8), box_extent=Vec3Fake(45.3, 13.2, 46.7)),
            get_editor_property=lambda name: skel if name == "skeleton" else None,
        )
        torso_mesh = types.SimpleNamespace(
            get_path_name=lambda: "/Game/MetaHumans/Common/Female/Tops/f_med_nrw_top.f_med_nrw_top",
            get_bounds=lambda: types.SimpleNamespace(origin=Vec3Fake(0.0, 2.2, 114.0), box_extent=Vec3Fake(41.9, 12.2, 24.7)),
        )
        comps = [
            MetaHumanCompFake("Body", body_mesh),
            MetaHumanCompFake("Torso", torso_mesh),
            MetaHumanCompFake("Legs", None),
            MetaHumanCompFake("Feet", None),
        ]
        return types.SimpleNamespace(
            get_actor_label=lambda: label,
            get_class=lambda: types.SimpleNamespace(get_name=lambda: f"{label}_C"),
            get_actor_location=lambda: Vec3Fake(100.0, 200.0, 90.0),
            get_components_by_class=lambda cls: comps,
            modify=lambda: True,
            comps={c.get_name(): c for c in comps},
        )

    def test_list_metahumans_detecta_cuerpo_y_ranuras(self) -> None:
        """@spec REQ-81"""
        ACTORS.append(self._make_mh_actor("BP_Victoria_med_nrw_55"))
        report = json.loads(editor.list_metahumans())
        self.assertEqual(report["count"], 1)
        mh = report["metahumans"][0]
        self.assertEqual(mh["label"], "BP_Victoria_med_nrw_55")
        self.assertEqual(mh["build"], "f_med_nrw")
        self.assertIn("Torso", mh["slots"])

    def test_equip_metahuman_outfit_transfiere_pesos_y_asigna_ranura(self) -> None:
        """@spec REQ-81"""
        mh = self._make_mh_actor("BP_Pablo")
        ACTORS.append(mh)
        sm = StaticMesh()
        sm.get_editor_property = lambda name: [types.SimpleNamespace(material_slot_name="M_Jacket", material_interface=object())] if name == "static_materials" else None
        ASSETS["/Game/DBV/Generated/Chaqueta"] = sm

        report = json.loads(editor.equip_metahuman_outfit("BP_Pablo", "/Game/DBV/Generated/Chaqueta", slot="Torso", asset_name="ChaquetaCuero"))
        self.assertTrue(report["success"], report)
        self.assertEqual(report["actor"], "BP_Pablo")
        self.assertEqual(report["slot"], "Torso")
        self.assertTrue(report["has_bone_weights"])
        self.assertIn("SK_ChaquetaCuero", report["skeletal_mesh"])
        self.assertIsNotNone(mh.comps["Torso"].get_skeletal_mesh_asset())
        self.assertEqual(mh.comps["Torso"].leader[0], mh.comps["Body"])
        self.assertTrue(mh.comps["Torso"].cleared_overrides)
        self.assertIn(0, mh.comps["Torso"].assigned_materials)

    def test_equip_metahuman_outfit_acepta_llamada_de_la_app_y_valida_tipos(self) -> None:
        """@spec REQ-81 — regresión del bug donde scale_multiplier caía en folder_path."""
        mh = self._make_mh_actor("BP_Victoria_med_nrw_55")
        ACTORS.append(mh)
        mat_obj = object()
        sm = StaticMesh()
        sm.get_editor_property = lambda name: [types.SimpleNamespace(material_slot_name="M_Top", material_interface=mat_obj)] if name == "static_materials" else None
        ASSETS["/Game/DBV/MetaHumanOutfits/TopHalter"] = sm

        # 1. Llamada con argumentos por nombre tal como la genera la app / MCP
        kw_report = json.loads(
            editor.equip_metahuman_outfit(
                actor_label="BP_Victoria_med_nrw_55",
                garment_source="/Game/DBV/MetaHumanOutfits/TopHalter",
                slot="Torso",
                scale_multiplier=1.05,
                z_offset=0.0,
            )
        )
        self.assertTrue(kw_report["success"], kw_report)
        self.assertEqual(mh.comps["Torso"].assigned_materials.get(0), mat_obj)

        # 2. Llamada posicional heredada de 5 argumentos (scale_multiplier en 4º lugar)
        pos_report = json.loads(
            editor.equip_metahuman_outfit(
                "BP_Victoria_med_nrw_55",
                "/Game/DBV/MetaHumanOutfits/TopHalter",
                "Torso",
                1.05,  # type: ignore[arg-type]
                0.0,  # type: ignore[arg-type]
            )
        )
        self.assertTrue(pos_report["success"], pos_report)

        # 3. folder_path inválido devuelve JSON limpio sin excepción
        bad_report = json.loads(
            editor.equip_metahuman_outfit(
                actor_label="BP_Victoria_med_nrw_55",
                garment_source="/Game/DBV/MetaHumanOutfits/TopHalter",
                folder_path="bad/folder",
            )
        )
        self.assertFalse(bad_report["success"])
        self.assertIn("folder_path", bad_report["error"])

    def test_equip_sin_destino_no_equipa_a_nadie_y_enumera_los_metahumans(self) -> None:
        """@spec REQ-83 — con 3 MetaHumans y sin etiqueta, ninguno cambia; con etiqueta, solo ese."""
        actors = [self._make_mh_actor(label) for label in ("BP_Uno", "BP_Dos", "BP_Tres")]
        ACTORS.extend(actors)
        sm = StaticMesh()
        sm.get_editor_property = lambda name: [types.SimpleNamespace(material_slot_name="M_Top", material_interface=object())] if name == "static_materials" else None
        ASSETS["/Game/DBV/MetaHumanOutfits/Top"] = sm

        sin_destino = json.loads(editor.equip_metahuman_outfit("", "/Game/DBV/MetaHumanOutfits/Top", slot="Torso"))
        self.assertFalse(sin_destino["success"], sin_destino)
        self.assertEqual(sin_destino["available"], ["BP_Uno", "BP_Dos", "BP_Tres"])
        self.assertIn("BP_Dos", sin_destino["error"])
        self.assertTrue(all(not a.comps["Torso"].leader for a in actors), "ningún personaje debe haber cambiado")

        con_destino = json.loads(editor.equip_metahuman_outfit("BP_Dos", "/Game/DBV/MetaHumanOutfits/Top", slot="Torso"))
        self.assertTrue(con_destino["success"], con_destino)
        self.assertEqual(con_destino["actor"], "BP_Dos")
        self.assertEqual(con_destino["build"], "f_med_nrw", "REQ-84: el resultado dice la complexión para avisar del corte")
        self.assertTrue(actors[1].comps["Torso"].leader)
        self.assertFalse(actors[0].comps["Torso"].leader or actors[2].comps["Torso"].leader)


class PlaytestTrackerTests(unittest.TestCase):
    """Play-testing autónomo por ticks de Slate en PIE (REQ-82)."""

    def setUp(self) -> None:
        unreal_mod = sys.modules["unreal"]
        self.moves: list[tuple[object, float]] = []
        self.jumps = 0
        self.pos = [0.0, 0.0, 92.0]

        def _add_move(vec, scale, force):
            self.moves.append((vec, scale))
            self.pos[0] += 120.0 * float(scale)

        def _jump():
            self.jumps += 1

        pawn = types.SimpleNamespace(
            get_class=lambda: types.SimpleNamespace(get_name=lambda: "BP_ThirdPersonCharacter_C"),
            get_actor_location=lambda: Vec3Fake(self.pos[0], self.pos[1], self.pos[2]),
            get_actor_forward_vector=lambda: Vec3Fake(1.0, 0.0, 0.0),
            get_actor_right_vector=lambda: Vec3Fake(0.0, 1.0, 0.0),
            add_movement_input=_add_move,
            jump=_jump,
        )
        unreal_mod.EditorLevelLibrary = types.SimpleNamespace(get_pie_worlds=lambda _: [object()])
        unreal_mod.GameplayStatics = types.SimpleNamespace(get_player_pawn=lambda world, idx: pawn)

    def test_playtest_tracker_mueve_pawn_y_calcula_telemetria(self) -> None:
        """@spec REQ-82"""
        started = json.loads(editor.start_pie_playtest_tracker(0.5, json.dumps(["forward", "jump"])))
        self.assertTrue(started["started"])
        # Simulamos ticks adicionales hasta superar 0.5s
        unreal_mod = sys.modules["unreal"]
        cb = unreal_mod.registered_post_tick[-1]
        for _ in range(5):
            cb(0.12)
        telemetry = json.loads(editor.read_pie_playtest_telemetry())
        self.assertTrue(telemetry["completed"])
        self.assertTrue(telemetry["pawn_spawned"])
        self.assertGreater(telemetry["distance_moved_cm"], 100.0)
        self.assertFalse(telemetry["fell_out_of_world"])
        self.assertEqual(self.jumps, 1)


class FakeGarmentMesh:
    """Malla mínima para ``_conform_skirt``: vértices y triángulos en listas."""

    def __init__(self, verts: list[tuple[float, float, float]], tris: list[tuple[int, int, int]]) -> None:
        self.verts = [Vec3Fake(*p) for p in verts]
        self.tris = tris

    def get_vertex_count(self) -> int:
        """Número de vértices."""
        return len(self.verts)

    def get_triangle_count(self) -> int:
        """Número de triángulos."""
        return len(self.tris)


def _ring(radius: float, z: float, count: int = 24, y_scale: float = 1.0) -> list[tuple[float, float, float]]:
    """Anillo horizontal de vértices alrededor del eje Z."""
    return [(radius * math.cos(2.0 * math.pi * i / count), y_scale * radius * math.sin(2.0 * math.pi * i / count), z) for i in range(count)]


def _tube_tris(first: int, count: int, rows: int) -> list[tuple[int, int, int]]:
    """Triángulos de un tubo formado por ``rows`` anillos consecutivos de ``count`` vértices."""
    tris: list[tuple[int, int, int]] = []
    for row in range(rows - 1):
        for i in range(count):
            a, b = first + row * count + i, first + row * count + (i + 1) % count
            c, d = a + count, b + count
            tris += [(a, b, c), (b, d, c)]
    return tris


class SkirtFitTests(unittest.TestCase):
    """Ajuste de faldas y cuello descubierto en MetaHumans (REQ-81)."""

    def setUp(self) -> None:
        self.unreal_mod = sys.modules["unreal"]
        self.unreal_mod.Vector = Vec3Fake

    def test_envolvente_convexa_y_radio_de_un_cuadrado(self) -> None:
        """@spec REQ-81"""
        hull = metahuman._convex_hull([(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (0.0, 0.0), (0.2, -0.3)])
        self.assertEqual(len(hull), 4)
        self.assertNotIn((0.0, 0.0), hull)
        self.assertAlmostEqual(metahuman._hull_radius(hull, 0.0, 0.0), 1.0, places=5)
        self.assertAlmostEqual(metahuman._hull_radius(hull, 0.0, math.pi / 4.0), math.sqrt(2.0), places=5)

    def test_falda_cae_desde_la_cadera_y_no_entra_entre_las_piernas(self) -> None:
        """Bajo la cadera el radio no disminuye y el objetivo rellena el hueco entre los muslos. @spec REQ-81"""
        body: list[tuple[float, float, float]] = []
        for z in range(60, 100):
            if z < 80:  # dos muslos separados: círculos a ambos lados del eje
                for x0 in (-9.0, 9.0):
                    body += [(x0 + 7.0 * math.cos(a / 12.0 * 2.0 * math.pi), 7.0 * math.sin(a / 12.0 * 2.0 * math.pi), float(z)) for a in range(12)]
            else:  # la cintura se estrecha hacia arriba
                radius = 18.0 - (z - 80) * 0.25
                body += _ring(radius, float(z), 24)
        body.append((40.0, 0.0, 70.0))  # una mano colgando: debe ignorarse
        cy, grid = metahuman._skirt_target_radii(body, 60.0, 99.0, 12, 24, 0.5)
        self.assertAlmostEqual(cy, 0.0, delta=0.5)
        front = 18  # theta = +90 grados: el hueco entre muslos, de frente
        self.assertGreater(grid[0][front], 6.5)
        side = 12  # theta = 0 grados
        hip = max(range(12), key=lambda iz: grid[iz][side])
        for iz in range(hip):
            self.assertGreaterEqual(grid[iz][side] + 1e-6, grid[iz + 1][side] - 0.6)
        self.assertLess(max(max(row) for row in grid), 24.0)
        self.assertLess(grid[11][side], grid[hip][side])

    def test_mapa_radial_guarda_solo_la_capa_exterior(self) -> None:
        """@spec REQ-81"""
        count, rows = 24, 4
        outer = [p for iz in range(rows) for p in _ring(12.0, 70.0 + iz * 5.0, count)]
        inner = [p for iz in range(rows) for p in _ring(6.0, 70.0 + iz * 5.0, count)]
        tris = _tube_tris(0, count, rows) + _tube_tris(len(outer), count, rows)
        fine = metahuman._outer_radius_map(outer + inner, tris, 0.0, 70.0, 85.0, 12, 48)
        values = [r for row in fine for r in row if r > 0.0]
        self.assertGreater(len(values), 400)
        self.assertGreater(min(values), 11.0)

    def test_ajuste_cine_saca_la_tela_de_la_piel_y_deja_dentro_lo_interior(self) -> None:
        """Regresión: el empuje a la superficie no debe sacar la geometría interior (restos del maniquí). @spec REQ-81"""
        count, rows = 36, 6
        zs = [40.0 + i * 8.0 for i in range(rows)]
        outer = [p for z in zs for p in _ring(24.0, z, count)]  # falda abombada
        inner = [p for z in zs for p in _ring(9.0, z, count)]  # resto del maniquí con textura de piel
        mesh = FakeGarmentMesh(outer + inner, _tube_tris(0, count, rows) + _tube_tris(len(outer), count, rows))

        def bounding_box(dyn):  # noqa: ANN001, ANN202
            xs, ys, zs_ = [v.x for v in dyn.verts], [v.y for v in dyn.verts], [v.z for v in dyn.verts]
            return types.SimpleNamespace(min=Vec3Fake(min(xs), min(ys), min(zs_)), max=Vec3Fake(max(xs), max(ys), max(zs_)))

        def set_position(dyn, vid, pos, _defer):  # noqa: ANN001, ANN202
            dyn.verts[vid] = pos

        self.unreal_mod.GeometryScript_MeshQueries = types.SimpleNamespace(
            get_mesh_bounding_box=bounding_box,
            get_vertex_position=lambda dyn, vid: (dyn.verts[vid], True),
            get_triangle_indices=lambda dyn, tid: (types.SimpleNamespace(x=dyn.tris[tid][0], y=dyn.tris[tid][1], z=dyn.tris[tid][2]), True),
        )
        self.unreal_mod.GeometryScript_MeshEdits = types.SimpleNamespace(set_vertex_position=set_position)
        body = [p for z in range(66, 97) for p in _ring(15.0, float(z), 36)]
        metahuman._conform_skirt(mesh, body, 66.0, 96.0)
        radii = [math.hypot(v.x, v.y) for v in mesh.verts]
        outer_radii, inner_radii = radii[: len(outer)], radii[len(outer):]
        self.assertGreater(min(outer_radii), 15.0, "la piel no debe atravesar la tela")
        self.assertLess(max(outer_radii), 17.0, "la falda abombada debe ceñirse a la cadera")
        self.assertLess(max(inner_radii), min(outer_radii) - 1.0, "lo interior debe seguir tapado")
        self.assertAlmostEqual(min(v.z for v in mesh.verts), 66.0, delta=0.01)
        self.assertAlmostEqual(max(v.z for v in mesh.verts), 96.0, delta=0.01)

    def test_pesos_de_falda_son_continuos_y_el_centro_sigue_a_las_dos_piernas(self) -> None:
        """Regresión: con los pesos copiados de la piel la tela se partía por el centro al andar. @spec REQ-81"""
        top = metahuman._skirt_weights(0.0, 95.0, 66.0, 90.0)
        self.assertEqual(top, (1.0, 0.0, 0.0))
        pelvis, left, right = metahuman._skirt_weights(0.0, 66.0, 66.0, 90.0)
        self.assertAlmostEqual(left, right, places=6)
        self.assertAlmostEqual(pelvis + left + right, 1.0, places=6)
        self.assertGreater(left, 0.3)
        side = metahuman._skirt_weights(15.0, 66.0, 66.0, 90.0)
        self.assertGreater(side[1], 0.8)
        self.assertAlmostEqual(side[2], 0.0, places=6)
        previous = metahuman._skirt_weights(-12.0, 70.0, 66.0, 90.0)
        for step in range(1, 25):
            current = metahuman._skirt_weights(-12.0 + step, 70.0, 66.0, 90.0)
            self.assertLess(max(abs(a - b) for a, b in zip(previous, current)), 0.15, "sin saltos entre vértices vecinos")
            previous = current

    def test_arriba_conserva_los_pesos_de_la_piel_y_abajo_usa_los_continuos(self) -> None:
        """Regresión: sin los pesos de la piel, la cadera asomaba por la cinturilla al andar. @spec REQ-81"""
        self.assertEqual(metahuman._skirt_blend(90.0, 84.0), 0.0)
        self.assertEqual(metahuman._skirt_blend(84.0, 84.0), 0.0)
        self.assertEqual(metahuman._skirt_blend(70.0, 84.0), 1.0)
        self.assertAlmostEqual(metahuman._skirt_blend(78.0, 84.0), 0.5, places=6)
        mixed = metahuman._blend_weights({1: 0.6, 2: 0.4}, {2: 0.5, 3: 0.5}, 0.5)
        self.assertAlmostEqual(sum(mixed.values()), 1.0, places=6)
        self.assertAlmostEqual(mixed[1], 0.3, places=6)
        self.assertAlmostEqual(mixed[2], 0.45, places=6)
        self.assertAlmostEqual(mixed[3], 0.25, places=6)
        self.assertEqual(metahuman._blend_weights({1: 1.0}, {2: 1.0}, 1.0), {2: 1.0})

    def test_capas_ocultas_se_quitan_pero_no_la_tela_bajo_una_solapa_parcial(self) -> None:
        """Lo interior desaparece y lo que se ve aunque sea en parte se conserva (REJECTED-10). @spec REQ-81"""
        count, rows = 24, 4
        outer = [p for iz in range(rows) for p in _ring(12.0, 70.0 + iz * 5.0, count)]
        inner = [p for iz in range(rows) for p in _ring(6.0, 70.0 + iz * 5.0, count)]
        verts = outer + inner
        out_tris = _tube_tris(0, count, rows)
        in_tris = _tube_tris(len(outer), count, rows)
        # solapa: un cuarto de vuelta de la franja central, más afuera que la tela
        flap_ring = [(15.0 * math.cos(math.pi * i / 24.0), 15.0 * math.sin(math.pi * i / 24.0), z) for z in (74.0, 81.0) for i in range(7)]
        first = len(verts)
        verts = verts + flap_ring
        flap_tris = [t for i in range(6) for t in ((first + i, first + i + 1, first + 7 + i), (first + i + 1, first + 8 + i, first + 7 + i))]
        all_tris = out_tris + in_tris + flap_tris
        tris = [(tid, a, b, c) for tid, (a, b, c) in enumerate(all_tris)]
        hidden = set(metahuman._hidden_triangle_ids(verts, tris, 0.0, 0.0, 0.8))
        inner_ids = set(range(len(out_tris), len(out_tris) + len(in_tris)))
        self.assertTrue(inner_ids <= hidden, "toda la capa interior está oculta")
        flap_ids = set(range(len(out_tris) + len(in_tris), len(all_tris)))
        self.assertFalse(hidden & flap_ids, "la solapa se ve desde fuera")
        visible_outer = set(range(len(out_tris))) - hidden
        self.assertGreater(len(visible_outer), len(out_tris) * 0.6, "la tela exterior no tapada se conserva")

    def test_no_importa_durante_play(self) -> None:
        """Regresión: importar en Play creaba una malla vacía con materiales dinámicos imposibles de guardar. @spec REQ-81"""

        class LevelEditorSubsystem:
            @staticmethod
            def is_in_play_in_editor() -> bool:
                return True

        previous_class = getattr(self.unreal_mod, "LevelEditorSubsystem", None)
        previous_getter = getattr(self.unreal_mod, "get_editor_subsystem", None)
        self.unreal_mod.LevelEditorSubsystem = LevelEditorSubsystem
        self.unreal_mod.get_editor_subsystem = lambda cls: cls()
        try:
            report = json.loads(editor.import_model("C:/no-importa.glb", "/Game/DBV/MetaHumanOutfits", "Vestido"))
        finally:
            self.unreal_mod.LevelEditorSubsystem = previous_class
            self.unreal_mod.get_editor_subsystem = previous_getter
        self.assertFalse(report["success"])
        self.assertIn("Play mode", report["error"])

    def test_uplugin_no_depende_de_plugins_inexistentes(self) -> None:
        """Regresión: ``GLTFImporter`` no existe en UE 5.8 y su dependencia impedía abrir el proyecto. @spec REQ-81"""
        uplugin = Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "DBVUnrealAI.uplugin"
        names = [p["Name"] for p in json.loads(uplugin.read_text(encoding="utf-8"))["Plugins"]]
        self.assertNotIn("GLTFImporter", names)
        self.assertIn("GeometryScripting", names)

    def test_criterio_de_falda_solo_en_legs(self) -> None:
        """@spec REQ-81"""
        self.assertTrue(metahuman._is_skirt_shape("Legs", 38.0, 30.0))
        self.assertFalse(metahuman._is_skirt_shape("Legs", 90.0, 30.0))
        self.assertFalse(metahuman._is_skirt_shape("Torso", 38.0, 30.0))

    def test_limite_de_brazos_no_recorta_la_cintura(self) -> None:
        """Con hombros a 14 cm y cintura real de 14,7 cm, el cuerpo no debe recortarse (REQ-81). @spec REQ-81"""
        self.assertGreater(metahuman._arm_cutoff("Torso", 96.0, 127.4, 133.0, 14.0), 15.0)
        self.assertLess(metahuman._arm_cutoff("Torso", 123.0, 127.4, 133.0, 14.0), 14.0)
        self.assertGreater(metahuman._arm_cutoff("Legs", 78.0, 127.4, 133.0, 14.0), 17.8)

    def test_apaga_neck_hide_en_la_cabeza_y_no_toca_lo_demas(self) -> None:
        """Quitar la camisa de fábrica deja a la vista el cuello encogido por material (REQ-81). @spec REQ-81"""

        class Constant:
            def __init__(self, path: str, value: float) -> None:
                self.path, self.value = path, value

            def get_path_name(self) -> str:
                return self.path

        class Dynamic:
            def __init__(self, parent: object, value: float) -> None:
                self.parent, self.value = parent, value

            def get_scalar_parameter_value(self, _name: str) -> float:
                return self.value

            def set_scalar_parameter_value(self, _name: str, value: float) -> None:
                self.value = value

            def get_editor_property(self, _name: str) -> object:
                return self.parent

        head = Constant("/Game/MetaHumans/V/Face/Materials/MI_Head", -2.0)
        teeth = Constant("/Game/MetaHumans/V/Face/Materials/MI_Teeth", 0.0)
        mats = [Dynamic(head, -2.0), Dynamic(teeth, 0.0), head]
        face = types.SimpleNamespace(get_num_materials=lambda: len(mats), get_material=lambda i: mats[i])
        saved: list[object] = []
        updated: list[object] = []
        self.unreal_mod.MaterialInstanceConstant = Constant
        self.unreal_mod.MaterialEditingLibrary = types.SimpleNamespace(
            get_material_instance_scalar_parameter_value=lambda mi, _n: mi.value,
            set_material_instance_scalar_parameter_value=lambda mi, _n, v: setattr(mi, "value", v),
            update_material_instance=updated.append,
        )
        self.unreal_mod.EditorAssetLibrary.save_loaded_asset = lambda asset, _dirty: saved.append(asset)
        self.assertEqual(metahuman._disable_neck_hide(face), [head.path])
        self.assertEqual((mats[0].value, head.value, teeth.value), (0.0, 0.0, 0.0))
        self.assertEqual(saved, [head])
        self.assertEqual(updated, [head])
        self.assertEqual(metahuman._disable_neck_hide(face), [], "es idempotente")
        self.assertEqual(metahuman._disable_neck_hide(None), [])


if __name__ == "__main__":
    unittest.main()

