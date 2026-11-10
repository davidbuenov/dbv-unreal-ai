import unreal

body_skm = unreal.load_asset("/Game/MetaHumans/Common/Common/m_tal_nrw_body_preview")
top_sm = unreal.load_asset("/Game/DBV/MetaHumanOutfits/model/StaticMeshes/Top_Pablo_Deportista")

asset_utils = unreal.GeometryScript_AssetUtils
lod = unreal.GeometryScriptMeshReadLOD()

body_dyn = unreal.DynamicMesh()
body_dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(body_skm, body_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod)

top_dyn = unreal.DynamicMesh()
top_dyn, _ = asset_utils.copy_mesh_from_static_mesh_v2(top_sm, top_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod, True)

print("Body has bones:", body_dyn.get_vertex_count())
print("Top verts:", top_dyn.get_vertex_count())

# Try copy_bones_from_mesh
opts = unreal.GeometryScriptCopyBonesFromMeshOptions()
top_dyn = unreal.GeometryScript_BoneWeights.copy_bones_from_mesh(body_dyn, top_dyn, opts)
print("copy_bones_from_mesh succeeded!")

# Try closest point on surface
trans_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
trans_opts.transfer_method = unreal.TransferBoneWeightsMethod.CLOSEST_POINT_ON_SURFACE
top_dyn = unreal.GeometryScript_BoneWeights.transfer_bone_weights_from_mesh(body_dyn, top_dyn, trans_opts)
print("CLOSEST_POINT_ON_SURFACE succeeded!")

_, has_w = unreal.GeometryScript_BoneWeights.mesh_has_bone_weights(top_dyn)
print("Top has bone weights:", has_w)

