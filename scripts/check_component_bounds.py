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

mq = unreal.GeometryScript_MeshQueries
asset_utils = unreal.GeometryScript_AssetUtils

# Body bounds
body_skm = body.get_skeletal_mesh_asset()
b_box = body_skm.get_bounds()

# Torso mesh bounds
torso_skm = torso.get_skeletal_mesh_asset()
t_box = torso_skm.get_bounds() if torso_skm else None

# Legs mesh bounds
legs_skm = legs.get_skeletal_mesh_asset()
l_box = legs_skm.get_bounds() if legs_skm else None

result = json.dumps({
    "body_bounds": {
        "origin": [float(b_box.origin.x), float(b_box.origin.y), float(b_box.origin.z)],
        "box_extent": [float(b_box.box_extent.x), float(b_box.box_extent.y), float(b_box.box_extent.z)]
    },
    "torso_bounds": {
        "origin": [float(t_box.origin.x), float(t_box.origin.y), float(t_box.origin.z)],
        "box_extent": [float(t_box.box_extent.x), float(t_box.box_extent.y), float(t_box.box_extent.z)]
    } if t_box else None,
    "legs_bounds": {
        "origin": [float(l_box.origin.x), float(l_box.origin.y), float(l_box.origin.z)],
        "box_extent": [float(l_box.box_extent.x), float(l_box.box_extent.y), float(l_box.box_extent.z)]
    } if l_box else None
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

