from run_ue import run_in_unreal
import json

code = """
import unreal, json

eal = unreal.EditorAssetLibrary
mq = unreal.GeometryScript_MeshQueries
j_sm = eal.load_asset("/Game/DBV/MetaHumanOutfits/SM_Chaqueta_Pablo_v2")

dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(j_sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# In SM_Chaqueta_Pablo_v2:
# X is width [-49.98, +49.98]
# Y is depth [-9.9, +9.6]
# Z is height [13.5, 50.0]

# Let's inspect the shoulder region: where does the torso end and arm begin?
# Between Z = 32 and Z = 44, inspect slices along X:
vc = int(dyn.get_vertex_count())
x_bins = {}
for i in range(vc):
    v, ok = mq.get_vertex_position(dyn, i)
    if ok and 30.0 <= v.z <= 46.0:
        bx = int(round(abs(float(v.x)) / 2.0) * 2.0)
        if bx not in x_bins:
            x_bins[bx] = {"min_z": float(v.z), "max_z": float(v.z), "min_y": float(v.y), "max_y": float(v.y), "count": 0}
        b = x_bins[bx]
        b["min_z"] = min(b["min_z"], float(v.z))
        b["max_z"] = max(b["max_z"], float(v.z))
        b["min_y"] = min(b["min_y"], float(v.y))
        b["max_y"] = max(b["max_y"], float(v.y))
        b["count"] += 1

out = [{"x": x, "count": x_bins[x]["count"], "z_range": [round(x_bins[x]["min_z"], 1), round(x_bins[x]["max_z"], 1)]} for x in sorted(x_bins.keys())]
result = json.dumps(out, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

