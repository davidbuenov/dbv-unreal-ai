# =============================================================================
# DBV Unreal AI — Recetas guiadas: comprobación final y Agent Skills
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Recetas de ``recipes.json`` (la misma definición que lee la app en Rust). @spec REQ-23

- ``check(id)`` mira el estado del editor sin cambiar nada y dice qué está y qué falta: «el estado del editor es el
  juez», como las tareas de REQ-37. La llama la app, el ``--mcp`` (``recipe_check``) y cualquier agente con
  ``run_python``.
- ``register_skills()`` publica cada receta como Agent Skill (una clase de Python que hereda de ``AgentSkill``),
  para que un agente la encuentre con ``ListSkills``. Verificado en vivo el 2026-11-01: el registro descubre las
  clases creadas desde Python.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Callable

import unreal

RECIPES_FILE = Path(__file__).with_name("recipes.json")
#: Altura máxima (cm) a la que un PlayerStart puede tener el suelo debajo.
GROUND_REACH = 500.0
SKILL_PREFIX = "DbvRecipe"
#: Clases de las skills ya creadas: sin una referencia viva, Python las libera y el registro deja de verlas
#: (visto en vivo el 2026-11-01), y redefinirlas en la misma sesión les añade un sufijo ``_0x…``.
_SKILLS: dict[str, Any] = {}


def load() -> list[dict[str, Any]]:
    """Las recetas, en el orden del fichero."""
    return json.loads(RECIPES_FILE.read_text(encoding="utf-8"))["recipes"]


def _actors() -> list[Any]:
    return list(unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors())


def _class_name(actor: Any) -> str:
    return actor.get_class().get_name()


def _of_class(actors: list[Any], *names: str) -> list[Any]:
    wanted = {name.lower() for name in names}
    return [actor for actor in actors if _class_name(actor).lower() in wanted]


def _has_collision(actor: Any) -> bool:
    component = actor.get_component_by_class(unreal.StaticMeshComponent)
    return component is not None and component.get_collision_enabled() != unreal.CollisionEnabled.NO_COLLISION


def _ground_below(actor: Any) -> Any:
    """El actor que hay justo debajo (hasta ``GROUND_REACH``), o ``None``."""
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    start = actor.get_actor_location()
    end = unreal.Vector(start.x, start.y, start.z - GROUND_REACH)
    hit = unreal.SystemLibrary.line_trace_single(
        world, start, end, unreal.TraceTypeQuery.ECC_VISIBILITY, False, [actor], unreal.DrawDebugTrace.NONE, True,
    ).to_tuple()
    return hit[9] if hit[0] else None


def _item(item_id: str, ok: bool, detail: str) -> dict[str, Any]:
    return {"id": item_id, "ok": bool(ok), "detail": detail}


def _player_start_item(actors: list[Any], on: Callable[[Any], bool], where: str) -> dict[str, Any]:
    starts = _of_class(actors, "PlayerStart")
    grounded = [start for start in starts if (ground := _ground_below(start)) is not None and on(ground)]
    detail = f"{len(grounded)} of {len(starts)} PlayerStart stand on {where}" if starts else "no PlayerStart"
    return _item("player_start", bool(grounded), detail)


def _check_test_level(actors: list[Any]) -> list[dict[str, Any]]:
    def is_floor(actor: Any) -> bool:
        _, extent = actor.get_actor_bounds(False)
        # El suelo de las plantillas de Epic mide 40 × 40 m con 2 m de semialtura (visto en vivo).
        large = extent.x >= 500 and extent.y >= 500 and extent.z <= 500
        return _class_name(actor) == "StaticMeshActor" and _has_collision(actor) and (actor.get_actor_label().lower().startswith("dbv_floor") or large)

    floors = [actor for actor in actors if is_floor(actor)]
    lights = _of_class(actors, "DirectionalLight")
    sky_lights = _of_class(actors, "SkyLight")
    skies = _of_class(actors, "SkyAtmosphere") + [a for a in actors if "sky" in a.get_actor_label().lower() and a not in sky_lights]
    navmesh = _of_class(actors, "NavMeshBoundsVolume")
    return [
        _item("floor", bool(floors), f"floor: {floors[0].get_actor_label()}" if floors else "no large flat floor with collision"),
        _item("light", bool(lights), f"{len(lights)} DirectionalLight"),
        _item("sky", bool(sky_lights) and bool(skies), f"{len(sky_lights)} SkyLight, {len(skies)} sky actor(s)"),
        _player_start_item(actors, lambda ground: True, "the ground"),
        _item("navmesh", bool(navmesh), f"{len(navmesh)} NavMeshBoundsVolume"),
    ]


def _dbv_assets(class_name: str) -> list[Any]:
    registry = unreal.AssetRegistryHelpers.get_asset_registry()
    found = registry.get_assets_by_path("/Game/DBV", recursive=True)
    return [data for data in found if str(data.asset_class_path.asset_name) == class_name]


def _character_mesh(blueprint_data: Any) -> Any:
    """La Skeletal Mesh del Character Blueprint, o ``None`` si no es un Character o no tiene malla."""
    blueprint = unreal.load_asset(str(blueprint_data.package_name))
    generated = blueprint.generated_class() if blueprint is not None else None
    default = unreal.get_default_object(generated) if generated is not None else None
    mesh = default.get_editor_property("mesh") if isinstance(default, unreal.Character) else None
    return mesh.get_editor_property("skeletal_mesh_asset") if mesh is not None else None


