from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
body = [c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Body"][0]

r_loc = body.get_editor_property("relative_location")
r_rot = body.get_editor_property("relative_rotation")

xf_head = body.get_socket_transform("head", unreal.RelativeTransformSpace.RTS_WORLD)
xf_pelvis = body.get_socket_transform("pelvis", unreal.RelativeTransformSpace.RTS_WORLD)
xf_hand_l = body.get_socket_transform("hand_l", unreal.RelativeTransformSpace.RTS_WORLD)
xf_hand_r = body.get_socket_transform("hand_r", unreal.RelativeTransformSpace.RTS_WORLD)

result = json.dumps({
    "body_rel_loc": [float(r_loc.x), float(r_loc.y), float(r_loc.z)],
    "body_rel_rot": [float(r_rot.roll), float(r_rot.pitch), float(r_rot.yaw)],
    "head_world": [float(xf_head.translation.x), float(xf_head.translation.y), float(xf_head.translation.z)],
    "pelvis_world": [float(xf_pelvis.translation.x), float(xf_pelvis.translation.y), float(xf_pelvis.translation.z)],
    "hand_l_world": [float(xf_hand_l.translation.x), float(xf_hand_l.translation.y), float(xf_hand_l.translation.z)],
    "hand_r_world": [float(xf_hand_r.translation.x), float(xf_hand_r.translation.y), float(xf_hand_r.translation.z)]
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

