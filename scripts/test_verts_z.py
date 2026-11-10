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
mq = unreal.GeometryScript_MeshQueries
print("Initial verts:", garment_dyn.get_vertex_count())
if fit is not None:
    garment_dyn = fit.weld_top_dynamic_mesh(garment_dyn)
print("After weld verts:", garment_dyn.get_vertex_count())

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

# Call _deform_garment_to_slices directly
g_fitted, avg_scale = mh._deform_garment_to_slices(by_name, body_comp, body_dyn, garment_dyn, "Torso", 1.05, 0.0)
vc = g_fitted.get_vertex_count()
print("Fitted verts count:", vc)

min_z, max_z = 999999.0, -999999.0
below_zero = 0
for i in range(vc):
    v, ok = mq.get_vertex_position(g_fitted, i)
    if ok:
        min_z = min(min_z, float(v.z))
        max_z = max(max_z, float(v.z))
        if float(v.z) < 50.0:
            below_zero += 1

print(f"min_z: {min_z:.2f}, max_z: {max_z:.2f}, count below 50cm: {below_zero}")

