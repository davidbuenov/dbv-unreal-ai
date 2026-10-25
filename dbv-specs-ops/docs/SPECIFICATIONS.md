# 📋 Especificaciones: DBV Unreal AI (`dbv-unreal-ai`)

> **Fase:** `/spec` (Especificación)
> **Estado:** Validado para `/plan` (v1.3: añadidos REQ-27..REQ-33 tras la primera prueba del usuario; v1.2 del documento — P-01..P-04 resueltas por el usuario el 2026-10-24; incorporado lo aprendido en MCPCLiente y la verificación en vivo contra DBVMCPFirst; P-05 y P-06 se deciden en `/plan`)
> **Última Revisión:** 2026-10-24

---

## 🎯 1. Contexto y Objetivos

- **Problema:** Unreal Engine 5.8 trae un servidor MCP experimental (`ModelContextProtocol` + `ToolsetRegistry` + ~25 toolsets), pero usarlo hoy exige saber qué plugins activar, editar ficheros `.ini` a mano, arrancar el servidor desde la consola del editor (por defecto **no arranca solo**: `bAutoStartServer = false`), configurar cada cliente de IA por separado y, para cualquier cosa que no cubran los toolsets de Epic (ejecutar Python, importar un GLB, colocar actores), escribir clases propias como `UMyCustomSceneToolset` en DBVMCPFirst. Además, los servicios de generación 3D (Tripo, World Labs) viven fuera del editor: se genera en una web, se descarga, se importa a mano y se pierde la pista de qué se generó, con qué prompt y cuánto costó.
- **Objetivo (Éxito):** una aplicación de escritorio nativa que, en menos de 5 minutos desde que se instala, permite a cualquier persona con un proyecto de UE 5.8 abierto **hablar con su proyecto** usando la IA que ya tiene (local, clave de API o suscripción), prepararlo con un clic si hace falta, y generar assets 3D que llegan importados al editor y quedan guardados en una biblioteca reutilizable. Éxito medible:
  - un proyecto recién creado desde una plantilla de Epic pasa de «no preparado» a «conectado» sin editar ningún fichero a mano;
  - la petición «genera una silla medieval low poly y ponla delante de la cámara» termina con el asset importado en `/Game/` y colocado en el nivel, sin salir de la aplicación;
  - cada acción que la IA ejecuta en el editor queda registrada y es reversible (Deshacer del editor) o se pidió permiso antes.

## 👥 2. Usuarios y Escenarios

- **Perfil de Usuario:**
  - *Desarrollador/a indie o técnico de Unreal* que quiere acelerar tareas repetitivas (colocar, renombrar, configurar, scripts de editor) y prototipar con assets generados.
  - *Artista técnico / diseñador de niveles* que no programa en C++ pero sí describe lo que quiere.
  - *Docente y estudiantes* (contexto universitario) que necesitan una puerta de entrada guiada a Unreal y a la IA, con modelos locales gratuitos y sin claves.
- **Escenarios Clave:**
  - *Escenario A — Conectar:* el usuario abre la app con el editor de UE abierto. La app detecta el editor, el proyecto y su versión, y muestra «Conectado» en menos de 3 s, o «Necesita preparación» con la lista exacta de lo que falta.
  - *Escenario B — Preparar:* el proyecto no tiene los plugins MCP activos. La app muestra qué ficheros va a tocar (`.uproject`, `.ini`, plugin propio), hace copia de seguridad, aplica los cambios y guía el reinicio del editor. Al volver a abrirse, queda conectado.
  - *Escenario C — Hablar y actuar:* «crea un círculo de 12 antorchas alrededor del jugador». La IA consulta la escena, propone las acciones, el usuario las aprueba y aparecen en el nivel. «Deshacer» del editor las revierte.
  - *Escenario D — Generar con Tripo:* «genérame un cofre del tesoro estilizado». La app muestra el coste en créditos, pide confirmación, sigue el progreso, descarga el GLB al instante (la URL caduca en 5 min), lo guarda en la biblioteca y lo importa al proyecto.
  - *Escenario E — Generar un mundo con World Labs:* «un bosque de hayas al atardecer». La app genera el mundo, guarda sus resultados (panorama, malla, splats) y lleva al editor lo que Unreal puede usar.
  - *Escenario F — Agente de suscripción:* el usuario con Claude Code (o Gemini CLI, Codex, Copilot) lo usa desde la app; el agente recibe automáticamente el MCP de Unreal y las herramientas de la app, y cada escritura o ejecución pasa por la petición de permiso.
  - *Escenario G — Biblioteca:* el usuario busca «silla», ve las miniaturas de todo lo generado (en cualquier proyecto), y envía una al proyecto actual con un clic.

## ✨ 3. Funcionalidades Principales (Requisitos)

*Cada requisito lleva un ID único e inmutable (`REQ-NN`, `RNF-NN`; el catálogo de IA usa `RNF-IA.N` y `RF-IA-NN`). Se cita en el código y en los tests con `@spec REQ-NN`. El siguiente ID es el máximo + 1; nunca se renumera ni se reutiliza (un requisito retirado se tacha).*

*Columna de versión propuesta entre corchetes (`[v0.1]`, `[v0.2]`, `[v0.3]`); ver §3.6 y la pregunta abierta P-01.*

### 3.1. Conexión con Unreal Engine

