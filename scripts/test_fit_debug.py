import unreal
import dbv_unreal_ai.metahuman as mh
from dbv_unreal_ai.outfit import fit

actor = None
for a in unreal.EditorLevelLibrary.get_all_level_actors():
    if a.get_actor_label() == "Pablo_Deportista":
        actor = a
        break

by_name = mh.metahuman_components(actor)
body_comp = by_name.get("Body")
slot_comp = by_name.get("Torso")
body_skm = body_comp.get_skeletal_mesh_asset()

build = mh.body_build(body_skm, actor)
full_skm, body_dyn, restored_skin, source_body_path = mh._ensure_full_body(
    body_comp, body_skm, build, "Torso", actor
)
print(f"full_skm: {full_skm.get_path_name() if full_skm else 'None'}")
print(f"body_dyn vertex count: {body_dyn.get_vertex_count()}")

garment_sm = unreal.load_asset("/Game/DBV/MetaHumanOutfits/model/StaticMeshes/Top_Pablo_Deportista")
lod = unreal.GeometryScriptMeshReadLOD()
garment_dyn = unreal.DynamicMesh()
garment_dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(
    garment_sm, garment_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod, True
)
garment_dyn = mh._clean_and_orient_garment(garment_dyn, "Torso")
if fit is not None:
    garment_dyn = fit.weld_top_dynamic_mesh(garment_dyn)

garment_dyn, target_center, target_size, uniform_scale = mh._fit_garment_to_body(
    by_name, body_comp, body_dyn, full_skm, slot_comp, garment_dyn, "Torso", 1.05, 0.0
)

mq = unreal.GeometryScript_MeshQueries
g_box = mq.get_mesh_bounding_box(garment_dyn)
b_box = mq.get_mesh_bounding_box(body_dyn)

print(f"Fitted garment bounds: min=({g_box.min.x:.2f}, {g_box.min.y:.2f}, {g_box.min.z:.2f}), max=({g_box.max.x:.2f}, {g_box.max.y:.2f}, {g_box.max.z:.2f})")
print(f"Body bounds: min=({b_box.min.x:.2f}, {b_box.min.y:.2f}, {b_box.min.z:.2f}), max=({b_box.max.x:.2f}, {b_box.max.y:.2f}, {b_box.max.z:.2f})")

# Check bone weights on body_dyn
has_weights_body = unreal.GeometryScript_BoneWeights.mesh_has_bone_weights(body_dyn)
print(f"body_dyn has_bone_weights: {has_weights_body}")

