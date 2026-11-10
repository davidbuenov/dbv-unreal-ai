from run_ue import run_in_unreal
import json

code = """
import unreal, json
sm_path = "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2"
sm = unreal.EditorAssetLibrary.load_asset(sm_path)
mq = unreal.GeometryScript_MeshQueries
dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# Inspect vertices between Z=10 and Z=20 in 0.5cm steps
vc = int(dyn.get_vertex_count())
step = max(vc // 50000, 1)

bins = {}
for i in range(0, vc, step):
    v, ok = mq.get_vertex_position(dyn, i)
    if ok and 10.0 <= v.z <= 20.0:
        bz = round(float(v.z) * 2.0) / 2.0 # 0.5cm bins
        if bz not in bins:
            bins[bz] = {"min_x": float(v.x), "max_x": float(v.x), "min_y": float(v.y), "max_y": float(v.y), "count": 0}
        b = bins[bz]
        b["min_x"] = min(b["min_x"], float(v.x))
        b["max_x"] = max(b["max_x"], float(v.x))
        b["min_y"] = min(b["min_y"], float(v.y))
        b["max_y"] = max(b["max_y"], float(v.y))
        b["count"] += 1

out = []
for z in sorted(bins.keys()):
    b = bins[z]
    out.append({
        "z": z,
        "width_y": round(b["max_y"] - b["min_y"], 2),
        "depth_x": round(b["max_x"] - b["min_x"], 2),
        "count": b["count"]
    })

result = json.dumps(out, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

