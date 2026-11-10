from run_ue import run_in_unreal
import json

code = """
import unreal, json

classes = [
    "GeometryScript_MeshBooleans",
    "GeometryScript_MeshEdits",
    "GeometryScript_MeshSelectionQueries",
    "GeometryScript_MeshSimplification",
    "GeometryScript_NewAssetUtils",
    "GeometryScript_AssetUtils"
]

info = {}
for c in classes:
    cls = getattr(unreal, c, None)
    if cls:
        info[c] = [m for m in dir(cls) if not m.startswith("_")]
    else:
        info[c] = None

result = json.dumps(info, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

