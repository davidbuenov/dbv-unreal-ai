# =============================================================================
# DBV Unreal AI — Puente para la variante C++ del toolset
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""``UDBVUnrealAICppToolset`` (C++) llama a estas funciones con los argumentos en base64.

Así las dos variantes ejecutan exactamente el mismo código (R-02b). @spec REQ-06
"""

from __future__ import annotations

import base64

from dbv_unreal_ai import editor


def _text(encoded: str) -> str:
    """Decodifica un argumento en base64 (UTF-8)."""
    return base64.b64decode(encoded).decode("utf-8")


def run_python(code: str) -> str:
    """Ver ``editor.run_python``."""
    return editor.run_python(_text(code))


def validate_python(code: str) -> str:
    """Ver ``editor.validate_python``."""
    return editor.validate_python(_text(code))


def describe_python_api(name: str) -> str:
    """Ver ``editor.describe_python_api``."""
    return editor.describe_python_api(_text(name))


def get_project_info() -> str:
    """Ver ``editor.get_project_info``."""
    return editor.get_project_info()


def import_model(source_file: str, folder_path: str, asset_name: str) -> str:
    """Ver ``editor.import_model``."""
    return editor.import_model(_text(source_file), _text(folder_path), _text(asset_name))


def import_sound(source_file: str, folder_path: str, asset_name: str) -> str:
    """Ver ``editor.import_sound``."""
    return editor.import_sound(_text(source_file), _text(folder_path), _text(asset_name))


def place_asset(asset_path: str, distance: str, folder: str) -> str:
    """Ver ``editor.place_asset``."""
    return editor.place_asset(_text(asset_path), float(_text(distance) or "300"), _text(folder))


def play_animation(actor_label: str, animation_path: str, looping: str) -> str:
    """Ver ``editor.play_animation``."""
    return editor.play_animation(_text(actor_label), _text(animation_path), _text(looping).lower() != "false")


def get_scene_summary(name_filter: str, class_filter: str, limit: str) -> str:
    """Ver ``editor.get_scene_summary``."""
    return editor.get_scene_summary(_text(name_filter), _text(class_filter), int(_text(limit) or "200"))


def load_gaussian_splat(source_file: str, actor_label: str) -> str:
    """Ver ``editor.load_gaussian_splat``."""
    return editor.load_gaussian_splat(_text(source_file), _text(actor_label) or "DBV_GaussianSplat")


def get_undo_count() -> str:
    """Ver ``editor.get_undo_count``."""
    return editor.get_undo_count()


def undo_turn(count: str, expected_current: str) -> str:
    """Ver ``editor.undo_turn``."""
    c = int(_text(count) or "0")
    exp = int(_text(expected_current) or "0")
    return editor.undo_turn(c, exp)


def list_metahumans() -> str:
    """Ver ``editor.list_metahumans``."""
    return editor.list_metahumans()


def equip_metahuman_outfit(
    actor_label: str,
    garment_source: str,
    slot: str,
    folder_path: str,
    asset_name: str,
    scale_multiplier: str = "",
    z_offset: str = "",
    smoothing_iterations: str = "",
) -> str:
    """Ver ``editor.equip_metahuman_outfit``."""
    return editor.equip_metahuman_outfit(
        _text(actor_label),
        _text(garment_source),
        _text(slot) or "Torso",
        _text(folder_path) or "/Game/DBV/MetaHumanOutfits",
        _text(asset_name),
        float(_text(scale_multiplier) or "1.05") if scale_multiplier else 1.05,
        float(_text(z_offset) or "0.0") if z_offset else 0.0,
        int(_text(smoothing_iterations) or "5") if smoothing_iterations else 5,
    )


def dress_metahuman(
    actor_label: str,
    garment_source: str,
    garment_kind: str = "",
    garment_cut: str = "",
    destination_folder: str = "",
    asset_name: str = "",
    scale_multiplier: str = "",
    z_offset: str = "",
    force: str = "",
    flip_orientation: str = "",
) -> str:
    """Ver ``editor.dress_metahuman``."""
    return editor.dress_metahuman(
        actor_label=_text(actor_label),
        garment_source=_text(garment_source),
        garment_kind=_text(garment_kind) or "top",
        garment_cut=_text(garment_cut) or None,
        destination_folder=_text(destination_folder) or "/Game/DBV/MetaHumanOutfits",
        asset_name=_text(asset_name),
        scale_multiplier=float(_text(scale_multiplier) or "1.05") if scale_multiplier else 1.05,
        z_offset=float(_text(z_offset) or "0.0") if z_offset else 0.0,
        force=_text(force).lower() in ("true", "1", "yes") if force else False,
        flip_orientation=_text(flip_orientation).lower() in ("true", "1", "yes") if flip_orientation else None,
    )


def dress_metahuman_start(
    actor_label: str,
    garment_source: str,
    garment_kind: str = "",
    garment_cut: str = "",
    destination_folder: str = "",
    asset_name: str = "",
    scale_multiplier: str = "",
    z_offset: str = "",
    force: str = "",
    flip_orientation: str = "",
) -> str:
    """Ver ``editor.dress_metahuman_start``."""
    return editor.dress_metahuman_start(
        actor_label=_text(actor_label),
        garment_source=_text(garment_source),
        garment_kind=_text(garment_kind) or "top",
        garment_cut=_text(garment_cut) or None,
        destination_folder=_text(destination_folder) or "/Game/DBV/MetaHumanOutfits",
        asset_name=_text(asset_name),
        scale_multiplier=float(_text(scale_multiplier) or "1.05") if scale_multiplier else 1.05,
        z_offset=float(_text(z_offset) or "0.0") if z_offset else 0.0,
        force=_text(force).lower() in ("true", "1", "yes") if force else False,
        flip_orientation=_text(flip_orientation).lower() in ("true", "1", "yes") if flip_orientation else None,
    )


def dress_metahuman_step(job_id: str) -> str:
    """Ver ``editor.dress_metahuman_step``."""
    return editor.dress_metahuman_step(_text(job_id))


def validate_metahuman_outfit(
    actor_label: str,
    garment_kind: str = "",
    run_pie: str = "",
    capture_angles: str = "",
) -> str:
    """Ver ``editor.validate_metahuman_outfit``."""
    return editor.validate_metahuman_outfit(
        actor_label=_text(actor_label),
        garment_kind=_text(garment_kind) or "top",
        run_pie=_text(run_pie).lower() in ("true", "1", "yes") if run_pie else False,
        capture_angles=_text(capture_angles).lower() not in ("false", "0", "no") if capture_angles else True,
    )


def export_tailored_outfit(
    actor_label: str = "",
    slot: str = "",
    asset_name: str = "",
    destination_dir: str = "",
) -> str:
    """Ver ``editor.export_tailored_outfit``. @spec REQ-100"""
    return editor.export_tailored_outfit(
        actor_label=_text(actor_label),
        slot=_text(slot) or "Torso",
        asset_name=_text(asset_name),
        destination_dir=_text(destination_dir),
    )




