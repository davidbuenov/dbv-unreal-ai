from run_ue import run_in_unreal
import json

code = """
import unreal, json

eal = unreal.EditorAssetLibrary
bp = eal.load_asset("/Game/DBV/Runway/Cloth/BP_Victoria_Cloth")
gen_class = eal.load_blueprint_class("/Game/DBV/Runway/Cloth/BP_Victoria_Cloth")
cdo = unreal.get_default_object(gen_class)

comps = [c.get_name() for c in cdo.get_components_by_class(unreal.ActorComponent)]

result = json.dumps({
    "blueprint": bp.get_path_name(),
    "class": gen_class.get_name(),
    "components": comps
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

