from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
body = [c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Body"][0]
body_skm = body.get_skeletal_mesh_asset()

eal = unreal.EditorAssetLibrary
mq = unreal.GeometryScript_MeshQueries
xform = unreal.GeometryScript_MeshTransforms
asset_utils = unreal.GeometryScript_AssetUtils

# Load body dynamic mesh
body_dyn = unreal.DynamicMesh()
body_dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(body_skm, body_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD())

# 1. Test Jacket Fit
j_sm = eal.load_asset("/Game/DBV/MetaHumanOutfits/SM_Chaqueta_Pablo_v2")
dyn_j = unreal.DynamicMesh()
dyn_j, _ = asset_utils.copy_mesh_from_static_mesh_v2(j_sm, dyn_j, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# Jacket bounds
bb_j = mq.get_mesh_bounding_box(dyn_j)
c_j = (bb_j.max + bb_j.min) * 0.5
# Scale: (1.10, 1.25, 1.276)
dyn_j = xform.scale_mesh(dyn_j, unreal.Vector(1.10, 1.25, 1.276), c_j)
bb_j2 = mq.get_mesh_bounding_box(dyn_j)
# Desired top: Z = 153.0, center X=0, Y=2.28
dz_j = 153.0 - float(bb_j2.max.z)
dx_j = 0.0 - float((bb_j2.max.x + bb_j2.min.x) * 0.5)
dy_j = 2.28 - float((bb_j2.max.y + bb_j2.min.y) * 0.5)
dyn_j = xform.translate_mesh(dyn_j, unreal.Vector(dx_j, dy_j, dz_j))
bb_j_final = mq.get_mesh_bounding_box(dyn_j)

# 2. Test Pants Fit
p_sm = eal.load_asset("/Game/DBV/MetaHumanOutfits/SM_Pantalones_Pablo_v2")
dyn_p = unreal.DynamicMesh()
dyn_p, _ = asset_utils.copy_mesh_from_static_mesh_v2(p_sm, dyn_p, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# Pants bounds
bb_p = mq.get_mesh_bounding_box(dyn_p)
c_p = (bb_p.max + bb_p.min) * 0.5
# Scale: (1.23, 1.30, 1.542)
dyn_p = xform.scale_mesh(dyn_p, unreal.Vector(1.23, 1.30, 1.542), c_p)
bb_p2 = mq.get_mesh_bounding_box(dyn_p)
# Desired top: Z = 108.5, center X=0, Y=2.28
dz_p = 108.5 - float(bb_p2.max.z)
dx_p = 0.0 - float((bb_p2.max.x + bb_p2.min.x) * 0.5)
dy_p = 2.28 - float((bb_p2.max.y + bb_p2.min.y) * 0.5)
dyn_p = xform.translate_mesh(dyn_p, unreal.Vector(dx_p, dy_p, dz_p))
bb_p_final = mq.get_mesh_bounding_box(dyn_p)

result = json.dumps({
    "jacket_final": {
        "min": [round(float(bb_j_final.min.x), 2), round(float(bb_j_final.min.y), 2), round(float(bb_j_final.min.z), 2)],
        "max": [round(float(bb_j_final.max.x), 2), round(float(bb_j_final.max.y), 2), round(float(bb_j_final.max.z), 2)]
    },
    "pants_final": {
        "min": [round(float(bb_p_final.min.x), 2), round(float(bb_p_final.min.y), 2), round(float(bb_p_final.min.z), 2)],
        "max": [round(float(bb_p_final.max.x), 2), round(float(bb_p_final.max.y), 2), round(float(bb_p_final.max.z), 2)]
    }
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

