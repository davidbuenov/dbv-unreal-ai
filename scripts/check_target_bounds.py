from run_ue import run_in_unreal
import json

code = """
import unreal, json
from dbv_unreal_ai.metahuman import target_slot_bounds, metahuman_components

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = metahuman_components(pablo)
body_comp = comps.get("Body")
body_skm = body_comp.get_skeletal_mesh_asset()
torso_comp = comps.get("Torso")
legs_comp = comps.get("Legs")

t_center, t_size = target_slot_bounds(body_skm, torso_comp, "Torso", 1.05, 0.0)
l_center, l_size = target_slot_bounds(body_skm, legs_comp, "Legs", 1.05, 0.0)

result = json.dumps({
    "Torso": {"center": t_center, "size": t_size},
    "Legs": {"center": l_center, "size": l_size}
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

