# 🧭 Walkthrough — DBV Unreal AI v0.6.0

> **Fecha:** 2026-11-14 · **Versión:** 0.6.0 (Minor, desde la 0.5.1) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.24) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md) · **Notas:** [`releases/v0.6.0.md`](./releases/v0.6.0.md)

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA (modelos locales, de nube o agentes por suscripción como Claude Code, Codex, Cursor u OpenCode) a un proyecto abierto de Unreal Engine 5.8.4 o 5.9, con permiso antes de actuar, generación 3D (Tripo, World Labs), audio y un servidor MCP propio.

## Qué se entrega en v0.6.0

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| **UE 5.8.4 y 5.9** | La misma app y el mismo plugin con los dos motores: versión real del editor, aviso si el proyecto es de otra, «Preparar» según el motor (`AllToolsets`, sección del `.ini`), captura y ayuda de toolsets de la 5.9, aprobaciones con `accessMode`, prompt con las herramientas nativas. | REQ-109..REQ-112, REQ-116 |
| **Agentes Cursor y OpenCode** | Por ACP, con los comandos del Unreal Assistant de la 5.9. | REQ-115 |
| **Autonomía y avisos** | Cinco niveles (*Preguntar todo* … *Total*); gastar créditos siempre pregunta; «Automático» pregunta antes de ejecutar código, comandos o tocar ficheros. Parpadeo y sonido cuando la IA espera una aprobación. | REQ-115, REQ-12 |
| **Unreal Canvas** | Sus ~66 herramientas del editor (splats, recortes, anotaciones, render, vídeo) como el toolset `EditorMcp.TopLevelTools`. La generación sigue en la app de Canvas. | REQ-113 |
| **Instrucciones y Knowledge Base** | Las instrucciones de Project Settings › MCP Server llegan a la IA integrada y a los agentes; la IA consulta la Knowledge Base si está activa. | REQ-115 |
| **Splats** | Sin espejo (conversión (z, x, −y)·100 con un eje negado) y renderizador a elegir en 5.9: el nuestro por defecto o el plugin GaussianSplat de Epic (girado 180° para los `.spz` de World Labs). | REQ-18, REQ-61, REQ-114 |
| **Correcciones** | «os error 17» al guardar en la versión de la Store; mundos enviados desde la ventana sin su escala; lista de toolsets recortada y con toolsets fantasma. | REQ-90, REQ-18, REQ-51 |

## Cómo probarlo

1. `npm install` y `npm run tauri dev`. Abre un proyecto de 5.8.4 o de 5.9 y pulsa **Preparar** (copia el plugin v0.6.0; en 5.9, con el editor cerrado, compila el C++).
2. Conecta: la píldora muestra la versión real del editor (la 5.9, como «Preview»).
3. En la conversación, cambia el nivel de autonomía y pide algo que modifique la escena: en *Automático* se ejecuta solo; un `execute_python` o un comando de consola siguen preguntando.
4. Envía un mundo de World Labs desde la Biblioteca: debe verse como su miniatura (sin espejo), a su escala y con el suelo en Z = 0. En 5.9, prueba también Ajustes › Biblioteca › «Plugin GaussianSplat de Epic» (con el plugin activo en el proyecto).
5. Con Unreal Canvas activo, pide a la IA que liste los splats de Canvas (`ListGsplats`).

## Verificación

- **Automática (2026-11-14):** Rust 357 tests (`cargo test --lib`), Vitest 456 (48 ficheros), Clippy sin avisos y `dbv-lint` sin errores en el Python cambiado.
- **En vivo (UE 5.9, `PruebaUnreal59`):** splats de tres mundos de World Labs y de la iglesia (9.975.598 splats) con los dos renderizadores, misma orientación que la miniatura; plugin C++ compilado con la corrección del espejo; colisión alineada comprobada con rayos; `load_splat_epic` y `place_world` a escala métrica; `--mcp` con el toolset de Canvas (`ListGsplats`, `GetStatus`) y con las instrucciones del editor.
- **Revisión de `/code-simplify` (2026-11-14):** un Crítico resuelto (el nivel «Automático» ejecutaba sin preguntar el Python de Epic, la consola y los ficheros de la 5.9) y un Importante registrado (la CLI de Cursor se busca también como `agent`).