- [ ] **REQ-01 · Detección de editores abiertos [v0.1]:** la app lista los procesos de Unreal Editor en ejecución con su `.uproject`, nombre del proyecto, versión del motor (de `EngineAssociation` y de la instalación) y estado del servidor MCP. *Aceptación:* con uno o dos editores abiertos, la lista es correcta en Windows, macOS y Linux en < 3 s; sin editores, ofrece abrir un proyecto reciente (REQ-03).
- [ ] **REQ-02 · Conexión MCP con el editor [v0.1]:** cliente MCP por HTTP (*Streamable HTTP*) contra `http://127.0.0.1:<puerto><ruta>` (por defecto `8000` y `/mcp`, leídos de la configuración del proyecto). Soporta los dos modos del servidor de Epic: *tool search* (`list_toolsets` / `describe_toolset` / `call_tool`, el modo por defecto) y registro nativo de herramientas. Detecta caídas y reconecta sin perder la conversación. *Aceptación:* contra el editor real de DBVMCPFirst se listan los toolsets y se ejecuta `GetActors` con resultado JSON válido; al cerrar y reabrir el editor, la app se reconecta sola.
- [ ] **REQ-03 · Abrir y lanzar proyectos [v0.1]:** lista de proyectos recientes (de la propia app y de la configuración del launcher de Epic) y botón «Abrir en Unreal» que lanza el editor de la versión correcta con ese `.uproject`. *Aceptación:* se lanza el `UnrealEditor` de la instalación 5.8 detectada; si no hay ninguna 5.8, se dice claramente.
- [ ] **REQ-04 · Diagnóstico de preparación [v0.1]:** lista de comprobación por proyecto con estado ✓/✗ y explicación de cada punto: versión del motor ≥ 5.8; plugins `ModelContextProtocol`, `ToolsetRegistry`, `PythonScriptPlugin`, `EditorToolset`, `AutomationTestToolset` y `SlateInspectorToolset` activos en el `.uproject` (los tres últimos son los que exponen las >100 herramientas reales; con solo los mínimos el editor ofrece 4 herramientas, lección de MCPCLiente); preferencia de editor «Auto Start Server» activada (es por usuario, no del `.uproject`); puerto libre y no compartido con otro editor abierto; toolset de dbv-unreal-ai instalado y en la versión esperada; tipo de proyecto (solo Blueprint o con C++). *Aceptación:* sobre un proyecto en blanco marca exactamente lo que falta; sobre DBVMCPFirst marca solo el toolset propio y el arranque automático.
- [ ] **REQ-05 · Preparación asistida del proyecto [v0.1]:** un botón «Preparar» que aplica los cambios detectados en REQ-04, mostrando **antes** qué ficheros se tocan y cómo (diff), con copia de seguridad previa en la carpeta de datos de la app y opción «Revertir preparación». Si el editor está abierto, pide guardar y cerrarlo (o lo cierra con confirmación si la plataforma lo permite) y lo vuelve a abrir. Nunca toca ficheros fuera de la carpeta del proyecto. *Aceptación:* un proyecto de plantilla de Epic queda conectado tras «Preparar» y un reinicio, sin editar nada a mano; «Revertir» deja los ficheros byte a byte como estaban.
- [ ] **REQ-06 · Toolset de dbv-unreal-ai para Unreal [v0.1]:** paquete que se instala en el proyecto durante la preparación y que añade al servidor MCP las capacidades que los toolsets de Epic no cubren. Se distribuye en **dos variantes** (P-02): **Python** por defecto (plugin de proyecto de solo contenido que registra el toolset con `toolset_registry`; sin compilar, válido en proyectos solo Blueprint) y **C++ opcional** para proyectos que ya tienen código, con las mismas herramientas y las capacidades extra que solo C++ permita. La preparación (REQ-05) elige la variante según el tipo de proyecto y lo que pida el usuario; las dos exponen el mismo contrato de herramientas:
  - `run_python(code)` → `{success, returnValue, stdout, stderr}`, dentro de una transacción del editor para que sea deshacible (heredado de `UMyCustomSceneToolset::RunPython`). Es **lo único que Epic no ofrece**: su `ProgrammaticToolset.execute_tool_script` es un sandbox sin acceso a `unreal`;
  - `get_scene_summary` / `get_actors(filtro)` → actores con nombre, clase, transformada y malla, en un JSON compacto (heredado de `GetActors` y `CountActorsWithMesh`);
  - `get_project_info` (versión, módulos, plugins, mapas, ruta de contenido, identidad del proyecto para R-09).
  - **No se reimplementa lo que ya trae `EditorToolset`**: importar (`StaticMeshTools.import_file`, `SkeletalMeshTools.import_file`, `TextureTools.import_file`), colocar (`SceneTools.add_to_scene_from_asset`), carpetas del Outliner (`SceneTools.set_actor_folder`), materiales y Blueprints. REQ-19 los orquesta.
  - La variante C++ incluye el registro explícito del toolset (`UToolsetRegistry::RegisterToolsetClass` en `OnPostEngineInit`); la Python lo registra desde su `init_unreal.py`, como hace Epic.
  *Aceptación:* cada herramienta tiene test automatizado en el editor (toolset de tests de Epic o `PythonTestRunner`) y devuelve errores estructurados, nunca excepciones sin capturar; la variante Python y la C++ pasan la **misma** batería de tests de contrato.
- [ ] **REQ-07 · Varias instancias y puertos [v0.2]:** se puede trabajar con dos editores abiertos a la vez; la preparación asigna a cada proyecto un puerto propio en la configuración **del usuario** (no la compartida del repositorio del proyecto) y la conversación indica siempre a qué proyecto se dirige.

### 3.2. Conversación con IA

