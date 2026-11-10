import subprocess, json, base64, os
from run_ue import run_in_unreal

# Position camera in FRONT of Pablo
# Pablo is at X = -240.0, Y = 280.0, Z = 0.0
# Front of Pablo is +Y.
# We place camera at X = -240.0, Y = 460.0, Z = 110.0
# Looking along -Y: Yaw = -90.0, Pitch = -5.0
cam_code = """
import unreal
loc = unreal.Vector(-240.0, 460.0, 110.0)
rot = unreal.Rotator(roll=0.0, pitch=-5.0, yaw=-90.0)
unreal.EditorLevelLibrary.set_level_viewport_camera_info(loc, rot)
"""
run_in_unreal(cam_code)

# Capture viewport via MCP
proc = subprocess.Popen([r"D:\Programacion\github-davidbuenov\dbv-unreal-ai\src-tauri\target\debug\dbv-unreal-ai.exe", "--mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
def send(msg):
    proc.stdin.write(json.dumps(msg) + "\n"); proc.stdin.flush()
    return json.loads(proc.stdout.readline())

send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}}})

res = send({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "unreal_capture_viewport", "arguments": {"annotate": False, "detail": "normal"}}})
content = res.get("result", {}).get("content", [])

for item in content:
    if item.get("type") == "image":
        b64 = item.get("data")
        dest = r"C:\Users\bueno\.gemini\antigravity\brain\3f701c76-b955-4cb8-a325-5910d8b6ef9d\pablo_front_v2.png"
        with open(dest, "wb") as f:
            f.write(base64.b64decode(b64))
        print(f"Saved screenshot to {dest}, size: {os.path.getsize(dest)} bytes")

proc.stdin.close(); proc.wait()

