# 🏗 Arquitectura Técnica: DBV Unreal AI (`dbv-unreal-ai`)

> **Fase:** `/spec` → se valida y completa en `/plan`
> **Estado:** Borrador
> **Última Revisión:** 2026-10-26 (hechos y módulos de la v0.2)

---

## 🛠 Stack Tecnológico

| Capa | Tecnología | Justificación |
| --- | --- | --- |
| **App de escritorio** | Rust (stable) + Tauri v2, WebView nativo del SO | Binarios ligeros en Windows/macOS/Linux; misma base que dbv-tauri-starter y dbv-typst-editor |
| **Frontend** | HTML/CSS/JS vanilla sin bundler, ficheros envueltos en IIFE, `withGlobalTauri` | Patrón de la familia DBV (`NATIVE_DESKTOP_APPS.md` §3); sin dependencias de npm en tiempo de ejecución |
| **Subsistema de IA** | `src/ai/*` + `src-tauri/src/ai/*` de dbv-tauri-starter (providers, ACP, keyring, agentLoop, propuestas) | Ya probado en producción en dbv-typst-editor; se reutiliza y se amplía con el host MCP |
| **Conexión con Unreal** | Cliente MCP *Streamable HTTP* propio en Rust contra el plugin `ModelContextProtocol` de UE 5.8 | El servidor de Epic es HTTP en `127.0.0.1:<puerto>/mcp`; sin dependencias pesadas |
| **Toolset en Unreal** | Plugin de proyecto con Python (`toolset_registry`) — alternativa C++ (`UToolsetDefinition`) | Sin compilar; válido en proyectos solo Blueprint (pendiente del spike R-02) |
| **Generación 3D** | Clientes HTTP de Tripo v3 y World Labs Marble en Rust, con proveedores simulados | Clave en el llavero; nunca cruza al frontend; tests sin gastar créditos |
| **Persistencia** | Ficheros JSON en la carpeta de datos de la app (conexiones, historial, biblioteca) | Volumen pequeño; sin servidor de base de datos. Se revisa SQLite en `/plan` si la biblioteca crece |
| **Testing** | `cargo test` (Rust), Vitest + jsdom (frontend), E2E con el editor real en local | Igual que el starter (`npm test`, `npm run test:ai`, `npm run test:e2e`) |
| **CI/CD** | GitHub Actions multiplataforma (`NATIVE_APPS_RELEASE_CI.md`) | Se concreta en `/plan` |

---

## 📂 Estructura de Directorios (propuesta)

```text
/
├── src/                      # Frontend (IIFE, sin bundler)
│   ├── ai/                   # Heredado del starter: conexión, chat, ACP, agentLoop, propuestas
│   ├── unreal/               # Vista de proyecto, preparación, píldora de conexión
│   ├── generation/           # Formularios de Tripo / World Labs, tarjetas de coste y progreso
│   ├── library/              # Biblioteca de assets
│   └── i18n.js, main.js, index.html, styles/
├── src-tauri/src/
│   ├── ai/                   # Heredado del starter + host MCP (traducción de herramientas)
│   ├── unreal/               # discovery.rs (instalaciones, editores), mcp_client.rs, toolsets.rs, prepare.rs
│   ├── generation/           # tripo.rs, worldlabs.rs, mock.rs (patrón puertos/adaptadores de Wordsmith)
│   ├── library/              # catálogo, descargas, miniaturas
│   └── mcp_server.rs         # modo --mcp por stdio (v0.2, REQ-20)
├── unreal-plugin/DBVUnrealAI/ # Toolset que instala la preparación (REQ-06)
│   ├── DBVUnrealAI.uplugin
│   └── Content/Python/       # init_unreal.py + dbv_toolset/*.py
├── tests/                    # Vitest (frontend) y E2E
└── dbv-specs-ops/            # SDD
```

---

## 🔬 Hechos verificados sobre UE 5.8 (2026-10-24)

Comprobados leyendo el código fuente del motor instalado (`UE_5.8`, `5.8.2-56702186`) y el proyecto DBVMCPFirst:

| Hecho | Fuente | Consecuencia |
| --- | --- | --- |
| El plugin `ModelContextProtocol` («Unreal MCP») es **Experimental**, `EnabledByDefault: false`, depende de `ToolsetRegistry` y `EngineAssetDefinitions` | `ModelContextProtocol.uplugin` | La preparación tiene que activarlo en el `.uproject` (REQ-05) |
| Ajustes en `UModelContextProtocolSettings` (`config=EditorPerProjectUserSettings`): `ServerUrlPath="/mcp"`, `ServerPortNumber=8000`, **`bAutoStartServer=false`**, `bEnableToolSearch=true` | `ModelContextProtocolSettings.h` | Sin preparar, el servidor no arranca solo; los ajustes son por usuario, lo que encaja con un puerto por proyecto (REQ-07) |
| Con `bEnableToolSearch`, `tools/list` solo devuelve `list_toolsets`, `describe_toolset` y `call_tool` | `ModelContextProtocolSettings.h` | El host MCP de la app debe hablar este protocolo de descubrimiento (REQ-10) |
| Comandos de consola: `ModelContextProtocol.StartServer [puerto]`, `StopServer`, `RefreshTools`, `GenerateClientConfig <ClaudeCode\|Cursor\|VSCode\|Gemini\|Codex\|All>` | `ModelContextProtocolModule.cpp`, `...EngineModule.cpp` | El propio motor genera `.mcp.json`, `.cursor/mcp.json`, `.vscode/mcp.json`, `.gemini/settings.json` y `.codex/config.toml` (así aparecieron en DBVMCPFirst) |
| ~25 plugins de toolsets de Epic en `Plugins/Experimental/Toolsets` (Editor, Plugin, ConfigSettings, LiveCoding, PCG, Niagara, MetaHuman, StateTree, UMG, SemanticSearch…) | árbol del motor | Se aprovechan en lugar de reimplementar |
| `EditorToolset` trae **16 toolsets en Python** (`editor_toolset/toolsets/*.py`): `ActorTools`, `AssetTools`, `BlueprintTools` (+ DSL de grafos), `MaterialTools`, `MaterialInstanceTools`, `SceneTools`, `StaticMeshTools`, `SkeletalMeshTools`, `TextureTools`, `ObjectTools`… y C++ `EditorAppToolset` (`CaptureViewport`, selección, cámara, PIE) y `LogsToolset` | `EditorToolset/Content/Python`, sesión en vivo | **Epic ya importa** (`StaticMeshTools.import_file`, `SkeletalMeshTools.import_file`, `TextureTools.import_file`, vía `AssetImportTask`/Interchange) **y coloca** (`SceneTools.add_to_scene_from_asset`, `find_actors`, `set_actor_folder`). El toolset propio no lo reimplementa (REQ-06) |
| `ProgrammaticToolset.execute_tool_script` ejecuta scripts **en un sandbox**: solo módulos de una lista blanca (`json`, `math`, `re`…), para encadenar herramientas; no da acceso libre a `unreal` | `toolsets/programmatic.py` | **Lo único que falta en Epic es Python sin restricciones** → `RunPython` propio |
| Los toolsets **se pueden escribir en Python** (`unreal.ToolsetDefinition` + `@toolset_registry.tool_call`) y Epic registra los suyos así desde `init_unreal.py` (`toolsets._registration.register()`); también *Agent Skills* (`@toolset_registry.agent_skill`; Epic trae `blueprint_basics`, `material_basics`, `default_outdoor_lighting`…) | `EditorToolset/Content/Python/init_unreal.py`; sesión en vivo: los 16 aparecen en `list_toolsets` | **Riesgo R-02 casi descartado**: la variante Python del toolset es viable; queda verificar un plugin *de proyecto* de solo contenido |
| Un toolset C++ propio **no se descubre solo**: hay que registrarlo con `UToolsetRegistry::RegisterToolsetClass(...)` en `FCoreDelegates::OnPostEngineInit` (nunca en `StartupModule`), desde el módulo del proyecto | `DBVMCPFirst.cpp`; lección de MCPCLiente (`EventoUnrealMCP`) | La variante C++ (REQ-06) debe incluir el módulo con este registro |
| DBVMCPFirst: `UMyCustomSceneToolset : UToolsetDefinition` con `UFUNCTION(meta=(AICallable))` `RunPython` (`IPythonScriptPlugin::ExecPythonCommandEx`, modo `ExecuteFile`, captura de stdout/stderr), `GetActors`, `CountActorsWithMesh` | `Source/DBVMCPFirst/MyCustomSceneToolset.*` | Referencia de la variante C++ |
| «Auto Start Server» es una **preferencia del editor** (por usuario y máquina), no un ajuste del `.uproject`; alternativa: `ModelContextProtocol.StartServer` en la consola | lección de MCPCLiente; `config=EditorPerProjectUserSettings` | La preparación escribe la preferencia del usuario, no un fichero versionado |