- [ ] **REQ-08 · Chat con el proyecto [v0.1]:** pantalla principal de conversación ligada al proyecto conectado: streaming, Markdown seguro (sin `innerHTML`), botón de cancelar inmediato, indicador del destino de la petición (local/nube/agente) y del proyecto. El historial se guarda por proyecto en la carpeta de datos de la app y se puede borrar.
- [ ] **REQ-09 · Motor de IA de tres niveles [v0.1]:** modelos locales (Ollama, LM Studio, llama.cpp/compatibles), nube con clave en el llavero del SO (Anthropic, OpenAI, Google Gemini, OpenRouter, compatibles OpenAI) y agentes de suscripción por ACP. Es el catálogo `RF-IA-01`/`RF-IA-02` (§3.5), heredado de dbv-tauri-starter / dbv-typst-editor, incluidas sus lecciones (Ollama por `/api/chat` nativo con `num_ctx`; `thought_signature` de Gemini 3; tope de tokens por respuesta; detección estricta de «no admite herramientas»).
- [ ] **REQ-10 · Bucle de herramientas para modelos directos [v0.1]:** para los niveles 1 y 2, la app actúa como *host* MCP: traduce las herramientas de Unreal (vía tool search: el modelo descubre toolsets bajo demanda) y las propias de la app (generación, biblioteca) al formato de *function calling* de cada proveedor, ejecuta las llamadas y devuelve los resultados acotados en tamaño. La traducción respeta las restricciones de cada API (lecciones de MCPCLiente: Gemini rechaza puntos en los nombres de función —`Toolset.Tool` → `Toolset_Tool`— y exige los tipos del esquema en mayúsculas) y desenvuelve los resultados doblemente codificados del editor (`returnValue` con JSON dentro de una cadena). Límite de pasos por petición configurable (15 por defecto) y presupuesto de contexto. *Aceptación:* el escenario C funciona con un modelo en la nube y con un modelo local de tamaño medio que admita herramientas.
- [ ] **REQ-11 · Agentes ACP con Unreal ya conectado [v0.1]:** al abrir una sesión ACP, la app le pasa al agente los servidores MCP en `session/new` (el de Unreal por HTTP si el agente lo admite; si no, por stdio mediante el modo MCP de la propia app, REQ-20). Cada petición de permiso del agente se muestra con su contenido (código Python, diff, parámetros) para permitir una vez / siempre en esta conversación / denegar.
- [ ] **REQ-12 · Permisos y seguridad de las acciones en el editor [v0.1]:** toda herramienta se clasifica como **lectura** (consultar escena, logs, capturas) o **modificación** (ejecutar Python, crear/importar/borrar assets, cambiar ajustes, compilar). Las de lectura se ejecutan sin preguntar; las de modificación muestran una tarjeta con lo que se va a hacer (el código Python completo, el asset y destino, el ajuste) y requieren aprobación, con «permitir siempre en esta conversación» por tipo de herramienta. Las modificaciones se envuelven en transacciones del editor siempre que sea posible. *Aceptación:* ningún `run_python` se ejecuta sin que su código se haya mostrado; un test lo verifica sobre el bucle de herramientas.
- [ ] **REQ-13 · Contexto del proyecto y del editor [v0.1]:** la IA puede consultar, bajo demanda y con presupuesto de tokens: estructura del proyecto (`Source/`, `Config/`, plugins), assets por ruta/tipo, actores del nivel, selección actual, logs recientes del editor y una **captura del viewport** (`CaptureViewport` de `EditorToolset`) para modelos multimodales. El usuario ve y puede quitar los elementos de contexto (fichas, `RF-IA-03.3`).
- [ ] **REQ-25 · Base de conocimiento de la API de Unreal 5.8 [v0.1]:** la IA dispone, sin red, de dos herramientas de consulta heredadas del skill `dbv-unreal-python-api` de MCPCLiente: búsqueda **conceptual** en las guías oficiales (`search_unreal_guides`) y búsqueda **sintáctica** de clases, métodos y firmas de la API Python (`search_unreal_api`, con `exact:Clase`). Se ofrecen a los tres niveles de IA (a los agentes ACP a través del MCP propio, REQ-20). Cómo llega la base de conocimiento al equipo del usuario: ver P-06 y R-15.
- [ ] **REQ-26 · Comprobación del código Python antes de ejecutarlo [v0.1]:** antes de mostrar la tarjeta de aprobación de un `run_python`, la app analiza el código (AST) y comprueba cada referencia `unreal.Clase.miembro` contra la API de la versión del motor conectado; las que no existen o están marcadas como obsoletas (p. ej. `EditorLevelLibrary.get_editor_world`, visto en vivo el 2026-10-24) se devuelven al modelo con la alternativa para que se autocorrija (máx. 2 reintentos) y, si persisten, se avisan en la tarjeta. Tras ejecutar, los `DeprecationWarning` de `stderr` también vuelven al modelo. *Por qué:* el protocolo MCP no permite obligar a un modelo a consultar la documentación antes de actuar (lección de MCPCLiente), pero aquí la app es el host y **sí** controla el bucle. Equivale a «compilar la propuesta en memoria» de dbv-typst-editor.
- [ ] **REQ-14 · Registro de acciones de la sesión [v0.1]:** línea de tiempo de todo lo que la IA hizo en el editor y fuera de él (herramienta, parámetros, resultado, coste en créditos si aplica), exportable a Markdown. Sirve de auditoría y de material didáctico.

### 3.3. Generación 3D con servicios de terceros

