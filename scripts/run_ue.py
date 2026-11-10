import subprocess
import json
import sys

def run_in_unreal(code: str):
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
        line = proc.stdout.readline()
        return json.loads(line) if line else None

    send({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "unreal-helper", "version": "1.0"}
        }
    })

    res = send({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "unreal_run_python",
            "arguments": {"code": code}
        }
    })

    proc.stdin.close()
    proc.wait()
    return res

if __name__ == "__main__":
    if len(sys.argv) > 1:
        with open(sys.argv[1], "r", encoding="utf-8") as f:
            code = f.read()
    else:
        code = sys.stdin.read()
    res = run_in_unreal(code)
    content = res.get("result", {}).get("content", [{}])[0].get("text", "")
    print(content)

