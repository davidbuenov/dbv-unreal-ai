from run_ue import run_in_unreal
import json

code = """
import unreal, json

opts_cls = getattr(unreal, "GeometryScriptSimplifyMeshOptions", None)
opts_info = [p for p in dir(opts_cls) if not p.startswith("_")] if opts_cls else []

cut_cls = getattr(unreal, "GeometryScriptMeshPlaneCutOptions", None)
cut_info = [p for p in dir(cut_cls) if not p.startswith("_")] if cut_cls else []

result = json.dumps({
    "SimplifyOptions": opts_info,
    "PlaneCutOptions": cut_info
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

