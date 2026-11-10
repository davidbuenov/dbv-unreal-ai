from run_ue import run_in_unreal
import json

code = """
import unreal, json

sm_path = "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2"
sm = unreal.EditorAssetLibrary.load_asset(sm_path)
booleans = unreal.GeometryScript_MeshBooleans
mq = unreal.GeometryScript_MeshQueries

dyn1 = unreal.DynamicMesh()
dyn1, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn1, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

cut1 = unreal.GeometryScriptMeshPlaneCutOptions()
cut1.fill_holes = False
cut1.flip_cut_side = False
frame1 = unreal.Transform(location=unreal.Vector(0.0, 0.0, 13.5))
res1 = booleans.apply_mesh_plane_cut(dyn1, frame1, cut1)
bb1 = mq.get_mesh_bounding_box(res1)

dyn2 = unreal.DynamicMesh()
dyn2, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn2, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

cut2 = unreal.GeometryScriptMeshPlaneCutOptions()
cut2.fill_holes = False
cut2.flip_cut_side = True
frame2 = unreal.Transform(location=unreal.Vector(0.0, 0.0, 13.5))
res2 = booleans.apply_mesh_plane_cut(dyn2, frame2, cut2)
bb2 = mq.get_mesh_bounding_box(res2)

result = json.dumps({
    "flip_false": {
        "min_z": float(bb1.min.z),
        "max_z": float(bb1.max.z),
        "tri_count": int(res1.get_triangle_count())
    },
    "flip_true": {
        "min_z": float(bb2.min.z),
        "max_z": float(bb2.max.z),
        "tri_count": int(res2.get_triangle_count())
    }
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

