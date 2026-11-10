from run_ue import run_in_unreal
import json

code = """
import unreal, json

sm_path = "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2"
sm = unreal.EditorAssetLibrary.load_asset(sm_path)
mq = unreal.GeometryScript_MeshQueries

dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# In original SM_Traje_Pablo_v2, Y is width [-50, +50]
# Find the extreme Y points (wrists / cuffs)
vc = int(dyn.get_vertex_count())
cuff_l = [] # max Y
cuff_r = [] # min Y
for i in range(vc):
    v, ok = mq.get_vertex_position(dyn, i)
    if ok and v.z > 20.0:
        if v.y > 47.0:
            cuff_l.append((float(v.x), float(v.y), float(v.z)))
        elif v.y < -47.0:
            cuff_r.append((float(v.x), float(v.y), float(v.z)))

result = json.dumps({
    "cuff_l_count": len(cuff_l),
    "cuff_l_center": [
        sum(p[0] for p in cuff_l) / len(cuff_l) if cuff_l else 0,
        sum(p[1] for p in cuff_l) / len(cuff_l) if cuff_l else 0,
        sum(p[2] for p in cuff_l) / len(cuff_l) if cuff_l else 0
    ],
    "cuff_r_center": [
        sum(p[0] for p in cuff_r) / len(cuff_r) if cuff_r else 0,
        sum(p[1] for p in cuff_r) / len(cuff_r) if cuff_r else 0,
        sum(p[2] for p in cuff_r) / len(cuff_r) if cuff_r else 0
    ]
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

