from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = {c.get_name(): c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent)}
body = comps.get("Body")
torso = comps.get("Torso")
legs = comps.get("Legs")

torso_leader = torso.get_leader_pose_component() if hasattr(torso, "get_leader_pose_component") else None
legs_leader = legs.get_leader_pose_component() if hasattr(legs, "get_leader_pose_component") else None

torso_skm = torso.get_skeletal_mesh_asset()
body_skm = body.get_skeletal_mesh_asset()

result = json.dumps({
    "torso_leader": torso_leader.get_name() if torso_leader else None,
    "legs_leader": legs_leader.get_name() if legs_leader else None,
    "body_skeleton": body_skm.get_editor_property("skeleton").get_path_name() if body_skm else None,
    "torso_skeleton": torso_skm.get_editor_property("skeleton").get_path_name() if torso_skm else None,
    "skeletons_match": body_skm.get_editor_property("skeleton") == torso_skm.get_editor_property("skeleton")
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

