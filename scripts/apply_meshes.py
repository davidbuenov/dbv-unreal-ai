from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = {c.get_name(): c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent)}
body = comps.get("Body")
torso = comps.get("Torso")
legs = comps.get("Legs")
feet = comps.get("Feet")

eal = unreal.EditorAssetLibrary
sk_j = eal.load_asset("/Game/DBV/MetaHumanOutfits/SK_Chaqueta_Pablo_v2")
sk_p = eal.load_asset("/Game/DBV/MetaHumanOutfits/SK_Pantalones_Pablo_v2")
mat = eal.load_asset("/Game/DBV/MetaHumanOutfits/model/Materials/tripo_material_80a26f15-2ff2-4fd6-9b54-57fc598949ed")

torso.set_skeletal_mesh_asset(sk_j)
if mat:
    torso.set_material(0, mat)
torso.set_leader_pose_component(body, force_update=True)

legs.set_skeletal_mesh_asset(sk_p)
if mat:
    legs.set_material(0, mat)
legs.set_leader_pose_component(body, force_update=True)

feet.set_skeletal_mesh_asset(None)

result = json.dumps({
    "success": True,
    "torso_skm": torso.get_skeletal_mesh_asset().get_path_name() if torso.get_skeletal_mesh_asset() else None,
    "legs_skm": legs.get_skeletal_mesh_asset().get_path_name() if legs.get_skeletal_mesh_asset() else None
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))
