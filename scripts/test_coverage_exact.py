import unreal
import dbv_unreal_ai.metahuman as mh

pablo_skm = unreal.load_asset("/Game/DBV/MetaHumanOutfits/_spike/SK_Body_Pablo")
asset_utils = unreal.GeometryScript_AssetUtils
dyn = unreal.DynamicMesh()
dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(pablo_skm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD())

vc = dyn.get_vertex_count()
mq = unreal.GeometryScript_MeshQueries

torso_count = 0
for i in range(vc):
    v, ok = mq.get_vertex_position(dyn, i)
    if ok and 80.0 <= float(v.z) <= 140.0 and abs(float(v.x)) <= 28.0:
        torso_count += 1

print(f"SK_Body_Pablo total verts: {vc}")
print(f"Torso count (80<=z<=140, |x|<=28): {torso_count}")

is_ok, ratio, desc = mh.check_body_coverage(dyn, "Torso")
print(f"check_body_coverage Torso: is_ok={is_ok}, ratio={ratio}, desc={desc}")

is_ok_legs, ratio_legs, desc_legs = mh.check_body_coverage(dyn, "Legs")
print(f"check_body_coverage Legs: is_ok={is_ok_legs}, ratio={ratio_legs}, desc={desc_legs}")

