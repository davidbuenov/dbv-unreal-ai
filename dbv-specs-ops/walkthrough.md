# 🧭 Walkthrough — DBV Unreal AI v0.2.0

> **Fecha:** 2026-10-27 · **Versión:** 0.2.0 · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md)

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA a proyectos abiertos de Unreal Engine 5.8 por su servidor MCP experimental, prepara el proyecto si hace falta, deja que la IA actúe en el editor **con permiso**, permite deshacer turnos completos con un clic, genera modelos 3D con Tripo y mundos fotorrealistas con World Labs Marble, y gestiona memoria persistente del proyecto y tareas comprobadas en el editor.

## Qué se entrega en v0.2.0

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| Generación de mundos (World Labs) | Integración completa con World Labs Marble API (`marble-1.1`, `marble-1.0-draft`): texto/imagen → mundo, estimación de créditos, descarga automática de `.spz`, mallas `.glb` y metadatos de escala a la biblioteca, pestaña dedicada en Generar y herramientas para el asistente y MCP | REQ-18, S27 |
| Deshacer turno de la IA | Botón «↩️ Deshacer este turno» en la conversación que revierte en bloque las transacciones en Unreal Engine vía Slate post-tick callback, con detección de cambios externos y borrado opcional de assets generados | REQ-40, S26 |
| Múltiples instancias | Detección y gestión de editores abiertos concurrentemente en puertos contiguos (8000, 8001…), propuesta de puerto libre al preparar, verificación de identidad por `get_project_info` y selector dinámico en la píldora | REQ-07, S24 |
| Animaciones y personajes | Selector interactivo de rig en el visor 3D que acopla clips esqueléticos a cualquier cuerpo de la biblioteca en tiempo real, con desempaquetado de sobres IPC y corrección de etiquetas modales | REQ-16, REQ-31 |
| Componente C++ 3DGS | Módulo C++ `DBVUnrealAICpp` con `UDbvGaussianSplatComponent` y `ADbvGaussianSplatActor` (licencia MIT) para carga nativa de `.spz` y `.ply`, renderizado anisótropo y colocación sincronizada en el nivel | REQ-41, REQ-42, S28b |
| Visor 3D y miniaturas | Miniaturas automáticas fuera de pantalla con Three.js, iluminación de estudio de 3 puntos, sombreado suave con arcilla mate neutra para mallas de colisión y conmutador visual con render fotorrealista | REQ-31, REQ-42, S28c |
| Importación de escenarios | Soporte para manifiestos JSON de World Labs (archivos o carpetas completas), progreso en vivo por pasos, bloqueo de botones y deduplicación por `world_id` | REQ-42, S28d |
| Memoria del proyecto | `dbv-unreal-ai/memory.md` y `task.md` junto al `.uproject` con control de concurrencia atómico; propuestas automáticas de la IA al decidir y consulta por herramientas MCP | REQ-34, REQ-35 |
| Vista Plan y verificación | Gestión visual de tareas con criterios de «hecho» comprobados en el editor (`asset exists`, `actor exists`, `level is`), estimación de costes y planificación asistida | REQ-36, REQ-37 |
| Captura del viewport | `unreal_capture_viewport` para modelos con visión con rejilla en metros y etiquetado de actores | REQ-13 |
| Tripo en vivo | Verificado contra la API real de Tripo (105 créditos gastados): generación, rigging, animación y reproducción en el viewport de Unreal 5.8.4 | REQ-15, REQ-16 |

## Cómo probarlo

1. `npm install` y `npm run tauri dev` (en la raíz).
2. Abre uno o más proyectos de Unreal 5.8; la píldora detecta los editores y permite alternar entre proyectos.
3. **Ajustes:** conecta una IA y configura tus claves de Tripo y World Labs (en el llavero del sistema).
4. **Generar:**
   - En **Modelos (Tripo)**: genera un modelo, riggealo o anímalo.
   - En **Mundos (World Labs)**: escribe un prompt o sube una imagen de referencia para generar un entorno fotorrealista con splats gaussianos y malla de colisión.
5. **Biblioteca:** visualiza los modelos y mundos generados en 3D con Three.js; abre un clip de animación y elige qué personaje de la biblioteca debe interpretarlo.
6. **Enviar a Unreal:** coloca el modelo o el actor Gaussian Splatting con su collider directamente en el nivel abierto.
7. **Conversación:** pide cambios a la IA; tras una ejecución que altere el nivel, pulsa «↩️ Deshacer este turno» para revertir todas las acciones de ese turno en bloque.
8. **Plan:** crea tareas y objetivos, o pide a la IA que planifique tu escena; marca tareas comprobando automáticamente su cumplimiento en el editor.

## Verificación

- **Automática (en verde el 2026-10-27):**
  - 170 tests de Rust (`cargo test --manifest-path src-tauri/Cargo.toml`).
  - 269 tests de Vitest en 29 suites (`npx vitest run`).
  - 34 tests de Python del toolset (`python -I -m unittest discover -s tests/python -v`).
  - Linter determinista `dbv-lint.py` en 9 ficheros Python: 0 errores, 0 avisos.
- **En vivo contra Unreal 5.8.4:**
  - Carga de nivel con gaussian splatting desde conversación, inserción y posicionamiento automático de más de 30 actores en escena.
  - Generación, rig y animación con Tripo verificados con saldo real.
  - Componente C++ `DBVUnrealAICpp` compilado e integrado en proyecto de Unreal.

## Revisión de código y estándares

- Revisión exhaustiva `/code-simplify` completada:
  - Garantizada limpieza de carpetas temporales ante cancelaciones o errores en operaciones World Labs.
  - Regla de un solo `return` + guard clauses aplicada en comandos Rust, JavaScript (`generateView.js`) y Python.
  - Documentación de IA actualizada en `docs/IA.md` y `docs/IA.en.md`.
