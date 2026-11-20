# Estudio previo: UEFN y Verse para la v0.7.0

> **Fecha:** 2026-11-19 · **Fuente:** los 5 repositorios públicos de Magnus Enebakk (Epic Games), `github.com/magnusenebakk-epic`, clonados y leídos en esta fecha.
> **Para qué:** dar base al `/spec` de la v0.7.0 (UEFN y Verse, `REQ-78`, `REQ-79`, spike `P-14`, `S92`). No es una especificación: recoge hechos, consecuencias para la app y decisiones pendientes.

## 1. Qué hay en cada repositorio

| Repo | Qué es | Tamaño | Licencia | Uso para nosotros |
|---|---|---|---|---|
| **verse-skill** (27 ★, el principal) | Skill de Claude Code para Verse, Scene Graph y las API de Verse/UnrealEngine/Fortnite. `SKILL.md` (índice y reglas que más fallan) + 25 ficheros de referencia en `language/`, `uefn/`, `apis/`, más `gotchas.md`, `patterns.md` y `error-codes.md` | ~350 KB de markdown | **Ninguna** | Fuente de conocimiento principal. **No se puede empaquetar ni copiar** a nuestro repo; sí instalar en la máquina del usuario desde GitHub o enlazar |
| **versebook** | Fuente del libro oficial de Verse (verselang.github.io/book): 18 capítulos + **copia literal de la batería de tests del compilador** (172 ficheros y carpetas: ~6.300 `assert`, ~3.370 `assert_semantic_error`, ~340 `assert_syntax_error`) | 8,7 MB | **CC0** | Se puede reutilizar libremente: corpus de ejemplos válidos e inválidos para evals y para el contexto de la IA |
| **verse-modules** | ~10.700 líneas de Verse real, Scene Graph-first: ajedrez con bot, crafteo, moneda, habilidades, desbloqueos, propiedad de bases, colocación, intérprete de un mini-lenguaje, estructuras de datos y **pads de prueba** | 122 ficheros | Ninguna | Referencia de estilo y patrones; enlazar, no copiar |
| **UEFNKit** | Script PowerShell (130 KB) para instalar, duplicar, exportar y generar variantes de proyectos UEFN, con modo sin menús (`-Json`, `-Yes`) pensado para agentes (`AGENTS.md`) | 6 ficheros | Ninguna | Nos da los **hechos de detección** de UEFN (rutas, versión, formato de proyecto). Se puede llamar a la copia del usuario |
| **learn-verse** | Página con enlaces: libro, guías de Epic, referencia de API, Scene Graph, foros, Discord | 1 HTML | Ninguna | Enlaces para la documentación de usuario |

La skill está escrita para **UEFN 42.30**, con etiquetas `[4240+]`, `[4300+]`, `[4310+]`, `[4400+]` donde cambian versiones posteriores. En este equipo está instalado **UEFN 43.10** (`Fortnite_Studio`, CL 60458657) y hay un proyecto en `D:/Programacion/UEFN-Fornite/DBVFornite`.

## 2. Lo que una IA tiene que saber para no fallar en Verse

Verse parece Python, pero es un lenguaje **lógico-funcional**: los hábitos de C, Python o JS fallan al compilar. Lo que la skill marca como causa principal de errores:

1. **Efectos por defecto `<transacts><no_rollback>`.** Una función sin anotar no se puede llamar dentro de un `if`, de un filtro de `for` ni de una función `<decides>`. Toda función `<decides>` necesita también `<computes>`, `<reads>` o `<transacts>`, y lo mismo cualquier función auxiliar llamada desde ellas. Es el error de compilación más frecuente (3512).
2. **No hay booleanos de control:** las expresiones tienen éxito o fallan. `x > 0` devuelve `x` o falla. Las funciones `<decides>` se llaman con `[]`, las totales con `()`.
3. **Números:** `int / int` da `rational` y puede fallar; solo `*` mezcla `int` y `float`; no existe `ToFloat()`. `1 = 1.0` compila pero siempre falla.
4. **Sin sombreado de nombres** en ningún ámbito, incluidos los nombres de carpetas del Content Browser (las carpetas son módulos).
5. **`for` es una expresión que devuelve un array**: sin `break` (solo en `loop`) ni `continue`. Un filtro que falla salta el elemento, no aborta.
6. **Concurrencia estructurada** (`sync`, `race`, `rush`, `branch`, `spawn`) sin forma iterativa; `<suspends>` y `<decides>` son incompatibles.
7. **Límites de ejecución:** ~120 marcos de pila, 100 millones de iteraciones por tick y 9 s por reanudación. Superarlos es un error fatal que detiene todo el Verse de la sesión.
8. **Dos `vector3` distintos:** `/Verse.org/SpatialMath` (Forward/Left/Up) y el obsoleto `/UnrealEngine.com/Temporary/SpatialMath` (X/Y/Z), que devuelve `fort_character`.
9. **Los ficheros `*.digest.verse` del build del usuario son la única fuente fiable de firmas.** Hay que buscar en ellos (`grep`) antes de suponer que una API existe o no.
10. **Diagnosticar el error que dio el compilador**, por su código, sin inventar otros. Y nunca decir que el código compila sin haberlo compilado.