- [ ] **REQ-15 · Generación de modelos con Tripo [v0.1]:** texto → modelo e imagen → modelo con la **API v3** (`openapi.tripo3d.ai/v3`, verificada en Holo-Companion: `model: v3.1-20260211`, `texture_quality`, `smart_low_poly`, `face_limit` orientativo). Antes de gastar: coste estimado en créditos y confirmación. Durante: progreso y cancelación. Al terminar: **descarga inmediata** del GLB (la URL caduca a los 5 minutos), guardado en la biblioteca (REQ-17) con prompt, parámetros, `task_id`, coste y miniatura. La clave de Tripo vive en el llavero del SO. Disponible como acción de la interfaz y como herramienta de la IA (`generate_3d_model`), siempre sujeta a la confirmación de coste.
- [ ] **REQ-16 · Rigging y animación con Tripo [v0.1.x, adelantado desde v0.2 en v1.3]:** rig (`/v3/animations/rig`, `v1.0-20240301` → esqueleto Mixamo limpio en bípedos; `v2.5-20260210` para criaturas no humanoides) y animaciones preset (`/v3/animations/retarget`: idle, walk, run, jump…), importadas como Skeletal Mesh + Animation Sequences.
- [ ] **REQ-18 · Generación de mundos con World Labs (Marble) [v0.2]:** prompt de texto o imagen → mundo (`/marble/v1/worlds:generate`, operación asíncrona con sondeo). Se guardan todos sus resultados (panorama, malla HQ, malla de colisión, splats `.spz`, miniatura) y se llevan a Unreal: malla HQ como Static Mesh, malla de colisión como colisionador, panorama como cielo HDRI y **los splats mediante un plugin de Gaussian Splatting de terceros** para UE 5.8 (P-03). Si el plugin no está instalado en el proyecto, la preparación lo ofrece; sin él, el resto del mundo se importa igual y el `.spz` queda en la biblioteca. Coste/confirmación y clave en el llavero, como REQ-15.
- [ ] **REQ-19 · Envío a Unreal [v0.1]:** «Enviar a Unreal» importa un asset de la biblioteca en `/Game/DBV/<TipoDeAsset>/<nombre>` (convención del usuario establecida en MCPCLiente: todo lo generado por IA bajo `/Game/DBV/`, nunca suelto en la raíz de `Content/`; prefijo configurable), corrige escala y ejes (glTF en metros y Y-arriba → Unreal en centímetros y Z-arriba), crea el material y, opcionalmente, lo coloca en el nivel (delante de la cámara o en la posición pedida) dentro de una carpeta del World Outliner que agrupe lo relacionado. Usa las herramientas de importación y colocación de `EditorToolset` (REQ-06). *Aceptación:* un GLB de Tripo aparece con la escala y la orientación correctas y su textura aplicada.

#### 3.3.1. Ampliación tras la primera prueba del usuario (v1.3, 2026-10-24) [v0.1.x]

Origen: el usuario probó la app con Claude Code y pidió un pirata. Hallazgos: el prompt se envía a Tripo **sin que el usuario vea qué se envía**; mientras Tripo trabaja la conversación «se queda parada» (la generación de un agente corre en el subproceso `--mcp`, que la ventana no ve); el nombre del asset es el prompt; solo se genera un objeto por petición; la biblioteca no admite objetos ajenos ni cambiar de sitio; no hay vista previa antes de enviar a Unreal; no se pueden adjuntar imágenes de referencia; y el MCP propio **solo expone `generate_3d_model`**, así que la IA no puede riggear ni animar aunque la API de Tripo sí lo permite (comprobado en la conversación: «esta herramienta no la expone»).

- [ ] **REQ-27 · Qué se envía y qué está pasando [v0.1.x]:** cada trabajo de generación tiene una **consola** con la petición exacta que sale hacia el proveedor (endpoint y cuerpo JSON, **nunca la clave**), el `task_id`, los cambios de estado y los errores. Se ve en la vista Generar y, para los trabajos lanzados por un agente (subproceso `--mcp`), **también en la conversación como una tarjeta viva** con barra de progreso, para que esperar a Tripo no parezca un bloqueo. Los trabajos se comparten entre procesos mediante ficheros de trabajo en la carpeta de datos de la app. La app **no reescribe el prompt del usuario**: lo que se envía es lo que escribió la persona o lo que decidió la IA, y la consola lo muestra. *Aceptación:* con un agente generando un modelo, la conversación muestra el cuerpo enviado y el progreso hasta que termina; en Generar se ve la misma consola.
- [ ] **REQ-28 · Nombres con sentido [v0.1.x]:** el nombre de un asset es corto y describe el objeto («Pirata», «Linterna de hierro»), no el prompt. La IA que genera (agente o modelo) debe darlo (parámetro obligatorio de la herramienta); desde la interfaz, si no se escribe, lo propone la IA conectada (local o de nube) y, sin IA, una heurística sobre el prompt. Se puede **renombrar** en la biblioteca.
- [ ] **REQ-29 · Variantes [v0.1.x]:** una petición puede pedir de 1 a 4 modelos con el mismo prompt (Tripo da resultados distintos cada vez, como en Wordsmith). La tarjeta de coste muestra el total (n × coste), el tope de sesión (RNF-04) se comprueba sobre el total y cada variante es una entrada con su nombre («Pirata», «Pirata 2»…). Disponible en la interfaz y como parámetro `variants` de la herramienta.
- [ ] **REQ-30 · Ubicación e importación de la biblioteca [v0.1.x]:** la carpeta de la biblioteca se elige en Ajustes (por defecto, la carpeta de datos de la app) para poder compartirla entre aplicaciones o equipos; cambiarla puede **copiar** lo que ya hay. La biblioteca puede **importar** modelos sueltos (`.glb`, `.gltf`, `.fbx`, `.obj`) y **carpetas de otra biblioteca** (con su `library.json`) sin duplicar entradas, y renombrar y abrir la carpeta de cada asset. Lo importado es una entrada más (proveedor `import`), enviable a Unreal.
- [ ] **REQ-31 · Vista previa 3D antes de enviar [v0.1.x]:** un visor 3D ligero y sin red (Three.js incluido en la app) muestra el GLB de una entrada, con órbita y zoom, en la biblioteca y **en la conversación** cuando termina una generación o la IA va a enviar un asset a Unreal; la miniatura se genera con él si el proveedor no la trajo. *Aceptación:* tras generar, el usuario ve el modelo girando antes de aprobar «Enviar a Unreal».
- [ ] **REQ-32 · Referencias adjuntas [v0.1.x]:** imágenes (PNG, JPG, WebP) y documentos de texto se pueden **adjuntar** a la conversación (para que la IA, si es multimodal, los vea: entornos de referencia, capturas) y a la vista Generar. Una imagen adjunta en Generar lanza **imagen → modelo** con Tripo (REQ-15), con su coste en la tarjeta; la imagen original se guarda en la entrada. Un modelo sin visión recibe solo el nombre y el tipo del adjunto y se le avisa.
- [ ] **REQ-33 · Superficie completa de Tripo en el MCP propio [v0.1.x]:** el servidor `--mcp` y el asistente integrado ofrecen las mismas herramientas, sobre un único núcleo en Rust: `generate_3d_model` (texto, `name` obligatorio, `variants`), `image_to_3d_model` (referencia adjunta o fichero), `rig_model` (rig de una entrada: bípedo humanoide con `v1.0-20240301`, criaturas con `v2.5-20260210`, `spec: tripo`), `animate_model` (clips `preset:biped:*` sobre un rig; lista de presets), `generation_jobs` (estado y progreso de trabajos), `library_list` (con `id`, nombre, prompt, ficheros y si tiene rig/animaciones), `library_rename`, `library_import` y `library_send_to_unreal`. Toda operación con coste pide confirmación (RNF-04) y muestra su coste (rig ≈ 25–30 créditos, clip ≈ 10, medidos en Holo-Companion). Las descripciones de las herramientas dicen **qué no hacen y cuál usar en su lugar**, para que el modelo no responda «esta herramienta no lo expone».

