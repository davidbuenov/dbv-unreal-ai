from run_ue import run_in_unreal
import json

code = """
import unreal, json

eal = unreal.EditorAssetLibrary
ca = eal.load_asset("/Game/DBV/Runway/Cloth/CA_Falda")

info = {}
for p in dir(ca):
    if not p.startswith("_"):
        try:
            val = ca.get_editor_property(p)
            info[p] = str(val)
        except Exception:
            pass

result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

