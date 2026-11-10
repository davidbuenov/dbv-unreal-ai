from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = {c.get_name(): c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent)}
body = comps.get("Body")
torso = comps.get("Torso")

# Check leader methods/properties on torso
leader_members = [m for m in dir(torso) if "leader" in m.lower()]

# Try setting leader explicitly with force_update=True
torso.set_leader_pose_component(body, force_update=True)
legs = comps.get("Legs")
legs.set_leader_pose_component(body, force_update=True)

# Also check how BP_Pablo construction script sets leader pose
# In MetaHuman blueprints, construction script sets leader pose on Torso, Legs, Feet!
result = json.dumps({
    "leader_members": leader_members,
    "leader_prop": str(getattr(torso, "leader_pose_component", None))
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

