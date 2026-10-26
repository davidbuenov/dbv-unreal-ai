---
# Frontmatter de la fase vigente: v0.2.0 (§5). El de la v0.1 se conserva en el historial de git (commit 0320758).
dependencies:
  - Código de la v0.1.0 (Tauri v2, subsistema de IA de 3 niveles, ACP, MCP propio, núcleo de trabajos D9, herramientas únicas D10)
  - Crates nuevos previstos: tauri-plugin-updater 2 y tauri-plugin-process 2 (REQ-39); ninguno más (el cliente MCP stdio y World Labs usan ureq/serde ya presentes)
  - Unreal Engine 5.8.2 con ToolsetRegistry (unreal.ToolsetLibrary.get_active_undo_count / undo_transaction, REQ-40) y EditorToolset (CaptureViewport, REQ-13)
  - API de World Labs Marble (api.worldlabs.ai/marble/v1, documentación del 2026-10-26) — sin llamadas reales en tests
  - Plugin de Gaussian Splatting de terceros (P-09): ninguno se redistribuye; candidatos XGrids LCC (lista UE 5.8 y SPZ) y NanoGS (PLY, 5.6+)
  - Clave de firma del actualizador generada por el usuario (P-08) y guardada en los secretos de GitHub
risks:
  - "R-16: memory.md ajeno o con secretos → dato y no orden, rechazo de entradas con forma de clave, toda escritura revisada"
  - "Dos procesos (ventana y --mcp) escriben la misma memoria → edición con hash base y escritura atómica: una edición sobre texto viejo falla, nunca pisa"
  - "R-18: historial de deshacer compartido → contador antes/después del turno; se niega si cambió"
  - "R-19: MCP de terceros por stdio = ejecutar un programa → comando visible y confirmado, todo de modificación por defecto"
  - "R-12/P-09: sin plugin de splats con importación por script verificada → los splats quedan en la biblioteca con guía; el resto del mundo se importa igual"
  - "R-17: clave del actualizador → sin secreto, el workflow compila sin artefactos de actualización en vez de fallar"
  - "REQ-07 cambia el estado central (state.connection) → slice propio temprano y getter compatible para no tocar todas las vistas"
  - "Créditos reales (Tripo, World Labs) → nunca en tests ni en CI; las pruebas reales solo con el usuario delante (S18)"
rollback_strategy: >
  Cada slice es un commit propio en master; revertir un slice es `git revert <commit>`. La memoria del proyecto
  solo escribe en `dbv-unreal-ai/` del proyecto (que el usuario controla con su git) y se prueba sobre proyectos
  temporales. El toolset sube de versión (0.3.0): la preparación actualiza y su «Revertir» vuelve a la anterior.
  El actualizador solo se activa con la clave pública configurada: sin ella, la app se comporta como la v0.1.
---

# Plan de implementación — DBV Unreal AI v0.1 → v0.2

> **Modo:** Orquestador (nueva aplicación, decenas de ficheros).
> **Aprobación:** el usuario autorizó el 2026-10-24 encadenar `/plan` → `/build` → `/test` → `/code-simplify`
> sin preguntas intermedias; las decisiones tomadas sin él están en `memory.md` (ADR «sin el usuario»).

## 1. Decisiones de este plan

