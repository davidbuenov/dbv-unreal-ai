from run_ue import run_in_unreal
import json

code = """
import unreal, json

cam_loc, cam_rot = unreal.EditorLevelLibrary.get_level_viewport_camera_info()
actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
p_loc = pablo.get_actor_location()

result = json.dumps({
    "cam_loc": [float(cam_loc.x), float(cam_loc.y), float(cam_loc.z)],
    "cam_rot": [float(cam_rot.pitch), float(cam_rot.yaw), float(cam_rot.roll)],
    "pablo_loc": [float(p_loc.x), float(p_loc.y), float(p_loc.z)]
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