**Cambio de paradigma:** Epic recomienda ahora **Scene Graph primero**: lógica en componentes (`class<final_super>(component)`) sobre entidades y prefabs, que reaccionan a eventos y se comunican con eventos de escena. **No** escribir la jugabilidad en clases `creative_device`. Los dispositivos de Epic se usan solo como acceso a API que Scene Graph aún no cubre (fin de ronda, aparición de jugadores, incorporación tardía), mediante campos `@editable`.

## 3. El servidor MCP del propio UEFN

UEFN ya trae un servidor MCP en el editor (toolsets con el prefijo `Valkyrie`). Es la pieza que hace posible la v0.7.0 sin plugin propio:

- **Activación:** Project Settings → **Python Editor Scripting** y **UEFN MCP Toolsets**. Escucha en `http://127.0.0.1:8000/mcp`. **Solo el primer editor abierto** sirve el puerto 8000; al reiniciar el editor hay que abrir una sesión MCP nueva.
- **Toolsets oficiales:** `VerseToolset` (leer, escribir, buscar y borrar `.verse`, y `BuildAll`), `EntityToolset` (entidades, componentes, propiedades, transformaciones), `SessionToolset` (Launch Session, iniciar y parar partida, `PushChanges`) y `DeviceToolset` (colocar dispositivos de Epic y enlazarlos a campos `@editable`).
- **Toolsets no documentados** (pueden cambiar): logs, captura del viewport, Python, actores, assets, materiales, mallas, texturas, Niagara, UMG, MVVM, VerseFields, GameplayTags, física y un `ProgrammaticToolset` que ejecuta Python en un entorno restringido.
- **Bucle de compilación:** `BuildAll` tarda unos 3 s y devuelve una lista de diagnósticos con gravedad, código, mensaje, fichero y posición. Una lista vacía significa que compila.
- **Bucle de ejecución:** `StartSession` (la primera preparación puede tardar más de 2 min) → `PushChanges {"bVerseOnly":true}` (~10 s) → leer la salida de Verse con `LogsToolset.GetLogEntries(category="LogVerse")`. La salida va al log **del editor**, no al del cliente de Fortnite.
- **Trampas medidas:**
  - **No borrar ni mover `.verse` en disco con el editor abierto**: a partir de ahí todas las compilaciones fallan. Hay que borrar con `VerseToolset.Delete`.
  - Si la validación de `StartSession` falla, o tras un `Err` en Verse, UEFN abre un **diálogo modal y todas las llamadas MCP se quedan colgadas** hasta que una persona lo cierra.
  - No llamar a `BuildAll` con una sesión en marcha.
  - Guardar un nivel grande bloquea el MCP unos 2 minutos.
  - El MCP **no puede simular la entrada del jugador**: las pruebas de entrada se dejan como guion manual.
  - **No hay herramienta para publicar**: el usuario sube la versión privada a mano y publica desde el Creator Portal.

## 4. Detección de UEFN (según UEFNKit)

- **Proceso:** `UnrealEditorFortnite*`.
- **Versión instalada:** entrada `Fortnite_Studio` de `%ProgramData%\Epic\UnrealEngineLauncher\LauncherInstalled.dat` (patrón `++Fortnite+Release-<ver>-CL-<cl>`). Como alternativa, la línea `LogInit: Build:` de `%LOCALAPPDATA%\UnrealEditorFortnite\Saved\Logs\UnrealEditorFortnite.log`.
  - **Ojo:** la entrada `Fortnite` es el juego, no el editor.