| # | Decisión | Motivo |
| --- | --- | --- |
| D1 | **P-05 Distribución:** v0.1 compila en GitHub Actions para Windows, macOS (universal) y Linux (AppImage/deb) en cada tag, y crea un **Release de GitHub en borrador**. Tiendas (Microsoft Store, Homebrew) y auto-actualización, en v0.2. | Gate de app nativa del MASTER_PROMPT; la firma y las tiendas necesitan cuentas del usuario. |
| D2 | **P-06 Base de conocimiento:** la **referencia sintáctica** se obtiene **del motor en marcha** con dos herramientas de solo lectura del toolset DBV (`describe_python_api`, `validate_python`), exactas para la versión y el proyecto del usuario y sin redistribuir nada de Epic. Las **guías conceptuales** se buscan en la base local de MCPCLiente si el usuario indica su carpeta (`index_guides.json`); la app no incluye documentación de Epic. | R-15; mejor que el stub `unreal.py` porque no exige activar el «Developer Mode» ni reiniciar. |
| D3 | **REQ-26 en el editor:** el análisis del Python (AST + resolución de `unreal.X.y` + detección de obsoletos por docstring) corre **dentro de Unreal** como herramienta de solo lectura que recibe el código como **dato** y nunca lo ejecuta. Si el proyecto aún no tiene el toolset DBV (p. ej. solo el C++ de DBVMCPFirst), el mismo validador se envía por `RunPython` (es código fijo de la app). | Exactitud de versión; sin parser de Python en Rust ni JS. |
| D4 | **Bucle de herramientas:** al modelo se le ofrecen las **meta-herramientas** de Epic (`unreal_list_toolsets`, `unreal_describe_toolset`, `unreal_call_tool`) más atajos propios (`unreal_run_python`, `unreal_scene_summary`, generación, biblioteca, guías), no las 282 herramientas aplanadas. | Contexto pequeño (R-08); nombres válidos para Gemini. El aplanado se usa solo en el MCP propio para clientes que no entienden toolsets. |
| D5 | **Clasificación lectura/modificación (REQ-12):** por **lista blanca** de prefijos de nombre de herramienta (`get`, `list`, `find`, `describe`, `exists`, `is_`, `can_`, `capture`, `search`, `count`, `validate`, `read_file`…); **todo lo demás es modificación**. El servidor de Epic no publica `annotations`. | Fallo seguro: lo desconocido pide permiso. |
| D6 | **MCP propio (REQ-20):** `dbv-unreal-ai --mcp [--project <.uproject>]` por stdio. Las herramientas con coste exigen `confirm_cost: true` y respetan el tope de gasto guardado en la app; la confirmación humana la pone el cliente MCP (Claude Code pide permiso por herramienta). | Sin canal con la app abierta en v0.1 (superficie de ataque menor). |
| D7 | **Procesos del editor:** crate `sysinfo` (MIT) para leer los procesos `UnrealEditor*` y su línea de comandos (ruta del `.uproject`). | Multiplataforma sin invocar shells. |
| D8 | **Variante C++ del toolset:** se entrega como plantilla (`unreal-plugin/DBVUnrealAICpp/`) con el registro en `OnPostEngineInit`, **sin compilar** en esta sesión. | Compilar exige UBT y cerrar el editor del usuario; queda como verificación pendiente. |

## 2. Slices (cada uno ≤ ~50 líneas por paso en `task.md`; un commit por slice)

1. **S1 · Andamiaje** — copiar el código de dbv-tauri-starter, renombrar identidad (DBV Unreal AI, `com.davidbuenov.dbv-unreal-ai`, crate `dbv_unreal_ai`), cabeceras con el copyright de los dos autores, retirar la demo de notas; `cargo test` y Vitest en verde.
2. **S2 · Descubrimiento (REQ-01, REQ-03)** — `src-tauri/src/unreal/discovery.rs`: `.uproject` (EngineAssociation, módulos, plugins), instalaciones de Epic (`LauncherInstalled.dat`), procesos del editor, puerto MCP del proyecto, proyectos recientes de la app.
3. **S3 · Cliente MCP (REQ-02)** — `unreal/mcp_client.rs`: `initialize` + `Mcp-Session-Id`, `notifications/initialized`, `tools/list`, `tools/call`; JSON o SSE; `list_toolsets` en texto; `describe_toolset` JSON-en-texto; desenvolver `returnValue`; tiempo límite; reconexión. Tests con servidor HTTP simulado y fixtures; test en vivo `#[ignore]`.
4. **S4 · Toolset DBV en Python (REQ-06, REQ-25, REQ-26)** — `unreal-plugin/DBVUnrealAI/` (plugin de solo contenido): `run_python` (transacción, stdout/stderr), `validate_python`, `describe_python_api`, `get_project_info`, `get_scene_summary`. Tests Python puros (fuera de UE, con un `unreal` simulado) + **spike en vivo** (registrar, llamar, desregistrar).
5. **S5 · Diagnóstico y preparación (REQ-04, REQ-05)** — `unreal/prepare.rs`: comprobaciones, plan de cambios con diff, copia de seguridad, aplicar, revertir. Tests sobre proyectos temporales.
6. **S6 · Herramientas de Unreal en el bucle y permisos (REQ-10, REQ-12, REQ-13, REQ-26)** — `src/unreal/unrealTools.js`, `src/unreal/permissions.js`, tarjeta de aprobación; integración en `aiApp.js` (herramientas extra y prompt de sistema de Unreal).
7. **S7 · Tripo v3 y biblioteca (REQ-15, REQ-17, RNF-04)** — `generation/tripo.rs` (tarea, sondeo, descarga inmediata, coste), `library/mod.rs` (catálogo JSON, ficheros, miniatura), clave en el llavero.
8. **S8 · Enviar a Unreal (REQ-19)** — importación con `StaticMeshTools.import_file` en `/Game/DBV/Generated/<nombre>`, colocación con `SceneTools.add_to_scene_from_asset` delante de la cámara y `set_actor_folder` en `DBV/Generated`.
9. **S9 · MCP propio (REQ-20, REQ-11)** — modo `--mcp` por stdio con puente aplanado a Unreal, biblioteca, generación y guías; paso de servidores MCP a los agentes ACP en `session/new`.
10. **S10 · Interfaz (REQ-08, REQ-14, REQ-24, DESIGN.md)** — barra lateral (Proyecto · Conversación · Generar · Biblioteca · Ajustes), píldora de conexión, lista de preparación, registro de acciones, ES/EN.
11. **S11 · CI y documentación (D1)** — `.github/workflows/build.yml`, README/CHANGELOG, `docs/IA.md`.

