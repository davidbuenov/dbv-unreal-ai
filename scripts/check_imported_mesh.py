import subprocess, json

proc = subprocess.Popen([r"D:\Programacion\github-davidbuenov\dbv-unreal-ai\src-tauri\target\debug\dbv-unreal-ai.exe", "--mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
def send(msg):
    proc.stdin.write(json.dumps(msg) + "\n"); proc.stdin.flush()
    return json.loads(proc.stdout.readline())
send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}}})

py_code = """
import unreal, json
sm_path = "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/SM_Traje_Pablo_v2"
sm = unreal.EditorAssetLibrary.load_asset(sm_path)
bounds = sm.get_bounds()
mq = unreal.GeometryScript_MeshQueries
dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)
bbox = mq.get_mesh_bounding_box(dyn)

info = {
    "bounds_box_extent": [round(float(bounds.box_extent.x), 2), round(float(bounds.box_extent.y), 2), round(float(bounds.box_extent.z), 2)],
    "bounds_origin": [round(float(bounds.origin.x), 2), round(float(bounds.origin.y), 2), round(float(bounds.origin.z), 2)],
    "bbox_min": [round(float(bbox.min.x), 2), round(float(bbox.min.y), 2), round(float(bbox.min.z), 2)],
    "bbox_max": [round(float(bbox.max.x), 2), round(float(bbox.max.y), 2), round(float(bbox.max.z), 2)],
    "vertex_count": int(dyn.get_vertex_count()),
    "triangle_count": int(dyn.get_triangle_count())
}
result = json.dumps(info, indent=2)
"""

res = send({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "unreal_run_python", "arguments": {"code": py_code}}})
print(res.get("result", {}).get("content", [{}])[0].get("text"))
proc.stdin.close(); proc.wait()

