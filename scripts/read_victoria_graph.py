import subprocess, json

proc = subprocess.Popen([r"D:\Programacion\github-davidbuenov\dbv-unreal-ai\src-tauri\target\debug\dbv-unreal-ai.exe", "--mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
def send(msg):
    proc.stdin.write(json.dumps(msg) + "\n"); proc.stdin.flush()
    return json.loads(proc.stdout.readline())

send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}}})

res1 = send({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
    "name": "unreal_call_tool",
    "arguments": {
        "toolset": "BlueprintTools",
        "tool": "get_graph",
        "arguments": {
            "blueprint": "/Game/DBV/Runway/Cloth/BP_Victoria_Cloth.BP_Victoria_Cloth",
            "graph_name": "EventGraph"
        }
    }
}})
graph_obj = res1.get("result", {}).get("returnValue")

res2 = send({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
    "name": "unreal_call_tool",
    "arguments": {
        "toolset": "BlueprintTools",
        "tool": "read_graph_dsl",
        "arguments": {
            "graph": graph_obj
        }
    }
}})
print(res2.get("result", {}).get("returnValue"))
proc.stdin.close(); proc.wait()

