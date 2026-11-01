# Backlog - DBV Unreal AI

## Contexto del Proyecto (Context Snapshot)

* **Objetivo**: copiloto de IA nativo (Win/macOS/Linux) para Unreal Engine 5.8: conecta cualquier IA al proyecto abierto, lo prepara y genera assets con Tripo y World Labs directamente en el editor.
* **Estado actual**: **v0.2.4** publicada (2026-10-29); **v0.3.0** en `/build` (rama `v0.3.0`): REQ-50..REQ-60 hechos; ampliación REQ-61..REQ-64 con S61–S68 hechos.
* **Última decisión técnica**: render de splats por GPU propio en el módulo `DBVUnrealAICppRender` (D33–D45); el editor se lanza fuera del Job Object de la app; **no usar Live Coding** (pedido por el usuario): cerrar el editor y compilar con UBT.
* **Próximo paso**: S69–S74 (`implementation_plan.md` §10), empezando por S69 (código del proyecto, REQ-22).

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

## v0.2.3 — Conexión a IAs común a todas las apps DBV (REQ-48, REQ-49; plan en `implementation_plan.md` §8)
- [x] **`/spec`** — REQ-48 (guía común dentro de la app) y REQ-49 (agentes que funcionan); correcciones en REQ-44, REQ-46 y REQ-47 (`agy --acp` y los flags `-c`/`--effort` no existían o no estaban documentados).
- [x] **`/plan`** — §8 (S40–S44), riesgos R-22..R-24.
- [x] **S40 · Framework 3.1.0 → 3.2.0** — plantillas de IA, `AI_DESKTOP_ARCHITECTURE`, `MASTER_PROMPT`, `UPGRADE_PROMPT`, `Framework Version`.
- [x] **S41 · Guía común** — lo propio de Unreal debajo de la marca en `docs/IA*.md`; `sync-ai.mjs --apply` desde el starter (detección, ACP, asistente, `guide.js`, `markdown.js` y tests); pestaña *Guía de IAs* de `F1` con `renderGuide()`.
- [x] **S42 · Integración** — traducciones (`ai.agentNoAcp`, `ai.agentSigningIn`, `ai.guideTitle`, `ai.copy`), `onAuthenticate` en `aiApp.js`, estilos `.ai-guide`, clase `ai-panel` en el contenedor y ✕ de las fichas.
- [x] **S43 · Trazabilidad propia** — `tests/app/aiIntegration.test.js` (REQ-09, 11, 20, 43, 44, 46, 47, 48, 49, RF-IA-01, RF-IA-02).
- [x] **`/test`** — Vitest 294/294, Rust 177/177, Python OK, `dbv-lint` sin errores (3 requisitos sin test, los mismos que en la v0.2.2), `sync-ai --check` desde el starter OK; la ayuda F1 comprobada en el banco con capturas.
- [x] **Banco e2e reparado** — llevaba roto desde la v0.2.0 (mismos fallos en la v0.2.2): su backend simulado no tenía `memory_status`/`memory_tool_specs` (la vista Proyecto fallaba y la conexión automática se quedaba en «Sin editor») y un test exigía que antes de aprobar solo se llamara a `validate_python`, cuando la v0.2.0 añadió dos consultas de solo lectura (`get_project_info`, `get_undo_count`). Ahora pasa entero y comprueba también la guía de F1 (REQ-48) y el «Acerca de» (REQ-45, sin prueba hasta ahora); el simulador da una versión a `getVersion()` (antes salía «Versión null»). Trazabilidad: solo quedan sin test REQ-21 y REQ-23, que aún no están construidos (v0.2/v0.3 de la hoja de ruta). `E2E_PORT` para no chocar con un navegador en el 9333.
- [x] **MSIX y Microsoft Store** — `npm run tauri:windows:build` → `src-tauri/target/msix/dbv-unreal-ai_0.2.3.0.msixbundle` (Identity 0.2.3.0, `runFullTrust`, assets sin relleno); `MICROSOFT_STORE.md` §4 y fichas `descripcionStore_*.md` (novedades v0.2.3, descripción, notas de certificación). Subirlo a Partner Center lo hace el usuario.
- [x] **`/ship`** — versión 0.2.3 (package.json, package-lock.json —estaba en 0.2.1—, Cargo.toml, Cargo.lock, tauri.conf.json), CHANGELOG ES/EN, notas `releases/v0.2.3*.md`, README (insignias, estado y GIF) y web (estaba en v0.2.0).

