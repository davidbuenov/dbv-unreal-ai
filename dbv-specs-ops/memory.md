# 🧠 Memory & Context

> **Frontera de uso (Memory vs. Tasks):**
> - `task.md` → progreso **operativo**: checklist de tareas, Snapshot de Contexto (el paso exacto siguiente), y estado de la sesión.
> - `memory.md` → contexto **cualitativo y temático**: conocimiento persistente, decisiones técnicas profundas, lecciones, y el área del producto en foco (no el paso específico).
> Si hay info que sirva para los dos, prioriza: datos con fecha/paso exacto → `task.md`; razonamiento/por-qué/lecciones → `memory.md`.
>
> *Instrucción para la IA: Consulta este archivo al inicio de cada sesión para recuperar el hilo técnico. Actualiza las secciones correspondientes cuando el workflow lo indique (triggers en `/plan`, `/build`, `/test` y gate en `/ship`).*

## 🎯 Contexto Activo

- **Estado actual del desarrollo:** Bootstrap hecho (2026-10-24). `/spec` v1.1 validado (`SPECIFICATIONS.md` REQ-01..24, RNF-01..08, catálogo RNF-IA/RF-IA adaptado; `DESIGN.md`; `ARCHITECTURE.md` en borrador). Aún no hay código: la base será el código de dbv-tauri-starter.
- **Foco inmediato:** `/plan` de la v0.1, empezando por un spike del toolset en Python sobre un editor real (R-02).

## 🏗️ Log de Decisiones Técnicas (ADR Ligero)
*Registro de por qué se tomaron ciertas rutas (ej. cambios en librerías, arquitectura o patrones). Formato: `- **AAAA-MM-DD - Título:** decisión y motivo.`*

- **2026-10-24 - Alcance y forma (respuestas del usuario a P-01..P-04):** v0.1 = MVP + **MCP propio** (REQ-20 sube de v0.2 a v0.1); toolset en **dos variantes** con un único contrato (Python por defecto, C++ opcional); splats de World Labs mediante **plugin de terceros** sin redistribuirlo (v0.2).
- **2026-10-24 - Identidad:** nombre visible **DBV Unreal AI** (se descartó «DBV Unreal Copilot» por la marca de Microsoft); identificador técnico `dbv-unreal-ai`. Autor David Bueno Vallejo, coautor David Bueno Carmona (ambos en el copyright de `LICENSE` y de las cabeceras).
- **2026-10-24 - Bootstrap:** MIT, git, documentación en español con espejo en inglés (`README.en.md`, `CHANGELOG.en.md`). Stack: Tauri v2 + Vanilla JS sin bundler partiendo del código de dbv-tauri-starter, conservando dbv-specs-ops v3.1.0 (el starter trae la 2.8.0).
- **2026-10-24 - La IA es el núcleo, no un añadido:** `RNF-IA.1` (cero huella) se adapta: la app debe funcionar sin IA para detectar, preparar, ver la biblioteca y enviar a Unreal, pero la conversación es la pantalla principal.
- **2026-10-24 - «El estado del editor es el juez»:** equivalente a «la compilación es el juez» de dbv-typst-editor. Las evals verifican el resultado consultando la escena o el registro de assets, sin modelo juez (`SPECIFICATIONS.md` §8).

## ⚠️ Lecciones Aprendidas / Errores Evitados
*Notas sobre bugs específicos, configuraciones que fallaron o refactors intentados para no repetirlos.*

- **2026-10-24 — Hechos de UE 5.8.2 que condicionan el diseño** (código del motor + verificación en vivo; detalle en `ARCHITECTURE.md`): el servidor MCP no arranca solo («Auto Start Server» es preferencia del editor, por usuario), escucha en `8000` + `/mcp`; con *tool search* solo expone `list_toolsets`/`describe_toolset`/`call_tool`; sesión por cabecera `Mcp-Session-Id`; `list_toolsets` responde texto plano y los toolsets C++ devuelven JSON doblemente codificado.
- **2026-10-24 — Error mío corregido el mismo día: buscar solo en cabeceras C++ no basta.** Afirmé que ningún toolset de Epic importaba assets; `EditorToolset` tiene 16 toolsets **en Python** (`Content/Python/editor_toolset/toolsets`) que importan (`import_file`) y colocan (`add_to_scene_from_asset`). Lo único que falta es Python sin restricciones (`execute_tool_script` es un sandbox). Al inventariar capacidades de un plugin de Unreal, mirar también `Content/Python` y, mejor aún, preguntar al editor en vivo con `list_toolsets`.
- **2026-10-24 — Un toolset C++ propio no se registra solo** (lección de MCPCLiente/`EventoUnrealMCP`): `UToolsetRegistry::RegisterToolsetClass` en `OnPostEngineInit` del módulo del proyecto. Los Python se registran desde `init_unreal.py`.
- **2026-10-24 — El protocolo MCP no puede obligar a un modelo a consultar la documentación antes de actuar** (MCPCLiente). Como esta app es el host, sí puede: REQ-26 comprueba el Python antes de ejecutarlo. En vivo, `EditorLevelLibrary.get_editor_world` dio `DeprecationWarning`: la IA tiende a usar APIs obsoletas.
- **2026-10-24 — Convención del usuario:** todo lo creado por IA en Unreal va bajo `/Game/DBV/<TipoDeAsset>` y los actores relacionados en carpetas del Outliner (MCPCLiente, Paso 7 de su guía; el nivel de pruebas es `/Game/DBV/Maps/dbv-unreal-ai`).
- **2026-10-24 — Synology Drive no sincroniza ficheros ocultos** (`.git`, `.github/`, `.claude/`…): MCPCLiente perdió su `.git` en otro equipo. Tener remoto desde el principio.
- **2026-10-24 — Tripo:** usar la API **v3** (`openapi.tripo3d.ai/v3`), no la v2 de Wordsmith (incompatible). La URL del GLB caduca a los 5 minutos: descargar al momento. `face_limit` es orientativo. El rig `v1.0-20240301` da un esqueleto Mixamo limpio en bípedos; `v2.5-20260210` es para criaturas (fuente: memoria de Holo-Companion).
- **2026-10-24 — World Labs:** los Gaussian Splats no tienen triángulos; para colisiones y suelo hay que usar la malla de colisión que entrega World Labs (lección de Wordsmith).
- **2026-10-24 — Rutas de Windows dentro de un heredoc con Python rompen los escapes** (`\U` de `D:\Users…` dio `SyntaxError: unicodeescape`). Para texto con barras invertidas: la herramienta de edición o escritura directa (misma lección que en dbv-typst-editor).

