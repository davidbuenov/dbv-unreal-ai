from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
body = [c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Body"][0]
body_skm = body.get_skeletal_mesh_asset()

sm_path = "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2"
sm = unreal.EditorAssetLibrary.load_asset(sm_path)
booleans = unreal.GeometryScript_MeshBooleans
mq = unreal.GeometryScript_MeshQueries
xform = unreal.GeometryScript_MeshTransforms

# 1. Load suit
dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# In SM_Traje_Pablo_v2:
# X is depth [-10, 10]
# Y is width [-50, 50]
# Z is height [-50, 50]
# We rotate 90 deg around Z so that Y width becomes X width [-50, 50]
# Let's check rotation:
rot_xf = unreal.Transform(
    location=unreal.Vector(0.0, 0.0, 0.0),
    rotation=unreal.Rotator(roll=0.0, pitch=0.0, yaw=90.0),
    scale=unreal.Vector(1.0, 1.0, 1.0)
)
dyn = xform.transform_mesh(dyn, rot_xf)
bb = mq.get_mesh_bounding_box(dyn)

info = {
    "rotated_bb_min": [float(bb.min.x), float(bb.min.y), float(bb.min.z)],
    "rotated_bb_max": [float(bb.max.x), float(bb.max.y), float(bb.max.z)]
}
result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

