from run_ue import run_in_unreal
import json

code = """
import unreal, json

eal = unreal.EditorAssetLibrary
ca = eal.load_asset("/Game/DBV/Runway/Cloth/CA_Falda")
inst = ca.get_editor_property("dataflow_instance") if ca else None

inst_info = {}
if inst:
    inst_props = [p for p in dir(inst) if not p.startswith("_")]
    inst_info["props"] = inst_props
    for p in inst_props:
        try:
            val = getattr(inst, p)
            inst_info[p] = str(val)
        except Exception as e:
            inst_info[p] = f"error: {e}"

result = json.dumps(inst_info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

