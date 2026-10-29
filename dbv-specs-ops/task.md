# Backlog - DBV Unreal AI

## Contexto del Proyecto (Context Snapshot)

* **Objetivo**: copiloto de IA nativo (Win/macOS/Linux) para Unreal Engine 5.8: conecta cualquier IA al proyecto abierto, lo prepara y genera assets con Tripo y World Labs directamente en el editor.
* **Estado actual**: **v0.2.2 en desarrollo** (2026-10-28). Selección dinámica de modelo y esfuerzo de razonamiento en agentes CLI (REQ-47).
* **Última decisión técnica**: Inclusión de botón de ajustes ⚙️ en agentes conectados, consulta dinámica de modelos CLI con versión mínima Gemini 3.8 Flash para Antigravity CLI, y paso de flags `--model` y reasoning a ACP.
* **Próximo paso**: Ejecución de `/build` v0.2.2 (Slices S38 y S39).

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

## v0.2.0 — memoria y plan del proyecto, World Labs (plan en `implementation_plan.md` §5)

- [x] **`/spec` v0.2** (2026-10-26) — `SPECIFICATIONS.md` v1.4: §3.7 memoria y plan (REQ-34..REQ-38), §3.8 actualizaciones y deshacer (REQ-39, REQ-40), REQ-07/13/18/21 concretados, R-16..R-19, P-07..P-09; `DESIGN.md` con la vista Plan y los componentes nuevos.
- [x] **`/plan` v0.2** (2026-10-26) — Architect Review, D13–D21, REJECTED-02..04. **Aprobado por el usuario el 2026-10-26** con dos cambios: fuera el actualizador (REQ-39, D21, S30) y la clave de World Labs (llavero + Ajustes) va antes del spike de S18. Gasto de Tripo autorizado para S18.
- [ ] **S18 · Verificación con el usuario y spikes en vivo** — *con el usuario delante (gasto y editor abierto)*:
  - [x] Tripo real (2026-10-26): texto, imagen (con subida), rig, animación; coste real = estimación (105 créditos). Arreglados: `model` obligatorio, trabajos huérfanos reanudados.
  - [x] Personajes S17 en UE 5.8.4 (variante C++ recompilada con `play_animation`): Skeletal Mesh + Animation Sequence importados; arreglados el esqueleto del clip (compatible) y la colocación en el suelo. Verificado después en una captura: el pirata baila cuando el editor hace tick.
  - [x] `CaptureViewport` (PNG base64, exige los tres parámetros) y contador de deshacer (deshacer diferido al siguiente tick).
  - [ ] Un mundo `marble-1.0-draft` (230 créditos) importado — necesita la clave de World Labs (S27, primero el sitio para guardarla).
  - [ ] Plugin de splats (P-09).
  - [ ] Pendiente de la v0.1: la ruta de importación lleva `model/SkeletalMeshes` (Interchange crea subcarpetas con el nombre del fichero); la convención es `/Game/DBV/<Tipo>/<nombre>`.
