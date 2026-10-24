# Changelog — DBV Unreal AI

**🇪🇸 Español · [🇬🇧 English](./CHANGELOG.en.md)**

Todos los cambios relevantes de este proyecto se documentan en este fichero.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y el proyecto se adhiere a [Semantic Versioning](https://semver.org/lang/es/).

> Este fichero tiene un espejo en inglés en [`CHANGELOG.en.md`](./CHANGELOG.en.md). Se mantienen sincronizados: **toda entrada del changelog se escribe en los dos idiomas, en el mismo commit.**

---

## [Sin publicar] / [Unreleased]

### Added

- Bootstrap del proyecto con el framework SDD dbv-specs-ops v3.1.0: identidad del proyecto, activadores para los asistentes de IA, licencia MIT y README inicial.
- Base de la aplicación de escritorio a partir de dbv-tauri-starter 0.3.0: Tauri v2, subsistema de IA de tres niveles (local, nube con llavero del sistema, agentes ACP), con la identidad de DBV Unreal AI (`com.davidbuenov.dbv-unreal-ai`).
- Conexión con Unreal Engine 5.8 (REQ-01..REQ-05): detección de instalaciones del motor (aunque la lista del launcher de Epic no las nombre), de proyectos `.uproject` y de editores abiertos; cliente MCP del editor (sesión `Mcp-Session-Id`, *tool search*, `list_toolsets` en texto, resultados doblemente codificados), verificado en vivo contra UE 5.8.2; diagnóstico de preparación y «Preparar» con diff, copia de seguridad y «Revertir» byte a byte; se niega a preparar con el editor del proyecto abierto.
- Toolset DBV Unreal AI para Unreal (REQ-06, REQ-25, REQ-26): plugin de proyecto de solo contenido en Python con `run_python` (transacción de deshacer, stdout/stderr/avisos), `validate_python` (detecta APIs inexistentes u obsoletas sin ejecutar el código), `describe_python_api` (referencia del motor en marcha), `get_project_info` y `get_scene_summary`; registrado, probado y desregistrado en vivo en el editor. Variante C++ como plantilla que delega en el mismo paquete Python (sin compilar todavía).
- Herramientas de Unreal para el asistente (REQ-10, REQ-12, REQ-26): meta-herramientas del servidor de Epic (`list_toolsets`, `describe_toolset`, `call_tool`) y atajos propios; clasificación lectura/modificación por verbo con lista blanca (lo desconocido pide permiso); tarjeta de aprobación con el código Python completo; el Python se valida contra el motor en marcha antes de pedir permiso y lo que no existe vuelve al modelo para que se corrija.
- Tripo v3 y biblioteca (REQ-15, REQ-17, RNF-04): texto → modelo con confirmación de coste obligatoria y tope de créditos por sesión, descarga inmediata del GLB (los enlaces caducan a los 5 minutos) y biblioteca local compartida entre proyectos. Imagen → modelo queda pendiente de verificar con una clave real.
- Enviar a Unreal (REQ-19): importación por Interchange con `import_model` del toolset DBV (el `import_file` de Epic fuerza FBX y rechaza los GLB) y colocación delante de la cámara en `DBV/Generated`; comprobado en vivo con un GLB real de Tripo (escala correcta, material y textura).
- Servidor MCP propio (REQ-20): `dbv-unreal-ai --mcp [--port n] [--project ruta]` por stdio, sin puerto de red; puente al MCP del editor (con validación obligatoria del Python), biblioteca, enviar a Unreal y generación con Tripo (exige `confirm_cost: true` y respeta el tope); verificado en vivo contra el editor. La plantilla C++ del toolset pasa a nombres `snake_case` para compartir contrato con la variante Python.
- Interfaz de DBV Unreal AI (REQ-08, REQ-14, REQ-24, DESIGN.md): barra lateral (Proyecto · Conversación · Generar · Biblioteca · Ajustes), píldora de conexión con Unreal, conexión automática con el editor que tiene el servidor MCP activo, diagnóstico con el porqué de cada punto, «Preparar» con diff por fichero y «Revertir», registro de acciones exportable a Markdown, tarjeta de coste con el foco en «Cancelar», tema oscuro por defecto y claro, ES/EN. El asistente de IA se carga al abrir la Conversación, con las herramientas de Unreal y de generación y los servidores MCP para los agentes ACP.
- CI y publicación (ADR-V010-001 D1): verificación en cada push (Rust, Vitest, toolset Python, linter) y Release en borrador con binarios de Windows, macOS universal y Linux al subir un tag.
- «Copiar diagnóstico» en Ajustes (RNF-08): versión, sistema, instalaciones de Unreal y editores abiertos, sin claves ni prompts.
- Evals con un modelo real contra el editor real (`npm run eval:unreal -- --model qwen3:8b --toolset unreal-plugin/DBVUnrealAI/Content/Python`): el mismo bucle y las mismas herramientas que la app, con la verdad calculada preguntando al editor. Con `qwen3:8b`: 5/5 en tres ejecuciones y cero modificaciones sin aprobación.
- Cuando una API está obsoleta, la validación y la referencia de la API dan la llamada equivalente en el subsistema del editor (`unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()`), y el prompt de sistema explica el patrón de subsistemas de Unreal 5.8.
- Tests etiquetados para todos los requisitos de la v0.1 (REQ-11 con servidores MCP en `session/new`, REQ-13, RNF-01, RNF-05 con la latencia medida, RNF-06, RNF-07, RNF-IA.2).

### Fixed

- La píldora de conexión volvía a «Sin editor» al cargarse el asistente de IA: `applyTranslations()` reescribía su texto porque llevaba `data-i18n` (detectado con el banco de interfaz en Edge headless).
- El diff de la preparación no marcaba con «…» las líneas omitidas al principio y al final.
- Con un modelo de 8B, contar actores con Python agotaba los 12 pasos: el aviso de API obsoleta («usa el subsistema») no decía cuál y el modelo no encontraba la alternativa (detectado por la eval; de 14 llamadas al editor a 4).
