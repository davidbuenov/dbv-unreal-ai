from run_ue import run_in_unreal
import json

code = """
import unreal, json
from dbv_unreal_ai.metahuman import metahuman_components, _disable_neck_hide

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = metahuman_components(pablo)
body_comp = comps.get("Body")
torso_comp = comps.get("Torso")
legs_comp = comps.get("Legs")
feet_comp = comps.get("Feet")
face_comp = comps.get("Face")

eal = unreal.EditorAssetLibrary

sk_jacket = eal.load_asset("/Game/DBV/MetaHumanOutfits/SK_Chaqueta_Pablo_v2")
sk_pants = eal.load_asset("/Game/DBV/MetaHumanOutfits/SK_Pantalones_Pablo_v2")
mat = eal.load_asset("/Game/DBV/MetaHumanOutfits/model/Materials/tripo_material_80a26f15-2ff2-4fd6-9b54-57fc598949ed")

with unreal.ScopedEditorTransaction("DBV Unreal AI: Equip Pablo Full Biker Outfit"):
    pablo.modify()
    
    # 1. Torso -> Jacket
    torso_comp.modify()
    torso_comp.set_skeletal_mesh_asset(sk_jacket)
    if hasattr(torso_comp, "empty_override_materials"):
        torso_comp.empty_override_materials()
    if mat and hasattr(torso_comp, "set_material"):
        torso_comp.set_material(0, mat)
    if hasattr(torso_comp, "set_leader_pose_component"):
        torso_comp.set_leader_pose_component(body_comp, False)
    
    # 2. Legs -> Pants
    legs_comp.modify()
    legs_comp.set_skeletal_mesh_asset(sk_pants)
    if hasattr(legs_comp, "empty_override_materials"):
        legs_comp.empty_override_materials()
    if mat and hasattr(legs_comp, "set_material"):
        legs_comp.set_material(0, mat)
    if hasattr(legs_comp, "set_leader_pose_component"):
        legs_comp.set_leader_pose_component(body_comp, False)

    # 3. Feet -> None (boots are built into pants)
    feet_comp.modify()
    feet_comp.set_skeletal_mesh_asset(None)

    # 4. Disable neck hide
    changed_mats = _disable_neck_hide(face_comp)

info = {
    "success": True,
    "torso_mesh": torso_comp.get_skeletal_mesh_asset().get_path_name() if torso_comp.get_skeletal_mesh_asset() else None,
    "legs_mesh": legs_comp.get_skeletal_mesh_asset().get_path_name() if legs_comp.get_skeletal_mesh_asset() else None,
    "feet_mesh": feet_comp.get_skeletal_mesh_asset().get_path_name() if feet_comp.get_skeletal_mesh_asset() else None,
    "neck_hide_fixed": changed_mats
}
result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

