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

body_skm = body.get_skeletal_mesh_asset()
torso_skm = torso.get_skeletal_mesh_asset()
legs_skm = legs.get_skeletal_mesh_asset()

def bounds_info(skm):
    if not skm: return None
    b = skm.get_bounds()
    return {
        "origin": [round(b.origin.x, 2), round(b.origin.y, 2), round(b.origin.z, 2)],
        "box_extent": [round(b.box_extent.x, 2), round(b.box_extent.y, 2), round(b.box_extent.z, 2)]
    }

info = {
    "body": bounds_info(body_skm),
    "torso": bounds_info(torso_skm),
    "legs": bounds_info(legs_skm)
}
result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

