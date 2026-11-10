from run_ue import run_in_unreal
import json

code = """
import unreal, json, time

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
body = [c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Body"][0]
body_skm = body.get_skeletal_mesh_asset()
skeleton = body_skm.get_editor_property("skeleton")

eal = unreal.EditorAssetLibrary
mq = unreal.GeometryScript_MeshQueries
xform = unreal.GeometryScript_MeshTransforms
asset_utils = unreal.GeometryScript_AssetUtils
bw = unreal.GeometryScript_BoneWeights
new_assets = unreal.GeometryScript_NewAssetUtils

# Load body dynamic mesh
body_dyn = unreal.DynamicMesh()
body_dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(body_skm, body_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD())

transfer_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
transfer_opts.transfer_method = unreal.TransferBoneWeightsMethod.INPAINT_WEIGHTS
transfer_opts.num_smoothing_iterations = 5
transfer_opts.smoothing_strength = 0.5

skm_opts = unreal.GeometryScriptCreateNewSkeletalMeshAssetOptions()
skm_opts.enable_recompute_normals = False
skm_opts.enable_recompute_tangents = True

folder = "/Game/DBV/MetaHumanOutfits"

# 1. PROCESS & RIG JACKET
t0 = time.time()
j_sm = eal.load_asset(f"{folder}/SM_Chaqueta_Pablo_v2")
mats_j = j_sm.get_editor_property("static_materials")
if mats_j:
    skm_opts.materials = {m.material_slot_name: m.material_interface for m in mats_j}

dyn_j = unreal.DynamicMesh()
dyn_j, _ = asset_utils.copy_mesh_from_static_mesh_v2(j_sm, dyn_j, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)
bb_j = mq.get_mesh_bounding_box(dyn_j)
c_j = (bb_j.max + bb_j.min) * 0.5
dyn_j = xform.scale_mesh(dyn_j, unreal.Vector(1.10, 1.25, 1.276), c_j)
bb_j2 = mq.get_mesh_bounding_box(dyn_j)
dz_j = 153.0 - float(bb_j2.max.z)
dx_j = 0.0 - float((bb_j2.max.x + bb_j2.min.x) * 0.5)
dy_j = 2.28 - float((bb_j2.max.y + bb_j2.min.y) * 0.5)
dyn_j = xform.translate_mesh(dyn_j, unreal.Vector(dx_j, dy_j, dz_j))

dyn_j = bw.transfer_bone_weights_from_mesh(body_dyn, dyn_j, transfer_opts)
_, has_weights_j = bw.mesh_has_bone_weights(dyn_j)

sk_jacket_path = f"{folder}/SK_Chaqueta_Pablo_v2"
if eal.does_asset_exist(sk_jacket_path):
    eal.delete_asset(sk_jacket_path)
sk_jacket, _ = new_assets.create_new_skeletal_mesh_asset_from_mesh(dyn_j, skeleton, sk_jacket_path, skm_opts)
if sk_jacket:
    eal.save_loaded_asset(sk_jacket, False)

# 2. PROCESS & RIG PANTS
p_sm = eal.load_asset(f"{folder}/SM_Pantalones_Pablo_v2")
mats_p = p_sm.get_editor_property("static_materials")
if mats_p:
    skm_opts.materials = {m.material_slot_name: m.material_interface for m in mats_p}

dyn_p = unreal.DynamicMesh()
dyn_p, _ = asset_utils.copy_mesh_from_static_mesh_v2(p_sm, dyn_p, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)
bb_p = mq.get_mesh_bounding_box(dyn_p)
c_p = (bb_p.max + bb_p.min) * 0.5
dyn_p = xform.scale_mesh(dyn_p, unreal.Vector(1.23, 1.30, 1.542), c_p)
bb_p2 = mq.get_mesh_bounding_box(dyn_p)
dz_p = 108.5 - float(bb_p2.max.z)
dx_p = 0.0 - float((bb_p2.max.x + bb_p2.min.x) * 0.5)
dy_p = 2.28 - float((bb_p2.max.y + bb_p2.min.y) * 0.5)
dyn_p = xform.translate_mesh(dyn_p, unreal.Vector(dx_p, dy_p, dz_p))

dyn_p = bw.transfer_bone_weights_from_mesh(body_dyn, dyn_p, transfer_opts)
_, has_weights_p = bw.mesh_has_bone_weights(dyn_p)

sk_pants_path = f"{folder}/SK_Pantalones_Pablo_v2"
if eal.does_asset_exist(sk_pants_path):
    eal.delete_asset(sk_pants_path)
sk_pants, _ = new_assets.create_new_skeletal_mesh_asset_from_mesh(dyn_p, skeleton, sk_pants_path, skm_opts)
if sk_pants:
    eal.save_loaded_asset(sk_pants, False)

dt = time.time() - t0

result = json.dumps({
    "success": True,
    "elapsed_sec": round(dt, 2),
    "jacket": {
        "path": sk_jacket_path,
        "created": sk_jacket is not None,
        "has_bone_weights": bool(has_weights_j)
    },
    "pants": {
        "path": sk_pants_path,
        "created": sk_pants is not None,
        "has_bone_weights": bool(has_weights_p)
    }
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

