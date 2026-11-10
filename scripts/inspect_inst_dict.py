from run_ue import run_in_unreal
import json

code = """
import unreal, json

eal = unreal.EditorAssetLibrary
ca = eal.load_asset("/Game/DBV/Runway/Cloth/CA_Falda")
inst = ca.get_editor_property("dataflow_instance") if ca else None

result = json.dumps(inst.to_dict() if inst else {}, indent=2, default=str)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