- [x] **S19 · Memoria: núcleo** (REQ-34) — `project_memory/` en Rust: confinamiento, secretos, hash base, formato tolerante, resumen; test «preparar → memoria → revertir → la memoria sigue».
- [x] **S20 · Memoria en la conversación** (REQ-34, REQ-35) — activar, ficha de resumen, preámbulo de agentes, `project_memory_read`/`_propose`, tarjeta «¿Guardar…?», «Recordar», `--project` al `--mcp` del agente.
- [x] **S21 · Vista Plan** (REQ-36) — tareas, «Planificar» con `project_tasks_propose` y coste de la app, «Hacer esta tarea».
- [x] **S22 · Tareas comprobadas** (REQ-37) — criterios, `check_task` (toolset 0.3.0 o `RunPython` con código fijo).
- [x] **S23 · Memoria en el MCP propio** (hecho a la vez que S20–S22: las cinco herramientas y el test de paridad) (REQ-38) — cinco herramientas, test de paridad.
- [x] **S24 · Varias instancias** (REQ-07) — `state.connections`, selector, identidad por sesión, puerto libre.
- [x] **S25 · La IA ve el viewport** (REQ-13) — captura como imagen (D16).
- [x] **S26 · Deshacer un turno** (REQ-40) — contadores, `undo_turn` (post-tick callback Slate), botón en UI, avisos de desincronización, confirmación para borrar assets creados.
- [x] **S27 · World Labs: generación** (REQ-18) — cliente Marble API, sondeo de operaciones, biblioteca «mundo» con `.spz` y `.glb` de colisión, `generate_world`, `export_world_mesh`, pestaña y saldo en Generar.
- [ ] **S28 · World Labs → Unreal** (REQ-18) — `import_world`, escala y suelo, cielo y splats según S18.
- [x] **S28b · Componente nativo propio de Gaussian Splatting en C++** (REQ-41, decidido con el usuario el 2026-10-26) — Tras el hallazgo de la ventana de pago/clave "Pro" en XGRIDS LCC, se rechaza la dependencia privativa. Desarrollo de `UDbvGaussianSplatComponent` y `ADbvGaussianSplatActor` (licencia MIT) en `Plugins/DBVUnrealAI` (módulo `DBVUnrealAICpp`), aprovechando las lecciones de YaGS (Apache 2.0: escritura de profundidad para oclusión mutua, GPU radix sort, y soporte nativo de `.spz` y `.ply`). Integrado con «Enviar a Unreal» (`sendToUnreal`) para colocar el splat junto con su malla de colisión.
- [x] **S28c · Spike y validación visual de splats** (P-10) — *Completado el 2026-10-26:* El spike de XGRIDS LCC (`LCC4Unreal`) validó que los `.spz` de World Labs funcionan a la perfección con escala 100x y orientación Roll=90.0° en UE 5.8.4, sirviendo de referencia de calidad idéntica a Wordsmith. Con esa referencia y las lecciones de YaGS, se avanza en la solución propia 100% nativa y gratuita.
- [x] **S28d · Importación de escenarios World Labs (Splat + Collider Mesh) [v0.2]** (REQ-42) — Detección e importación directa de manifiestos JSON (individual o carpeta completa `Wordsmith/storage/generations/worldlabs/`): descarga de splat `.spz`, malla de colisión `.glb` y miniatura `.webp`, guardado en biblioteca con metadatos de escala/suelo (`metric_scale_factor`, `ground_plane_offset`), y soporte en UI y backend con tests unitarios en Rust y Vitest.
- [ ] **S29 · MCPs de terceros** (REQ-21) — Ajustes, stdio/HTTP, asistente y ACP, permisos.
- [x] ~~**S30 · Actualizador** (REQ-39)~~ — retirado por el usuario el 2026-10-26: Microsoft Store gestiona las actualizaciones en Windows.
- [x] **`/test`** (v0.2.0) — 170 tests Rust, 269 Vitest (29 suites), 34 Python, lint 0 errores.
- [x] **`/code-simplify`** (v0.2.0) — limpieza de temporales en jobs World Labs, unificación de retornos en generateView.js, doc IA actualizada.
- [x] **`/ship` v0.2.0** (2026-10-27) — Minor bump a v0.2.0, CHANGELOG y notas de release ES/EN, README ES/EN y walkthrough actualizados, tag v0.2.0.

## v0.2.1 — Conexión estructurada de IAs, Antigravity CLI, Ayuda y Acerca de (REQ-43..REQ-46)
- [x] **`/spec` v0.2.1** — Especificación de clasificación tripartita, Antigravity CLI (`agy`), modal Acerca de enriquecido y Centro de Ayuda integrado.
- [x] **`/plan` v0.2.1** — Plan de implementación detallado para frontend, backend y documentación.
- [x] **S31 · Detección y ejecución de Antigravity CLI en Rust** (`detect.rs`, `acp.rs`) — Detección de `agy` o `agy_acp_server`, lanzamiento ACP, manejo de enlaces de instalación.
- [x] **S32 · Reorganización visual del modal en 3 categorías** (`connectWizard.js`, `ai.css`) — Secciones de Suscripción (`--ai-agent`), Cloud API (`--ai-cloud`) y Local (`--ai-local`).
- [x] **S33 · Badges semánticos en conexiones guardadas** (`connectWizard.js`, `connections.js`, `ai.css`) — Píldoras de categoría clara en «Tus conexiones».
- [x] **S34 · Documentación y textos bilingües** (`translations.js`, `IA.md`, `IA.en.md`) — Enlaces duales oficiales a Google Antigravity y Gemini CLI.
- [x] **S35 · Limpieza del warning de compilación Rust** (`worldlabs.rs`) — Retirar `use std::io::Read;`.
- [x] **S36 · Renovación del modal Acerca de** (`index.html`, `main.js`, `styles.css`) — Diseño Typst-style con logo, links a web/github, telemetría de entorno (`window.__TAURI__` / OS) y licencia.
- [x] **S37 · Centro de Ayuda interactivo y atajo F1** (`index.html`, `helpModal.js`, `styles.css`) — Pestañas temáticas (UE 5.8, Conexión de IAs, Atajos, Enlaces).
- [x] **`/test` v0.2.1** — Pruebas unitarias de Rust (`cargo test`: 170 passed) y frontend (`npx vitest run`: 269 passed).
- [x] **`/ship` v0.2.1** — Actualización de versiones a 0.2.1, CHANGELOG y notas de release ES/EN.

