---
# Frontmatter de la fase vigente: ampliación de la v0.3.0 (§10). Los anteriores se conservan en el historial de git.
dependencies:
  - Rama v0.3.0 (commit 5dbcec6) con REQ-50..REQ-60 construidos y probados
  - UE 5.8.4 con D3D12 SM6; motor del Launcher (precompilado) para compilar el plugin C++ con UBT
  - APIs públicas del motor: FSceneViewExtensionBase + SubscribeToPostProcessingPass (SceneViewExtension.h), SortGPUBuffers (Engine/Public/GPUSort.h), shaders globales de plugin (AddShaderSourceDirectoryMapping)
  - Plugins experimentales de Epic LiveCodingToolset (CompileLiveCoding) y AutomationTestToolset (RunTestsByFilter), y UAgentSkill de ToolsetRegistry
  - Base de guías local en formato MCPCLiente (index_guides.json + guides/*.md) para REQ-25
risks:
  - "R-28: APIs internas del renderizador cambian entre versiones → un solo módulo de render, solo puntos de extensión públicos, captura de referencia por versión"
  - "R-29: un splat enorme agota la VRAM → presupuesto: primero se quitan los SH, después se seleccionan los más visibles; el informe lo dice"
  - "R-30: la IA edita código del proyecto → solo texto dentro del proyecto, diff revisado por fichero, copia y Revertir, dos reintentos"
  - "R-31: Live Coding no aplica todo → se detecta el caso y se ofrece compilar con el editor cerrado"
  - "R-32 (nuevo): ordenar 10 M de claves por fotograma y por vista es caro → solo los visibles, claves de 32 bits, una ordenación por vista y nube; se mide en S67"
  - "R-15: guías de Epic → se leen de una carpeta local del usuario; descargarlas queda tras P-13 (decisión del usuario)"
rollback_strategy: >
  Un commit por slice en la rama v0.3.0; se revierte el slice que falle. El render por GPU se construye detrás de
  la puerta del spike S61: si el spike no cumple, se vuelve al render por CPU mejorado (lector de más de 2 GB y
  selección de los más visibles, ya escritos) y REQ-61/REQ-62 pasan a otra versión con el motivo en la Rejection
  Memory. REQ-18/22/23/25/64 son independientes entre sí y del render: cada uno se puede retirar sin tocar los demás.
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

---

## 7. Plan v0.2.2 — Selección Dinámica de Modelo y Nivel de Razonamiento en Agentes CLI (REQ-47)

### S38 · Comando de consulta dinámica de modelos de agentes (`commands.rs`, `detect.rs`, `acp.rs`)
- Nuevo comando Tauri `ai_list_agent_models(agent_id: String) -> Result<Vec<String>, AiError>`.
- Introspección dinámica del CLI con timeout estricto (1.5 s).
- Catálogo para Antigravity CLI garantizando modelos modernos: `gemini-3.8-flash` (mínimo), `gemini-3.8-pro`, `gemini-3.8-flash-lite`.
- Catálogo para Claude Code (`claude-3-7-sonnet-latest`, `claude-3-5-sonnet-latest`, etc.), Copilot, Gemini CLI y Codex.
- Soporte en `Connection` y `AgentSpec` para `model` y `reasoning_effort` (`off`, `low`, `medium`, `high`).
- Inyección de flags `--model <model>` y parámetros de reasoning en `acp.rs`.

### S39 · Interfaz de configuración inline en modal de conexión (`connectWizard.js`, `ai.css`, `translations.js`)
- Botón de engranaje ⚙️ (`ai-agent-settings-btn`) en cada fila de agente conectado/activo.
- Despliegue de tarjeta de ajustes (`ai-agent-settings-panel`) con selectores de Modelo y Razonamiento.
- Guardado reactivo con `aiSaveConnection` y actualización en caliente de la conexión activa.
- Textos bilingües (ES/EN) en `translations.js`.


## 8. Plan v0.2.3 — Conexión a IAs común a todas las apps DBV (REQ-48, REQ-49)

Fuente: `dbv-tauri-starter` `template-v0.4.0` (ADR-036 del starter). Cada paso se verifica con `node scripts/sync-ai.mjs --check ../dbv-unreal-ai` desde el starter.

### S40 · Framework 3.1.0 → 3.2.0
- Plantillas `IA*.template.md`, `AI_DESKTOP_ARCHITECTURE.md`, `MASTER_PROMPT.md`, `UPGRADE_PROMPT.md` y `Framework Version` (manifiesto v3.2.0).

### S41 · Guía común (REQ-48)
- Mover lo propio de Unreal (MCP propio que recibe el agente, tarjeta de Tripo, permisos, imágenes, uso desde otros agentes) debajo de la marca `<!-- dbv-ia:fin-guia-comun` en `docs/IA.md` y `docs/IA.en.md`.
- `sync-ai.mjs --apply`: ficheros canónicos (detección, conexiones, ACP, asistente, `acpSession.js`, `backend.js`, `guide.js`, `markdown.js` y sus tests), `docs/IA*.md` y `src/ai/guideContent.js`.
- Pestaña *Guía de IAs* del Centro de Ayuda (`F1`): pinta la guía común con `renderGuide()` en vez de su texto propio.

### S42 · Puntos de integración que avisa `[REVISAR]`
- Traducciones nuevas (ES/EN): `ai.agentNoAcp`, `ai.agentSigningIn`, `ai.guideTitle`, `ai.copy` y las que pida el script.
- `aiApp.js`: `onAuthenticate` en `createAcpSession` (aviso de inicio de sesión).
- `ai.css`: estilos `.ai-guide` y ✕ de las fichas de contexto.

### S43 · Trazabilidad propia (R-24)
- Test propio de la app que cita REQ-09, REQ-11, REQ-20, REQ-47, REQ-48 y REQ-49 sobre lo que Unreal añade (servidores MCP que recibe el agente, guía con la sección de Unreal).

### S44 · Verificación y entrega
- Vitest, `cargo test`, `dbv-lint.py --changed` y prueba en la app real (Conectar una IA, guía, agentes).
- Versión 0.2.3 (package.json, package-lock.json, Cargo.toml, Cargo.lock, tauri.conf.json), CHANGELOG, notas de versión, README, web, `task.md`, `memory.md`; commit, push y tag `v0.2.3`.

## 9. Plan v0.3.0 — memoria con varias entradas, contexto acotado, biblioteca visible y Convai (REQ-50..REQ-57, RNF-08)

> **Modo:** Orquestador (más de 20 ficheros entre Rust y JS). **Aprobación:** el usuario aprobó el 2026-10-31 todas
> las mejoras del análisis y autorizó encadenar `/spec` → `/plan` → `/build` en su ausencia. Las decisiones D22–D30
> se tomaron sin él y quedan para revisar (ADR del 2026-10-31 en `memory.md`).

### Architect Review (resumen; el debate completo se imprimió en la conversación)

- **REQ-51:** si la prueba de consumo se mide contra respuestas inventadas, el 50 % no significa nada. Se graban del editor real (`DBVUnrealAITest1`, 2026-10-31): `list_toolsets`, `describe_toolset` de `EditorAppToolset` (23 KB), `NiagaraToolset_System` (306 KB) y de los dos toolsets de Convai. La línea base de la v0.2.4 se calcula con su mismo formateo (JSON entero), conservado como función de test.
- **REQ-53:** `%USERPROFILE%\Documents` no es la carpeta Documentos si OneDrive la redirige. Se usa `dirs::document_dir()`, que pregunta a Windows por la carpeta conocida. Sin carpeta Documentos (servicio, Linux mínimo), se mantiene `<datos>/library`.
- **REQ-53, concurrencia:** la ventana y un `--mcp` pueden arrancar a la vez y migrar los dos. `merge_from` ya toma el cerrojo de la biblioteca de destino y deduplica por `id`, y la marca `library_migrated` se escribe después. Una doble migración no duplica nada.
- **REQ-57:** con Python, un `ConvaiObjectComponent` añadido a una *instancia* de actor puede no guardarse con el nivel ni deshacerse. Spike de escritura al principio de S56 (`SubobjectDataSubsystem`); si falla, la zona se crea como `Note` con nombre y la IA lo explica (Convai no la verá), y se registra en REJECTED.
- **REQ-55, seguridad:** leer `DefaultEngine.ini` deja la clave en memoria. La función devuelve solo booleanos, no guarda el valor y un test con una clave falsa comprueba que no aparece en la salida ni en el log.

### Decisiones (sin el usuario, para revisar)

| # | Decisión | Motivo |
| --- | --- | --- |
| D22 | El formato compacto de `describe_toolset` y el tope se implementan en Rust (MCP propio) y en JS (asistente), con **un fichero de referencia común** (`tests/fixtures/compact/`) que deben producir los dos. | Una definición verificable sin añadir un comando IPC nuevo ni tocar el banco e2e. |
| D23 | El uso de herramientas se anota en `<datos>/tool-usage.jsonl` (una línea por llamada: hora, pid, proyecto, herramienta, bytes, recortado), que rota a los 512 KB. La ventana suma las líneas de su proyecto entre el inicio y el fin del turno (`tool_usage_since`). | Es el mismo patrón que los trabajos (estado compartido = ficheros, lección del 2026-10-24). |
| D24 | La biblioteca por defecto es `dirs::document_dir()/DBV Unreal AI/Library`, con el nombre fijo en inglés; en el código no hay más opción porque la ruta debe ser la misma para la ventana y el `--mcp`. | REQ-53; la memoria del proyecto también va en inglés. |
| D25 | Qué ficheros se envían (modelo, splat, colisión) lo decide `library::send_plan` en Rust, que usa el MCP. La ventana conserva `sendToUnreal.js`, y un test de paridad con las mismas entradas exige la misma elección. Se ajusta el texto de REQ-54. | Mover el envío de la ventana a Rust es un refactor arriesgado sin el editor delante. |
| D26 | El log de RNF-08 es un módulo propio (`logging.rs`, sin crates nuevos) que limpia todo lo que tenga forma de clave antes de escribir. | Superficie mínima. |
| D27 | Convai: lo que se ve en ficheros (plugin, motor, sesión, git) lo detecta Rust (`unreal/convai.rs`, comando `convai_status`); lo que necesita el editor (toolsets, guías) lo comprueba la vista con `listToolsets` y `ListSkills`. | El diagnóstico funciona con el editor cerrado. |
| D28 | Una zona es un actor `Note` (su `text` guarda la descripción) con etiqueta `DBV_Zone_<Nombre>`, tag `dbv_zone`, carpeta `DBV/Zones` y, si existe la clase, un `ConvaiObjectComponent` con `object_entry.name`, `description` y `acceptance_radius`. El código Python lo fija la app y los datos viajan en hexadecimal, como las comprobaciones de tareas (REQ-37). | Sin recompilar el toolset C++. `Note` lleva texto propio, así que las zonas se listan también sin Convai. |
| D29 | Aceptar la tarjeta de zonas es la aprobación (como el plan, REQ-36). Se ejecuta con el `run_python` del toolset, dentro de su transacción, y por eso «Deshacer este turno» lo revierte. | Coherente con REQ-12 y REQ-40. |
| D31 | *(Añadida en `/build`.)* Si Documentos está dentro de OneDrive, la biblioteca va a `<carpeta de usuario>/DBV Unreal AI/Library` (`paths::library_folder`). La creación y la migración se hacen en el arranque (`begin_session`, `mcp_server::run`), nunca en `config_from_args`, que llaman los tests. | Una prueba copió 1,9 GB a la carpeta de OneDrive del equipo de desarrollo. |
| D32 | *(Añadida a petición del usuario, 2026-10-31.)* Los personajes de Convai (nombre, Character ID, nota) se guardan en `<config>/convai-characters.json`, no en el llavero ni en el proyecto: el ID no es secreto, vale para cualquier proyecto y el `--mcp` lo lee igual que los ajustes de generación. | Como Tripo y World Labs en la interfaz, pero sin clave (REJECTED-07). |
| D30 | Las herramientas de `ConvaiToolset.*` siguen la lista blanca de D5 (todas son de modificación); su tarjeta de aprobación y el pie del turno añaden el aviso de R-27. | Fallo seguro. |

### Slices (un commit por slice; los pasos en `task.md`)

1. **S45 · Memoria con varias entradas (REQ-50)** — `project_memory/tools.rs` (esquema `entries`, informe por entrada, compatibilidad con `section`/`entry`); `memory.js` (`propose` en lista, tarjeta con filas, sin `alreadyProposed`); `app.js`; textos del prompt, del botón «Recordar» y de las traducciones; tests en Rust y Vitest.
2. **S46 · Tope y `describe_toolset` compacto (REQ-51.1–.3, .6)** — `unreal/compact.rs` (`RESULT_LIMIT`, `clip`, `compact_toolset`); caché de toolsets en `McpServer`; `unrealTools.js` con el mismo formato; ficheros de referencia y fixtures grabadas; descripciones y preámbulo.
3. **S47 · `unreal_scene_summary` en el MCP y capturas ligeras (REQ-51.4–.5)** — `mcp_server.rs`; `capture.rs` (`detail`); comando `unreal_capture_viewport` con `detail`; `unrealTools.js`.
4. **S48 · Medición visible (REQ-51.7)** — `tool_usage.rs` (anotar, rotar, sumar); comando `tool_usage_since`; contador al pie del turno (agente y asistente); simulador e2e.
5. **S49 · Prueba de consumo (REQ-51.8)** — test de Rust: guion fijo contra un editor HTTP simulado con las fixtures; línea base v0.2.4 contra v0.3.0, ≥ 50 % menos.
6. **S50 · Aviso de accesos duplicados (REQ-52)** — `unreal/duplicates.rs` (`.mcp.json`, `.cursor/mcp.json`, `.gemini/settings.json`, `.codex/config.toml`, bloque `Convai primer`); en el estado del proyecto; franja en la vista Proyecto; línea en el preámbulo del agente.
7. **S51 · Biblioteca en Documentos (REQ-53)** — `paths.rs` (`default_library_dir`); `GenerationSettings.library_migrated`; `library::ensure_ready` (crear + migrar) en el arranque de la ventana y del `--mcp`; «Abrir carpeta» crea si falta; nota de migración en Ajustes.
8. **S52 · Splats desde el MCP (REQ-54)** — `library::send_plan`; `mcp_server::send_to_unreal` con `load_gaussian_splat` y colisión; test de paridad con `sendToUnreal.js`.
9. **S53 · Log rotativo y diagnóstico (RNF-08)** — `logging.rs`; líneas en el arranque, importar, enviar y conectar; «Copiar diagnóstico» con rutas y las últimas 50 líneas.
10. **S54 · Convai: detección (REQ-55)** — `unreal/convai.rs` y `convai_status`; sección en la vista Proyecto con las comprobaciones en vivo.
11. **S55 · Convai en la conversación (REQ-56)** — bloque en `unrealPrompt` y en el preámbulo del agente; aviso en la tarjeta de aprobación y en Deshacer; sugerencia «Haz que este MetaHuman hable con Convai».
12. **S56 · Zonas del nivel (REQ-57)** — spike de escritura; `zones/` (especificaciones, código Python fijo, MCP y asistente); tarjeta de zonas; tests.
14. **S58 · Personajes de Convai en Ajustes (REQ-58)** — `convai_characters.rs` (cargar, validar, guardar de forma atómica, especificación de `convai_characters`), comandos `convai_characters_list`/`convai_characters_save`, herramienta en el MCP y en el asistente (paridad), tarjeta Convai en Ajustes, reglas de Convai con la lista de personajes.
15. **S59 · Tutorial de Convai en F1 (REQ-59)** — pestaña Convai (ES/EN) en `helpModal.js`, enlaces desde Ajustes y desde la vista Proyecto, `docs/IA*.md`.
13. **S57 · Textos y documentación** — traducciones ES/EN, ayuda F1 (Unreal), `docs/IA*.md` debajo de la marca (Convai, consumo), CHANGELOG ES/EN.

### Verificación

- `cargo test`, `npx vitest run`, tests de Python, `dbv-lint`, banco e2e (lección del 2026-10-29: en el `/test` de cada versión).
- En vivo contra `DBVUnrealAITest1`: describir Niagara por el MCP (< 12.000 caracteres), sección Convai en ✓, crear y deshacer tres zonas, enviar un `.ply` por el MCP.
- Lo que solo puede comprobar el usuario: un personaje Convai que habla en Play y va a una zona; la MSIX en un equipo limpio.

## 10. Plan de la ampliación de la v0.3.0 — splats por GPU y pendientes adelantados (REQ-61..REQ-64, REQ-18, REQ-22, REQ-23, REQ-25)

> **Modo:** Orquestador (C++ con shaders, Rust, JS, Python; decenas de ficheros).
> **Aprobación:** el usuario aprobó el plan el 2026-10-31 («adelante»), con un commit y la actualización del estado tras cada slice.

### Architect Review (resumen; el debate completo se imprimió en la conversación)

- **Evidencia:**
  - `DbvGaussianSplatSceneProxy.cpp:58`: tope de 262.144 y ordenación en la CPU en cada fotograma.
  - `DbvPlyReader.cpp`: antes leía con `LoadFileToArray` en un `TArray<uint8>`.
  - `Engine/Public/GPUSort.h:52`: `ENGINE_API int32 SortGPUBuffers(FRHICommandList&, FGPUSortBuffers, …)`.
  - `Engine/Public/SceneViewExtension.h:222`: `SubscribeToPostProcessingPass`.
  - `ToolsetRegistry/AgentSkill.cpp:35`: `GetDerivedClasses(UAgentSkill::StaticClass())`, así que basta una clase C++ derivada.
  - `LiveCodingToolset.h`: `static FString CompileLiveCoding()`.
  - `MCPCLiente/.../knowledge/index_guides.json`: 3.374 guías y 5.294 fragmentos con URL y palabras clave.
- **Adversario:** las **capturas de la app (REQ-13)** salen de `EditorAppToolset.CaptureViewport`. Si esa captura renderiza sin la pasada de tonemap, los splats compuestos tras el tonemap no aparecerían en lo que ve la IA. Además, **9.975.598 splats a 30 fps** obligan a ordenar unos 10 M de claves por vista, y el editor puede tener varias vistas abiertas.
- **Respuesta:**
  - El spike S61 solo se da por bueno si el splat de prueba se ve en el viewport, en PIE **y en una captura de la app**. Si la captura no pasa por el tonemap, esas vistas tendrán un camino que dibuja antes del postproceso.
  - Contra el coste: se ordenan solo los visibles (descarte por frustum y opacidad), con claves de 32 bits y la radix del motor. Se mide con `stat GPU` en S67; si no llega a 30 fps, se baja el presupuesto por defecto.
- **Confianza:**
  - Requisitos: **alta**.
  - Arquitectura del render: **media**. `SortGPUBuffers` dentro de RDG y la profundidad tras el tonemap no se habrán verificado hasta S61.
  - Verificación: **media**, porque necesita reinicios del editor con el usuario.
  - REQ-22: **media**, porque `CompileLiveCoding` es experimental.
  - REQ-25: **baja** para «Construir índice», que no se construye sin la decisión del usuario sobre P-13; **alta** para leer una carpeta local.

### Decisiones (para aprobar)

| # | Decisión | Motivo |
| --- | --- | --- |
| D33 | **Composición tras el tonemap.** Una extensión de vista (`FSceneViewExtensionBase`) suscrita a `EPostProcessingPass::Tonemap`. Los splats se mezclan sobre la imagen ya tonemapeada, en espacio de pantalla, que es donde se entrenan los 3DGS. La profundidad se prueba en el píxel contra `SceneDepth`, convirtiendo las UV de la resolución de render a la de salida. | El color sale igual que en la captura y no se toca la exposición del proyecto. XGrids, en cambio, pone `r.DefaultFeature.AutoExposure 0` para todo el proyecto (visto en el log). |
| D34 | **Datos en la GPU, subidos una sola vez:** posición (`float3`), covarianza 3D (6 × `half`), color base y opacidad (4 × `half`) y, si los hay, SH de grado 1 a 3 (45 × `half`). Son 32 o 122 B por splat; la iglesia ocupa unos 1,2 GB. | Cabe en 12 GB, y la covarianza se calcula al cargar, no en cada fotograma. |
| D35 | **Por vista y por nube, tres pasos:** (1) un compute descarta por frustum y opacidad y calcula la cónica 2D, el radio, el color con los SH según la dirección de vista y la clave de profundidad; (2) se ordena con `SortGPUBuffers` del motor (radix, clave y valor de 32 bits); (3) se dibujan quads instanciados que leen el índice ordenado. Si `SortGPUBuffers` no encaja en RDG, se usa una radix propia de 4 pasadas de 8 bits. | La ordenación del motor ya está probada con partículas y no hay que mantener una propia. |
| D36 | **Presupuesto de VRAM:** por defecto, el 40 % de la memoria de vídeo dedicada; se cambia con `dbv.Splat.VRAMBudgetMB`. Si un splat no cabe, primero se quitan los SH y después se eligen los más visibles (opacidad × área). | REQ-61 y REQ-62; R-29. |
| D37 | **En la CPU, una estructura de arrays empaquetada** (`FDbvSplatCloud`, un `TArray64` por campo, ya en el formato de la GPU). Los lectores la llenan directamente y se libera tras subirla. Desaparece `FDbvGaussianPoint`, que llevaba un `TArray` por punto (10 M de reservas de memoria). | Menos memoria y menos tiempo de carga. Corrige de raíz el `SetNumUninitialized`. |
| D38 | **Varias nubes y visibilidad:** la extensión lleva un registro en el hilo de render por escena (`FSceneInterface`). Cada proxy se da de alta al crear sus recursos y de baja al destruirlos. Se respetan la visibilidad del componente y «oculto en juego», y las nubes se dibujan de la más lejana a la más cercana. El camino por CPU desaparece. | Que funcione en el editor, en PIE y con varios niveles abiertos. |
| D39 | **Tests de C++:** tests de Automation en el propio módulo (`WITH_DEV_AUTOMATION_TESTS`) para los lectores (tipos, formatos rechazados, lectura por bloques con un tamaño pequeño), el empaquetado de SH y las claves de orden. Se lanzan por MCP con `AutomationTestToolset.RunTestsByFilter("DBV")`. **Antes de pedir un reinicio al usuario**, compruebo con UBT que compila, sobre un proyecto anfitrión temporal en el scratchpad con los dos plugins. | Solo se reinicia el editor del usuario con código que ya compila. |
| D40 | **Versiones:** `DBVUnrealAICpp` pasa de 0.2.0 a 0.3.0 y el toolset Python de 0.3.0 a 0.3.1, para que la vista Proyecto pida actualizar. El `.uplugin` del C++ pasa a `LoadingPhase` `PostConfigInit`, que lo exige el registro de shaders de un plugin. | `readiness` solo compara la versión del plugin Python (`prepare.rs:189`). |
| D41 | **Mundos (REQ-18):** `send_world` en el toolset Python (escala métrica, suelo en Z = 0, carpeta `DBV/Worlds/<nombre>`), elegido desde `send_plan` (Rust) y `sendToUnreal.js`, con test de paridad. El cielo necesita un spike: si el panorama se importa como `TextureCube` (lat-long), se usa un `HDRIBackdrop`, plugin del motor que «Preparar» activaría; si no, una esfera de cielo con material sin iluminación y un `SkyLight` que captura la escena. | No añade fuentes nuevas de terceros. |
| D42 | **Código del proyecto (REQ-22):** herramienta `propose_code_changes { changes: [{ path, edits: [{ old, new }] } o { path, content }], reason }`.<br>• Un módulo Rust `code_changes` valida (dentro del proyecto, rutas permitidas, nunca los plugins de la app, texto de 1 MB como máximo), calcula el diff con la crate `similar` (MIT/Apache), aplica con copia de seguridad (el mecanismo de `prepare`) y revierte.<br>• Compila con `CompileLiveCoding` por el MCP de Epic.<br>• Si cambian cabeceras con `UCLASS`, `USTRUCT` o `UENUM`, o un `Build.cs`, `.uplugin` o `.Target.cs`, avisa y ofrece compilar con el editor cerrado (UBT `Build.bat`).<br>• Como mucho, dos compilaciones fallidas seguidas por turno. | Las ediciones «antiguo → nuevo», como las de un agente, son fiables para un modelo y fáciles de revisar. |
| D43 | **Recetas (REQ-23):**<br>• Datos en `recipes/recipes.json`, incluido en el binario: textos ES/EN/ZH, qué necesita cada receta y el id de su comprobación.<br>• Herramienta `recipes_list` en las dos superficies y un menú en la conversación.<br>• La comprobación final se hace con Python fijo del toolset (`check_recipe`).<br>• Para los agentes, cuatro clases C++ `UAgentSkill` en `DBVUnrealAICpp`, que se descubren solas. En la variante solo Python se intenta una clase `@unreal.uclass`; si el registro no la descubre, no se publica. | Cada receta se define una sola vez. |
| D44 | **Guías (REQ-25):** `search_unreal_guides` lee una carpeta local en formato MCPCLiente (`index_guides.json` y `guides/*.md`), que se elige en Ajustes → Conocimiento. La búsqueda la hace Rust en memoria, puntuando título, palabras clave y texto. Devuelve como mucho 5 fragmentos con su URL, dentro del tope de REQ-51. **«Construir índice» desde la web no se implementa** hasta que el usuario decida P-13 tras leer los términos de Epic. | Nada de Epic se redistribuye ni se descarga sin una decisión humana. |
| D45 | **Chino (REQ-64):**<br>• Diccionarios `zh-CN` en `i18n.js` y en los dos `translations.js`.<br>• La ayuda (`helpModal.js`, que hoy usa ramas `isEs ? … : …`) pasa a una tabla por idioma, con el inglés como respaldo.<br>• Los idiomas `zh*` se detectan como `zh-CN`, y un test comprueba que tiene las mismas claves que `es`.<br>• La traducción la hace la IA y conviene que la revise un hablante. | El mismo patrón que `es` y `en`. |

### Slices (un commit por slice; los pasos en `task.md`)

**Bloque A — splats por GPU (va primero porque es lo más arriesgado):**
1. **S61 · Spike con puerta:**
   - Montaje: proyecto anfitrión en el scratchpad, compilado con UBT.
   - Qué se prueba: una extensión de vista y un shader global del plugin dibujan una nube de prueba fija, compuesta tras el tonemap y con la prueba de profundidad, y `SortGPUBuffers` se llama dentro de RDG.
   - Puerta: la nube se ve en el viewport, en PIE y en una captura de la app. Si no, se aplica el rollback.
2. **S62 · Nube empaquetada y lectores:** `FDbvSplatCloud`; PLY y SPZ con SH (`f_rest_*`); tests de Automation (D39).
3. **S63 · Recursos en la GPU y presupuesto:** subida única; presupuesto con SH quitados y selección de los más visibles; `sh_degree` y `drawn` en el informe.
4. **S64 · Proyección, ordenación y dibujo, con el color base.**
5. **S65 · Armónicos esféricos de grado 1 a 3** en la proyección.
6. **S66 · Varias nubes, PIE y capturas.** Se retira el camino por CPU, se suben las versiones (D40) y se completan el informe de Python (REQ-63) y sus tests.
7. **S67 · Verificación con el usuario:** iglesia con Victoria dentro, desierto, PIE, `stat GPU` y capturas comparadas.

**Bloque B — pendientes adelantados:**
8. **S68 · Mundos completos (REQ-18):** spike del cielo, `send_world`, plan de envío con su test de paridad y prueba en vivo con el desierto.
9. **S69 · Código del proyecto, núcleo (REQ-22):** `code_changes` en Rust (validar, diff, aplicar, revertir) y la herramienta en las dos superficies, con tests.
10. **S70 · Código del proyecto, ventana y compilación:** tarjeta de diff, Live Coding, caso «editor cerrado» y reintentos; prueba en vivo con `ADemoActor`.
11. **S71 · Recetas (REQ-23):** datos, `recipes_list`, menú y `check_recipe`; AgentSkills en C++.
12. **S72 · Guías sin red (REQ-25):** lector y búsqueda en Rust, herramienta en las dos superficies y pestaña Conocimiento (Ajustes pasa a ocho pestañas).
13. **S73 · Chino (REQ-64):** diccionarios, ayuda por idioma, detección y test de paridad.
14. **S74 · Documentación y cierre:** `docs/IA*.md`, ayuda de splats y CHANGELOG ES/EN; después, `/test` y `/code-simplify` de la ampliación.

## 11. Plan de implementación — DBV Unreal AI v0.4.0 (Audio & SFX, Visión, Autoría y Puertos)

### Decisiones de arquitectura v0.4.0

| # | Decisión | Motivo |
|---|---|---|
| D46 | **Generación de audio desacoplada (REQ-71):** En Rust (`src-tauri/src/generation/audio.rs`), un cliente unificado `AudioClient` gestiona los tres proveedores: (1) Local (`http://127.0.0.1:8765`), (2) ElevenLabs (`/v1/text-to-sound-effects`), (3) Freesound (`/apiv2/search/text/`). El audio generado/descargado se guarda como fichero `.wav` en la biblioteca del usuario. | Mantiene la aplicación ligera (~15 MB) y libre del peso de PyTorch (~4 GB), permitiendo que quien tenga GPU use `dbv-audio-service` en local y quien no use la nube. |
| D47 | **Biblioteca y visor de audio (`kind: "sound"`, REQ-71):** Los assets de sonido se almacenan en `library/` con extensión `.wav`. En el frontend (`src/app/libraryView.js`), la tarjeta muestra miniatura 🎵, duración, prompt de origen y un reproductor de audio HTML5 `<audio controls>` para preescucha instantánea. | Experiencia de usuario coherente con los modelos 3D y mundos de la biblioteca. |
| D48 | **Importación de Audio a Unreal (REQ-72):** (1) En la UI, el botón «Enviar a Unreal» de una tarjeta de sonido copia el `.wav` a `Content/DBV/Audio/` y llama al método nativo de importación de assets del motor (`unreal.AssetToolsHelpers.get_asset_tools().import_asset_tasks(...)`); (2) Para los agentes, se expone la herramienta MCP `unreal_import_sound`. | Integración directa con el motor para que los efectos de sonido estén listos para usarse en Sound Waves o Sound Cues. |
| D49 | **Visión multimodal en la conversación (REQ-73):** Se añade el botón `📷 Capturar viewport` junto a la caja de entrada de chat (`chatPanel.js`). Llama al comando existente `unreal_capture_viewport`, el backend guarda la imagen como adjunto clipboard/temporal (`ai_save_clipboard_image`) y se inyecta en la lista de `attachments`. Al enviar el mensaje, viaja tanto a modelos locales/cloud con visión como a agentes ACP. | Resuelve la deuda de `REQ-13` aprovechando el pipeline multimodal consolidado en `REQ-68`. |
| D50 | **Autoría configurable con huella inmutable (REQ-74):** Se añade un campo de texto en Ajustes → Proyecto: *«Titular de Copyright»* (`project_copyright`), guardado en `app_config_dir()`. En `unrealPrompt` (`unrealTools.js`), la cabecera se genera con ese autor en la línea de Copyright, pero manteniendo de forma inmutable las líneas 4 y 5 de atribución a `DBV Unreal AI` y `dbv-specs-ops`. | Respetar la propiedad intelectual del usuario sin perder la atribución del software libre y su herramienta de desarrollo. |
| D51 | **Autodetección del puerto MCP (REQ-75):** Al conectar (`unrealConnect`), si el puerto configurado en el `.ini` falla o devuelve 404, el backend sondea en paralelo los puertos `8000..8005`. Si encuentra un servidor MCP que responde a `initialize`, se actualiza el endpoint y se conecta automáticamente sin error. | Previene errores 404 por desincronización entre el puerto guardado y el puerto dinámico asignado por Unreal Engine al arrancar. |

### Rebanadas de implementación (Slices S80..S84)

1. **S80 · Generación de audio y catálogo de biblioteca (REQ-71):**
   - Backend Rust: módulo `src-tauri/src/generation/audio.rs`, comandos Tauri `generation_generate_sound`, `generation_search_freesound`, `generation_check_local_audio`.
   - Frontend: Pestaña «Sonidos» en *Generar* (`generateView.js`) con selector de proveedor (Local / ElevenLabs / Freesound), prompt, duración y bucle. Pestaña «Sonidos» en *Biblioteca* (`libraryView.js`) con reproductor integrado.
2. **S81 · «Enviar a Unreal» de Audio y herramienta MCP (REQ-72):**
   - Script Python en toolset DBV para importación de audio (`USoundWave` / `SoundCue`).
   - Herramienta MCP `unreal_import_sound` en `mcp_server.rs` y botón en `libraryView.js`.
3. **S82 · Visión multimodal en vivo: «Capturar viewport» (REQ-73):**
   - Botón `📷 Capturar viewport` en `src/ai/chatPanel.js` conectado con `unrealCaptureViewport` y el sistema de adjuntos con miniatura.
4. **S83 · Autoría de Copyright configurable con huella inmutable (REQ-74):**
   - Ajustes de proyecto (`settingsView.js`) con campo `projectCopyright`. Inyección en `unrealPrompt` manteniendo la huella DBV de 6 líneas.
5. **S84 · Autodetección y resiliencia de puertos MCP (REQ-75):**
   - Sondeo en `mcp_client.rs` / `commands.rs` en el rango `8000..8005` al fallar el puerto del `.ini`.

