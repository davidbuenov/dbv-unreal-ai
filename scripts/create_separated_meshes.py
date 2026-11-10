from run_ue import run_in_unreal
import json

code = """
import unreal, json, time

source_sm_path = "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2"
source_sm = unreal.EditorAssetLibrary.load_asset(source_sm_path)
if not source_sm:
    result = json.dumps({"error": f"Source asset not found: {source_sm_path}"})
else:
    # Retrieve materials
    mats = source_sm.get_editor_property("static_materials")
    mat_interface = mats[0].material_interface if mats else None

    eal = unreal.EditorAssetLibrary
    booleans = unreal.GeometryScript_MeshBooleans
    xform = unreal.GeometryScript_MeshTransforms
    simp = unreal.GeometryScript_MeshSimplification
    new_assets = unreal.GeometryScript_NewAssetUtils
    mq = unreal.GeometryScript_MeshQueries

    folder = "/Game/DBV/MetaHumanOutfits"
    if not eal.does_directory_exist(folder):
        eal.make_directory(folder)

    rot_xf = unreal.Transform(
        location=unreal.Vector(0.0, 0.0, 0.0),
        rotation=unreal.Rotator(roll=0.0, pitch=0.0, yaw=90.0),
        scale=unreal.Vector(1.0, 1.0, 1.0)
    )

    simp_opts = unreal.GeometryScriptSimplifyMeshOptions()
    simp_opts.allow_seam_collapse = False
    simp_opts.retain_quadric_memory = False

    sm_opts = unreal.GeometryScriptCreateNewStaticMeshAssetOptions()
    sm_opts.enable_collision = True
    sm_opts.enable_recompute_normals = False
    sm_opts.enable_recompute_tangents = True

    # 1. PROCESS JACKET
    t0 = time.time()
    dyn_j = unreal.DynamicMesh()
    dyn_j, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(
        source_sm, dyn_j, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True
    )

    # Cut below 13.5 (keep above)
    cut_j = unreal.GeometryScriptMeshPlaneCutOptions()
    cut_j.fill_holes = False
    cut_j.flip_cut_side = True # Keep above 13.5
    frame_j = unreal.Transform(location=unreal.Vector(0.0, 0.0, 13.5))
    dyn_j = booleans.apply_mesh_plane_cut(dyn_j, frame_j, cut_j)

    # Rotate 90 deg Yaw
    dyn_j = xform.transform_mesh(dyn_j, rot_xf)

    # Simplify to 25k tris
    dyn_j = simp.apply_simplify_to_triangle_count(dyn_j, 25000, simp_opts)

    jacket_path = f"{folder}/SM_Chaqueta_Pablo_v2"
    if eal.does_asset_exist(jacket_path):
        eal.delete_asset(jacket_path)

    sm_jacket, _ = new_assets.create_new_static_mesh_asset_from_mesh(dyn_j, jacket_path, sm_opts)
    if sm_jacket and mat_interface:
        sm_jacket.set_material(0, mat_interface)
        eal.save_loaded_asset(sm_jacket, False)
    bb_j = mq.get_mesh_bounding_box(dyn_j)

    # 2. PROCESS TROUSERS
    dyn_p = unreal.DynamicMesh()
    dyn_p, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(
        source_sm, dyn_p, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True
    )

    # Cut above 15.0 (keep below)
    cut_p = unreal.GeometryScriptMeshPlaneCutOptions()
    cut_p.fill_holes = False
    cut_p.flip_cut_side = False # Keep below 15.0
    frame_p = unreal.Transform(location=unreal.Vector(0.0, 0.0, 15.0))
    dyn_p = booleans.apply_mesh_plane_cut(dyn_p, frame_p, cut_p)

    # Rotate 90 deg Yaw
    dyn_p = xform.transform_mesh(dyn_p, rot_xf)

    # Simplify to 25k tris
    dyn_p = simp.apply_simplify_to_triangle_count(dyn_p, 25000, simp_opts)

    pants_path = f"{folder}/SM_Pantalones_Pablo_v2"
    if eal.does_asset_exist(pants_path):
        eal.delete_asset(pants_path)

    sm_pants, _ = new_assets.create_new_static_mesh_asset_from_mesh(dyn_p, pants_path, sm_opts)
    if sm_pants and mat_interface:
        sm_pants.set_material(0, mat_interface)
        eal.save_loaded_asset(sm_pants, False)
    bb_p = mq.get_mesh_bounding_box(dyn_p)

    total_time = time.time() - t0

    result = json.dumps({
        "success": True,
        "elapsed_sec": round(total_time, 2),
        "jacket": {
            "path": jacket_path,
            "tri_count": int(dyn_j.get_triangle_count()),
            "bb_min": [round(float(bb_j.min.x), 2), round(float(bb_j.min.y), 2), round(float(bb_j.min.z), 2)],
            "bb_max": [round(float(bb_j.max.x), 2), round(float(bb_j.max.y), 2), round(float(bb_j.max.z), 2)]
        },
        "pants": {
            "path": pants_path,
            "tri_count": int(dyn_p.get_triangle_count()),
            "bb_min": [round(float(bb_p.min.x), 2), round(float(bb_p.min.y), 2), round(float(bb_p.min.z), 2)],
            "bb_max": [round(float(bb_p.max.x), 2), round(float(bb_p.max.y), 2), round(float(bb_p.max.z), 2)]
        }
    }, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

