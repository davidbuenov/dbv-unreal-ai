from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
body = [c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Body"][0]

r_loc = body.get_relative_location()
r_rot = body.get_relative_rotation()

w_loc = body.get_socket_location("head")
w_chest = body.get_socket_location("spine_03")

result = json.dumps({
    "relative_location": [float(r_loc.x), float(r_loc.y), float(r_loc.z)],
    "relative_rotation": [float(r_rot.roll), float(r_rot.pitch), float(r_rot.yaw)],
    "world_head": [float(w_loc.x), float(w_loc.y), float(w_loc.z)],
    "world_chest": [float(w_chest.x), float(w_chest.y), float(w_chest.z)]
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