### 3.4. Biblioteca, MCP y extensibilidad

- [ ] **REQ-17 · Biblioteca de assets generados [v0.1]:** catálogo local (fuera de cualquier proyecto, compartido entre proyectos) con miniatura, prompt, proveedor, parámetros, fecha, coste, ficheros y proyectos a los que se envió. Buscar, filtrar, abrir carpeta, regenerar con el mismo prompt, borrar (con confirmación) y enviar a Unreal (REQ-19). Visor 3D ligero del GLB dentro de la app [v0.2].
- [ ] **REQ-20 · Servidor MCP propio de dbv-unreal-ai [v0.1]:** modo `--mcp` del propio ejecutable (patrón de dbv-typst-editor, RF-113) por **stdio**, sin puerto de red, que expone las herramientas de generación, biblioteca y envío a Unreal, y hace de **puente** hacia el MCP de Unreal para los agentes que no hablan HTTP o no entienden los toolsets anidados, aplanándolos en una lista de herramientas (patrón `bridge.py` de MCPCLiente). Reglas de stdio aprendidas allí: una respuesta JSON-RPC por línea, nunca `id: null` y nunca responder a notificaciones. Permite usar todo lo anterior desde Claude Code, Cursor o el propio asistente del editor de Unreal (`MCPClientToolset`). Las herramientas con coste o que modifican requieren confirmación en la app abierta.
- [ ] **REQ-21 · MCPs de terceros [v0.2]:** el usuario añade servidores MCP (stdio o HTTP), los activa por proyecto y sus herramientas entran en el bucle con la misma clasificación lectura/modificación de REQ-12 (por defecto, todas de modificación hasta que el usuario indique lo contrario).
- [ ] **REQ-22 · Edición de código del proyecto con revisión [v0.3]:** la IA propone cambios en C++, Python e `.ini` del proyecto como diffs revisables (`propose_changes`, `RF-IA-04`), confinados a la carpeta del proyecto; tras aplicarlos, compila con Live Coding (`LiveCodingToolset`) y devuelve los errores al modelo para que se autocorrija (máx. 2 reintentos).
- [ ] **REQ-23 · Recetas guiadas [v0.3]:** flujos predefinidos de varios pasos («preparar un nivel de prueba», «personaje Tripo con rig y animaciones en un Character Blueprint», «iluminar una escena con un HDRI de World Labs») que se pueden lanzar como un prompt y que el toolset puede publicar como *Agent Skills* de Unreal (`toolset_registry.agent_skill`).
- [ ] **REQ-24 · Idiomas [v0.1]:** interfaz en español e inglés (selector sin librería, patrón `i18n.js` del starter); el idioma del sistema se elige por defecto.

### 3.5. Subsistema de IA de escritorio (catálogo estándar, adaptado)

*Catálogo de `docs/templates/AI_SPECIFICATIONS.template.md`. En este producto la IA **no es opcional**: es el núcleo. Las adaptaciones se indican en cada punto.*