## 3. Verificación

- `cargo test` (Rust), `npm run test:ai` (Vitest), tests Python del toolset (`python -I -m unittest`), `dbv-lint.py --changed`.
- En vivo contra el editor del usuario **solo lectura** + registro temporal del toolset DBV (se desregistra).
- Lo que NO se verifica en esta sesión (queda anotado): compilar la variante C++, la preparación sobre un proyecto real con reinicio del editor, una generación real de Tripo (créditos), macOS y Linux.

## 4. Ampliación v0.1.x (REQ-27..REQ-33, tras la primera prueba del usuario, 2026-10-24)

> **Aprobación:** el usuario pidió estas mejoras el 2026-10-24 («tenemos que mejorar mucho el mcp de tripo…»); se construyen por slices con un commit cada una y revisión posterior.

### Architect Review (resumen)

- **Evidencia:** conversación de Claude Code («esta herramienta no lo expone»), `mcp_server.rs` (solo `generate_3d_model`), `generation/commands.rs` (el gasto y los trabajos viven en memoria del proceso de la ventana), jupiterBueno2 `DatosTecnicos.md` §13.1 (rig y retarget verificados, costes medidos).
- **Adversario:** «*dos procesos (ventana y `--mcp`) comparten la biblioteca pero NO el estado de los trabajos ni el gasto*: añadir una consola y un tope de gasto sin resolverlo mostraría datos falsos». → **D9.**
- **Riesgo 2:** las variantes multiplican el coste → la confirmación y el tope usan el total (n × coste); si el total no cabe, no se lanza ninguna.
- **Riesgo 3:** imagen → modelo, rig y retarget no se pueden verificar sin gastar créditos del usuario → proveedor simulado en tests y **marcado «sin verificar» en `task.md`** hasta una prueba real con su confirmación.
- **Confianza:** alta en S12–S14 y S16 (mecánica conocida), media en S15 (visor sin bundler) y baja-media en el envío a Unreal de esqueletos y animaciones (S17): requiere probar con un GLB riggeado real.

| # | Decisión | Motivo |
| --- | --- | --- |
| D9 | **Núcleo de trabajos compartido:** `generation/jobs.rs` escribe un fichero por trabajo en `<datos>/jobs/<id>.json` (escritura atómica) con la petición enviada (sin clave), estado, progreso, `task_id`, créditos, entrada resultante y registro de eventos. Lo usan la ventana y el subproceso `--mcp`; la ventana los lee cada ~1,5 s. El gasto de la sesión es la suma de los créditos de los trabajos iniciados desde `session.json` (lo escribe la ventana al arrancar). | Un solo camino para consola, tarjeta viva en la conversación y tope de gasto, sin canal entre procesos. |
| D10 | **Lógica en Rust, dos superficies:** las herramientas se definen una vez y se exponen por el MCP propio (`mcp_server.rs`) y, para el asistente integrado, por comandos Tauri que `generationTools.js` solo adapta. Las descripciones dicen qué no hacen y cuál usar. | Evita que la IA integrada y el agente vean herramientas distintas (lo que provocó el hallazgo). |
| D11 | **Biblioteca:** ubicación en `generation.json` (`libraryDir`), el mismo fichero que lee `--mcp`; el índice sigue siendo `library.json` (compartible con otras apps). Importar = copiar al interior de la biblioteca, nunca referenciar fuera. | Que mover o borrar el original no rompa la entrada. |
| D12 | **Visor 3D:** Three.js (MIT) copiado en `src/vendor/three/` (módulo + `GLTFLoader` + `OrbitControls`) con un mapa de importación; sin red; carga perezosa. | Offline-first (RNF-01) y sin bundler. |

