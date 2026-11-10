from run_ue import run_in_unreal
import json

code = """
import unreal, json
from dbv_unreal_ai.metahuman import metahuman_components

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = metahuman_components(pablo)

info = {}
for slot in ["Torso", "Legs", "Feet"]:
    c = comps.get(slot)
    mesh = c.get_skeletal_mesh_asset() if c else None
    info[slot] = mesh.get_path_name() if mesh else None

result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