- [ ] **RNF-IA.1 · Arranque ligero (adaptado):** la app debe arrancar y mostrar la detección de proyectos y la biblioteca sin ninguna IA configurada (la preparación, la biblioteca y el envío a Unreal funcionan sin IA). Los módulos de conversación se cargan de forma perezosa; la detección de servidores locales y agentes en `PATH` solo se ejecuta al abrir «Conectar una IA».
- [ ] **RNF-IA.2 · Privacidad por diseño:** ningún dato del proyecto sale del equipo sin consentimiento. Con modelos locales la comunicación es solo `127.0.0.1`. La primera vez que un proyecto usa un proveedor en la nube, **o un servicio de generación (Tripo, World Labs)**, se avisa de qué se envía (prompt, imagen de referencia, capturas del viewport, fragmentos de código) y se pide confirmación.
- [ ] **RNF-IA.3 · Confinamiento:** las herramientas de ficheros se limitan a la carpeta del proyecto de Unreal conectado y a la carpeta de la biblioteca; se rechazan rutas con `..` y enlaces simbólicos que escapen. *Nota:* `run_python` dentro del editor no está confinado por diseño (Python de Unreal tiene acceso completo); por eso REQ-12 exige mostrar el código y pedir aprobación.
- [ ] **RNF-IA.4 · Custodia de credenciales:** claves de proveedores de IA **y de Tripo/World Labs** solo en el llavero del SO (`keyring`), nunca en JSON, `localStorage` ni `.env`; el frontend solo ve `hasKey`; sin llavero, solo en memoria de la sesión.
- [ ] **RF-IA-01 · Conectar una IA:** asistente con autodetección (Ollama `:11434`, LM Studio `:1234`, CLIs `claude`/`gemini`/`codex`/`copilot`/`node` en `PATH`), proveedores de nube, «Probar conexión» con lista real de modelos, y explicación de que una suscripción no es una clave de API.
- [ ] **RF-IA-02 · Agentes ACP:** sesión ya autenticada del CLI; permisos interactivos (una vez / siempre en esta conversación / denegar); punto de restauración por turno de los ficheros de texto del proyecto (`Source/`, `Config/`, `*.py`) con informe de cambios en disco y «Deshacer». Los assets binarios (`.uasset`) no se fotografían: su red de seguridad es la transacción del editor y el control de versiones del proyecto (ver riesgo R-04).
- [ ] **RF-IA-03 · Panel de conversación:** como REQ-08; atajo `Ctrl+Mayús+I` / `Cmd+Shift+I`; etiqueta verde (local) / ámbar (nube) / violeta (agente); fichas de contexto removibles; streaming y cancelación.
- [ ] **RF-IA-04 · Revisión de cambios en ficheros:** propuestas atómicas con diff por trozos para todo cambio en ficheros de texto del proyecto (REQ-05, REQ-22); nada se escribe a ciegas.

### 3.6. No funcionales

- [ ] **RNF-01 · Multiplataforma:** binarios nativos para Windows, macOS y Linux con Tauri v2. Las rutas de instalación de Unreal, del launcher de Epic y del ejecutable del editor se resuelven por plataforma.
- [ ] **RNF-02 · Compatibilidad de versiones de Unreal:** objetivo **UE 5.8.x** (verificado con 5.8.2). El plugin MCP de Epic es **Experimental**: todo el acceso a él pasa por un adaptador con la versión del motor detectada, para que un cambio de API en 5.9 se resuelva en un solo sitio. Las versiones < 5.8 se detectan y se informa de que no son compatibles.
- [ ] **RNF-03 · Robustez frente al editor:** toda llamada al editor tiene tiempo límite (configurable; 60 s por defecto, más para importaciones) y un `run_python` largo no congela la app. Si el editor se cierra o se cuelga a mitad de una acción, la conversación lo explica y no se pierde.
- [ ] **RNF-04 · Gasto controlado:** ninguna operación con coste (créditos de Tripo/World Labs, tokens de nube) se lanza sin confirmación explícita; tope de gasto por sesión configurable; el registro (REQ-14) suma el gasto.
- [ ] **RNF-05 · Rendimiento:** arranque de la app < 2 s en frío en un equipo de desarrollo; latencia añadida por la app a una llamada de herramienta < 50 ms.
- [ ] **RNF-06 · Seguridad local:** el servidor MCP de Unreal no tiene autenticación; la app solo se conecta a `127.0.0.1`, comprueba al preparar en qué interfaz escucha el servidor y avisa si es accesible desde la red. El MCP propio (REQ-20) es solo stdio.
- [ ] **RNF-07 · Accesibilidad:** navegación completa por teclado, foco visible, contraste WCAG AA en los temas, `aria-live` acotado en el streaming (lección de dbv-typst-editor).
- [ ] **RNF-08 · Observabilidad:** log local rotativo de la app (sin claves ni contenido de prompts salvo que el usuario active el modo diagnóstico), y botón «Copiar diagnóstico» para incidencias.

## 🏗️ 4. Propuesta de Solución Técnica (Resumen)

- **Enfoque:** aplicación Tauri v2 (Rust + Vanilla JS sin bundler) que parte del código de **dbv-tauri-starter** (que ya incluye el subsistema de IA genérico extraído de dbv-typst-editor: `src/ai/*`, `src-tauri/src/ai/*`). Sobre esa base se añaden cuatro módulos:
  1. **`unreal/`** — detección de instalaciones, proyectos y editores; cliente MCP Streamable HTTP; adaptador del protocolo de toolsets de Epic (tool search); preparación de proyectos con diff y copia de seguridad.
  2. **Toolset DBV para Unreal** — paquete que se instala en el proyecto (preferentemente un plugin de solo contenido con Python que registra un `ToolsetDefinition` mediante `toolset_registry`, sin necesidad de compilar; ver P-02).
  3. **`generation/`** — clientes de Tripo v3 y World Labs con sondeo, descarga inmediata, control de coste y proveedores simulados para tests (patrón de puertos/adaptadores de Wordsmith).
  4. **`library/`** — catálogo local de assets generados y canal de importación a Unreal.
- **Dependencias Críticas:**
  - Plugins de UE 5.8 `ModelContextProtocol` (Experimental), `ToolsetRegistry`, `PythonScriptPlugin`, `EditorToolset` y, opcionales, `LiveCodingToolset`, `PCGToolset`, `NiagaraToolsets`, etc.
  - API de Tripo v3 y API de World Labs Marble (de pago, con clave).
  - Agentes ACP: `@agentclientprotocol/claude-agent-acp`, `@zed-industries/codex-acp`, `gemini --acp` (solo accesos de pago desde el 2026-06-18; las cuentas gratuitas y de Google One pasaron a Antigravity CLI, cuyo servidor ACP es el binario aparte `agy_acp_server`, pendiente de integrar), `copilot --acp`.
