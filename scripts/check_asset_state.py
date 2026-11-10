from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = {c.get_name(): c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent)}
torso = comps.get("Torso")
legs = comps.get("Legs")

eal = unreal.EditorAssetLibrary
j_exists = eal.does_asset_exist("/Game/DBV/MetaHumanOutfits/SK_Chaqueta_Pablo_v2")
p_exists = eal.does_asset_exist("/Game/DBV/MetaHumanOutfits/SK_Pantalones_Pablo_v2")

result = json.dumps({
    "j_exists": j_exists,
    "p_exists": p_exists
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

