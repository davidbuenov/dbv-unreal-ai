from run_ue import run_in_unreal
import json

code = """
import unreal, json

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
actors = actor_sub.get_all_level_actors()

labels = [a.get_actor_label() for a in actors if "Victoria" in a.get_actor_label() or "Cloth" in a.get_actor_label()]

info = {}
for l in labels:
    a = [act for act in actors if act.get_actor_label() == l][0]
    comps = [c.get_name() + ' (' + c.get_class().get_name() + ')' for c in a.get_components_by_class(unreal.ActorComponent)]
    info[l] = comps

result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

