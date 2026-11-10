from run_ue import run_in_unreal
import json

code = """
import unreal, json

eal = unreal.EditorAssetLibrary
ca_path = "/Game/DBV/Runway/Cloth/CA_Falda"
ca = eal.load_asset(ca_path)

if ca:
    ca_class = ca.get_class().get_name()
    props = [p for p in dir(ca) if not p.startswith("_")]
    
    # Check dataflow reference
    df_prop = None
    for p in ["dataflow", "dataflow_asset", "dataflow_instance"]:
        if hasattr(ca, p):
            df_prop = p
    
    result = json.dumps({
        "ca_class": ca_class,
        "props": props[:30],
        "df_prop": df_prop,
        "physics_asset": ca.get_physics_asset().get_path_name() if hasattr(ca, "get_physics_asset") and ca.get_physics_asset() else None
    }, indent=2)
else:
    result = json.dumps({"error": "CA_Falda not found"}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

