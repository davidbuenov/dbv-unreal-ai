# Respuestas reales del servidor MCP de Unreal Engine 5.8.2

Capturadas el 2026-10-24 contra el editor con el proyecto DBVMCPFirst abierto (nivel `/Game/DBV/Maps/dbv-unreal-ai`),
por HTTP en `http://127.0.0.1:8000/mcp`, con llamadas de solo lectura. Base del editor simulado de los tests
(`SPECIFICATIONS.md` §8). Detalle del protocolo observado en `dbv-specs-ops/docs/ARCHITECTURE.md`.

- `initialize.response.json` — protocolo `2025-06-18`; la sesión llega en la cabecera `Mcp-Session-Id`.
- `tools_list.response.json` — modo *tool search*: `list_toolsets`, `describe_toolset`, `call_tool`.
- `list_toolsets.response.json` — 22 toolsets, en texto plano.
- `describe_toolset.MyCustomSceneToolset.response.json` — JSON dentro de un bloque de texto.
- `call_tool.GetActors.response.json` — resultado doblemente codificado (`returnValue`).
- `call_tool.RunPython.readonly.response.json` — versión del motor, nivel, Python 3.11.8 y un `DeprecationWarning` en `stderr`.
