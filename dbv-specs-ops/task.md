# Backlog - DBV Unreal AI

## Contexto del Proyecto (Context Snapshot)

* **Objetivo**: copiloto de IA nativo (Win/macOS/Linux) para Unreal Engine 5.8: conecta cualquier IA al proyecto abierto, lo prepara y genera assets con Tripo y World Labs directamente en el editor.
* **Estado actual**: `/spec` v1.1 validado (P-01..P-04 resueltas); listo para `/plan`.
* **Última decisión técnica**: v0.1 = MVP + MCP propio (REQ-20); toolset en dos variantes (Python por defecto, C++ opcional); splats con plugin de terceros (v0.2); nombre visible «DBV Unreal AI»; coautor David Bueno Carmona.
* **Próximo paso**: `/plan` de la v0.1 con el spike del toolset Python sobre un editor real (R-02) y la decisión de distribución (P-05).

## Checklist de Tareas

- [x] **Bootstrap** — framework v3.1.0, configuración, activadores, LICENSE, README, git (commit `9fc9b6c`).
- [x] **`/spec` v1.0** — `SPECIFICATIONS.md`, `DESIGN.md`, `ARCHITECTURE.md` (borrador) tras analizar DBVMCPFirst, el motor 5.8.2, dbv-typst-editor, dbv-tauri-starter, Wordsmith y Holo-Companion.
- [x] **Cerrar preguntas abiertas** — P-01 MVP + MCP propio, P-02 Python + C++, P-03 plugin de terceros, P-04 «DBV Unreal AI». P-05 (distribución) pasa a `/plan`.
- [x] **`/spec` v1.2** — incorporado MCPCLiente (lecciones, skill de API, puente) y verificación en vivo contra DBVMCPFirst (protocolo, 22 toolsets, `RunPython`); nuevos REQ-25 (base de conocimiento de la API) y REQ-26 (comprobación del Python antes de ejecutar); respuestas reales guardadas en `tests/fixtures/unreal-mcp/ue-5.8.2/`.
- [x] **`/plan` v0.1** — Architect Review y desglose en `implementation_plan.md` (decisiones D1–D8).
- [ ] **S1 · Andamiaje** — código de dbv-tauri-starter con la identidad de DBV Unreal AI; tests en verde.
- [ ] **S2 · Descubrimiento** (REQ-01, REQ-03) — `.uproject`, instalaciones de Epic, procesos del editor, puerto MCP, recientes.
- [ ] **S3 · Cliente MCP** (REQ-02) — sesión, tools/list, tools/call, texto/JSON/SSE, `returnValue`, tiempo límite.
- [ ] **S4 · Toolset DBV en Python** (REQ-06, REQ-25, REQ-26) — `run_python`, `validate_python`, `describe_python_api`, `get_project_info`, `get_scene_summary`; spike en vivo.
- [ ] **S5 · Diagnóstico y preparación** (REQ-04, REQ-05) — comprobaciones, diff, copia de seguridad, aplicar, revertir.
- [ ] **S6 · Herramientas de Unreal y permisos** (REQ-10, REQ-12, REQ-13, REQ-26) — bucle de herramientas, lista blanca de lectura, tarjeta de aprobación.
- [ ] **S7 · Tripo v3 y biblioteca** (REQ-15, REQ-17, RNF-04) — cliente con sondeo y descarga inmediata; catálogo local.
- [ ] **S8 · Enviar a Unreal** (REQ-19) — importar, colocar delante de la cámara, carpeta del Outliner.
- [ ] **S9 · MCP propio** (REQ-20, REQ-11) — `--mcp` por stdio y servidores MCP para los agentes ACP.
- [ ] **S10 · Interfaz** (REQ-08, REQ-14, REQ-24) — barra lateral, conexión, preparación, registro, ES/EN.
- [ ] **S11 · CI y documentación** — workflow multiplataforma, README, CHANGELOG.

## Deuda Técnica
*Mejoras detectadas fuera del foco de la tarea actual, para abordarlas después.*

- [Ninguna por ahora]

---

## 🔄 Context Snapshot / Snapshot de Contexto

> **Last update / Última actualización:** 2026-10-24
> **Exact point / Punto exacto:** `/spec` v1.1 cerrado con las respuestas del usuario.
> **Pending / Pendiente:** `/plan` v0.1 (P-05 distribución y P-06 base de conocimiento incluidas).
> **Next step / Próximo paso:** `/plan`: Architect Review, spike del toolset Python con un proyecto de UE 5.8 abierto (pedir al usuario que active el MCP) y desglose en slices.
