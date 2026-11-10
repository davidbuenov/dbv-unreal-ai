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
mq = unreal.GeometryScript_MeshQueries
dyn = unreal.DynamicMesh()
dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# Sample 10000 vertices to find cross-section dimensions along Z
vc = int(dyn.get_vertex_count())
step = max(vc // 50000, 1)

z_slices = {}
for i in range(0, vc, step):
    v, ok = mq.get_vertex_position(dyn, i)
    if ok:
        # Bin by 5cm in Z from -50 to +50
        bin_z = int(round(float(v.z) / 2.0) * 2.0)
        if bin_z not in z_slices:
            z_slices[bin_z] = {"min_x": float(v.x), "max_x": float(v.x), "min_y": float(v.y), "max_y": float(v.y), "count": 0}
        s = z_slices[bin_z]
        s["min_x"] = min(s["min_x"], float(v.x))
        s["max_x"] = max(s["max_x"], float(v.x))
        s["min_y"] = min(s["min_y"], float(v.y))
        s["max_y"] = max(s["max_y"], float(v.y))
        s["count"] += 1

out = []
for z in sorted(z_slices.keys()):
    s = z_slices[z]
    out.append({
        "z": z,
        "width_y": round(s["max_y"] - s["min_y"], 2),
        "depth_x": round(s["max_x"] - s["min_x"], 2),
        "count": s["count"]
    })

result = json.dumps(out, indent=2)
"""

res = send({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "unreal_run_python", "arguments": {"code": py_code}}})
print(res.get("result", {}).get("content", [{}])[0].get("text"))
proc.stdin.close(); proc.wait()

