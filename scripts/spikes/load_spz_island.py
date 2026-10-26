# =============================================================================
# DBV Unreal AI — Carga de Gaussian Splat (.spz) de World Labs con LCC4Unreal
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================

import os
import unreal

SPZ_FILE = "D:/Programacion/Unreal/DBVUnrealAITest1/Content/DBV/Splats/abandoned_pirate_island.spz"
METRIC_SCALE = 2.3623971939086914
GROUND_OFFSET_M = 1.4525254964828491

# Comprobar que el plugin LCC4Unreal está cargado
spz_class = unreal.load_class(None, "/Script/LCC4UnrealRuntime.SpzActor")
if not spz_class:
    raise RuntimeError("El plugin LCC4Unreal no está cargado. Reinicia el editor de Unreal para cargarlo.")

eass = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)

# Eliminar actor previo si existe
for a in eass.get_all_level_actors():
    if a.get_actor_label() == "DBV_Splat_PirateIsland":
        eass.destroy_actor(a)

# Spawnear SpzActor
location = unreal.Vector(0, 0, 0)
# En LCC4Unreal, los SPZ de World Labs (OpenCV) quedan erguidos y nivelados con roll = 90
rotation = unreal.Rotator(pitch=0.0, yaw=0.0, roll=90.0)
actor = eass.spawn_actor_from_class(spz_class, location, rotation)
actor.set_actor_label("DBV_Splat_PirateIsland")
actor.set_folder_path("DBV/Splats")

# Cargar el fichero .spz
actor.load(os.path.abspath(SPZ_FILE).replace("\\", "/"))

# Ajustar escala (1 metro = 100 unidades Unreal)
scale_factor = 100.0
actor.set_actor_scale3d(unreal.Vector(scale_factor, scale_factor, scale_factor))
actor.set_actor_location(unreal.Vector(0, 0, 0), False, False)

# Ocultar la malla de colisión en el render si existe en el nivel
for a in eass.get_all_level_actors():
    if a.get_actor_label() == "WorldMesh_PirateIsland":
        a.set_actor_hidden_in_game(True)
        if hasattr(a, "static_mesh_component") and a.static_mesh_component:
            a.static_mesh_component.set_visibility(False)

# Posicionar la cámara del visor en la playa mirando hacia la cala y el monolito
ue_sub = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem)
ue_sub.set_level_viewport_camera_info(unreal.Vector(-450, -1300, 260), unreal.Rotator(-2, 65, 0))

print(f"SpzActor 'DBV_Splat_PirateIsland' cargado y posicionado con éxito desde: {SPZ_FILE}")

