from run_ue import run_in_unreal
import json

code = """
import unreal, json

eal = unreal.EditorAssetLibrary
df_path = "/Game/DBV/Runway/Cloth/DF_Falda"
df = eal.load_asset(df_path)

lib = getattr(unreal, "DataflowEditorBlueprintLibrary", None)
term_nodes = []
nodes_info = []

if df and lib:
    # Check methods on DataflowEditorBlueprintLibrary
    members = [m for m in dir(lib) if not m.startswith("_")]
    result = json.dumps({
        "df_class": df.get_class().get_name(),
        "library_methods": members
    }, indent=2)
else:
    result = json.dumps({"error": "df or lib missing"}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

