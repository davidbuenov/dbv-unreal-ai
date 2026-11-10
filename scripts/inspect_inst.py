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
    dataflow = inst.get_editor_property("dataflow") if hasattr(inst, "get_editor_property") else None
    inst_info = {
        "props": inst_props,
        "dataflow": dataflow.get_path_name() if dataflow else None
    }

result = json.dumps(inst_info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

