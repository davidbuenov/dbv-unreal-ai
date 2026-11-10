import subprocess, json

proc = subprocess.Popen([r"D:\Programacion\github-davidbuenov\dbv-unreal-ai\src-tauri\target\debug\dbv-unreal-ai.exe", "--mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
def send(msg):
    proc.stdin.write(json.dumps(msg) + "\n"); proc.stdin.flush()
    return json.loads(proc.stdout.readline())

send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}}})

res = send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
for t in res.get("result", {}).get("tools", []):
    if t.get("name") == "unreal_capture_viewport":
        print(json.dumps(t, indent=2))
proc.stdin.close(); proc.wait()

