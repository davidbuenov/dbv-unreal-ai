from run_ue import run_in_unreal
import json

code = """
import unreal, json
eal = unreal.EditorAssetLibrary

sk_jacket = eal.load_asset("/Game/DBV/MetaHumanOutfits/SK_Chaqueta_Pablo_v2")
mats_sk = []
if sk_jacket:
    for m in sk_jacket.get_editor_property("materials"):
        mats_sk.append({
            "slot": str(m.material_slot_name),
            "interface": m.material_interface.get_path_name() if m.material_interface else None
        })

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = {c.get_name(): c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent)}
torso = comps.get("Torso")
legs = comps.get("Legs")

torso_mats = [torso.get_material(i).get_path_name() if torso.get_material(i) else None for i in range(torso.get_num_materials())] if torso else []
legs_mats = [legs.get_material(i).get_path_name() if legs.get_material(i) else None for i in range(legs.get_num_materials())] if legs else []

info = {
    "sk_jacket_mats": mats_sk,
    "torso_comp_mats": torso_mats,
    "legs_comp_mats": legs_mats
}
result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

