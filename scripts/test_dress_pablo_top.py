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
torso_comp = by_name.get("Torso")

top_dyn, avg_scale = mh._deform_garment_to_slices(by_name, body_comp, body_dyn, top_dyn, "Torso", 1.05, 0.0)

# Copy bones first
opts = unreal.GeometryScriptCopyBonesFromMeshOptions()
top_dyn = unreal.GeometryScript_BoneWeights.copy_bones_from_mesh(body_dyn, top_dyn, opts)

# Transfer weights using CLOSEST_POINT_ON_SURFACE
trans_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
trans_opts.transfer_method = unreal.TransferBoneWeightsMethod.CLOSEST_POINT_ON_SURFACE
top_dyn = unreal.GeometryScript_BoneWeights.transfer_bone_weights_from_mesh(body_dyn, top_dyn, trans_opts)

# Create SkeletalMesh
dest_path = "/Game/DBV/MetaHumanOutfits/SK_Top_Pablo_Deportista"
eal = unreal.EditorAssetLibrary
if eal.does_asset_exist(dest_path):
    eal.delete_asset(dest_path)

skeleton = body_skm.get_editor_property("skeleton")
skm_opts = unreal.GeometryScriptCreateNewSkeletalMeshAssetOptions()
skm_opts.enable_recompute_normals = False
skm_opts.enable_recompute_tangents = True

new_skm, ok = unreal.GeometryScript_NewAssetUtils.create_new_skeletal_mesh_asset_from_mesh(
    top_dyn, skeleton, dest_path, skm_opts
)

if new_skm:
    # Assign material from static mesh
    sm_mats = top_sm.get_editor_property("static_materials")
    if sm_mats and len(sm_mats) > 0 and sm_mats[0].material_interface:
        mat = sm_mats[0].material_interface
        # Make two sided
        bpo = mat.get_editor_property("base_property_overrides")
        bpo.set_editor_property("override_two_sided", True)
        bpo.set_editor_property("two_sided", True)
        mat.set_editor_property("base_property_overrides", bpo)
        eal.save_loaded_asset(mat, False)
        
        mats = new_skm.get_editor_property("materials")
        if mats and len(mats) > 0:
            mats[0].set_editor_property("material_interface", mat)
            new_skm.set_editor_property("materials", mats)
    
    eal.save_loaded_asset(new_skm, False)
    
    # Equip on Pablo_Deportista
    torso_comp.set_skeletal_mesh_asset(new_skm)
    if sm_mats and len(sm_mats) > 0 and sm_mats[0].material_interface:
        torso_comp.set_material(0, sm_mats[0].material_interface)
    torso_comp.set_leader_pose_component(body_comp, force_update=True)
    print("SUCCESSFULLY EQUIPPED SK_Top_Pablo_Deportista on Torso!")
else:
    print("Failed to create SK_Top_Pablo_Deportista")