### Slices

12. **S12 · Trabajos, consola y progreso (REQ-27, REQ-28, REQ-29)** — `generation/jobs.rs` (D9), consola en Generar, tarjeta viva en la conversación para trabajos de agentes, variantes 1–4 con coste total, nombres (parámetro obligatorio, propuesta con la IA conectada o heurística).
13. **S13 · Biblioteca: ubicación e importación (REQ-30)** — `libraryDir` en Ajustes (con copia opcional), `library_import` (ficheros y carpetas), `library_rename`, abrir carpeta.
14. **S14 · Superficie completa de Tripo en el MCP (REQ-33, REQ-16)** — imagen → modelo, `animations/rig`, `animations/retarget`; herramientas `image_to_3d_model`, `rig_model`, `animate_model`, `generation_jobs`. Proveedor simulado.
15. **S15 · Visor 3D (REQ-31)** — D12; en Biblioteca y en la conversación antes de enviar a Unreal; miniatura propia.
16. **S16 · Referencias adjuntas (REQ-32)** — adjuntar imágenes y documentos en la conversación (multimodal si el modelo lo admite) y en Generar (imagen → modelo).
17. **S17 · Esqueletos y animaciones en Unreal (REQ-16, REQ-19)** — importar el GLB riggeado como Skeletal Mesh y los clips como Animation Sequence con ese esqueleto; spike con un modelo real.

## 5. v0.2.0 (SPECIFICATIONS.md v1.4, 2026-10-26)

> **Modo:** Orquestador (siete requisitos nuevos o concretados, decenas de ficheros, datos sensibles: claves de World Labs y de MCPs de terceros, firma del actualizador).
> **Aprobación:** el usuario acordó el alcance (P-07) y autorizó encadenar `/plan` tras `/spec` en su ausencia. **`/build` necesita su aprobación explícita de este plan**; las decisiones tomadas sin él están en `memory.md` (ADR-V020-001).

### Architect Review

