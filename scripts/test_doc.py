from run_ue import run_in_unreal

code = """
import unreal
help_str = help(unreal.GeometryScript_NewAssetUtils.create_new_static_mesh_asset_from_mesh)
result = str(unreal.GeometryScript_NewAssetUtils.create_new_static_mesh_asset_from_mesh.__doc__)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

