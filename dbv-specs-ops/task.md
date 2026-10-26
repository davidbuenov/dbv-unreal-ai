# Backlog - DBV Unreal AI

## Contexto del Proyecto (Context Snapshot)

* **Objetivo**: copiloto de IA nativo (Win/macOS/Linux) para Unreal Engine 5.8: conecta cualquier IA al proyecto abierto, lo prepara y genera assets con Tripo directamente en el editor (World Labs en v0.2).
* **Estado actual**: **v0.1.0 publicada** el 2026-10-24 (tag `v0.1.0`, en local: no hay remoto). Incluye la v0.1 (S1–S11) y la ampliación tras la primera prueba del usuario (S12–S17).
* **Última decisión técnica**: D9–D12 de `implementation_plan.md` §4 (trabajos compartidos por ficheros, herramientas de Tripo con una sola definición, biblioteca configurable, Three.js incluido). D1–D8 se tomaron sin el usuario (ADR-V010-001 en `memory.md`), para revisar.
* **Próximo paso**: aprobación del plan de la v0.2.0 (`implementation_plan.md` §5) y después S18/S19.

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
- [ ] **S24 · Varias instancias** (REQ-07) — `state.connections`, selector, identidad por sesión, puerto libre.
- [x] **S25 · La IA ve el viewport** (REQ-13) — captura como imagen (D16).
- [ ] **S26 · Deshacer un turno** (REQ-40) — contadores, `undo_turn`, borrar importados con tarjeta.
- [ ] **S27 · World Labs: generación** (REQ-18) — cliente, trabajos, biblioteca «mundo», `generate_world`, `export_world_mesh`, saldo.
- [ ] **S28 · World Labs → Unreal** (REQ-18) — `import_world`, escala y suelo, cielo y splats según S18.
- [ ] **S28b · Splats de cualquier origen** (REQ-41, añadido por el usuario el 2026-10-26) — capa de adaptadores; conversión `.spz` → `.ply`; activación con diff/copia/revertir; sin adaptador, fichero preparado y paso manual explicado. Primer adaptador: el plugin de terceros que instale el usuario (P-09).
- [~] **S28c · Spike del plugin propio de splats** (P-10) — *hecho el 2026-10-26 sin Niagara (su grafo no se puede montar por Python): malla de quads + material por Python. Funciona con el splat real de World Labs como vista previa (ver memory.md). Pendiente: valoración del usuario en su viewport; siguientes mejoras: gaussianas anisótropas (covarianza proyectada en el material), más splats o LOD, y convertirlo en adaptador (`src-tauri/src/splats` + `scripts/spikes`).* Original: — `.ply` → texturas de datos en Rust, sistema Niagara + material creados por Python; medir calidad y rendimiento con un splat real de World Labs. Si convence, adaptador por defecto.
- [ ] **S29 · MCPs de terceros** (REQ-21) — Ajustes, stdio/HTTP, asistente y ACP, permisos.
- [x] ~~**S30 · Actualizador** (REQ-39)~~ — retirado por el usuario el 2026-10-26: Microsoft Store gestiona las actualizaciones en Windows.
- [ ] `/test` · `/code-simplify` · `/ship` v0.2.0.

## Próxima versión — hallazgos de la prueba de la v0.1.0

- [x] «Cómo instalarlo» no abría el navegador: `opener.openUrl` en vez de `shell.open` (2026-10-25).
- [x] Gemini CLI: flag `--acp` y avisos de que solo sirve con accesos de pago desde el 2026-06-18 (2026-10-25).
- [x] README (ES/EN): sección «Descárgalo e instálalo» por sistema operativo con los instaladores de la Release, avisos de SmartScreen y Gatekeeper, y canales «próximamente» (2026-10-25).
- [ ] **Canales de distribución** (anunciados como «próximamente» en el README): tap de Homebrew `davidbuenov/homebrew-dbv-unreal-ai` que se actualice solo en cada Release (como el de dbv-md-reader); Microsoft Store (MSIX firmado por la Store, ver `dbv-specs-ops/docs/MARKETPLACE_PUBLISHING.md`); ficha en Uptodown para Windows y macOS. Al publicarlos, cambiar la sección «Próximamente» del README.
- [ ] **Actualizaciones:** la app no se actualiza sola (`includeUpdaterJson: false`, sin plugin updater). Decidir si se añade el updater de Tauri para los instaladores de Releases o se delega en Store/Homebrew.
- [ ] **Antigravity CLI como quinto agente** (pendiente de `/spec` + `/plan`, requisito nuevo). Datos del 2026-10-25: el CLI es `agy` (instalador por script, no npm) y **no** tiene modo ACP (issue #31 de `google-antigravity/antigravity-cli`, abierta). El ACP oficial es un binario aparte de Google, `agy_acp_server`, publicado en el registro ACP como `antigravity-acp` (v1.4.3, por plataforma, con SHA-256; Linux lleva `--uid=`). Decidir: descargarlo y verificarlo desde la app o pedir su ruta; arranque en frío de ~16 s en Windows (`AGY_ACP_DEFAULT_MODEL` lo reduce); `session/load` no reproduce el historial.


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

> **Last update / Última actualización:** 2026-10-26
> **Exact point / Punto exacto:** `/spec` y `/plan` de la v0.2.0 terminados (`SPECIFICATIONS.md` v1.4, `implementation_plan.md` §5, ADR-V020-001, REJECTED-02..04). Sin código todavía.
> **Pending / Pendiente:** aprobación explícita del plan por el usuario; revisión de ADR-V020-001 (sobre todo D18: splats sin importación automática salvo spike); P-08 (clave de firma del actualizador, la genera el usuario).
> **Next step / Próximo paso:** con el plan aprobado, S18 si el usuario está delante (editor abierto y gasto de créditos); si no, empezar por S19 (memoria: núcleo), que no depende del editor.
