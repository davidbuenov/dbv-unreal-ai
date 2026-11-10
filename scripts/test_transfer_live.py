import unreal
import dbv_unreal_ai.metahuman as mh
from dbv_unreal_ai.outfit import fit

garment_sm = unreal.load_asset("/Game/DBV/MetaHumanOutfits/model/StaticMeshes/Top_Pablo_Deportista")
lod = unreal.GeometryScriptMeshReadLOD()
garment_dyn = unreal.DynamicMesh()
garment_dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(
    garment_sm, garment_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod, True
)
garment_dyn = mh._clean_and_orient_garment(garment_dyn, "Torso")
if fit is not None:
    garment_dyn = fit.weld_top_dynamic_mesh(garment_dyn)

actor = None
for a in unreal.EditorLevelLibrary.get_all_level_actors():
    if a.get_actor_label() == "Pablo_Deportista":
        actor = a
        break

by_name = mh.metahuman_components(actor)
body_comp = by_name.get("Body")
body_skm = body_comp.get_skeletal_mesh_asset()
build = mh.body_build(body_skm, actor)
full_skm, body_dyn, restored_skin, source_body_path = mh._ensure_full_body(
    body_comp, body_skm, build, "Torso", actor
)

g_fitted, avg_scale = mh._deform_garment_to_slices(by_name, body_comp, body_dyn, garment_dyn, "Torso", 1.05, 0.0)

transfer_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
transfer_opts.transfer_method = unreal.TransferBoneWeightsMethod.INPAINT_WEIGHTS
transfer_opts.num_smoothing_iterations = 0

print("Calling transfer_bone_weights_from_mesh...")
g_transferred = unreal.GeometryScript_BoneWeights.transfer_bone_weights_from_mesh(body_dyn, g_fitted, transfer_opts)

_, has_weights = unreal.GeometryScript_BoneWeights.mesh_has_bone_weights(g_transferred)
print("SUCCESS! has_bone_weights:", has_weights)

