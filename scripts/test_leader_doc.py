from run_ue import run_in_unreal

code = """
import unreal
help_str = help(unreal.SkinnedMeshComponent.set_leader_pose_component)
result = str(unreal.SkinnedMeshComponent.set_leader_pose_component.__doc__)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

