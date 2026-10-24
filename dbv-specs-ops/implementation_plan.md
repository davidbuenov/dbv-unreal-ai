---
dependencies:
  - dbv-tauri-starter 0.3.0 (código base: Tauri v2, subsistema de IA de 3 niveles, ACP, keyring)
  - Rust stable 1.98 + crates ya presentes en el starter (tauri 2, serde, serde_json, ureq 3, keyring 4, tokio, walkdir)
  - Crates nuevos previstos: sysinfo (procesos del editor), tauri-plugin-dialog (elegir .uproject), dirs (rutas del launcher de Epic)
  - Unreal Engine 5.8.2 con los plugins ModelContextProtocol, ToolsetRegistry, PythonScriptPlugin, EditorToolset, AutomationTestToolset y SlateInspectorToolset
  - API de Tripo v3 (openapi.tripo3d.ai/v3) — sin llamadas reales en tests (proveedor simulado)
  - Node 24 + Vitest/jsdom (tests del frontend)
risks:
  - "R-01: plugin MCP Experimental → adaptador único (src-tauri/src/unreal/mcp_client.rs) y fixtures grabadas"
  - "R-02: toolset Python de proyecto → spike en vivo registrando el toolset en la sesión del editor y desregistrándolo"
  - "R-03: run_python destructivo → clasificación lectura/modificación por lista blanca, tarjeta con el código, validación previa (REQ-26)"
  - "R-05: hilo de juego bloqueado → llamadas MCP fuera del hilo principal de Tauri (spawn_blocking) con tiempo límite"
  - "R-06: créditos de Tripo → nunca en tests ni en CI; confirmación obligatoria en la app y parámetro confirm_cost en el MCP propio"
  - "Variante C++ del toolset sin compilar en esta sesión (requiere UBT y cerrar el editor del usuario)"
rollback_strategy: >
  Cada slice es un commit propio en master; revertir un slice es `git revert <commit>`. Nada de esta fase toca
  el proyecto de Unreal del usuario en disco: la verificación en vivo solo registra el toolset en la sesión del
  editor y lo desregistra al terminar (sin ficheros). La preparación (REQ-05) guarda copia de seguridad antes de
  escribir y su «Revertir» restaura byte a byte; se prueba sobre copias temporales, nunca sobre DBVMCPFirst.
---

# Plan de implementación — DBV Unreal AI v0.1

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
