# Backlog - DBV Unreal AI

## Contexto del Proyecto (Context Snapshot)

* **Objetivo**: copiloto de IA nativo (Win/macOS/Linux) para Unreal Engine 5.8: conecta cualquier IA al proyecto abierto, lo prepara y genera assets con Tripo directamente en el editor (World Labs en v0.2).
* **Estado actual**: **v0.1.0 publicada** el 2026-10-24 (tag `v0.1.0`, en local: no hay remoto). Incluye la v0.1 (S1–S11) y la ampliación tras la primera prueba del usuario (S12–S17).
* **Última decisión técnica**: D9–D12 de `implementation_plan.md` §4 (trabajos compartidos por ficheros, herramientas de Tripo con una sola definición, biblioteca configurable, Three.js incluido). D1–D8 se tomaron sin el usuario (ADR-V010-001 en `memory.md`), para revisar.
* **Próximo paso**: el usuario prueba la v0.1.0; verificar con la API real de Tripo y con el editor abierto (ver «Deuda técnica»).

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
- [x] **`/code-simplify`** — 2 Críticos y 5 Importantes resueltos (ver CHANGELOG); clippy con unwrap/expect prohibidos, regla de un solo return; todo en verde y eval 5/5.

## v0.1.x — Ampliación tras la primera prueba (REQ-27..REQ-33; plan en `implementation_plan.md` §4)

- [x] **S12 · Trabajos, consola y progreso** (REQ-27, REQ-28, REQ-29) — ficheros de trabajo compartidos, consola en Generar y en la conversación (también del agente), variantes, nombres.
- [x] **S13 · Biblioteca: ubicación e importación** (REQ-30) — Ajustes (carpeta, copia, saldo) y Biblioteca (importar, renombrar, riggear, animar, imagen → 3D).
- [x] **S14 · Superficie completa de Tripo en el MCP** (REQ-33, REQ-16) — 18 operaciones, herramientas únicas para MCP y asistente, saldo. **Sin verificar con Tripo real** salvo texto → modelo: imagen → modelo, subida de ficheros, rig, retarget y el resto solo contra un Tripo simulado según la documentación.
- [x] **S15 · Visor 3D** (REQ-31) — Three.js 0.180 en `src/vendor/three/`, ventana y en línea, miniatura propia; probado en el E2E con un GLB de prueba.
- [x] **S16 · Referencias adjuntas** (REQ-32) — conversación (modelo con/sin visión, agente ACP) y Generar (imagen → modelo). Falta pasar la captura del viewport de Unreal a los modelos con visión (deuda REQ-13).
- [x] **S17 · Esqueletos y animaciones en Unreal** (REQ-16, REQ-19) — `import_model` distingue Skeletal Mesh/Skeleton/AnimSequence; `play_animation`; toolset 0.2.0. **Sin verificar contra el motor real** (el editor no estaba abierto): probar con `D:/Programacion/Tripo/jupiterBueno2/glbs/raw/pumuki_anim_dance_01.glb` (animación sin geometría: puede necesitar el esqueleto del rig) y con un rig nuevo de Tripo.

- [x] **`/test` v0.1.x** — 137 Rust, 236 Vitest, 25 Python, E2E con visor 3D y adjuntos; el banco E2E ya cierra los Edge que abre.
- [x] **`/code-simplify` v0.1.x** — 1 Crítico (subida a Tripo de cualquier fichero local con una ruta escrita por la IA) y 2 Importantes resueltos, 3 Nits anotados (ver CHANGELOG); lint sin errores.
- [x] **`/ship` v0.1.0** (2026-10-24) — versión elegida por el usuario, `walkthrough.md`, `docs/IA.md`/`IA.en.md`, CHANGELOG publicado, tag `v0.1.0`.

## Próxima versión — hallazgos de la prueba de la v0.1.0