- **Builder:** REQ-34..REQ-38 sobre un núcleo en Rust (`project_memory/`) con una sola definición de herramientas para el asistente y el MCP propio (como D10); REQ-18 sobre el núcleo de trabajos de Tripo (D9); REQ-40 y REQ-13 como herramientas del toolset DBV; REQ-07 cambiando el estado central; REQ-21 con un cliente MCP genérico; REQ-39 con el plugin oficial de Tauri.
- **Adversario 1 — `memory.md` con dos escritores:** la ventana y el subproceso `--mcp` de un agente pueden proponer a la vez, y el usuario puede editar el fichero a mano o hacer `git pull` entre la propuesta y el «Guardar». Escribir la propuesta tal cual pisaría lo nuevo. → **D13:** toda edición lleva el hash del texto sobre el que se calculó; si no coincide al aplicar, falla y se recalcula. Escritura atómica (temporal + renombrado).
- **Adversario 2 — «Revertir preparación» y la carpeta `dbv-unreal-ai/`:** `revert` borra cualquier fichero que no existía al preparar (`prepare.rs:376`). Si la memoria se creara en «Preparar», revertir borraría meses de decisiones. → Fuera de «Preparar» (ya en REQ-34) y un test que prepara, escribe la memoria y revierte. REJECTED-03.
- **Adversario 3 — «Deshacer este turno» y el historial de deshacer:** la spec original pedía comparar títulos de transacción, pero Python no los ve. `unreal.ToolsetLibrary` solo da `get_active_undo_count()` y `undo_transaction()` (`ToolsetRegistry/ToolsetLibrary.h:129,144`). Además, `UTransBuffer` descarta del principio las transacciones viejas cuando llena su memoria (`TransBuffer.h:96-121`) y el agente ACP modifica el editor desde otro proceso, que la ventana no ve. → **D15:** contador al empezar y al terminar el turno (válido para los tres niveles); se deshace solo si al pulsar sigue igual; si el búfer se recortó, el contador no cuadra y se niega. R-18 corregido en la spec. REJECTED-02.
- **Adversario 4 — `spz_urls` de World Labs y el plugin de splats:** la documentación no define las claves de `spz_urls`, y ningún plugin de splats tiene verificada una importación por Python en 5.8 (XGrids LCC lista 5.8 y SPZ, pero solo describe C++, Blueprint y consola; NanoGS dice 5.6+ y PLY). → **D18:** se guardan todas las URL de `spz_urls` tal cual vengan; la importación automática de splats queda tras un spike (S18) y, si falla, los splats se quedan en la biblioteca con una guía. REQ-18 se cumple igualmente (escala y colisión).
- **Adversario 5 — Un MCP de terceros y el tope de Gemini:** nombres `<servidor>__<herramienta>` con caracteres no válidos, o 200 herramientas de un servidor, desbordan el contexto de un modelo local (R-08). → **D20:** se normalizan los nombres (`[A-Za-z0-9_]`, ≤ 64) y el usuario activa servidores por proyecto. Por encima de 40 herramientas, se ofrecen por *tool search* (`<servidor>__list` / `__call`) como con Unreal.
- **Evidencia:** `SPECIFICATIONS.md` §3.7 (REQ-34 «Fuera de Preparar»), REQ-40, R-18; `src-tauri/src/unreal/prepare.rs:363-388`; `UE_5.8/Engine/Plugins/Experimental/ToolsetRegistry/Source/ToolsetRegistry/Public/ToolsetRegistry/ToolsetLibrary.h`; `EditorToolset/.../toolsets/programmatic.py:764-830` (Epic usa el mismo contador para deshacer); `docs.worldlabs.ai/api/reference/worlds/get.md` y `/api/pricing.md`; caché de MCPCLiente: `CaptureViewport` no publica `outputSchema`; `src-tauri/src/lib.rs:39` (el agente recibe `--mcp --port` pero no `--project`, imprescindible para REQ-38).
- **Resolución:** D13–D21 y los spikes en vivo de S18 antes de construir lo que depende del editor (captura, deshacer, importar un mundo).
- **Confianza:** `edge_cases="medium"` (splats y cielo con el panorama dependen de spikes), `security="medium"` (stdio de terceros y memoria versionada, mitigados), `concurrency="medium"` (dos escritores de memoria y dos editores a la vez, cubiertos con D13 y D17). Ningún eje en `low`.

### Decisiones

