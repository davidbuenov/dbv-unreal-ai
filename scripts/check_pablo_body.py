from run_ue import run_in_unreal
import json

code = """
import unreal, json
from dbv_unreal_ai.metahuman import metahuman_components, body_build, _ensure_full_body

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = metahuman_components(pablo)
body_comp = comps.get("Body")
body_skm = body_comp.get_skeletal_mesh_asset() if body_comp else None
build = body_build(body_skm) if body_skm else None

info = {
    "current_body_skm": body_skm.get_path_name() if body_skm else None,
    "build": build,
    "skeleton": body_skm.get_editor_property("skeleton").get_path_name() if body_skm else None
}
result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