### Verificación en vivo contra el editor (2026-10-24, DBVMCPFirst, nivel `/Game/DBV/Maps/dbv-unreal-ai`)

Llamadas de solo lectura por HTTP desde la máquina de desarrollo:

- `initialize` → `protocolVersion: "2025-06-18"`, capacidades `tools.listChanged` y `resources`; la sesión llega en la cabecera **`Mcp-Session-Id`** y hay que reenviarla. `notifications/initialized` → HTTP 202.
- Las respuestas son `application/json` (no SSE) y el JSON viene **formateado en varias líneas**.
- `tools/list` → solo `list_toolsets`, `describe_toolset`, `call_tool`. Argumentos reales: `describe_toolset {toolset_name}`, `call_tool {toolset_name, tool_name, arguments}`.
- `list_toolsets` devuelve **texto plano** (`- Nombre: descripción`), no JSON: 22 toolsets activos en este proyecto. `describe_toolset` devuelve JSON **dentro** de un bloque de texto, con `inputSchema` y `outputSchema` por herramienta.
- Los resultados de los toolsets C++ llegan **doblemente codificados**: `{"returnValue": "<JSON en una cadena>"}`. El adaptador desenvuelve los niveles.
- `GetActors` respondió en **0,7 s**. `RunPython` (código de solo lectura) devolvió `UE 5.8.2-56702186`, Python **3.11.8** y, en `stderr`, un **`DeprecationWarning`** por `EditorLevelLibrary.get_editor_world` → la IA puede usar APIs obsoletas; motiva REQ-26.
- MCPCLiente ya tiene en caché (`.mcp_tools_cache.json`, con `toolsets_hash`) las **282 herramientas** aplanadas de este mismo proyecto: base para el editor simulado de los tests.
| `MCPClientToolset`: «adaptador para que los clientes del ToolsetRegistry (como el asistente del editor) se conecten a servidores MCP locales/privados» | `MCPClientToolset.uplugin` | El MCP propio (REQ-20) también podría usarse desde dentro del editor |
| `unreal.ToolsetLibrary.get_active_undo_count()` y `undo_transaction(can_redo)` están expuestos a Python; los **títulos** del historial no (`UTransBuffer::GetTransaction` es solo C++). Epic los usa para deshacer un script fallido (`_TransactionScope`). El búfer descarta transacciones viejas al llenarse (2026-10-26) | `ToolsetRegistry/Public/ToolsetRegistry/ToolsetLibrary.h:129,144`; `EditorToolset/.../programmatic.py:764`; `TransBuffer.h:96` | «Deshacer este turno» (REQ-40) por contador (D15, REJECTED-02) |
| `EditorAppToolset.CaptureViewport` admite una pose y anotaciones (rejilla y etiquetas de actores para un agente con visión) pero no publica `outputSchema` | caché de herramientas de MCPCLiente | Verificar en vivo si devuelve ruta o imagen (S18) antes de REQ-13 |

### Módulos de la v0.2 (plan del 2026-10-26)

- `src-tauri/src/project_memory/` — memoria y tareas del proyecto (`dbv-unreal-ai/` junto al `.uproject`), con las herramientas definidas una vez para el asistente y el MCP propio (D13, D19).
- `src-tauri/src/generation/worldlabs.rs` — cliente de World Labs sobre el núcleo de trabajos (D18).
- `src-tauri/src/ai/mcp_hub.rs` — clientes MCP de terceros por stdio y HTTP (D20).
- `src/app/planView.js` — vista Plan. Toolset DBV 0.3.0: `check_task`, `undo_turn`, `import_world`.
- `tauri-plugin-updater` + `tauri-plugin-process` (D21).

---

## 🔑 Decisiones Técnicas Clave (borrador)

### Seguridad

- **Credenciales:** llavero del SO (`keyring`) para claves de IA, Tripo y World Labs; el frontend solo ve `hasKey`.
- **Acciones en el editor:** clasificación lectura/modificación con aprobación (REQ-12); `run_python` siempre muestra el código.
- **Red:** solo `127.0.0.1` hacia Unreal; el MCP propio solo por stdio.

