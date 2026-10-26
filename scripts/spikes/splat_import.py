# =============================================================================
# DBV Unreal AI — Spike: importar la malla de vista previa de un splat
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
#
# Spike P-10 (REQ-41), 2026-10-26: se ejecuta DENTRO del editor (run_python del toolset). No es código del producto
# todavía: es la referencia para el adaptador de splats propio. Ver memory.md (lecciones del spike).

import unreal

src = r"C:/ruta/al/splat_preview.glb"  # GLB de src-tauri/src/splats (write_glb)
FOLDER = "/Game/DBV/Splats/Spike"
task = unreal.AssetImportTask()
task.filename = src
task.destination_path = FOLDER
task.destination_name = "Forest50k"
task.replace_existing = True
task.automated = True
task.save = True
unreal.AssetToolsHelpers.get_asset_tools().import_asset_tasks([task])
paths = [str(p) for p in task.get_editor_property("imported_object_paths")]
mesh = next((unreal.load_asset(p) for p in paths if isinstance(unreal.load_asset(p), unreal.StaticMesh)), None)
sub = unreal.get_editor_subsystem(unreal.StaticMeshEditorSubsystem)
ns = sub.get_nanite_settings(mesh)
ns.set_editor_property("enabled", False)
sub.set_nanite_settings(mesh, ns, True)
mesh.set_material(0, unreal.load_asset(f"{FOLDER}/M_DBVSplat"))
mesh.set_editor_property("positive_bounds_extension", unreal.Vector(1000, 1000, 1000))
mesh.set_editor_property("negative_bounds_extension", unreal.Vector(1000, 1000, 1000))
unreal.EditorAssetLibrary.save_asset(mesh.get_path_name())
md = mesh.get_static_mesh_description(0)
result = {"mesh": mesh.get_path_name(), "triangles": mesh.get_num_triangles(0)}