- **Carpetas de proyectos:** sección `[/Script/ValkyrieEditor.ValkyrieEditorConfig]` de `%LOCALAPPDATA%\UnrealEditorFortnite\Saved\Config\WindowsEditor\EditorPerProjectUserSettings.ini`: `LastCreatedProjectLocation` y `AdditionalProjectPaths`. Si no están, se usa `Documentos\Fortnite Projects`.
- **Un proyecto** es una carpeta con `<Nombre>.uefnproject` (JSON: `projectId`, `projectVersePath`, módulos, `compatibilityVersion`), un `.uplugin` y `Content\`. El nombre del plugin puede no coincidir con el del proyecto.
- **Estado del editor fuera del proyecto:** `%LOCALAPPDATA%\UnrealEditorFortnite\Saved\VerseProject\<Nombre>` (espacio de trabajo de Verse, donde probablemente están los digests), `VerseSnapshot`, `VK_Projects` y `Autosaves`.

## 5. Contraste con lo que ya tenemos escrito

1. **`ARCHITECTURE.md` describe código de UEFN que no existe.** Cita `discovery.rs`, `prepare.rs`, `files.rs` y `unrealTools.js` con soporte UEFN, pero el repositorio no contiene ni una referencia a `uefn`, `.uefnproject` ni `UnrealEditorFortnite`. Hay que corregirlo en el `/spec`.
2. **`REQ-79` está anticuado:** su prompt de dominio (`uefnPrompt`) enseña a crear `<nombre>_device := class(creative_device)` con `OnBegin`, justo lo que Epic desaconseja. Además usa el `SpatialMath` obsoleto y no menciona la regla de efectos por defecto, que es el error número uno. Hay que reescribirlo con el enfoque Scene Graph primero.
3. **`REQ-78`** acierta con el puerto 8000 y el cliente MCP existente (`mcp_client.rs`). Falta confirmar en el spike `P-14` si UEFN expone las herramientas directamente (`VerseToolset.BuildAll`…) o mediante `list_toolsets`/`describe_toolset`/`call_tool`, como el MCP de UE 5.8/5.9 que ya usamos.
4. **`files.rs` escribe en disco.** En UEFN, borrar o mover en disco rompe el editor, así que con el editor abierto los cambios en `.verse` deberían pasar por `VerseToolset.WriteFile`/`Replace`/`Delete`. El spike debe comprobar también si el editor detecta un fichero nuevo escrito en disco.
5. **El play-testing autónomo en PIE** (v0.5.0) tiene su equivalente en UEFN: Launch Session + `PushChanges` + `LogVerse`, con mensajes de prueba etiquetados. Los pads de prueba de `verse-modules/Testing` son un buen modelo.
6. **«Enviar a Unreal» con assets de Tripo** pasaría por los toolsets de assets no documentados. Hay que vigilar el límite de memoria de la isla (100.000 unidades).

## 6. Consecuencias para la v0.7.0 (propuesta para el `/spec`)

| # | Bloque | Qué haría la app | Base en el estudio |
|---|---|---|---|
| A | Detectar y conectar | Encontrar UEFN, su versión y sus proyectos como en §4; conectar al MCP del editor; preparación que comprueba los dos ajustes de Project Settings y el puerto 8000, y avisa si hay dos editores abiertos | §3, §4 |
| B | Conocimiento de Verse | Botón para **instalar la skill de Epic** (`git clone`, actualizable con `git pull`) en `.claude/skills/verse` del proyecto o del usuario. Para Codex, Gemini o modelos locales, cargar desde esa copia local los ficheros de referencia que tocan. Más un prompt de dominio propio y corto con las 10 reglas de §2 | §1 (sin licencia: no se empaqueta), §2 |
| C | Consultar la API real | Herramienta para la IA que busca en los `*.digest.verse` del build del usuario (con `VerseToolset.Grep` o en disco) | §2 regla 9 |
| D | Compilación en bucle cerrado | Escribir con `VerseToolset` → `BuildAll` → devolver los diagnósticos a la IA → repetir hasta N intentos. La tarjeta de diff muestra el resultado real; nunca se dice «compila» sin `BuildAll` | §3 |
| E | Probar en sesión | `StartSession`/`PushChanges` + lectura de `LogVerse` con etiquetas; tiempos de espera y aviso de «revisa el editor, puede haber un diálogo abierto» | §3, §5.5 |
| F | Recetas Scene Graph primero | Rehacer las recetas de `REQ-79` como componentes (zona de puntuación, seguidor de jugadores con incorporación tardía, temporizador con `race`), cada una con su plan de prueba | §2, `building-games.md` |
| G | Evals | Conjunto de tareas de Verse que se aceptan solo si `BuildAll` devuelve `[]`. Ejemplos sacados del corpus CC0 de versebook | §1 |

## 7. Decisiones pendientes para el `/spec`

1. **Alcance:** ¿A–D y F–G en la v0.7.0 y E (probar en sesión) en la v0.7.x? ¿O todo junto?
2. **Licencia de la skill:** ¿la instalamos desde GitHub en la máquina del usuario (no redistribuye nada) o pedimos a Magnus que añada una licencia para poder empaquetarla?
3. **UEFNKit:** ¿integrar sus comandos con `-Json` (instalar el demo *MobileGamesTD*, duplicar proyectos) llamando a la copia del usuario, o quedarnos solo con la detección propia?
4. **Spike `P-14`:** necesita UEFN 43.10 abierto con `DBVFornite` y los dos ajustes activados para ver la forma real de las herramientas MCP, si existen los toolsets no documentados y dónde están los digests en disco.
