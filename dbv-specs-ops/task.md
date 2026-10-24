# Backlog - DBV Unreal AI

## Contexto del Proyecto (Context Snapshot)

* **Objetivo**: copiloto de IA nativo (Win/macOS/Linux) para Unreal Engine 5.8: conecta cualquier IA al proyecto abierto, lo prepara y genera assets con Tripo directamente en el editor (World Labs en v0.2).
* **Estado actual**: v0.1 construida, probada y revisada (`/build` → `/test` → `/code-simplify` encadenados con autorización del usuario, ausente). Sin publicar: falta `/ship`.
* **Última decisión técnica**: decisiones D1–D8 de `implementation_plan.md` tomadas SIN el usuario (ADR-V010-001 en `memory.md`), para revisar.
* **Próximo paso**: el usuario revisa ADR-V010-001 y la deuda técnica de abajo; después `/ship` v0.1.

## Checklist de Tareas

- [x] **Bootstrap** — framework v3.1.0, configuración, activadores, LICENSE, README, git (commit `9fc9b6c`).
- [x] **`/spec` v1.0–v1.2** — `SPECIFICATIONS.md`, `DESIGN.md`, `ARCHITECTURE.md`; MCPCLiente y verificación en vivo contra DBVMCPFirst; REQ-25 y REQ-26.
- [x] **`/plan` v0.1** — Architect Review y `implementation_plan.md` (D1–D8).
- [x] **S1 · Andamiaje** — código de dbv-tauri-starter con la identidad de DBV Unreal AI.
- [x] **S2 · Descubrimiento** (REQ-01, REQ-03).
- [x] **S3 · Cliente MCP** (REQ-02) — verificado en vivo.
- [x] **S4 · Toolset DBV en Python** (REQ-06, REQ-25, REQ-26) — registrado, probado y retirado en vivo; plantilla C++ sin compilar.
- [x] **S5 · Diagnóstico y preparación** (REQ-04, REQ-05) — probado sobre proyectos temporales; se niega con el editor abierto.
- [x] **S6 · Herramientas de Unreal y permisos** (REQ-10, REQ-12, REQ-13, REQ-26).
- [x] **S7 · Tripo v3 y biblioteca** (REQ-15, REQ-17, RNF-04) — texto → modelo contra servidor simulado (sin gastar créditos).
- [x] **S8 · Enviar a Unreal** (REQ-19) — GLB real de Tripo importado y colocado en vivo, y limpiado.
- [x] **S9 · MCP propio** (REQ-20, REQ-11) — verificado en vivo por stdio.
- [x] **S10 · Interfaz** (REQ-08, REQ-14, REQ-24) — banco E2E en Edge headless.
- [x] **S11 · CI y documentación** — workflows CI y Release (sin ejecutar aún en GitHub: no hay remoto).
- [x] **`/test`** — 116 Rust, 193 Vitest, 13 Python, E2E, evals `qwen3:8b` 5/5 ×3 con cero modificaciones sin aprobación; RNF-08 «Copiar diagnóstico».
- [ ] **`/ship` v0.1** — con el usuario: versión, `walkthrough.md`, `docs/IA.md`, tag.

## Deuda Técnica
*Mejoras detectadas fuera del foco de la tarea actual, para abordarlas después.*

- **Imagen → modelo con Tripo (REQ-15):** no implementado; el endpoint v3 no está verificado (necesita una clave real).
- **Generación real con Tripo:** solo probada contra un servidor simulado; falta una prueba con clave (≈30 créditos).
- **Variante C++ del toolset (REQ-06, D8):** plantilla sin compilar; verificar con UBT en un proyecto con código.
- **Preparar un proyecto real (REQ-05):** probado en proyectos temporales; falta el ciclo completo con un proyecto de plantilla de Epic y reinicio del editor.
- **RNF-08:** hecho «Copiar diagnóstico»; falta el log local rotativo.
- **REQ-25 (guías conceptuales):** la referencia sintáctica sale del motor; la búsqueda en las guías de MCPCLiente (D2) no está conectada.
- **REQ-13 (captura del viewport para modelos multimodales):** las herramientas de Epic `CaptureViewport` se pueden llamar, pero la imagen no se pasa al modelo.
- **El MCP propio no tiene «run_python» sin el toolset DBV** (la app sí usa el `RunPython` del proyecto como alternativa): decidir si se iguala.
- **Banco E2E intermitente:** 1 de 4 ejecuciones falló al importar `app/app.js` (ver Lecciones en `memory.md`).
- **macOS y Linux:** sin probar en máquina real (solo compilan en CI cuando haya remoto).
- **Regla de un solo `return` en JS/Rust:** el linter solo la verifica en Python; revisada a mano en `/code-simplify`.

---

## 🔄 Context Snapshot / Snapshot de Contexto

> **Last update / Última actualización:** 2026-10-24
> **Exact point / Punto exacto:** v0.1 construida y probada; `/code-simplify` en curso o hecho (ver último commit).
> **Pending / Pendiente:** revisión del usuario (ADR-V010-001, deuda técnica) y `/ship` v0.1.
> **Next step / Próximo paso:** leer `memory.md` (ADR-V010-001) y esta deuda; decidir versión y ejecutar `/ship`.
