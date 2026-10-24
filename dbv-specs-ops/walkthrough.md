# 🧭 Walkthrough — DBV Unreal AI v0.1.0

> **Fecha:** 2026-10-24 · **Versión:** 0.1.0 (primera) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) v1.3 · **Plan:** [`implementation_plan.md`](./implementation_plan.md) §2 (S1–S11) y §4 (S12–S17)

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA a un proyecto abierto de Unreal Engine 5.8 por su servidor MCP experimental, prepara el proyecto si hace falta, deja que la IA actúe en el editor **con permiso** y genera assets 3D con Tripo que llegan importados al editor y quedan en una biblioteca reutilizable.

## Qué se entrega

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| Conexión | Detecta los editores abiertos y se conecta a `127.0.0.1:8000/mcp`; píldora con proyecto, versión y puerto | REQ-01, REQ-02, REQ-03 |
| Preparación | Diagnóstico punto por punto; «Preparar» enseña el diff, guarda copia, activa plugins, arranque automático del MCP e instala el toolset DBV (Python o C++); «Revertir» | REQ-04, REQ-05 |
| Toolset DBV (0.2.0) | `run_python` (transacción), `validate_python` contra el motor en marcha, `describe_python_api`, `import_model` (glTF por Interchange; Skeletal Mesh y animaciones), `place_asset`, `play_animation`, `get_scene_summary` | REQ-06, REQ-16, REQ-19, REQ-25, REQ-26 |
| Conversación | IA local, de nube o agentes ACP (Claude Code…), con las herramientas del editor; tarjeta de aprobación para lo que modifica; registro de acciones; adjuntos 📎 (imágenes y documentos) | REQ-08…REQ-14, REQ-32 |
| Tripo | Las 18 operaciones v3 con coste de la tabla oficial: texto/imagen/multivista → modelo, variantes, rig, 101 animaciones, convertir, retopología, texturizar, segmentar, imágenes, splats; saldo | REQ-15, REQ-16, REQ-29, REQ-33, RNF-04 |
| Trabajos | Consola con la petición exacta (sin clave) y progreso, en Generar y en la conversación, también los de un agente | REQ-27 |
| Biblioteca | Miniaturas, tipos y origen (modelo → rig → animación), importar modelos e imágenes u otra biblioteca, renombrar, carpeta configurable, visor 3D, «Enviar a Unreal» | REQ-17, REQ-19, REQ-28, REQ-30, REQ-31 |
| MCP propio | `dbv-unreal-ai --mcp` por stdio: puente al editor, biblioteca y toda la API de Tripo para Claude Code, Cursor o Gemini CLI | REQ-20 |
| Interfaz | Español/inglés, temas Claro/Oscuro/Sepia, chincheta «siempre encima», logo de la Unreal Dev Community Málaga | REQ-24, DESIGN.md |

## Cómo probarlo

1. `npm install` y `npm run tauri dev` (en la raíz).
2. Abre tu proyecto de Unreal 5.8; la app se conecta sola si el servidor MCP está activo. Si no, **Proyecto → Preparar proyecto…** con el editor cerrado. Con el toolset 0.1.0 instalado, la preparación propone actualizarlo a 0.2.0.
3. **Ajustes:** conecta una IA y pega tu clave de Tripo (verás el saldo).
4. **Conversación:** «Resume el nivel abierto», «Coloca un cubo de 100 cm delante de la cámara»; con 📎, una imagen de referencia.
5. **Generar:** un prompt o una imagen, variantes 1–4, la tarjeta de coste con lo que se envía; abajo, la consola de cada trabajo.
6. **Biblioteca:** «Ver en 3D», «Riggear…», «Animar…» (bailes) y «Enviar a Unreal».

## Verificación

- **Automática (en verde el 2026-10-24):** 137 tests de Rust (`cargo test`), 236 de Vitest (`npm run test:ai`), 25 de Python del toolset (`npm run test:python`), `cargo clippy -D warnings`, `dbv-lint.py` sin errores y el banco E2E en Edge (`npm run test:e2e`), que pinta la app real con un backend simulado e incluye el visor 3D con un GLB de prueba.
- **En vivo contra Unreal 5.8.2 (DBVMCPFirst):** conexión, herramientas del editor, toolset Python, Python validado y ejecutado, importación y colocación de un GLB real de Tripo. El usuario instaló el toolset C++ desde la interfaz y arrancó el editor.
- **Eval de IA:** `qwen3:8b` con el prompt completo de la app, 5/5 en las tareas del banco (`scripts/eval/unreal-eval.mjs`).

## Lo que falta verificar (ver «Deuda técnica» en [`task.md`](./task.md))

- Todo Tripo salvo texto → modelo, contra la API real: imagen → modelo, `/v3/files`, rig, retarget, convert y el resto (se probó contra un servidor simulado hecho a partir de la documentación). Hace falta la clave del usuario y su permiso para gastar créditos.
- Personajes en Unreal (S17): importar un GLB riggeado de Tripo y reproducir su animación en el editor; la variante C++ recompilada con `play_animation`.
- macOS y Linux en máquina real; el log local rotativo (RNF-08); las guías conceptuales de la API (REQ-25); la captura del viewport para modelos con visión (REQ-13).

## Revisión

Dos revisiones de `/code-simplify` (v0.1 y v0.1.x). La segunda encontró y corrigió un **Crítico**: la subida a Tripo de cualquier fichero local con una ruta escrita por la IA (ahora solo imágenes y modelos con tope de tamaño). Detalle en [`CHANGELOG.md`](./CHANGELOG.md).
