# =============================================================================
# DBV Unreal AI — Herramientas del toolset que necesitan el editor
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Implementación compartida por la variante Python (``toolset``) y la C++ (``cpp_bridge``).

Cada función devuelve una cadena JSON. @spec REQ-06, REQ-25, REQ-26
"""

from __future__ import annotations

import json
import sys
from typing import Any

import unreal

from dbv_unreal_ai import analysis

#: Versión del contrato de herramientas (la app la compara con la que trae).
TOOLSET_VERSION = "0.1.0"


def _json(value: Any) -> str:
    """Serializa el resultado de una herramienta."""
    return json.dumps(value, ensure_ascii=False, default=str)


def _transaction() -> Any:
    """Contexto de deshacer del editor para lo que ejecuta ``run_python``."""
    return unreal.ScopedEditorTransaction("DBV Unreal AI")


def _round(vector: Any) -> list[float]:
    """Vector de Unreal → lista con un decimal."""
    return [round(vector.x, 1), round(vector.y, 1), round(vector.z, 1)]


def run_python(code: str) -> str:
    """Ejecuta ``code`` en una transacción del editor. @spec REQ-06"""
    return _json(analysis.run_code(code, unreal, _transaction))


def validate_python(code: str) -> str:
    """Valida ``code`` contra el motor en marcha sin ejecutarlo. @spec REQ-26"""
    return _json(analysis.validate_code(code, unreal))


def describe_python_api(name: str) -> str:
    """Referencia de ``unreal.<name>``. @spec REQ-25"""
    return _json(analysis.describe_name(name, unreal))


def get_project_info() -> str:
    """Versión del motor, proyecto, nivel abierto y versión del toolset. @spec REQ-06"""
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    info = {
        "engine_version": unreal.SystemLibrary.get_engine_version(),
        "project_name": unreal.SystemLibrary.get_game_name(),
        "project_dir": unreal.Paths.convert_relative_path_to_full(unreal.Paths.project_dir()),
        "content_dir": unreal.Paths.convert_relative_path_to_full(unreal.Paths.project_content_dir()),
        "level": world.get_path_name() if world else None,
        "python_version": sys.version.split()[0],
        "toolset_version": TOOLSET_VERSION,
    }
    return _json(info)


def get_scene_summary(name_filter: str = "", class_filter: str = "", limit: int = 200) -> str:
    """Actores del nivel abierto en formato compacto. @spec REQ-06, REQ-13"""
    actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
    name_part, class_part = (name_filter or "").lower(), (class_filter or "").lower()
    counts: dict[str, int] = {}
    listed: list[dict[str, Any]] = []
    maximum = max(1, min(int(limit), 1000))
    for actor in actors:
        label, cls = actor.get_actor_label(), actor.get_class().get_name()
        if name_part in label.lower() and class_part in cls.lower():
            counts[cls] = counts.get(cls, 0) + 1
            if len(listed) < maximum:
                listed.append({"label": label, "class": cls, "location": _round(actor.get_actor_location()), "folder": str(actor.get_folder_path())})
    total = sum(counts.values())
    return _json({"total": total, "counts": counts, "actors": listed, "truncated": total > len(listed)})
