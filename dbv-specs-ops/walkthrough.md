# 🧭 Walkthrough — DBV Unreal AI v0.6.1

> **Fecha:** 2026-11-15 · **Versión:** 0.6.1 (Patch, desde la 0.6.0; decisión del usuario: la v0.7.0 queda para UEFN y Verse) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.23 y §3.25) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md) · **Notas:** [`releases/v0.6.1.md`](./releases/v0.6.1.md) · **Plan:** [`implementation_plan.md`](./implementation_plan.md) §19 y §20

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA (modelos locales, de nube o agentes por suscripción como Claude Code, Codex, Cursor u OpenCode) a un proyecto abierto de Unreal Engine 5.8.4 o 5.9, con permiso antes de actuar, generación 3D (Tripo, World Labs), contenido de Fab, audio y un servidor MCP propio.

## Qué se entrega en v0.6.1

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| **Fab en 5.8.4** | El plugin C++ usa la sesión de Fab del propio editor (botón Fab del Content Browser), y el token no sale del editor. Biblioteca del usuario, `download-info` e instalación como trabajos que no bloquean el editor. | REQ-101, REQ-103 |
| **Buscar y ver antes de instalar** | Catálogo sin sesión desde Rust. Cada resultado con miniatura (solo `media.fab.com`), vendedor, gratis o tuyo, formatos compatibles y enlace a fab.com: en la conversación como tarjetas, y en la nueva **vista Fab** con pestañas, filtros, ficha y «Cargar más». Nunca un total. | REQ-102, REQ-105, REQ-106, REQ-117 |
| **Instalar con reglas en el código** | Solo lo gratuito o lo tuyo (`no_owned` con enlace: nada se compra) y solo lo compatible con el editor (`incompatible_engine`). `glb`/`gltf`/`fbx` a `/Game/Fab/<Título>/` con Blueprint si llega por piezas, packs con BuildPatchServices en una carpeta temporal, Quixel en calidad media. Un listing maduro solo se abre si lo acepta la persona. | REQ-104, REQ-105 |
| **Biblioteca** | Lo instalado queda como referencia (`fab-<uid>`: miniatura, enlace y proyectos, sin ficheros). «Instalar en este proyecto» lo vuelve a pedir a Fab. | REQ-107 |
| **«Preparar» y 5.9** | Reactiva `Fab` y `EOSShared` si el proyecto los desactiva. En 5.9 sirve el mismo cliente, porque la API pública de Fab es la misma. | REQ-108 |
| **Encontrar el motor** | GUID o nombre registrado (registro de Windows, `Install.ini`), asociación vacía, y **elegir la carpeta a mano**, guardada en Documentos (que no se virtualiza en la Store). | REQ-118, REQ-119 |
| **Diagnóstico** | Pasos en orden con la solución de cada fallo y **«Copiar diagnóstico»** sin claves ni carpeta ni nombre de usuario. | REQ-120 |
| **Plugin C++ compilado** | Binarios de `DBVUnrealAICpp` para 5.8 y la 5.9 Preview (1 MB cada uno). «Preparar» los instala sin `Source/` si el `BuildId` casa con el del editor, así que no hace falta Visual Studio, también en proyectos de solo Blueprints. | REQ-121 |

## Cómo probarlo

1. `npm install` y `npm run tauri dev`. Abre `DBVMCPFirst` (5.8.4), pulsa **Preparar** con la variante C++ (instala el plugin ya compilado) y abre el editor.
2. En el editor, pulsa el botón **Fab** del Content Browser (sin él no hay sesión de Fab).
3. En la conversación, escribe «quiero un coche gratis para Unreal»: salen tarjetas con miniatura y «Ver en fab.com ↗». «Instalar» pide tu aprobación y deja el modelo en `/Game/Fab/<Título>/`, con un `BP_<Título>` si viene por piezas.
4. Abre la vista **Fab**: busca, abre una ficha, instala; en **Mi biblioteca** salen tus productos. Lo instalado aparece en la **Biblioteca** con la píldora Fab.
5. Copia un proyecto y pon un GUID inventado en su `EngineAssociation`: la vista Proyecto explica el problema y ofrece **«Elegir la carpeta de Unreal…»**. Prueba **«Copiar diagnóstico»** y mira que no sale tu nombre de usuario.

