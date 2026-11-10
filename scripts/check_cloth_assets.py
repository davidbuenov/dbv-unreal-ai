from run_ue import run_in_unreal
import json

code = """
import unreal, json
eal = unreal.EditorAssetLibrary

assets = eal.list_assets("/Game/DBV/Runway/Cloth", recursive=True) if eal.does_directory_exist("/Game/DBV/Runway/Cloth") else []
info = [str(a) for a in assets]

result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

