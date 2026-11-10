from run_ue import run_in_unreal
import json

code = """
import unreal, json, math

eal = unreal.EditorAssetLibrary
mq = unreal.GeometryScript_MeshQueries
me = unreal.GeometryScript_MeshEdits
j_sm = eal.load_asset("/Game/DBV/MetaHumanOutfits/SM_Chaqueta_Pablo_v2")

dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(j_sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# Deform arms down by 38 degrees
theta_deg = 38.0
theta = math.radians(theta_deg)
cos_t = math.cos(theta)
sin_t = math.sin(theta)

shoulder_x = 13.5
blend_x = 17.5
pivot_z = 40.0
pivot_y = 0.0

vc = int(dyn.get_vertex_count())
cuff_pts_l = []
cuff_pts_r = []

for i in range(vc):
    v, ok = mq.get_vertex_position(dyn, i)
    if not ok:
        continue
    x, y, z = float(v.x), float(v.y), float(v.z)
    ax = abs(x)
    if ax > shoulder_x and z > 25.0:
        # Weight from 0 to 1
        t = min(1.0, max(0.0, (ax - shoulder_x) / (blend_x - shoulder_x)))
        w = t * t * (3.0 - 2.0 * t)
        
        # Local angle
        th = theta * w
        c_th = math.cos(th)
        s_th = math.sin(th)
        
        if x > 0: # Left sleeve
            dx = x - shoulder_x
            dz = z - pivot_z
            nx = shoulder_x + dx * c_th + dz * s_th
            nz = pivot_z - dx * s_th + dz * c_th
        else: # Right sleeve
            dx = x - (-shoulder_x)
            dz = z - pivot_z
            nx = -shoulder_x + dx * c_th - dz * s_th
            nz = pivot_z + dx * s_th + dz * c_th
        
        me.set_vertex_position(dyn, i, unreal.Vector(nx, y, nz), i < vc - 1)
        if ax > 45.0:
            if x > 0:
                cuff_pts_l.append((nx, y, nz))
            else:
                cuff_pts_r.append((nx, y, nz))

bb = mq.get_mesh_bounding_box(dyn)

result = json.dumps({
    "bb_min": [round(float(bb.min.x), 2), round(float(bb.min.y), 2), round(float(bb.min.z), 2)],
    "bb_max": [round(float(bb.max.x), 2), round(float(bb.max.y), 2), round(float(bb.max.z), 2)],
    "cuff_l_avg": [
        round(sum(p[0] for p in cuff_pts_l) / len(cuff_pts_l), 2),
        round(sum(p[1] for p in cuff_pts_l) / len(cuff_pts_l), 2),
        round(sum(p[2] for p in cuff_pts_l) / len(cuff_pts_l), 2)
    ] if cuff_pts_l else None
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

