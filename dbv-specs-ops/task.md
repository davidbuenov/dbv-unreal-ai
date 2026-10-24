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
- [ ] **`/plan` v0.1** — Architect Review, spike del toolset Python con un editor real (R-02), desglose en slices.

## Deuda Técnica
*Mejoras detectadas fuera del foco de la tarea actual, para abordarlas después.*

- [Ninguna por ahora]

---

## 🔄 Context Snapshot / Snapshot de Contexto

> **Last update / Última actualización:** 2026-10-24
> **Exact point / Punto exacto:** `/spec` v1.1 cerrado con las respuestas del usuario.
> **Pending / Pendiente:** `/plan` v0.1 (P-05 distribución y P-06 base de conocimiento incluidas).
> **Next step / Próximo paso:** `/plan`: Architect Review, spike del toolset Python con un proyecto de UE 5.8 abierto (pedir al usuario que active el MCP) y desglose en slices.
