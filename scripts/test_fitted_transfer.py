import unreal
import dbv_unreal_ai.metahuman as mh
from dbv_unreal_ai.outfit import fit

body_skm = unreal.load_asset("/Game/MetaHumans/Common/Common/m_tal_nrw_body_preview")
top_sm = unreal.load_asset("/Game/DBV/MetaHumanOutfits/model/StaticMeshes/Top_Pablo_Deportista")

asset_utils = unreal.GeometryScript_AssetUtils
lod = unreal.GeometryScriptMeshReadLOD()

body_dyn = unreal.DynamicMesh()
body_dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(body_skm, body_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod)

top_dyn = unreal.DynamicMesh()
top_dyn, _ = asset_utils.copy_mesh_from_static_mesh_v2(top_sm, top_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod, True)
top_dyn = mh._clean_and_orient_garment(top_dyn, "Torso")
if fit is not None:
    top_dyn = fit.weld_top_dynamic_mesh(top_dyn)

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
actor = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "Pablo_Deportista"][0]

by_name = mh.metahuman_components(actor)
body_comp = by_name.get("Body")

top_dyn, avg_scale = mh._deform_garment_to_slices(by_name, body_comp, body_dyn, top_dyn, "Torso", 1.05, 0.0)

# Test 1: copy_bones_from_mesh
opts = unreal.GeometryScriptCopyBonesFromMeshOptions()
top_dyn = unreal.GeometryScript_BoneWeights.copy_bones_from_mesh(body_dyn, top_dyn, opts)
print("copy_bones_from_mesh succeeded on fitted top!")

# Test 2: Try CLOSEST_POINT_ON_SURFACE
trans_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
trans_opts.transfer_method = unreal.TransferBoneWeightsMethod.CLOSEST_POINT_ON_SURFACE
top_dyn = unreal.GeometryScript_BoneWeights.transfer_bone_weights_from_mesh(body_dyn, top_dyn, trans_opts)
_, has_w = unreal.GeometryScript_BoneWeights.mesh_has_bone_weights(top_dyn)
print("CLOSEST_POINT_ON_SURFACE succeeded! Has weights:", has_w)