### Estilo de Código

- Estándares de `MASTER_PROMPT.md` (`<coding_standards>`): un solo `return` con guard clauses, `Result<T, String>` en todo comando IPC, `#![deny(clippy::unwrap_used, clippy::expect_used)]`, IIFE en el frontend.

### Gestión de Estado

- Estado de conexión con Unreal y sesiones de IA en Rust (`tauri::State` con `Mutex`); estado de la vista en el frontend; persistencia en JSON en la carpeta de datos de la app.

---

## 🔗 Integraciones Externas

| Servicio | Propósito | Notas / Límites |
| --- | --- | --- |
| Unreal Engine 5.8 MCP | Controlar el editor | HTTP local sin autenticación; experimental |
| Tripo API v3 (`openapi.tripo3d.ai/v3`) | Texto/imagen → modelo, rig, animación | ≈30 créditos por modelo, rig 30, animación ≈20; la URL del GLB caduca a los 5 min; `face_limit` es orientativo |
| World Labs Marble (`api.worldlabs.ai/marble/v1`) | Texto/imagen → mundo (pano, malla, splats) | Operación asíncrona (sondeo cada ~4 s, hasta ~12 min); cabecera `WLT-Api-Key`. Documentación del 2026-10-26: modelos `marble-1.0-draft` (230 créditos con texto), `marble-1.0`/`marble-1.1` (1.580), `marble-1.1-plus` (1.580–3.080); 1.250 créditos = 1 $; malla HQ por `:export` = 3.500; PLY gratis; `assets.mesh.{collider_mesh_url, full_res_mesh_url, hq_mesh_url}`, `assets.splats.spz_urls` (claves sin documentar), `semantics_metadata.{metric_scale_factor, ground_plane_offset}`; saldo en `GET /credits`; imágenes en `data_base64` |
| Ollama / LM Studio / nube / ACP | Modelos de IA | Ver `AI_DESKTOP_ARCHITECTURE.md` |

---

## ⚠️ Restricciones y Riesgos Técnicos

Ver `SPECIFICATIONS.md` §6 (R-01 a R-11).

---

## 🤖 Agent Harness (Arnés del Agente que desarrolla)

### 1. Gestión de Contexto

- **Estático:** `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` → `dbv-specs-ops/docs/MASTER_PROMPT.md`, `memory.md`, `task.md`.
- **Dinámico:** comandos de fase en `.claude/commands/`.

### 2. Herramientas y MCP

- **Durante el desarrollo:** el MCP de Unreal de un proyecto de pruebas abierto (el usuario lo activa cuando haga falta) para verificar el toolset y los flujos reales.

### 3. Entorno de Ejecución

- Sin claves reales en CI: proveedores simulados para Tripo/World Labs y un editor simulado (respuestas grabadas) para el MCP de Unreal.

### 4. Guardrails Deterministas

- `dbv-specs-ops/scripts/dbv-lint.py --changed` en `/code-simplify`; escaneo de secretos antes de commit (se decide en `/plan`).

---

## 🧩 Subsistema de IA Integrada en Cliente (Desktop 3-Tier Engine)

| Nivel | Qué conecta | Credenciales / coste |
| --- | --- | --- |
| **1. Modelos locales** | Ollama (`/api/chat` nativo con `num_ctx`), LM Studio, llama.cpp | Ninguna |
| **2. Nube con API key** | Anthropic, OpenAI, Google Gemini, OpenRouter, compatibles OpenAI | Llavero del SO |
| **3. Agentes (ACP)** | Claude Code, Codex CLI, Gemini CLI, Copilot CLI | Suscripción del usuario |

- **Decisión en este proyecto:** los tres niveles desde la v0.1. A diferencia de la plantilla, la IA es el núcleo del producto (RNF-IA.1 adaptado). Novedad respecto a dbv-typst-editor: el bucle de herramientas de los niveles 1 y 2 hace de **host MCP** (herramientas de Unreal por *tool search* y herramientas propias de la app); en el nivel 3 los servidores MCP se pasan al agente en `session/new`.

---

**Instrucción para la IA:** Respeta las decisiones y configuraciones del arnés documentadas aquí. Si necesitas desviarte por un motivo técnico o sugerir una nueva herramienta MCP/Skill para el proyecto, regístralo como "Decisión Técnica" en `memory.md` y obtén la aprobación del desarrollador.
