# =============================================================================
# DBV Unreal AI — Ensamblar en un Blueprint un modelo de Fab importado por piezas
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Un FBX o glTF de Fab con varios objetos llega como varias Static Meshes (ruedas, carrocería, cristales…).

Interchange guarda la posición de cada pieza en sus vértices (comprobado el 2026-11-15 con «Camo muscle car»:
las ruedas salen en las esquinas de la carrocería), así que un Blueprint con un StaticMeshComponent por pieza,
todos en la transformación identidad, recompone el objeto. Así se coloca y se mueve como uno solo.

@spec REQ-104
"""

from __future__ import annotations

import re
from typing import Any

import unreal

#: Por debajo de esto no hace falta Blueprint: una sola malla ya es el objeto.
MIN_PIECES = 2
#: Tope de piezas: un pack con cientos de mallas no es «un objeto por piezas».
MAX_PIECES = 64


def blueprint_name(title: str) -> str:
    """`BP_<Título>` con solo letras, números y guiones bajos."""
    clean = re.sub(r"[^A-Za-z0-9_]+", "_", title or "").strip("_") or "FabModel"
    return f"BP_{clean[:60]}"


def _static_meshes(folder: str) -> list[Any]:
    eal = unreal.EditorAssetLibrary
    meshes = []
    for path in eal.list_assets(folder, recursive=True, include_folder=False):
        asset = eal.load_asset(path)
        if isinstance(asset, unreal.StaticMesh):
            meshes.append(asset)
    return meshes


def assemble_blueprint(folder: str, title: str) -> dict[str, Any]:
    """Crea `<folder>/BP_<Título>` con una pieza por Static Mesh de la carpeta. No hace nada con 0 o 1 malla."""
    res: dict[str, Any] = {"created": False, "blueprint": None, "pieces": 0, "error": None}
    meshes = _static_meshes(folder)
    res["pieces"] = len(meshes)
    name = blueprint_name(title)
    path = f"{folder}/{name}"
    eal = unreal.EditorAssetLibrary
    if len(meshes) < MIN_PIECES or len(meshes) > MAX_PIECES:
        res["error"] = None if len(meshes) < MIN_PIECES else f"{len(meshes)} meshes: too many to assemble as one object"
    elif eal.does_asset_exist(path):
        res["blueprint"] = path
    else:
        factory = unreal.BlueprintFactory()
        factory.set_editor_property("parent_class", unreal.Actor)
        bp = unreal.AssetToolsHelpers.get_asset_tools().create_asset(name, folder, unreal.Blueprint, factory)
        sub = unreal.get_engine_subsystem(unreal.SubobjectDataSubsystem)
        lib = unreal.SubobjectDataBlueprintFunctionLibrary
        handles = sub.k2_gather_subobject_data_for_blueprint(bp)
        root = handles[0] if handles else None
        used: set[str] = set()
        for mesh in meshes:
            params = unreal.AddNewSubobjectParams()
            params.set_editor_property("blueprint_context", bp)
            params.set_editor_property("new_class", unreal.StaticMeshComponent)
            if root is not None:
                params.set_editor_property("parent_handle", root)
            handle, fail = sub.add_new_subobject(params)
            if not lib.is_handle_valid(handle):
                res["error"] = f"could not add a component for {mesh.get_name()}: {fail}"
                break
            label = re.sub(r"[^A-Za-z0-9_]+", "_", mesh.get_name()) or "Piece"
            while label in used:
                label += "_"
            used.add(label)
            sub.rename_subobject(handle, unreal.Text(label))
            component = lib.get_object_for_blueprint(sub.k2_find_subobject_data_from_handle(handle), bp)
            component.set_editor_property("static_mesh", mesh)
        if res["error"] is None:
            unreal.BlueprintEditorLibrary.compile_blueprint(bp)
            eal.save_asset(path, only_if_is_dirty=False)
            res["created"] = True
            res["blueprint"] = path
    return res
