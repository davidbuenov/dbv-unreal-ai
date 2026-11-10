from run_ue import run_in_unreal
import json

code = """
import unreal, json
eal = unreal.EditorAssetLibrary
mq = unreal.GeometryScript_MeshQueries
asset_utils = unreal.GeometryScript_AssetUtils

sm = eal.load_asset("/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2")
dyn = unreal.DynamicMesh()
dyn, _ = asset_utils.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

vc = int(dyn.get_vertex_count())
# Find collar opening border vertices or vertices in center (|Y| < 1.0)
pts_pos_x = []
pts_neg_x = []
for i in range(vc):
    v, ok = mq.get_vertex_position(dyn, i)
    if ok and abs(v.y) < 1.0 and 30.0 < v.z < 50.0:
        if v.x > 0:
            pts_pos_x.append(v.z)
        else:
            pts_neg_x.append(v.z)

result = json.dumps({
    "pos_x_max_z": max(pts_pos_x) if pts_pos_x else None,
    "neg_x_max_z": max(pts_neg_x) if pts_neg_x else None,
    "pos_x_min_z": min(pts_pos_x) if pts_pos_x else None,
    "neg_x_min_z": min(pts_neg_x) if pts_neg_x else None,
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

