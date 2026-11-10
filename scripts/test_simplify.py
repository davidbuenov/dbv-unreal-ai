from run_ue import run_in_unreal
import json

code = """
import unreal, json, time

sm_path = "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2"
sm = unreal.EditorAssetLibrary.load_asset(sm_path)
booleans = unreal.GeometryScript_MeshBooleans
mq = unreal.GeometryScript_MeshQueries
simp = unreal.GeometryScript_MeshSimplification

dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

cut = unreal.GeometryScriptMeshPlaneCutOptions()
cut.fill_holes = False
cut.flip_cut_side = True # Keep above 13.5 -> Jacket
frame = unreal.Transform(location=unreal.Vector(0.0, 0.0, 13.5))
jacket = booleans.apply_mesh_plane_cut(dyn, frame, cut)

t0 = time.time()
opts = unreal.GeometryScriptSimplifyMeshOptions()
opts.allow_seam_collapse = False
opts.retain_quadric_memory = False
simplified = simp.apply_simplify_to_triangle_count(jacket, 25000, opts)
dt = time.time() - t0

bb = mq.get_mesh_bounding_box(simplified)

result = json.dumps({
    "simplification_time_sec": round(dt, 2),
    "tri_count_after": int(simplified.get_triangle_count()),
    "min_z": float(bb.min.z),
    "max_z": float(bb.max.z)
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