- **Oportunidades de Skills y MCPs:** el producto **es** un host MCP. Además:
  - el toolset DBV puede publicar *Agent Skills* de Unreal (instrucciones del tipo «cómo importar un asset de Tripo») con `toolset_registry.agent_skill`;
  - el modo `--mcp` (REQ-20, en v0.1) convierte la app en un servidor MCP reutilizable por cualquier agente; su empaquetado como **Agent Plugin 1.0.0** (`plugin.json` + `mcp.json` + `skills/`) para Claude Code, Gemini CLI, Cursor, etc. se valora en `/plan`.
- **Sistema de Diseño:** ver `docs/DESIGN.md`.

### 4.1. Agent Readiness Checklist (Proyectos Web)

*No aplica: aplicación de escritorio sin superficie web pública (`Agent Readiness (Web): Not Applicable`).*

### 4.2. Subsistema de IA Generativa de Escritorio

- [x] **Arquitectura:** motor de 3 niveles de `docs/AI_DESKTOP_ARCHITECTURE.md`, ya presente en dbv-tauri-starter.
- [x] **Requisitos:** catálogo incorporado y adaptado en §3.5.
- [ ] **Ayuda al usuario:** generar `docs/IA.md` y `docs/IA.en.md` en `/ship`.

## 🚫 5. Fuera de Alcance (Out of Scope)

- [ ] Versiones de Unreal anteriores a 5.8 (no traen el servidor MCP integrado) y UEFN.
- [ ] Empaquetado/cocinado de juegos, despliegue a consolas o móviles desde la app.
- [ ] Colaboración multiusuario, sincronización en la nube o cuentas propias de dbv-unreal-ai.
- [ ] Alojar o entrenar modelos propios; la app usa los que el usuario ya tiene.
- [ ] Integración con Fab/Marketplace o con otros servicios de generación distintos de Tripo y World Labs en v0.x (la arquitectura de proveedores los admite más adelante: Meshy, Rodin, etc.).
- [ ] Un renderizador propio de Gaussian Splats dentro de Unreal (ver P-03).
- [ ] Sustituir el editor de Unreal: la app no edita Blueprints visualmente ni muestra el viewport en vivo.

## ⚠️ 6. Riesgos y Mitigación

- **R-01 · El plugin MCP de Epic es Experimental** y su API (o el modo tool search) puede cambiar en 5.8.x o 5.9.
  - **Mitigación:** adaptador versionado (RNF-02); tests de contrato contra respuestas grabadas del editor real; el toolset DBV se mantiene pequeño y apoyado en APIs Python estables (`unreal.EditorAssetLibrary`, `AssetImportTask`).
- **R-02 · El toolset en Python podría no exponerse por MCP igual que uno en C++.** *Riesgo muy reducido el 2026-10-24:* los 16 toolsets Python de `EditorToolset`, registrados desde su `init_unreal.py`, aparecen en `list_toolsets` del editor en vivo. Queda por verificar el caso concreto de un plugin **de proyecto** de solo contenido.
  - **Mitigación:** *spike* obligatorio en `/plan` sobre un proyecto de plantilla. Como se mantienen las dos variantes (P-02), si la Python falla la C++ cubre los proyectos con código y se decide qué hacer con los solo Blueprint (convertirlos o limitar funciones).
- **R-02b · Dos variantes del toolset pueden divergir.**
  - **Mitigación:** un único contrato de herramientas (nombres, parámetros, resultados) definido una vez y una batería de tests de contrato común (REQ-06).
- **R-03 · Ejecutar Python arbitrario en el editor es destructivo en potencia** (borrar assets, sobrescribir mapas).
  - **Mitigación:** REQ-12 (código visible y aprobación), transacciones del editor, recomendación activa de control de versiones (la app detecta si el proyecto está en git/Perforce y avisa si no), y registro de acciones (REQ-14).
- **R-04 · Los cambios en `.uasset` no se pueden «fotografiar» como los ficheros de texto.**
  - **Mitigación:** apoyarse en el *undo* del editor y en el control de versiones; las operaciones de borrado masivo requieren confirmación reforzada.
- **R-05 · El hilo de juego del editor se bloquea** durante un Python largo o una importación grande; el MCP no responde mientras tanto.
  - **Mitigación:** tiempos límite (RNF-03), indicador de «el editor está ocupado», importaciones por lotes.
- **R-06 · Costes de terceros** (créditos de Tripo ≈30 por modelo, rig 30, animación ≈20; World Labs por mundo).
  - **Mitigación:** RNF-04, caché local de resultados y proveedores simulados en los tests (nunca se gastan créditos en CI).
- **R-07 · URLs de descarga de Tripo caducan a los 5 minutos.**
  - **Mitigación:** descarga inmediata al completar la tarea, reintento con nueva consulta de la tarea si caduca.
- **R-08 · Modelos locales pequeños usan mal las herramientas** (aprendido en dbv-typst-editor: llamadas mal formadas, respuestas vacías, cortes por tope).
  - **Mitigación:** tool search reduce el número de esquemas en contexto; reintento único con recordatorio; diagnóstico de respuestas (`diagnoseResponse`); recomendación de modelo mínimo en el asistente.
- **R-09 · Varios editores abiertos con el mismo puerto (8000)**: el segundo no arranca el servidor o la app habla con el proyecto equivocado.
  - **Mitigación:** REQ-07 y comprobación de identidad del proyecto en cada conexión (`get_project_info`).
- **R-10 · Seguridad y privacidad (IA/datos):** fuga de claves, prompts con código del proyecto enviados a la nube, dependencias alucinadas en el código que genera la IA.
  - **Mitigación:** RNF-IA.2/4, revisión de seguridad en `/code-simplify` (secretos, *slopsquatting*), y escaneo de secretos antes de cada commit del repositorio de la app.
