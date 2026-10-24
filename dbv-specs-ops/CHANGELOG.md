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