| # | Decisión | Motivo |
| --- | --- | --- |
| D13 | **Memoria en Rust (`src-tauri/src/project_memory/`):** lectura tolerante del formato (secciones por `## `, tareas por casilla), edición por búsqueda y sustitución o «añadir a la sección X» con **hash base**, escritura atómica, confinamiento a `<proyecto>/dbv-unreal-ai/{memory,task}.md` (ruta canónica, sin enlaces que salgan) y detector de secretos (`sk-`, `sk-ant-`, `tsk_`, `AIza`, `ghp_`, `Bearer `, `Authorization:`, bloques PEM). Resumen ≤ ~600 tokens calculado en Rust. Comandos IPC `memory_*`. | Un solo sitio que decide qué se escribe, para la ventana y para `--mcp`; los dos procesos comparten fichero, no memoria (lección de la v0.1). |
| D14 | **Comprobaciones de tareas (REQ-37):** el criterio se analiza en Rust (`done when: asset exists /Game/...`) y se ejecuta con **código fijo de la app** (función `check_task(kind, arg)` del toolset DBV 0.3.0; si el proyecto aún tiene un toolset antiguo, el mismo código se envía por `RunPython`, como D3). Nunca código de la IA. `no errors in log` usa `LogsToolset` de Epic si está; si no, «no se puede comprobar». | «El estado del editor es el juez» sin abrir una puerta a ejecutar Python arbitrario como criterio. |
| D15 | **Deshacer un turno:** al empezar y terminar cada turno que pudo modificar el editor, la app guarda `get_active_undo_count()`. «Deshacer» llama a `undo_turn(expected, n)` del toolset, que comprueba el contador y deshace `n` veces o no hace nada. Los assets importados durante el turno (resultados de `import_model`/`import_world`) se ofrecen para borrar con una tarjeta aparte. | Ver el adversario 3; mismo patrón que `_TransactionScope` de Epic. |
| D16 | **La IA ve el viewport:** `unreal_capture_viewport` llama a `EditorAppToolset.CaptureViewport` (S18 averigua si devuelve ruta o base64), reduce la imagen y la entrega: al asistente integrado como **mensaje de usuario con imagen** inmediatamente después del resultado de la herramienta (todos los proveedores con visión admiten imágenes de usuario; no todos en resultados de herramienta), y a los agentes como contenido `image` de MCP. Se ofrece solo si el modelo tiene visión (detección de REQ-32). | Un camino que vale para Anthropic, OpenAI, Gemini y Ollama sin depender de cada API. |
| D17 | **Varias instancias (REQ-07):** `state.connections` (mapa por `.uproject`) + `activeProject`; el getter `state.connection` sigue devolviendo la activa, así que las vistas no cambian. Comprobación de identidad (`get_project_info`) **al abrir cada sesión MCP y tras cada reconexión**, no en cada llamada: la sesión es por puerto y un cambio de editor la invalida. La preparación propone el primer puerto libre desde 8000 que no use otro proyecto conocido. El `--mcp` del agente recibe `--project`. | Cambio mínimo en las vistas; misma garantía que comprobar en cada llamada sin añadir una ida y vuelta (0,7 s) por herramienta. |
| D18 | **World Labs (`generation/worldlabs.rs`):** cliente sobre el núcleo de trabajos (D9); `marble-1.1` por defecto y `marble-1.0-draft` como «borrador»; tabla de precios del 2026-10-26 en un solo sitio (`catalog.rs`); descarga al terminar de miniatura, panorama, `collider_mesh_url`, `full_res_mesh_url`/`hq_mesh_url` si vienen y todas las `spz_urls`; `semantics_metadata` en la entrada. Operación aparte `export_world_mesh` (3.500 créditos) con su confirmación. Imagen de referencia en `data_base64` con la misma lista blanca y el mismo tope que Tripo. Clave en el llavero (`worldlabs`). **Envío a Unreal:** `import_world` en el toolset (colisionador por Interchange como `import_model`, escala `metric_scale_factor × 100` y `ground_plane_offset`, colisión compleja como simple). Cielo: spike en S18 (TextureCube desde el panorama long-lat o esfera con material sin luz); si ninguno funciona, el panorama entra como textura y se dice. Splats: ver adversario 4. | Reutiliza consola, tarjeta viva, tope de gasto y biblioteca; los precios reales evitan que la tarjeta de coste mienta (una malla HQ cuesta más que dos mundos). |
| D19 | **Vista Plan y planificar (REQ-36):** vista nueva `src/app/planView.js`; `project_tasks_propose` recibe tareas con `generation: [{tool, variants}]` y la app calcula el coste con la tabla de D18 y la de Tripo. Las acciones del usuario en el panel escriben directamente con hash base (D13). | El modelo no inventa precios; una sola tabla. |
| D20 | **MCPs de terceros (`src-tauri/src/ai/mcp_hub.rs`):** cliente stdio (JSON-RPC por línea, las reglas de stdio de REQ-20) y HTTP (generalizando `unreal/mcp_client.rs`). Configuración en `<config>/mcp_servers.json` (sin secretos); cabeceras y variables secretas en el llavero (`mcp:<servidor>:<nombre>`). Nombres normalizados; tool search por encima de 40 herramientas. A los agentes ACP se les pasan en `session/new` (stdio siempre; HTTP solo si el agente anuncia `mcpCapabilities.http`). | Ver el adversario 5; el usuario ve y confirma el comando (R-19). |
| D21 | **Actualizador (REQ-39):** `tauri-plugin-updater` + `tauri-plugin-process`; `createUpdaterArtifacts` y `includeUpdaterJson: true` en `release.yml` **solo si existe el secreto** `TAURI_SIGNING_PRIVATE_KEY` (paso que ajusta `tauri.conf.json` en CI); endpoint `https://github.com/davidbuenov/dbv-unreal-ai/releases/latest/download/latest.json` (solo apunta a Releases publicadas, nunca a borradores). Sin clave pública configurada, la app no comprueba. En Linux sin `APPIMAGE` (instalación `.deb`), solo avisa y enlaza. «Omitir esta versión» se guarda en los ajustes. | Gate de app nativa: la CI de la v0.1 (D1) no cambia salvo los artefactos firmados; la clave la genera el usuario (P-08). |

