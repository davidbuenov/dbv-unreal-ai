from run_ue import run_in_unreal
import json

code = """
import unreal, json

eal = unreal.EditorAssetLibrary
bp = eal.load_asset("/Game/DBV/Runway/Cloth/BP_Victoria_Cloth")

bp_methods = [m for m in dir(unreal) if "dsl" in m.lower() or "graph" in m.lower()]
subsys_methods = [m for m in dir(unreal.SubobjectDataSubsystem) if not m.startswith("_")] if hasattr(unreal, "SubobjectDataSubsystem") else []

result = json.dumps({
    "dsl_methods": bp_methods,
    "subsys_methods": subsys_methods
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