## 📌 Entrega v0.2.4 (2026-10-29) — Resolución de Incidencia: Alias de Ejecución para MCP en Microsoft Store

- [x] **Diagnóstico y confirmación:** Causa confirmada. `WindowsApps` deniega la ejecución directa a procesos Win32 externos (Go/`os/exec` de `agy_acp_server`) con `ERROR_ACCESS_DENIED` (error 5) por restricciones de AppModel/sandbox. La solución canónica de Microsoft es `uap3:AppExecutionAlias`.
- [x] **Declaración del alias de ejecución:** Añadido `"appExecutionAliases": [{ "alias": "dbv-unreal-ai" }]` en `src-tauri/gen/windows/bundle.config.json` (generado por `@choochmeque/tauri-windows-bundle`).
- [x] **Resolución en backend Rust (`src-tauri/src/lib.rs`):** Función `resolve_mcp_executable` que detecta si el ejecutable reside en `\WindowsApps\` y resuelve hacia `%LOCALAPPDATA%\Microsoft\WindowsApps\dbv-unreal-ai.exe` verificando su existencia mediante `symlink_metadata` (reparse point `AppExecLink`). Fallback transparente a `current_exe` en desarrollo o versiones no Store.
- [x] **Tests unitarios:** Añadidos tests en `src-tauri/src/lib.rs` (`resolve_mcp_executable_preserva_rutas_normales_y_fallback_seguro` y `app_mcp_command_genera_argumentos_correctos`). 179 tests Rust y 294 tests Vitest pasando al 100%.
- [x] **Empaquetado MSIX y entrega:** v0.2.4 publicada (commit `7db0d0e`; 0.2.4.0 instalada desde la Store el 2026-10-31).

## v0.3.0 — memoria con varias entradas, contexto acotado, biblioteca visible y Convai (REQ-50..REQ-57, RNF-08; plan en `implementation_plan.md` §9)
- [x] **`/spec`** (2026-10-31) — §3.12 (REQ-50..REQ-57), RNF-08 concretado, REQ-35 sin el límite de una propuesta por turno, R-20, R-21, R-25..R-27, P-11, P-12; `DESIGN.md` v0.3.0. Verificado en vivo: Convai dentro del MCP de Epic; `describe_toolset` de Niagara = 306 KB.
- [x] **`/plan`** (2026-10-31) — Architect Review, D22–D30 (sin el usuario, ADR-V030-001), REJECTED-06 y 07; spike de solo lectura de P-11 (`ConvaiObjectComponent` en Python) y P-12 resuelta.
- [x] **S45 · Memoria con varias entradas** (REQ-50) — `entries` (hasta 8), tarjeta con casillas, `memory_check_entries`, sin límite por turno.
- [x] **S46 · Tope y `describe_toolset` compacto** (REQ-51) — `unreal/compact.rs` y `unrealTools.js` contra la misma referencia (`tests/fixtures/compact/`); Niagara de 306 KB a 5,5 KB; caché de toolsets; `instructions`.
- [x] **S47 · `unreal_scene_summary` en el MCP y capturas ligeras** (REQ-51) — 1024 px / calidad 75, `detail: "high"`.
- [x] **S48 · Medición visible del uso de herramientas** (REQ-51) — `tool_usage.rs`, `tool_usage_since`, contador al pie (asistente y agente).
- [x] **S49 · Prueba de consumo ≥ 50 %** (REQ-51) — guion fijo con respuestas grabadas del editor real: 79 % menos texto (67 % con la imagen).
- [x] **S50 · Aviso de accesos duplicados al editor** (REQ-52) — `unreal/duplicates.rs` en `unreal_readiness`; franja en la vista Proyecto; preámbulo e `instructions`.
- [x] **S51 · Biblioteca en Documentos, creada al arrancar y migrada** (REQ-53) — `ensure_library`; excepción OneDrive (D31).
- [x] **S52 · Splats desde el MCP propio** (REQ-54) — `library::send_plan` + test de paridad con `sendToUnreal.js` (`tests/fixtures/send-plan/`).
- [x] **S53 · Log rotativo y diagnóstico** (RNF-08) — `logging.rs` (claves tapadas), «Copiar diagnóstico» con rutas y log.
- [x] **S54 · Convai: detección** (REQ-55) — `unreal/convai.rs` en `unreal_readiness` (no hizo falta un comando `convai_status` aparte); sección en la vista Proyecto.
- [x] **S55 · Convai en la conversación** (REQ-56) — `convaiPrompt`, avisos de Blueprints guardados, sugerencia.
- [x] **S56 · Zonas del nivel** (REQ-57) — spike P-11 resuelto en vivo (crear, actualizar sin duplicar, deshacer); `zones.rs` en las dos superficies, tarjeta de zonas.
- [x] **S58 · Personajes de Convai en Ajustes** (REQ-58) — `convai_characters.rs` (`<config>/convai-characters.json`, D32), herramienta `convai_characters` en el MCP y en el asistente, tarjeta en Ajustes.
- [x] **S59 · Tutorial de Convai en F1** (REQ-59) — pestaña Convai (ES/EN) con requisitos, pasos y problemas conocidos; enlaces desde Ajustes y desde la vista Proyecto.
- [x] **S60 · Ajustes con pestañas** (REQ-60) — pedido por el usuario.
- [x] **S57 · Textos y documentación** — traducciones ES/EN, `docs/IA*.md` (debajo de la marca), CHANGELOG ES/EN.
- [ ] **Prueba de Convai con el usuario** — *(2026-10-31: configurado por el MCP de la app en `/Game/DBV/Maps/ConvaiTest`: `BP_VictoriaNew` con chatbot, Character ID, FaceSync y movimiento (Pawn); `BP_ThirdPersonCharacter` con el componente de jugador; NavMesh ajustado al suelo; zonas «Inicio», «Rincón de Victoria» y «Mirador norte». Falta que el usuario hable con ella en Play y le pida ir a una zona.)* Antes: ensamblar un MetaHuman (VictoriaNew no se puede construir aún desde script: `can_build_meta_human` = False; necesita abrirlo en el editor de MetaHuman y el auto-rigging en la nube), un Character ID del Playground y el Blueprint del jugador; luego «Haz que este MetaHuman hable con Convai» y zonas en `/Game/DBV/Maps/ConvaiTest`.
- [x] **`/test`** v0.3.0 (2026-10-31) — Rust 206, Vitest 324 (37 ficheros), Python OK, clippy sin avisos, `dbv-lint` sin errores (75 requisitos, 73 con test: faltan REQ-21 y REQ-23, fuera de esta versión), banco e2e correcto. Prueba de consumo: 79 % menos texto. **Evals con modelo real sin ejecutar** (no había Ollama ni otro modelo disponible sin el usuario): preparados `convai-skills-first` (REQ-56) y `convai-registered-id` (REQ-58) en `scripts/eval/unreal-eval.mjs`. Verificado en vivo: zonas, Convai (Victoria habla, sigue, «Move to <zona>» y «Take me to <zona>» con `Escort` redefinido), MCP compacto.
- [x] **`/code-simplify`** v0.3.0 (2026-10-31) — 0 Críticos; 2 Importantes resueltos y en el CHANGELOG (pie del turno perdido al reabrir una conversación, desde la v0.2.0; consola de `git` visible en Windows); refactor: `unreal::commands::run_fixed_python` en lugar de dos copias (zonas y comprobaciones de tareas). Seguridad: la clave de Convai solo se mira vacía y no llega al log (test), el log tapa claves (test), las zonas viajan como dato en hexadecimal, `dirs` 6 ya estaba en `Cargo.lock` (dependencia real de Tauri). Vitest 325, Rust 206, clippy limpio, `dbv-lint` sin errores, e2e correcto.
- [x] **Commit de seguridad** (2026-10-31, pedido por el usuario): rama `v0.3.0`, commit `5dbcec6`, sin push.
- [x] **`/spec` de la ampliación** (2026-10-31, con el usuario) — §3.13: REQ-61 (render de splats por GPU propio), REQ-62 (armónicos esféricos), REQ-63 (splats grandes y fallos explicados; lector por bloques ya hecho, C++ sin compilar), REQ-64 (chino) y concreciones para la v0.3.0 de REQ-18, REQ-22, REQ-23 y REQ-25. P-09 y P-10 cerradas; P-13 abierta (fuente de las guías, no bloquea); riesgos R-28..R-31.
- [x] **`/plan` de la ampliación** (2026-10-31) — `implementation_plan.md` §10: Architect Review, D33–D45, S61–S74. **Aprobado por el usuario el 2026-10-31**: commit y estado actualizado tras cada slice.
  - [ ] S61 · Spike del render por GPU con puerta (viewport, PIE y captura de la app) (REQ-61)
    - [x] Módulo `DBVUnrealAICppRender` (PostConfigInit): `DbvSplat.usf` (ProjectCS, SplatVS, SplatPS), `FDbvSplatGpuCloud` (subida única; proyección y `SortGPUBuffers` en un pase inmediato), registro de proxies y `FDbvSplatViewExtension` (tras el tonemap, prueba de profundidad con el uniform de vista). El proxy ya no dibuja en la CPU; el componente empaqueta (cuaternión normalizado) y sube. `DBVUnrealAICpp` 0.3.0.
    - [x] Compila con UBT en el anfitrión `%TEMP%\dbvh\DbvHost` (`build.ps1`; el scratchpad supera 260 caracteres) y los tres shaders compilan al arrancar el motor sin ventana.
    - [x] **Puerta superada (2026-10-31, con el usuario):** la iglesia (9.975.598 splats) se ve en el viewport, en Play y en la captura de la app; el maniquí la tapa y queda tapado; ~57 fps en Play (Frame 17,4 ms, GPU 16,6 ms, 20,1 M triángulos: se dibujan todos los quads). Arreglado en la prueba: el duplicado de Play no tenía nube (ahora `OnRegister` reutiliza la subida por fichero con una caché y, si no está, recarga conservando la transformación; la nube se libera con la última referencia). Aplicado con Live Coding (`LiveCoding.Compile`) sin reiniciar.
    - [ ] Pendiente para otras porciones: dibujar solo los visibles (S64), suelo automático de un splat importado (densidad de puntos → Z = 0; propuesta, sin REQ aún), `LiveCodingToolset` no está activado en el proyecto (REQ-22 lo tendrá que activar «Preparar»).
  - [x] S61 cerrado.
  - [x] S62 · Nube empaquetada y lectores con SH + tests de Automation (REQ-62, REQ-63) — `DbvSplatPack` (formato de la GPU, SH en half por coeficiente y canal, selección de los más visibles, escala); los lectores escriben directamente ahí (desaparece `FDbvGaussianPoint`); PLY con `f_rest_*` (grado 1–3); **SPZ reescrito según el formato de Niantic** (magic NGSP, orden de bloques, color DC, rotaciones v2/v3, SH, RUB→RDF); la nube sube también los SH; informe con `sh_degree`; toolset Python 0.3.1. Tests: 4 de Automation (`%TEMP%\dbvh\test.ps1`, sin RHI) y 4 de Python. Falta verlo en el editor (cabeceras cambiadas: necesita reinicio; se junta con S63–S65).
  - [x] S63 · Recursos en GPU y presupuesto de VRAM (REQ-61, REQ-62) — `dbv.Splat.VRAMBudgetMB` (0 = 40 % de la VRAM dedicada); `FitBudget`: primero quita los SH, luego los más visibles; búfer de SH subido; informe con `sh_degree` y nota si se reducen los SH o los splats.
  - [x] S64 · Proyección, ordenación y dibujo (REQ-61) — solo los visibles: la proyección cuenta por grupo (memoria compartida) y suma al argumento del dibujo indirecto; los descartados quedan al final tras ordenar.
  - [x] S65 · Armónicos esféricos de grado 1 a 3 (REQ-62) — evaluados en la proyección con la dirección de vista en el espacio local (constantes y signos del 3DGS original).
  - Compila, `ProjectCS` compila al arrancar el anfitrión con D3D12 sin ventana, y los tests de Automation y Python (39) pasan. **Falta la verificación en el editor del usuario** (reinicio: cambiaron cabeceras) → S66/S67.
  - [x] S66 · Varias nubes, PIE, capturas, retirar CPU, versiones, informe (REQ-61, REQ-63)
    - [x] En el editor del usuario (2026-10-31, tras reiniciar): la iglesia recargada con el lector nuevo (9.975.598 splats, **SH grado 3**, todos dibujados, 2,5 s) se ve con más brillo y naturalidad; **el desierto `.spz` de World Labs se ve por primera vez con el actor propio**, bien orientado (1.920.000 splats, 0,2 s); dos nubes en el mismo nivel; capturas de la app correctas. CPU retirado, versiones 0.3.0/0.3.1 e informe hechos.
    - [x] Rendimiento en Play con iglesia + desierto (usuario): GPU 15,2 ms, Frame 16,1 ms (~62 fps), Prims 184 K (antes 20,1 M: el dibujo indirecto de los visibles funciona). El desierto, ya sin el giro RUB→RDF, se ve del derecho con sus cactus. Desierto movido al nivel `isla` a petición del usuario.
    - [x] Parpadeo de la iglesia en Play (solo en Play; el editor solo redibuja cuando algo cambia). **Causa: faltaba una barrera UAV entre `ClearUAVUint` del contador de visibles y las sumas atómicas de `ProjectCS`**, así que algunos fotogramas dibujaban casi 0 splats. Corregido con una transición UAV → UAV; el usuario confirma que ya no parpadea. También se pasó a una clave logarítmica de 20 bits (precisión cerca de la cámara) y se añadió `dbv.Splat.IndirectDraw` para diagnóstico.
    - [ ] Calidad del desierto: el usuario no lo recuerda «tan mal»; pendiente de comparar con XGrids desde la misma cámara si hace falta.
    - Incidente: el editor del usuario se cerró solo a las 21:48 y a las 22:36. Causa real (diagnosticada en S68): la app lo lanzaba dentro del Job Object de `cargo run` y `tauri dev` lo mataba al reiniciarse por cada edición de Rust. Arreglado en `unreal_open_project`. El UDP del anfitrión se dejó desactivado por higiene.
  - [x] S67 · Verificación con el usuario (2026-10-31): iglesia de 10 M de splats con SH 3 en el editor y en Play sin parpadeo (GPU 12,7 ms, ~72 fps, Prims 184 K); el maniquí la tapa y queda tapado; desierto `.spz` del derecho en el nivel `isla`; capturas de la app. Queda como propuesta, sin REQ: suelo automático de un splat importado y comparar la calidad con XGrids si se notan diferencias.
  - [x] S68 · Mundos de World Labs completos (REQ-18)
    - [x] Verificado en vivo con el desierto (2026-10-31): splat a escala 100·m; **colisión con escala (m, m, −m)** (el `.glb` viene en RDF y al importarlo como glTF queda reflejado en Z; sin reflejo sus triángulos miran hacia abajo y los rayos la atraviesan); los dos subidos 100·m·g, con el suelo en Z ≈ 0 (rayos simples a 1–1,8 m sobre las dunas); colisión compleja como simple (si no, el personaje pisaría la caja). Antes, `place_asset` dejaba la colisión «delante de la cámara», sin alinear.
    - [x] `editor.place_world` (toolset); `library::world_metadata` y `world_place_code` en Rust (una sola definición; textos como literales JSON); el MCP importa la colisión sin colocarla y ejecuta `place_world`; la ventana pide el código con `library_world_place_code` y lo ejecuta con `run_python`. Tests: Rust, Vitest (2) y Python.
    - [x] `place_world` verificado en vivo (2026-10-31): escala ×1,976, elevación 217 cm, rayos a 97–183 cm sobre la colisión. El desierto salía boca abajo porque la DLL instalada era anterior a los arreglos hechos con Live Coding (giro del `.spz` y parpadeo): recompilado desde cero con UBT. El usuario pide no usar Live Coding.
    - [x] El editor lanzado desde la app ya no muere con la app (`CREATE_BREAKAWAY_FROM_JOB` o WMI). Verificado en vivo: el editor reabierto desde la app tiene `IsProcessInJob = False` y su padre es `WmiPrvSE.exe`.
    - [x] `place_world` sube sobre el terreno el `PlayerStart` que quede debajo (en vivo: de 92 a 204 cm; la duna está a 104). Con la DLL nueva el desierto sale del derecho; el usuario confirma que Play no parpadea.
    - [x] Un personaje corrió por las dunas en Play (2026-10-31).
    - [x] Spike del cielo (2026-11-01, en vivo): esfera de 1 km con `M_DBV_PanoSky` (`IsSky`, sin luz, panorama con espejo y 90°) en el nivel `isla`; orientación verificada contra el splat. Solo se ve ocultando `SM_SkySphere`, `SkyAtmosphere`, `VolumetricCloud` y `ExponentialHeightFog` (ver `memory.md`). Desde dentro lo tapa la cáscara de cielo del propio splat.
    - [x] **Panorama y cielo descartados por el usuario (2026-11-01)**: desde dentro no se ve y obliga a ocultar el cielo del nivel (REJECTED-08). Se revirtió la descarga del panorama en la biblioteca.
  - [x] S69 · Código del proyecto: regla de rutas (REQ-22) — `unreal/code_policy.rs` (una definición, tests Rust) que consulta la conversación al proponer (`applyChange` con `checkPath`), al pedir permiso un agente (`permissionRefusal`) y al escribir un agente (`stageWrite`), y que `ai_write_file` vuelve a aplicar; regla en el prompt de Unreal. Se reutiliza `propose_changes` en vez de un módulo nuevo (memoria 2026-11-01). Vitest +4.
  - [ ] S70 · Código del proyecto: compilar (REQ-22)
    - [x] Núcleo en Rust (`unreal/compile.rs`): plan mixto (solo `.cpp` → Live Coding; cabeceras/`Build.cs`/`Target.cs` → editor cerrado y UBT), `CompileLiveCoding` por la sesión MCP, UBT por sistema operativo con el target de editor del proyecto, errores recortados; comandos `unreal_compile_plan`, `unreal_compile_live`, `unreal_build_project`; «Preparar» activa `LiveCodingToolset` solo con C++. Tests Rust (5).
    - [x] Ventana (`src/app/codeCompile.js`): al aceptar una propuesta (`host.onProposalApplied`) compila; errores a la IA con `ask` (2 reintentos seguidos, luego botón); tarjeta de editor cerrado con «Compilar» (UBT) y «Abrir el editor». `--mcp`: `compile_project_code`. Vitest +5, Rust +1.
    - [ ] Prueba en vivo con el usuario: «añade a `ADemoActor` una propiedad `Speed`» (cabecera → editor cerrado), un `.cpp` con Live Coding, un error que la IA corrige y «Revertir».
  - [ ] S71 · Recetas y AgentSkills (REQ-23) — la receta 3 pasa a «Caminar por un mundo de World Labs» (usuario, 2026-11-01).
  - [ ] S72 · Guías de la API sin red desde carpeta local (REQ-25)
  - [ ] S73 · Chino (REQ-64)
  - [ ] S75 · Logos en Generar y «Mejorar con IA» el prompt de Tripo y World Labs (REQ-65, REQ-66) — pedidos por el usuario el 2026-11-01.
  - [ ] S74 · Documentación, `/test` y `/code-simplify` de la ampliación
- [ ] **`/ship`** v0.3.0 — con el usuario (tipo de versión, commit y tag).

## Próxima versión — hallazgos de la prueba de la v0.1.0

- [x] «Cómo instalarlo» no abría el navegador: `opener.openUrl` en vez de `shell.open` (2026-10-25).
- [x] Gemini CLI: flag `--acp` y avisos de que solo sirve con accesos de pago desde el 2026-06-18 (2026-10-25).
- [x] README (ES/EN): sección «Descárgalo e instálalo» por sistema operativo con los instaladores de la Release, avisos de SmartScreen y Gatekeeper, y canales «próximamente» (2026-10-25).
- [ ] **Canales de distribución** (anunciados como «próximamente» en el README): tap de Homebrew `davidbuenov/homebrew-dbv-unreal-ai` que se actualice solo en cada Release (como el de dbv-md-reader); Microsoft Store (MSIX firmado por la Store, ver `dbv-specs-ops/docs/MARKETPLACE_PUBLISHING.md`); ficha en Uptodown para Windows y macOS. Al publicarlos, cambiar la sección «Próximamente» del README.
- [ ] **Actualizaciones:** la app no se actualiza sola (`includeUpdaterJson: false`, sin plugin updater). Decidir si se añade el updater de Tauri para los instaladores de Releases o se delega en Store/Homebrew.
- [x] **Antigravity CLI como quinto agente** (formalizado en v0.2.1 con REQ-44). Datos del 2026-10-25: el CLI es `agy` (instalador por script, no npm). ~~El ACP oficial corre vía `agy_acp_server` o `agy --acp`~~ *(corregido el 2026-10-29: `agy --acp` no existe; solo `agy_acp_server`, del registro ACP — REQ-49)*. Enlace: `https://antigravity.google/product/antigravity-cli`.


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

