from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
loc = pablo.get_actor_location()
rot = pablo.get_actor_rotation()

result = json.dumps({
    "location": [float(loc.x), float(loc.y), float(loc.z)],
    "rotation": [float(rot.pitch), float(rot.yaw), float(rot.roll)]
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