- **R-11 · Licencias y términos de uso de los assets generados** (Tripo y World Labs tienen condiciones distintas según plan).
  - **Mitigación:** la biblioteca guarda el proveedor y el plan; la ayuda enlaza los términos de cada servicio.
- **R-12 · Plugin de Gaussian Splatting de terceros** (P-03): puede no existir para 5.8, quedarse sin mantener o tener licencia copyleft o comercial incompatible con la redistribución.
  - **Mitigación:** la app **no redistribuye** el plugin: lo detecta y guía su instalación por el usuario; auditoría de licencias y compatibilidad con 5.8 en `/plan` de la v0.2 (`WEB_TO_DESKTOP_MIGRATION.md` §6 como guía de auditoría); los splats son opcionales (REQ-18).
- **R-14 · Synology Drive no sincroniza ficheros ni carpetas ocultos** (`.git`, `.github/`, `.gitignore`, `.claude/`…): MCPCLiente perdió así su `.git` en otro equipo.
  - **Mitigación:** remoto en GitHub desde el principio; si el repositorio aparece sin `.git` en otro equipo, reconectar con `git fetch` + `git reset origin/<rama>` (sin `checkout`) en vez de asumir un borrado.
- **R-15 · La base de conocimiento de la API (REQ-25) pesa ~118 MB en 17.000 ficheros y es contenido de Epic obtenido por *scraping*** (la web de documentación está protegida por Cloudflare; MCPCLiente lo resolvió con Playwright).
  - **Mitigación:** no redistribuir la documentación de Epic dentro del instalador sin revisar sus términos (P-06). Alternativas: generarla en el equipo del usuario, o usar el *stub* `unreal.py` que el propio motor genera con el «Developer Mode» de Python activado, que da la API exacta de la instalación del usuario (incluidas las clases de su proyecto) sin descargar nada.
- ~~**R-13 · «Copilot» es una marca registrada de Microsoft.**~~ *Retirado el 2026-10-24: el usuario cambió el nombre visible a «DBV Unreal AI», que no usa la marca.*

## ❓ 7. Preguntas Abiertas

- [x] **P-01 (crítica) · Alcance de la v0.1.** *Resuelta (2026-10-24):* **MVP + MCP propio.** v0.1 = conectar + preparar + chat con los tres niveles + permisos + Tripo texto/imagen → modelo + biblioteca + envío a Unreal + **servidor MCP propio (REQ-20)**: todos los `[v0.1]`. v0.2 = World Labs, rig/animación, MCPs de terceros, varias instancias, visor 3D. v0.3 = edición de código con compilación y recetas.
- [x] **P-02 (crítica) · Forma del toolset.** *Resuelta (2026-10-24):* **las dos variantes**: Python por defecto (plugin de solo contenido, sin compilar) y C++ opcional para proyectos con código, con el mismo contrato de herramientas (REQ-06, R-02b).
- [x] **P-03 · Gaussian Splats de World Labs.** *Resuelta (2026-10-24):* **plugin de terceros** para llevarlos a Unreal, sin redistribuirlo (REQ-18, R-12). La elección del plugin se investiga en `/plan` de la v0.2.
- [x] **P-04 · Nombre visible.** *Resuelta (2026-10-24):* **DBV Unreal AI** (se descartó «DBV Unreal Copilot» por la marca de Microsoft); `dbv-unreal-ai` se mantiene como identificador técnico (repositorio, bundle id, ejecutable).
- [ ] **P-06 · Cómo llega la base de conocimiento de la API al usuario (REQ-25, R-15).** Propuesta: la **referencia sintáctica** sale del *stub* `unreal.py` que genera el motor del propio usuario (exacta para su versión y su proyecto, sin redistribuir nada de Epic; la preparación activa el «Developer Mode» de Python para generarlo) y alimenta REQ-26; las **guías conceptuales** se construyen en el equipo del usuario con el crawler de MCPCLiente como descarga opcional. Se decide en `/plan`, tras revisar los términos de uso de la documentación de Epic.
- [ ] **P-05 · Distribución.** Como dbv-typst-editor: Microsoft Store en Windows, GitHub Releases + Homebrew en macOS, AppImage/deb en Linux, con auto-actualización. Se decide en `/plan` (gate de app nativa).

## 🧪 8. Criterios de Evaluación y Evals (No Deterministas)

*Lección heredada de dbv-typst-editor («la compilación es el juez»): aquí **el estado del editor es el juez**. Tras cada tarea de la suite se consulta la escena o el registro de assets y se comprueba de forma determinista si el resultado es correcto, sin necesitar un modelo juez.*

- [ ] **Métricas de Output:**
  - tasa de éxito sobre un corpus de tareas con verificación por estado (p. ej. «12 antorchas en círculo de radio 5 m» → 12 actores, distancias 500 ± 5 cm);
  - JSON/argumentos de herramientas válidos al primer intento;
  - cero herramientas o rutas de assets inventadas (verificadas contra `list_toolsets` y el registro de assets).
- [ ] **Métricas de Trayectoria:**
  - número de llamadas por tarea y proporción de `describe_toolset` útiles;
  - ninguna herramienta de modificación sin aprobación (debe ser 0 siempre);
  - gasto de créditos simulado por tarea dentro del presupuesto.
- [ ] **Ejecución:** suite contra un **editor simulado** (respuestas grabadas del editor real; base inicial: las 282 herramientas aplanadas de `.mcp_tools_cache.json` de MCPCLiente y las respuestas capturadas el 2026-10-24) en CI, y contra el **editor real** de un proyecto de pruebas en local, con guardado tras cada tarea, tiempo límite por petición y parada tras tres fallos de conexión seguidos (lecciones de dbv-typst-editor).

---
**Instrucción para la IA:** Las preguntas abiertas críticas (P-01, P-02) están resueltas; P-05 se resuelve en `/plan`.