> **Last update / Última actualización:** 2026-11-01 (mañana), `/build` de la ampliación v0.3.0, rama `v0.3.0`. S68 cerrado. Logos de Tripo, World Labs y Convai en Ajustes. Rust 209, Vitest 328.
>
> **Exact point / Punto exacto:** S68 (REQ-18, mundos de World Labs) verificado en vivo con el usuario. El desierto se ve del derecho, Play no parpadea y el usuario corre y sube por las dunas: colisión `.glb` alineada con escala (m, m, −m), compleja como simple, y `PlayerStart` subido sobre el terreno. El editor abierto desde la app ya no muere al reiniciarse la app (`CREATE_BREAKAWAY_FROM_JOB` o WMI; comprobado con `IsProcessInJob = False`).
> **Estado del editor de pruebas** (`D:\Programacion\Unreal\DBVUnrealAITest1`): plugins instalados y compilados desde cero con UBT (`%TEMP%\dbvh\install-test1.ps1`) a las 23:06. Nivel `/Game/DBV/Maps/isla`: `DBV_Desierto_GPU` (×1,976, Z 217), `Arid_Desert_Sand_Dunes_Collider` y `PlayerStart` a 204 cm; el usuario borró `Floor`. La iglesia está en `version030`.
> **Pending / Pendiente:** (1) el panorama y el cielo quedan descartados (REJECTED-08); (2) S69–S74 (en S70, proponer al usuario compilar normal con el editor cerrado en vez de Live Coding); (3) `/test`, `/code-simplify` y `/ship` v0.3.0 con el usuario; (4) propuestas sin REQ: suelo automático de un splat importado, y comparar la calidad con XGrids.
> **Next step / Próximo paso:** S69 (código del proyecto, núcleo en Rust, REQ-22); al tocar C++ del plugin, pedir al usuario que cierre el editor y compilar con UBT.