## Verificación

- **Automática (2026-11-15):** Rust 382 tests (`cargo test --lib`; más 5 en vivo con `--include-ignored`), Vitest 487, Python 212, Clippy sin avisos y `dbv-lint` sin errores. Automation `DBV.Fab.*` en 5.8.4 y en la 5.9 Preview.
- **Eval de Fab con modelos reales** (`scripts/eval/fab-eval.mjs`): `qwen3:8b` 7/7 y `qwen2.5:3b` 4/7. En los dos, 0 instalaciones sin aprobación, 0 productos de pago dados por instalados, 0 totales inventados y 0 ofertas de compra.
- **En vivo en `DBVMCPFirst` (5.8.4):** sesión de Fab tras el botón Fab; biblioteca 200; `glb` «car», `fbx` «Camo muscle car» (con `BP_Camo_muscle_car` de 5 piezas), roca de Quixel en calidad media y el pack «Low Poly Cars Drivable» (30 assets) instalados. Rechazados sin descargar uno de pago que no se tiene y uno solo de 4.27. Cancelado al 33 % un pack de 360 MB sin dejar nada en `Content/`. Token 0 veces en el log. La referencia en la Biblioteca se crea con solo la miniatura.
- **Plugin compilado:** proyectos mínimos de solo Blueprints en 5.8.4 y en la 5.9 Preview cargan las DLL sin `Source/` y pasan `DBV.Fab.*`.
- **Revisión de `/code-simplify` (2026-11-15):** 0 Críticos. Corregidos dos Importantes (el diagnóstico podía no anonimizar; un error de biblioteca salía como «no lo tienes»). Tres anotados en el CHANGELOG («Conocido»).

## Límites conocidos y lo que NO está comprobado

- **Las tarjetas de la conversación, la vista Fab, el botón «Elegir la carpeta de Unreal…» y el diagnóstico no se han visto aún dentro de la app** (`tauri dev`); su lógica está cubierta por Vitest y el resto, en vivo por el `--mcp`.
- **La lectura del registro de Windows desde el MSIX de la Store** no se ha comprobado en un equipo con un motor registrado.
- **Fab en 5.9 con nuestro cliente** compila y carga, pero no se ha probado una instalación en `PruebaUnreal59` (allí funciona el `FabToolset` de Epic).
- **Los binarios de la 5.9 Preview** valen solo para esta Preview (BuildId 60351404). Con otra, «Preparar» copia el código y el diagnóstico lo dice.
- **Cursor y OpenCode** siguen sin probarse con sus CLI instaladas, y la línea diagonal de *Fantasy Seaside Golden Sands* sigue por revisar (de la v0.6.0).

## Compatibilidad UE 5.8.4 × 5.9 (REQ-116)

> Proyectos: `DBVMCPFirst` (5.8.4; no se abre nunca con 5.9) y `PruebaUnreal59` (5.9 MegaJam Preview, C++). Fixtures de 5.9 en `tests/fixtures/unreal-mcp/ue-5.9/`. Estado al 2026-11-15.

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
| Fab: buscar, biblioteca, instalar (cliente propio) | ✅ | ⏳ (compila y carga; en 5.9 va el `FabToolset` de Epic) | En vivo en `DBVMCPFirst`; Automation `DBV.Fab.*` en los dos |
| Plugin C++ ya compilado, sin `Source/` | ✅ | ✅ (esta Preview) | Proyectos de solo Blueprints en los dos motores |
| Motor por GUID, nombre o elegido a mano | ✅ | ✅ | Tests de `discovery.rs` y `engine_choice.rs` |