- [x] «Cómo instalarlo» no abría el navegador: `opener.openUrl` en vez de `shell.open` (2026-10-25).
- [x] Gemini CLI: flag `--acp` y avisos de que solo sirve con accesos de pago desde el 2026-06-18 (2026-10-25).
- [x] README (ES/EN): sección «Descárgalo e instálalo» por sistema operativo con los instaladores de la Release, avisos de SmartScreen y Gatekeeper, y canales «próximamente» (2026-10-25).
- [ ] **Canales de distribución** (anunciados como «próximamente» en el README): tap de Homebrew `davidbuenov/homebrew-dbv-unreal-ai` que se actualice solo en cada Release (como el de dbv-md-reader); Microsoft Store (MSIX firmado por la Store, ver `dbv-specs-ops/docs/MARKETPLACE_PUBLISHING.md`); ficha en Uptodown para Windows y macOS. Al publicarlos, cambiar la sección «Próximamente» del README.
- [ ] **Actualizaciones:** la app no se actualiza sola (`includeUpdaterJson: false`, sin plugin updater). Decidir si se añade el updater de Tauri para los instaladores de Releases o se delega en Store/Homebrew.
- [ ] **Antigravity CLI como quinto agente** (pendiente de `/spec` + `/plan`, requisito nuevo). Datos del 2026-10-25: el CLI es `agy` (instalador por script, no npm) y **no** tiene modo ACP (issue #31 de `google-antigravity/antigravity-cli`, abierta). El ACP oficial es un binario aparte de Google, `agy_acp_server`, publicado en el registro ACP como `antigravity-acp` (v1.4.3, por plataforma, con SHA-256; Linux lleva `--uid=`). Decidir: descargarlo y verificarlo desde la app o pedir su ruta; arranque en frío de ~16 s en Windows (`AGY_ACP_DEFAULT_MODEL` lo reduce); `session/load` no reproduce el historial.


## Deuda Técnica
*Mejoras detectadas fuera del foco de la tarea actual, para abordarlas después.*

- **Personajes en Unreal (S17) sin probar en el motor:** importar un GLB riggeado de Tripo por Interchange, el Skeletal Mesh resultante, `override_animation_data` en el editor y la variante C++ recompilada con `play_animation`.
- **Tripo sin probar con la API real:** todo se ha probado solo contra un servidor simulado según la documentación del 2026-10-24, incluido texto → modelo con la app (Holo-Companion sí lo usó en real). Probar con la clave del usuario y su confirmación de gasto: texto → modelo (20 créditos), imagen → modelo con `/v3/files` (30), rig (25), una animación (10); y comprobar que el coste real (`credits_consumed`) cuadra con la estimación y con el saldo.
- **Variante C++ del toolset (REQ-06, D8):** el usuario la instaló desde la interfaz en su proyecto y el editor arrancó bien (2026-10-24); falta verificar en otro proyecto y en CI con UBT.
- **Preparar un proyecto real (REQ-05):** el usuario lo ha ejecutado con su proyecto y el editor arrancó bien (2026-10-24); falta el ciclo con un proyecto de plantilla de Epic y la reversión.
- **RNF-08:** hecho «Copiar diagnóstico»; falta el log local rotativo.
- **REQ-25 (guías conceptuales):** la referencia sintáctica sale del motor; la búsqueda en las guías de MCPCLiente (D2) no está conectada.
- **REQ-13 (captura del viewport para modelos multimodales):** las herramientas de Epic `CaptureViewport` se pueden llamar, pero la imagen no se pasa al modelo.
- **El MCP propio no tiene «run_python» sin el toolset DBV** (la app sí usa el `RunPython` del proyecto como alternativa): decidir si se iguala.
- **Banco E2E intermitente:** probablemente causado por los Edge que el banco dejaba abiertos (corregido el 2026-10-24); vigilar.
- **macOS y Linux:** sin probar en máquina real (solo compilan en CI cuando haya remoto).
- **Regla de un solo `return` en JS/Rust:** el linter solo la verifica en Python; revisada a mano en `/code-simplify`.

---

## 🔄 Context Snapshot / Snapshot de Contexto

> **Last update / Última actualización:** 2026-10-24
> **Exact point / Punto exacto:** v0.1.0 publicada en local (commit de release + tag `v0.1.0`; sin remoto). Revisión de la v0.1.x cerrada sin Críticos.
> **Pending / Pendiente:** verificar con la API real de Tripo y con el editor abierto (S17); crear el remoto de GitHub y subir (`git push origin master --tags`) para que el workflow de Release compile los instaladores.
> **Next step / Próximo paso:** el usuario prueba la v0.1.0; los hallazgos entran como `[Sin publicar]` de la próxima versión.