def _check_tripo_character(actors: list[Any]) -> list[dict[str, Any]]:
    del actors
    meshes = [(data, skm) for data in _dbv_assets("Blueprint") if (skm := _character_mesh(data)) is not None]
    skeletons = {str(skm.get_editor_property("skeleton").get_path_name()) for _, skm in meshes if skm.get_editor_property("skeleton")}
    animations = [
        data for data in _dbv_assets("AnimSequence")
        if (anim := unreal.load_asset(str(data.package_name))) is not None and anim.get_editor_property("skeleton") is not None
        and anim.get_editor_property("skeleton").get_path_name() in skeletons
    ]
    return [
        _item("character_blueprint", bool(meshes), f"Character Blueprint: {meshes[0][0].package_name}" if meshes else "no Character Blueprint under /Game/DBV with a Skeletal Mesh"),
        _item("animations", bool(animations), f"{len(animations)} animation(s) for its skeleton"),
    ]


def _in_world_folder(actor: Any) -> bool:
    return str(actor.get_folder_path()).startswith("DBV/Worlds")


def _check_world_walk(actors: list[Any]) -> list[dict[str, Any]]:
    splats = [actor for actor in actors if "gaussiansplat" in _class_name(actor).lower()]
    colliders = [actor for actor in actors if _in_world_folder(actor) and _class_name(actor) == "StaticMeshActor" and _has_collision(actor)]
    return [
        _item("splat", bool(splats), f"{len(splats)} Gaussian splat actor(s)"),
        _item("collision", bool(colliders), f"collision: {colliders[0].get_actor_label()}" if colliders else "no world collision mesh in DBV/Worlds"),
        _player_start_item(actors, _in_world_folder, "the world collision"),
    ]


def _convai_character_id(component: Any) -> str:
    try:
        value = str(component.get_editor_property("character_id") or "")
    except Exception:  # noqa: BLE001 — la propiedad depende de la versión del plugin de Convai
        value = "?"
    return value


def _check_convai_metahuman(actors: list[Any]) -> list[dict[str, Any]]:
    chatbots = [
        component for actor in actors for component in actor.get_components_by_class(unreal.ActorComponent)
        if "convaichatbot" in component.get_class().get_name().lower()
    ]
    with_id = [component for component in chatbots if _convai_character_id(component)]
    metahumans = [actor for actor in actors if "/game/metahumans/" in actor.get_class().get_path_name().lower()]
    navmesh = _of_class(actors, "NavMeshBoundsVolume")
    return [
        _item("metahuman", bool(metahumans), f"{len(metahumans)} MetaHuman Blueprint(s) in the level"),
        _item("convai_character", bool(with_id), f"{len(with_id)} of {len(chatbots)} Convai chatbot component(s) with a Character ID"),
        _item("navmesh", bool(navmesh), f"{len(navmesh)} NavMeshBoundsVolume"),
    ]


CHECKS: dict[str, Callable[[list[Any]], list[dict[str, Any]]]] = {
    "test_level": _check_test_level,
    "tripo_character": _check_tripo_character,
    "world_walk": _check_world_walk,
    "convai_metahuman": _check_convai_metahuman,
}


def check(recipe_id: str) -> dict[str, Any]:
    """Qué cumple el editor de la receta ``recipe_id`` y qué falta. Solo lee. @spec REQ-23"""
    if recipe_id not in CHECKS:
        return {"passed": None, "items": [], "detail": f"unknown recipe {recipe_id!r}; known: {', '.join(CHECKS)}"}
    items = CHECKS[recipe_id](_actors())
    missing = [item["id"] for item in items if not item["ok"]]
    detail = "all done" if not missing else "missing: " + ", ".join(missing)
    return {"passed": not missing, "items": items, "detail": detail}


def skill_class_name(recipe_id: str) -> str:
    """``test_level`` → ``DbvRecipeTestLevel``."""
    return SKILL_PREFIX + "".join(part.capitalize() for part in recipe_id.split("_"))


def skill_instructions(recipe: dict[str, Any]) -> str:
    """Lo que lee un agente con ``GetSkills``: los pasos y cómo comprobar el resultado."""
    return (
        f"{recipe['steps']}\n\n"
        "Ask for approval before each change, as usual. When you finish, verify the result in the editor with "
        f"Python (run_python of the DBV Unreal AI toolset): `from dbv_unreal_ai import recipes; result = recipes.check(\"{recipe['id']}\")`, "
        f"or with the recipe_check tool of the dbv-unreal-ai MCP server (id \"{recipe['id']}\"). Tell the user what passed and what is missing."
    )


def register_skills() -> int:
    """Publica cada receta como Agent Skill. Devuelve cuántas; 0 si el motor no tiene ``AgentSkill``. @spec REQ-23"""
    if not hasattr(unreal, "AgentSkill"):
        return 0
    registered = 0
    for recipe in load():
        name = skill_class_name(recipe["id"])
        skill_class = _SKILLS.get(name) or unreal.uclass()(type(name, (unreal.AgentSkill,), {}))
        _SKILLS[name] = skill_class
        default = unreal.get_default_object(skill_class)
        default.set_editor_property("description", f"DBV Unreal AI recipe: {recipe['title']['en']}. {recipe['summary']['en']}")
        default.set_editor_property("instructions", skill_instructions(recipe))
        registered += 1
    return registered
