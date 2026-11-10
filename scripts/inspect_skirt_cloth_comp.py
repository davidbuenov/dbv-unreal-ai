from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
actors = actor_sub.get_all_level_actors()

res = {}
for label in ["Victoria_Cloth", "Victoria_Vestido"]:
    matching = [a for a in actors if a.get_actor_label() == label]
    if not matching:
        continue
    act = matching[0]
    cloth_comp = [c for c in act.get_components_by_class(unreal.ActorComponent) if c.get_name() == "SkirtCloth"][0]
    
    # Check cloth asset
    ca = cloth_comp.get_cloth_asset() if hasattr(cloth_comp, "get_cloth_asset") else None
    if not ca and hasattr(cloth_comp, "get_editor_property"):
        ca = cloth_comp.get_editor_property("ClothAsset")
    
    # Check leader pose
    leader = cloth_comp.get_leader_pose_component() if hasattr(cloth_comp, "get_leader_pose_component") else getattr(cloth_comp, "leader_pose_component", None)
    
    # Check materials
    num_mats = cloth_comp.get_num_materials() if hasattr(cloth_comp, "get_num_materials") else 0
    mats = [cloth_comp.get_material(i).get_path_name() if cloth_comp.get_material(i) else None for i in range(num_mats)]
    
    res[label] = {
        "cloth_asset": ca.get_path_name() if ca else None,
        "leader": leader.get_name() if leader else None,
        "materials": mats,
        "transform": {
            "loc": [float(cloth_comp.get_relative_location().x), float(cloth_comp.get_relative_location().y), float(cloth_comp.get_relative_location().z)] if hasattr(cloth_comp, "get_relative_location") else None
        }
    }

result = json.dumps(res, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