## Límites conocidos y lo que NO está comprobado

- **Cursor y OpenCode no se han probado con sus CLI instaladas.**
- **La tarjeta del renderizador de splats no se ha probado dentro de la app** (sí su lógica, en tests y en vivo desde el editor).
- **Una línea diagonal clara** en una captura de *Fantasy Seaside Golden Sands* con nuestro renderizador, por revisar.
- **Fab en 5.8.4** (REQ-101) sigue pendiente; en 5.9 funciona con el toolset de Epic.
- **La Microsoft Store** sigue pendiente de la matriz de REQ-90.

## Compatibilidad UE 5.8.4 × 5.9 (REQ-116)

> Proyectos: `DBVMCPFirst` (5.8.4; no se abre nunca con 5.9) y `PruebaUnreal59` (5.9 MegaJam Preview, C++). Fixtures de 5.9 en `tests/fixtures/unreal-mcp/ue-5.9/`. Estado al 2026-11-14.

| Caso | 5.8.4 | 5.9 | Cómo se ha comprobado |
| --- | --- | --- | --- |
| Detectar el motor y conectar | ✅ | ✅ | En vivo en los dos; la píldora marca la 5.9 como «Preview» (Vitest) |
| Aviso si el editor conectado es de otra versión que el proyecto | — | ✅ | Vitest (`versionMismatch`); la app nunca abre un proyecto con otro motor |
| «Preparar» (plugins, `AllToolsets` en 5.9, sección del `.ini` de cada versión) | ✅ | ✅ | En vivo en `PruebaUnreal59` (variante C++) y tests de `prepare.rs`/`discovery.rs` |
| Plugin C++ `DBVUnrealAICpp` (incluido el render de splats) | ✅ | ✅ | Compilado y cargado en `PruebaUnreal59` con la corrección del espejo |
| Ayuda de los toolsets (`describe_toolset` / `describe_toolsets`) | ✅ | ✅ | Rust con fixtures de las dos versiones y en vivo |
| Lista de toolsets dentro del tope de contexto | ✅ (55) | ✅ (117; 154 con Canvas y GaussianSplat) | Referencia común Rust/JS y tope adaptable; en vivo, 10.156 caracteres |
| Aprobaciones por la marca `accessMode` | — (regla del verbo) | ✅ | Vitest con el `EditorAppToolset` real de 5.9 |
| Capturar el viewport y adjuntarlo | ✅ | ✅ | En vivo en 5.9; miniatura con test de integración |
| Conversación, colocar actores, deshacer | ✅ | ✅ (parcial) | En vivo en 5.9 (coche de Fab colocado y texturizado) |
| Splats (nuestro renderizador) | ✅ | ✅ | En vivo: tres mundos de World Labs y la iglesia |
| Splats con el plugin de Epic | — | ✅ | En vivo: `load_splat_epic` y `place_world` a escala métrica |
| Herramientas de Canvas, instrucciones del MCP | — | ✅ | En vivo con el `--mcp` |
| Generar con Tripo e importar | ✅ | ⏳ | Pendiente en 5.9 |
| Biblioteca | ✅ | ⏳ | Pendiente en 5.9 |
| Vestir un MetaHuman | ✅ (copia) | ⏳ | Pendiente con un MetaHuman de `PruebaUnreal59` |
| Play-testing y caja negra | ✅ | ⏳ | Pendiente en 5.9 |
| Convai, informe de VRAM | ✅ | ⏳ | Pendiente en 5.9 |
| Proyecto de partida del workshop | ⏳ | — | Se prepara en 5.8.4 |
