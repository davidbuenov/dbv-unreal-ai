from run_ue import run_in_unreal
import json

code = """
import unreal, json

cls = getattr(unreal, "GeometryScriptCreateNewStaticMeshAssetOptions", None)
opts_info = [p for p in dir(cls) if not p.startswith("_")] if cls else []

result = json.dumps({
    "CreateStaticMeshOptions": opts_info
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

