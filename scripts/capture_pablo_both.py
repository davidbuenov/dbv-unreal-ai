import subprocess, json, base64, os, time
from run_ue import run_in_unreal

def capture_view(loc, rot, filename):
    cam_code = f"""
import unreal
loc = unreal.Vector({loc[0]}, {loc[1]}, {loc[2]})
rot = unreal.Rotator(roll={rot[0]}, pitch={rot[1]}, yaw={rot[2]})
unreal.EditorLevelLibrary.set_level_viewport_camera_info(loc, rot)
"""
    run_in_unreal(cam_code)
    time.sleep(1.0)
    
    proc = subprocess.Popen(
        [r"D:\Programacion\github-davidbuenov\dbv-unreal-ai\src-tauri\target\debug\dbv-unreal-ai.exe", "--mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8"
    )
    def send(msg):
        proc.stdin.write(json.dumps(msg) + "\n")
        proc.stdin.flush()
        return json.loads(proc.stdout.readline())

    send({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    })

    res = send({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "unreal_capture_viewport",
            "arguments": {"annotate": False, "detail": "normal"}
        }
    })
    content = res.get("result", {}).get("content", [])
    dest = os.path.join(r"C:\Users\bueno\.gemini\antigravity\brain\3f701c76-b955-4cb8-a325-5910d8b6ef9d", filename)
    saved = False
    for item in content:
        if item.get("type") == "image":
            b64 = item.get("data")
            with open(dest, "wb") as f:
                f.write(base64.b64decode(b64))
            print(f"Saved {filename}: {os.path.getsize(dest)} bytes")
            saved = True
            break
    proc.stdin.close()
    proc.wait()
    return saved

# Front
capture_view([-240.0, 470.0, 105.0], [0.0, -5.0, -90.0], "pablo_front_s103.png")

# Back
capture_view([-240.0, 90.0, 105.0], [0.0, -5.0, 90.0], "pablo_back_s103.png")