| D22 | **Splats de cualquier origen (REQ-41, añadido el 2026-10-26):** capa de **adaptadores** (`splat_adapters`): cada uno sabe detectarse en el proyecto, qué formato acepta y cómo importar y colocar. Primero el plugin de terceros que instale el usuario (P-09); en paralelo, spike de un plugin **propio de solo contenido sobre Niagara** (P-10: la app convierte el splat a texturas de datos y el toolset crea el sistema y el material por Python). Un rasterizador propio en C++ queda fuera de la v0.x. | Lo que importa al usuario es «arrastrar un splat y que funcione»; los adaptadores aíslan a la app de qué plugin lo pinta, y el propio quitaría la dependencia y el problema de licencias (R-12). |

**Gate de app nativa:** se mantiene D1 (Windows, macOS universal y Linux en GitHub Actions, Release en borrador que se publica a mano), con artefactos de actualización firmados desde que exista la clave (D21).

### Slices (un commit por slice; los pasos en `task.md`, ≤ ~50 líneas cada uno)

- **S18 · Verificación con el usuario y spikes en vivo** *(requiere al usuario: gasta créditos y necesita el editor abierto)* — Tripo real: texto → modelo, imagen → modelo, rig y una animación, comparando `credits_consumed` con la estimación; personajes en el motor (S17, `pumuki_anim_dance_01.glb`). Spikes: salida de `CaptureViewport`; `get_active_undo_count`/`undo_transaction` alrededor de `run_python`; importar un `collider_mesh` y un panorama de World Labs (un mundo `marble-1.0-draft`, 230 créditos, con su permiso); el plugin de splats elegido en un proyecto de prueba. Los hallazgos se anotan en `memory.md` antes de S25–S28.
- **S19 · Memoria: núcleo (REQ-34)** — D13 en Rust con tests sobre proyectos temporales: confinamiento, secretos, hash base, formato tolerante, resumen, y el test «preparar → escribir memoria → revertir → la memoria sigue».
- **S20 · Memoria en la conversación (REQ-34, REQ-35)** — «Activar memoria del proyecto» en Proyecto; ficha de contexto con el resumen; preámbulo de los agentes; herramientas `project_memory_read`/`project_memory_propose`; tarjeta «¿Guardar…?»; botón «Recordar»; `--project` en el `--mcp` del agente.
- **S21 · Vista Plan (REQ-36)** — D19: vista, edición en el panel, «Planificar» con `project_tasks_propose` y coste calculado, «Hacer esta tarea».
- **S22 · Tareas comprobadas (REQ-37)** — D14: criterios, `check_task` en el toolset 0.3.0 y alternativa por `RunPython`, estado en la vista; tests con el editor simulado.
- **S23 · Memoria en el MCP propio (REQ-38)** — las cinco herramientas con una sola definición y el test de paridad entre superficies.
- **S24 · Varias instancias (REQ-07)** — D17: estado, selector en la píldora, identidad por sesión, puerto libre al preparar.
- **S25 · La IA ve el viewport (REQ-13)** — D16, según lo que diga S18.
- **S26 · Deshacer un turno (REQ-40)** — D15: contadores por turno en los tres niveles, `undo_turn`, botón y tarjeta de borrado de importados.
- **S27 · World Labs: generación (REQ-18)** — D18: cliente, trabajos, biblioteca con entradas «mundo», `generate_world` y `export_world_mesh` en las dos superficies, pestaña en Generar, saldo, World Labs simulado.
- **S28 · World Labs → Unreal (REQ-18)** — `import_world`, escala y suelo, cielo y splats según S18; visor 3D del colisionador en la biblioteca.
- **S29 · MCPs de terceros (REQ-21)** — D20: Ajustes, cliente stdio/HTTP, asistente integrado y ACP, permisos.
- **S30 · Actualizador (REQ-39)** — D21: plugin, franja de aviso, Ajustes, CI condicionada al secreto; prueba con una Release de prueba cuando exista la clave.
- Después: `/test` (incluida la eval de memoria de §8), `/code-simplify` y `/ship` v0.2.0.

### Verificación

- `cargo test`, `npm run test:ai`, tests Python del toolset, banco E2E y `dbv-lint.py --changed`, como en la v0.1.
- Evals de §8 con `--prompt full`: «recuerda que…» → tarjeta y cero escrituras sin aceptar; una convención guardada se respeta en una conversación nueva; una tarea solo acaba hecha si su comprobación pasa.
- Sin el usuario no se puede verificar: gasto real (Tripo, World Labs), el editor abierto (S18, S25, S26, S28), la firma del actualizador (P-08), macOS y Linux reales.