## 🚫 Rejection Memory (Caminos Descartados)
*Enfoques probados o evaluados y descartados (una arquitectura que no superó el Architect Review, un arreglo o un refactor que hubo que revertir). La IA la lee antes de proponer un enfoque en `/plan`, `/test`, `/code-simplify` y `/maintain` para no repetirlos. Es evidencia, no una orden: si el contexto cambia, se reabre explicándolo. El número es el máximo + 1 y nunca se reutiliza; una entrada que deja de aplicar se tacha y se anota por qué. Formato: `- **REJECTED-NN** (AAAA-MM-DD) · Ámbito: <dónde aplica> — Intento: <qué se probó> → Motivo: <evidencia del fallo> → En su lugar: <alternativa>`.*

- **REJECTED-01** (2026-10-24) · Ámbito: integración con Tripo — Intento: reutilizar `httpTripoProvider.ts` de Wordsmith (API v2 `api.tripo3d.ai/v2/openapi`) → Motivo: Holo-Companion verificó que la v2 no es compatible con los modelos y endpoints actuales (v3) → En su lugar: cliente nuevo contra `openapi.tripo3d.ai/v3` con los parámetros verificados.

## 🗺️ Mapa de Relaciones
*Breve descripción de cómo interactúan los módulos actuales para ayudar a la IA a navegar el código.*

- **Proyectos de referencia (fuera de este repo, solo lectura):**
  - `D:\Programacion\github-davidbuenov\dbv-tauri-starter` — base de código (Tauri v2 + subsistema de IA genérico en `src/ai/` y `src-tauri/src/ai/`).
  - `D:\Programacion\github-davidbuenov\dbv-typst-editor` — IA en producción: providers, ACP con permisos y diff, agentLoop, evals, modo `--mcp` por stdio (`src-tauri/src/mcp.rs`); lecciones en su `memory.md`.
  - `D:\Programacion\Unreal\MCPCLiente` — **el proyecto con todo el aprendizaje sobre el MCP de Unreal** (no es un proyecto de UE): cliente web + `bridge.py` (aplanado de toolsets para Claude Desktop), `docs/UNREAL_MCP_INTEGRATION_GUIDE.md` (pasos verificados en un proyecto nuevo), `docs/UNREAL_MCP_NATIVE_CAPABILITIES_ANALYSIS.md`, skill `dbv-unreal-python-api` (API Python 5.8 + guías, buscador local, servidor MCP, crawlers con Playwright), `.mcp_tools_cache.json` (282 herramientas). Su `memory.md` tiene las lecciones.
  - `D:\Programacion\Unreal\DBVMCPFirst` — **el proyecto de UE 5.8 preparado** para ejecutar cualquier código por MCP: `.uproject` con los plugins, `MyCustomSceneToolset` (C++) registrado en `DBVMCPFirst.cpp`, configs de cliente generadas por el motor. Nivel de pruebas `/Game/DBV/Maps/dbv-unreal-ai`.
  - `D:\Programacion\Tripo\Wordsmith-main` — proveedores Tripo (v2) y World Labs con puertos/adaptadores, caché y mocks.
  - `D:\Programacion\Tripo\jupiterBueno2` (Holo-Companion) — Tripo v3 verificado, rigging y animación, scripts `generate-assets` y `rig-character`.
  - Motor: `D:\Program Files\Epic Games\UE_5.8` (5.8.2); plugins MCP en `Engine/Plugins/Experimental/{ModelContextProtocol,ToolsetRegistry,Toolsets}`.

---

## 🧹 Política de Mantenimiento

*Aplicar en cada `/ship` de tipo Major, o cuando este fichero supere las 200 líneas activas (Hot/Warm/Cold, ver `<context_management>` de `MASTER_PROMPT.md`):*

- **Consolida** decisiones relacionadas en una sola entrada.
- **Archiva** los ADRs antiguos y las lecciones ya internalizadas en el código: muévelos a `memory.archive.md` (créalo si no existe) y deja en este fichero una línea por ADR: `- (AAAA-MM-DD) <título>: decisión → motivo clave (detalle en memory.archive.md)`.
- **Elimina** entradas que describan decisiones revertidas o ya obsoletas.
- **Objetivo:** mantener `memory.md` por debajo de ~200 líneas activas para que la IA pueda leerlo íntegramente en cada sesión sin pérdida de atención.
