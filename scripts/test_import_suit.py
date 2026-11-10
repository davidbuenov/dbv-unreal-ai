import subprocess
import json

proc = subprocess.Popen(
    [r"D:\Programacion\github-davidbuenov\dbv-unreal-ai\src-tauri\target\debug\dbv-unreal-ai.exe", "--mcp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    encoding="utf-8"
)

def send(msg):
    raw = json.dumps(msg) + "\n"
    proc.stdin.write(raw)
    proc.stdin.flush()
    line = proc.stdout.readline()
    return json.loads(line) if line else None

send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "import-suit", "version": "1.0"}}})

py_code = """
import unreal, json
from dbv_unreal_ai import editor

glb_path = r"C:\\Users\\bueno\\DBV Unreal AI\\Library\\chaqueta_pablo_v2-1794225333306\\model.glb"
res_path = editor.import_model(glb_path, "/Game/DBV/MetaHumanOutfits", "SM_Traje_Pablo_v2")

sm = unreal.EditorAssetLibrary.load_asset(res_path)
bounds = sm.get_bounds() if sm else None

info = {
    "imported_path": res_path,
    "box_extent": [float(bounds.box_extent.x), float(bounds.box_extent.y), float(bounds.box_extent.z)] if bounds else None,
    "origin": [float(bounds.origin.x), float(bounds.origin.y), float(bounds.origin.z)] if bounds else None
}
result = json.dumps(info, indent=2)
"""

res = send({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "unreal_run_python", "arguments": {"code": py_code}}})
print(res.get("result", {}).get("content", [{}])[0].get("text"))

proc.stdin.close()
proc.wait()

