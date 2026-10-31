# Respuestas grabadas del editor real (UE 5.8.4)

Grabadas el 2026-10-31 contra `DBVUnrealAITest1` (UE 5.8.4, toolset C++ DBV 0.3.0, plugin ConvAI 4.0.0-beta.27) con
el servidor MCP del editor en `127.0.0.1:8000/mcp`. Son la entrada de los tests del formato compacto
(`unreal/compact.rs`, `tests/unreal/compact.test.js`) y de la prueba de consumo del MCP propio (REQ-51).

- `list_toolsets.response.json`: los toolsets del proyecto, Convai incluido.
- `describe_toolset.<toolset>.response.json`: `NiagaraToolset_System` pesa 306 KB y es el caso que motivó REQ-51.
- `call_tool.ListSkills.response.json`: las 37 AgentSkills (17 de Convai).
- `call_tool.get_scene_summary.response.json`: el nivel de pruebas (62 actores).

No contienen claves: la de Convai vive en `Config/DefaultEngine.ini` del proyecto y nunca pasa por el MCP.
