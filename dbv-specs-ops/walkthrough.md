# 🧭 Walkthrough — DBV Unreal AI v0.6.2

> **Fecha:** 2026-11-15 · **Versión:** 0.6.2 (Patch: arreglo urgente de la 0.6.1, que a su vez fue Patch desde la 0.6.0) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.23 y §3.25) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md) · **Notas:** [`releases/v0.6.2.md`](./releases/v0.6.2.md) y [`releases/v0.6.1.md`](./releases/v0.6.1.md) · **Plan:** [`implementation_plan.md`](./implementation_plan.md) §19 y §20

## Qué cambia en v0.6.2

Un usuario de la Microsoft Store (**Victor Osuna**) probó la v0.6.1 recién aprobada y Claude Code no arrancaba: «"C:\Program" no se reconoce como un comando interno o externo».

| Qué | Detalle |
| --- | --- |
| **Causa** | `detect::launcher` lanzaba `npx.cmd` como `cmd.exe /d /s /c <ruta> …`. Con Node en `C:\Program Files\nodejs` (la instalación por defecto), `/s` quita las comillas y Windows ejecuta `C:\Program`. Afectaba a Claude Code y a Codex (los dos van por `npx`) y a cualquier CLI instalada como `.cmd` en una ruta con espacios. |
| **Arreglo** | `launcher` se elimina: el `.cmd` se lanza por su ruta y `std::process::Command` lo pasa por `cmd.exe` con las comillas y el escapado correctos (`ai/detect.rs`, `ai/acp.rs`). De paso, un argumento con `&` ya no lo interpreta `cmd.exe`. |
| **Por qué no lo vieron los tests** | En el equipo de desarrollo Node está en `C:\nodejs` y el test solo miraba los argumentos. El test nuevo (`un_cmd_en_una_carpeta_con_espacios_se_lanza_y_recibe_sus_argumentos`) lanza de verdad un `.cmd` desde `Program Files\nodejs`. |
| **Conector MCP con la ruta antigua** | El segundo problema que detectó: Claude Desktop apuntaba a un ejecutable que ya no existe. La app no escribe esa configuración; se resuelve copiando otra vez el bloque de Ajustes. Documentado en el README; sin cambios de código. |
| **Plugins de Unreal** | Sin cambios: siguen en `0.6.1` (`DBVUnrealAI` Version 9, `DBVUnrealAICpp` Version 5) para que nadie tenga que volver a preparar sus proyectos. |

**Comprobado:** reproducción con el `npx` real detrás de una ruta con espacios (la orden antigua da el error del usuario; la nueva devuelve la versión). **No comprobado:** la app instalada arrancando Claude Code de principio a fin en un equipo con Node en `Program Files`.

Lo que sigue describe la v0.6.1, que es todo lo demás que lleva esta versión.

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


## UEFN y Verse (v0.7.0, en construcción — REQ-122..REQ-131)

> Construido el 2026-11-20 (S152..S164, un commit por rebanada). Validado en vivo con **UEFN 43.12** (el Launcher dice 43.10: ver REQ-122) y el proyecto de pruebas `DBVUnrealAITest` (`D:\Programacion\UEFN-Fornite\DBVUnrealAITest`), nunca en `DBVFornite`. Tests: Rust 444, Vitest 530.

| Qué | Estado | Cómo se comprobó |
| --- | --- | --- |
| Detectar UEFN, versión y proyectos (REQ-122) | ✅ en vivo | `uefn_overview` en esta máquina: 43.12 por `Build.version`, `DBVFornite` y `DBVUnrealAITest` (abierto). Fortnite ya no sale como «UE 6.0» |
| No leer la línea de comandos de UEFN (REQ-122) | ✅ | Test: el filtro de procesos va antes de leer argumentos |
| Reconocer el MCP de UEFN y diagnóstico (REQ-123) | ✅ tests · ⏳ en la app | Fixtures `uefn-43.10` (63 toolsets) y `ue-5.8.4`; puerto ocupado en un log falso. Falta verlo en `tauri dev` |
| Herramientas de la IA con UEFN (REQ-123) | ✅ tests | `--mcp` contra un UEFN falso: oculta las del plugin y explica por qué |
| Skill de Verse de Epic (REQ-124) | ✅ tests · ⏳ en vivo | Repositorio git local en ruta con espacios, zip sin git; falta instalarla de GitHub desde la app |
| Prompt de Verse y `verse_reference` (REQ-125) | ✅ tests | Skill falsa: temas, desconocido, sin skill, tope |
| `verse_api_search` (REQ-126) | ✅ en vivo | Digests reales: `GetFortCharacter` → su firma en `Characters`, build 43.12 |
| Escribir y compilar Verse en bucle (REQ-127) | ✅ en vivo | `WriteFile` por MCP de un `zz_dbv_live.verse` con `IsHigh(S)` sobre `<decides>` → **3511** en `Content/zz_dbv_live.verse:6`; con `IsHigh[S]` → `[]`; `Delete` por MCP |
| Sesión de UEFN (REQ-128) | ✅ en vivo | `uefn_session_start` 102 s → `Connected · CanStart`; `uefn_session_push` con la partida sin empezar: «Refresh … not currently available» → se envió todo y apareció `dbv_spike_device.verse:8 ZZTEST hola` en 30 s; `uefn_session_stop` |
| Recetas de UEFN (REQ-129) | ✅ tests · ⏳ en vivo | Falta que la IA siga las recetas 1–3 y la 4 importe un `.glb` en `DBVUnrealAITest` |
| Evals de Verse (REQ-130) | ✅ jueces · ver §8 de la spec | `qwen3:8b` en local; falta un modelo en la nube |
| UEFNKit (REQ-131) | ✅ tests · ⏳ en vivo | Script falso en ruta con espacios. **El script real no se ha ejecutado:** la herramienta de la sesión no permite lanzar con `-ExecutionPolicy Bypass` un script descargado; lo prueba el usuario desde la app (`duplicate` de `DBVFornite`) |

**Lo que se vio en vivo y cambia el diseño:**

- `()` sobre una función `<decides>` es el error **3511**; el 3512 es otro caso (una función `<decides>` llamada desde un contexto que no puede fallar).
- Tras arrancar la sesión, la partida puede quedar en `CanStart`; enviar solo Verse entonces falla y hay que enviar todo (el plan B ya lo hace solo).
- `EntityToolset` marca **todas** sus herramientas como `execute` en 43.x, también las de leer: con UEFN, leer entidades pide aprobación.
