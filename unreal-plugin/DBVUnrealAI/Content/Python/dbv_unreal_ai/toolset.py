# =============================================================================
# DBV Unreal AI — Toolset de Unreal para el servidor MCP del editor
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Toolset que DBV Unreal AI instala en el proyecto. @spec REQ-06, REQ-25, REQ-26

Solo añade lo que los toolsets de Epic no traen (ver ``ARCHITECTURE.md``):
Python sin restricciones, su validación previa, referencia de la API del motor en
marcha, información del proyecto y un resumen compacto de la escena. Importar y
colocar assets lo hacen ``StaticMeshTools`` y ``SceneTools`` de ``EditorToolset``.

Cada herramienta devuelve una cadena JSON (el cliente de la app la desenvuelve).
Se registra como Epic registra los suyos: desde ``init_unreal.py``. Verificado en vivo
contra UE 5.8.2 el 2026-10-24 (registro, las cinco herramientas y desregistro).

Excepción a ``from __future__ import annotations`` (estándar del proyecto): aquí NO se
usa, porque ``toolset_registry.tool_call`` genera las ``UFunction`` a partir de las
anotaciones y necesita los tipos reales, no cadenas.
"""

from typing import Optional

import toolset_registry
import unreal
from toolset_registry.registration import Registration

from dbv_unreal_ai import editor

_registration: Optional[Registration] = None


@unreal.uclass()
class DBVUnrealAIToolset(unreal.ToolsetDefinition):
    """DBV Unreal AI: Python execution with editor undo, Python validation against the running engine, live API reference, project info and a compact scene summary."""

    @toolset_registry.tool_call
    @staticmethod
    def run_python(code: str) -> str:
        """Run Python inside the Unreal Editor (``unreal`` is already imported) inside one undoable editor transaction.

        Validate it first with ``validate_python``. To return a value, end the code with an expression (like a console) or set a variable named ``result``.

        Args:
            code: Python source code to execute.

        Returns:
            JSON with success, stdout, stderr, warnings (e.g. DeprecationWarning), error (traceback) and result.
        """
        return editor.run_python(code)

    @toolset_registry.tool_call
    @staticmethod
    def validate_python(code: str) -> str:
        """Check Python code WITHOUT running it: syntax, and that every ``unreal.Class.member`` it references exists and is not deprecated in this engine.

        Args:
            code: Python source code to analyse.

        Returns:
            JSON with ok, syntax_error and findings (kind unknown|deprecated, line, name, message, suggestions).
        """
        return editor.validate_python(code)

    @toolset_registry.tool_call
    @staticmethod
    def describe_python_api(name: str) -> str:
        """Reference of an ``unreal`` Python API name taken from the running engine (exact for this version and project).

        Args:
            name: Dotted name such as ``EditorAssetLibrary`` or ``EditorAssetLibrary.load_asset``.

        Returns:
            JSON with exists, kind, deprecated, doc and members (for classes), or suggestions.
        """
        return editor.describe_python_api(name)

    @toolset_registry.tool_call
    @staticmethod
    def get_project_info() -> str:
        """Engine version, project name and folders, open level, Python version and DBV toolset version.

        Returns:
            JSON with the project information.
        """
        return editor.get_project_info()

    @toolset_registry.tool_call
    @staticmethod
    def import_model(source_file: str, folder_path: str = "/Game/DBV/Generated", asset_name: str = "") -> str:
        """Import a 3D model file (glTF/GLB, FBX, OBJ) letting the engine choose the importer (Interchange for glTF), with its materials and textures.

        Args:
            source_file: Absolute path of the file on disk.
            folder_path: Content Browser folder under /Game/ (created if missing).
            asset_name: Asset name; a numeric suffix is added if it already exists. Empty to use the file name.

        Returns:
            JSON with success, imported (asset paths), primary (the SkeletalMesh of a rigged model, else the StaticMesh), kind (skeletal|static|other), skeleton and animations (AnimSequence paths).
        """
        return editor.import_model(source_file, folder_path, asset_name)

    @toolset_registry.tool_call
    @staticmethod
    def place_asset(asset_path: str, distance: float = 300.0, folder: str = "DBV/Generated") -> str:
        """Spawn an asset in the open level in front of the editor camera, inside a World Outliner folder (undoable).

        Args:
            asset_path: Asset to place (e.g. /Game/DBV/Generated/lantern).
            distance: Distance from the camera in centimetres.
            folder: World Outliner folder for the new actor.

        Returns:
            JSON with success, actor label, location and folder.
        """
        return editor.place_asset(asset_path, distance, folder)

    @toolset_registry.tool_call
    @staticmethod
    def play_animation(actor_label: str, animation_path: str, looping: bool = True) -> str:
        """Make a skeletal mesh actor of the open level play an animation in the editor viewport (single-node mode, undoable).

        Args:
            actor_label: Label of the actor in the World Outliner (as returned by place_asset).
            animation_path: AnimSequence asset (e.g. one of the animations returned by import_model).
            looping: Loop the animation.

        Returns:
            JSON with success, actor and animation.
        """
        return editor.play_animation(actor_label, animation_path, looping)

    @toolset_registry.tool_call
    @staticmethod
    def get_scene_summary(name_filter: str = "", class_filter: str = "", limit: int = 200) -> str:
        """Compact list of the actors in the open level: label, class, location (cm), outliner folder.

        Args:
            name_filter: Only actors whose label contains this text (case-insensitive). Empty for all.
            class_filter: Only actors whose class name contains this text (case-insensitive). Empty for all.
            limit: Maximum number of actors returned (1-1000).

        Returns:
            JSON with total, counts by class and the actor list.
        """
        return editor.get_scene_summary(name_filter, class_filter, limit)

    @toolset_registry.tool_call
    @staticmethod
    def load_gaussian_splat(source_file: str, actor_label: str = "DBV_GaussianSplat") -> str:
        """Load a 3D Gaussian Splat (.spz or .ply) into a native ADbvGaussianSplatActor in the scene.

        Args:
            source_file: Absolute path to the .spz or .ply file.
            actor_label: Outliner label for the splat actor (default "DBV_GaussianSplat").

        Returns:
            JSON with success, actor, source_file.
        """
        return editor.load_gaussian_splat(source_file, actor_label)

    @toolset_registry.tool_call
    @staticmethod
    def get_undo_count() -> str:
        """Get the active transaction count from the Unreal undo buffer.

        Returns:
            JSON with active_undo_count.
        """
        return editor.get_undo_count()

    @toolset_registry.tool_call
    @staticmethod
    def undo_turn(count: int, expected_current: int = 0) -> str:
        """Undo a specified number of transactions performed during an AI assistant turn.

        Args:
            count: Number of transactions to undo.
            expected_current: Expected current active undo count to prevent undoing user actions.

        Returns:
            JSON with success, undone count, error.
        """
        return editor.undo_turn(count, expected_current)



def register() -> bool:
    """Registra el toolset en el ``ToolsetRegistry`` (lo hace ``init_unreal.py``)."""
    global _registration  # noqa: PLW0603 — una sola registración por sesión del editor
    _registration = Registration([DBVUnrealAIToolset])
    return _registration.register()


def unregister() -> None:
    """Retira el toolset del registro."""
    if _registration is not None:
        _registration.unregister()
