from run_ue import run_in_unreal
import json

code = """
import unreal, json

sm_path = "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2"
sm = unreal.EditorAssetLibrary.load_asset(sm_path)
mq = unreal.GeometryScript_MeshQueries

dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# Check toes at Z = -49.5
vc = int(dyn.get_vertex_count())
shoe_pts = []
for i in range(vc):
    v, ok = mq.get_vertex_position(dyn, i)
    if ok and v.z < -49.0:
        shoe_pts.append((float(v.x), float(v.y)))

# Ankle at Z = -35
ankle_pts = []
for i in range(vc):
    v, ok = mq.get_vertex_position(dyn, i)
    if ok and -36.0 < v.z < -34.0:
        ankle_pts.append((float(v.x), float(v.y)))

ankle_avg_x = sum(p[0] for p in ankle_pts) / len(ankle_pts)
shoe_min_x = min(p[0] for p in shoe_pts)
shoe_max_x = max(p[0] for p in shoe_pts)

result = json.dumps({
    "ankle_avg_x": ankle_avg_x,
    "shoe_min_x": shoe_min_x,
    "shoe_max_x": shoe_max_x,
    "forward_dir": "negative_x" if abs(shoe_min_x - ankle_avg_x) > abs(shoe_max_x - ankle_avg_x) else "positive_x"
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