## v0.2.2 — Modelos Dinámicos y Razonamiento en Agentes CLI (REQ-47)
- [x] **`/spec` v0.2.2** — Especificación REQ-47: consulta dinámica de modelos CLI (mínimo Gemini 3.8 Flash para Antigravity), selector de modelo y razonamiento, y botón ⚙️ en agentes conectados.
- [x] **`/plan` v0.2.2** — Architect review adversarial, ampliación de `Connection` y `AgentSpec`, diseño UI inline.
- [x] **S38 · Backend: consulta dinámica de modelos y paso a ACP** (`commands.rs`, `detect.rs`, `connections.rs`, `acp.rs`) — Comando `ai_list_agent_models`, campos `reasoning_effort` en `Connection` y propagación de `--model` y reasoning en lanzamiento ACP.
- [x] **S39 · Frontend: botón de ajustes ⚙️ y panel inline** (`connectWizard.js`, `backend.js`, `ai.css`, `translations.js`) — Componente de ajustes inline por agente, selectores dinámicos, guardado reactivo y textos ES/EN.
- [x] **`/test` v0.2.2** — Pruebas unitarias de Rust y Vitest para listado de modelos reales de agentes (`agy`, `claude`, `codex`), caché de sesión, botón de refresco, resumen estilo Visual Studio y persistencia de `reasoning_effort`.
- [x] **`/code-simplify` v0.2.2** — Revisión de código y cumplimiento (0 warnings en Rust, 0 errores en `dbv-lint`).
- [x] **`/ship` v0.2.2** — Bump de versión a 0.2.2, CHANGELOG y notas de release.

## Próxima versión — hallazgos de la prueba de la v0.1.0

- [x] «Cómo instalarlo» no abría el navegador: `opener.openUrl` en vez de `shell.open` (2026-10-25).
- [x] Gemini CLI: flag `--acp` y avisos de que solo sirve con accesos de pago desde el 2026-06-18 (2026-10-25).
- [x] README (ES/EN): sección «Descárgalo e instálalo» por sistema operativo con los instaladores de la Release, avisos de SmartScreen y Gatekeeper, y canales «próximamente» (2026-10-25).
- [ ] **Canales de distribución** (anunciados como «próximamente» en el README): tap de Homebrew `davidbuenov/homebrew-dbv-unreal-ai` que se actualice solo en cada Release (como el de dbv-md-reader); Microsoft Store (MSIX firmado por la Store, ver `dbv-specs-ops/docs/MARKETPLACE_PUBLISHING.md`); ficha en Uptodown para Windows y macOS. Al publicarlos, cambiar la sección «Próximamente» del README.
- [ ] **Actualizaciones:** la app no se actualiza sola (`includeUpdaterJson: false`, sin plugin updater). Decidir si se añade el updater de Tauri para los instaladores de Releases o se delega en Store/Homebrew.
- [x] **Antigravity CLI como quinto agente** (formalizado en v0.2.1 con REQ-44). Datos del 2026-10-25: el CLI es `agy` (instalador por script, no npm). El ACP oficial corre vía `agy_acp_server` o `agy --acp`. Enlace: `https://antigravity.google/product/antigravity-cli`.


- [ ] **Idioma chino (`zh-CN`)** — aplazado por el usuario el 2026-10-26 (la comunidad de Tripo en China es grande; dbv-md-reader añadió coreano). Al retomarlo: un tercer idioma en `i18n.js` y en los dos `translations.js` (~900 textos), selector y detección de `zh-*`; `LANGUAGE` de `context.js` y `agentPreamble` (hoy `lang === 'es' ? … : 'Answer in English.'`: un tercer idioma caería en inglés); los errores del backend en Rust están en español; `README.zh-CN.md` e `IA.zh-CN.md`; revisión por una persona nativa; investigar el acceso desde China (proveedores como DeepSeek o Qwen, y si Tripo tiene allí otra plataforma).

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

> **Last update / Última actualización:** 2026-10-29, entrega v0.2.2 completada.
> **Exact point / Punto exacto:** (1) Descubrimiento dinámico de modelos por CLI (Antigravity CLI con Argon/Flash, Claude Code stream-json handshake, Codex debug models); (2) Selector dinámico de reasoning effort; (3) Píldora resumen estilo Visual Studio (Gemini 4 Argon Medium, Claude Sonnet 5.5 Medium, GPT-6-Luna Medium); (4) Caché de sesión y refresco manual (🔄); (5) Propagación a sesiones ACP; (6) Enlaces de Microsoft Store en README y docs. 176 tests Rust y 273 tests Vitest pasando.
> **Estado del editor de pruebas** (`D:\Programacion\Unreal\DBVUnrealAITest1`, UE 5.8.4, toolset C++ 0.2.0): Nivel activo `/Game/DBV/Maps/PruebasGaussianSplat`.
> **Pending / Pendiente:** (1) S28: cielo y automatización de iluminación; (2) S29: MCPs de terceros (REQ-21); (3) Subida de paquete MSIX a Partner Center de Microsoft Store.
> **Next step / Próximo paso:** Cierre de release v0.2.2 con commit y tag en git.



