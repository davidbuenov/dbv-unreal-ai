from run_ue import run_in_unreal
import json

code = """
import unreal, json

sm_path = "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2"
sm = unreal.EditorAssetLibrary.load_asset(sm_path)
mq = unreal.GeometryScript_MeshQueries

dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# Check toes at Z < -48
vc = int(dyn.get_vertex_count())
xs = []
for i in range(vc):
    v, ok = mq.get_vertex_position(dyn, i)
    if ok and v.z < -48.0:
        xs.append(float(v.x))

result = json.dumps({
    "feet_min_x": min(xs),
    "feet_max_x": max(xs)
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

