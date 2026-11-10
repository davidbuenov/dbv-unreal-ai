# 🧭 Walkthrough — DBV Unreal AI v0.5.0

> **Fecha:** 2026-11-10 · **Versión:** 0.5.0 (Minor, desde la 0.4.1) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.17..§3.20) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md) · **Notas:** [`releases/v0.5.0.md`](./releases/v0.5.0.md)

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA (modelos locales, de nube o agentes por suscripción como Claude Code, Codex o Antigravity) a un proyecto abierto de Unreal Engine 5.8, con permiso antes de actuar, generación 3D (Tripo, World Labs), audio y un servidor MCP propio.

## Qué se entrega en v0.5.0

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| **Caja negra** (grabar el juego) | Botón **⏺ Caja negra** junto a «Probar en Play». Graba el juego **sin tocar el proyecto**: personaje, cámara, objetos, sockets, variables de Blueprint, FPS y eventos (pausa, modo de movimiento, base). Formato propio por bloques con CRC (`frames.bin`), `meta.json` y `events.jsonl` en `Saved/DBVUnrealAI/Sessions/`. Un fallo en una señal la apaga con un solo aviso; límites de 10 min / 500 MB / 20 sesiones con topes duros. | REQ-91 |
| **Marcar y terminar** | Tecla **F10 dentro del juego** (configurable con `mark_key`) que añade una marca numerada sin quitarle el foco; panel fijo con tiempo, marcas y «Marcar momento» / «Terminar», que se enciende leyendo ficheros (también si grabó un agente por `--mcp`). Mientras graba quita «Usar menos CPU en segundo plano» (en memoria) para que el editor no caiga a ~3 FPS al perder el foco, y lo restaura. | REQ-91, REQ-94 |
| **Análisis con datos** | En Rust, sobre los ficheros (sirve con el editor cerrado): `gaps` (¿es fiable la medición?, por tiempo y por tramos), `frame_budget`, `stillness`, `oscillation`, `jumps`, `follow`, `transitions`, `timeline`, `around_marks`. Respuestas de menos de 2,5 KB; `gaps` va siempre primero. | REQ-93 |
| **Diez herramientas `session_*`** | `session_plan`, `start`, `status`, `mark`, `stop`, `wait`, `analyze`, `query`, `list`, `delete`: una sola definición para el asistente integrado y el MCP propio (agentes). Grabar y borrar piden aprobación. | REQ-92 |
| **Instrucciones y receta** | Receta `runtime_diagnosis`, prompt de depuración y regla contra callbacks por fotograma; `docs/IA.md` y `docs/IA.en.md`. | REQ-97 |
| **Vestir MetaHumans** | Herramienta determinista (`unreal_dress_metahuman`), elegir a quién se viste, cinco tipos de prenda, copia segura, piel limpia, tela con Chaos Cloth (faldas y vestidos), validación con capturas. **Camisetas y pantalones ceñidos: experimentales** (REJECTED-13, REJECTED-15). | REQ-80..REQ-89, REQ-98 |
| **`unreal_playtest` y registro** | Ahora arranca Play en UE 5.8.2 (`StartPIE` exige `options`) y pide el log con su esquema real. | REQ-82 |

Aplazado a la **v0.5.1**: visor VR con marca desde el mando (REQ-95), aislar/conducir/comparar (REQ-96), informe de memoria gráfica (REQ-99), ajuste de la tecla de marca, tarjeta del plan editable, historial en la interfaz y los análisis `correlate`, `wrong_axis` y `relative_yaw`.

## Cómo probarlo

1. `npm install` y `npm run tauri dev`. Abre un proyecto de Unreal 5.8 y pulsa **Preparar** (copia el plugin nuevo, v0.5.0).
2. Con el editor conectado, abre la conversación y pulsa **⏺ Caja negra → En el editor (PIE)**. Contesta a la IA y acepta la tarjeta de `session_start`.
3. Haz **un clic dentro del juego** para que reciba el teclado, muévete 20–30 s y pulsa **F10** un par de veces cuando notes algo. Termina con **Terminar** (o cierra Play).
4. La IA responde en tres bloques (lo que notaste / lo que dicen los datos / conclusión) y avisa si la medición no es fiable.
5. Para probar las herramientas sin la ventana: `dbv-unreal-ai --mcp --project <.uproject>` y llama a `session_plan`, `session_start`, `session_stop`, `session_analyze`.

## Verificación

- **Automática (2026-11-10):**
  - Rust: 335 tests (`cargo test --lib`), `cargo clippy --lib --all-targets` sin avisos.
  - Vitest: 426 tests en 44 ficheros.
  - Python: 173 tests de `unittest` en verde y 1 error previo de importación (`test_outfit_skin` necesita `numpy`, que no está instalado en este equipo); `dbv-lint.py` limpio.
  - Banco e2e (`npm run test:e2e`): correcto.
- **En vivo, con el editor abierto (UE 5.8.2, `DBVMCPFirst`):**
  - Spikes P-22 (coste: 0,19 ms con 30 señales) y P-23 (fin de PIE: `get_game_world()` a `None`; un mundo retenido lanza «ObjectInstance is null»).
  - **50 sesiones seguidas** con el `--mcp` real: 50 de 50 bien, 11.430 filas, Play parado al final, ninguna grabación activa, sin errores repetidos en el log. Coste medio de la grabadora 0,25–0,34 ms por fotograma (p95 ≤ 0,46 ms, bajo el objetivo de 0,5 ms).
  - Quitarle el foco al editor 9 s con otra ventana: se mantuvo entre 72 y 81 FPS (antes ~3), y el ajuste volvió a su valor.
  - Tres rondas de prueba con la persona usuaria; salieron y se corrigieron: IA que pedía escribir en un chat bloqueado, desplegable ilegible en el tema sepia, caída a ~3 FPS al perder el foco, F9 que chocaba con su captura de pantalla y un grabador viejo en memoria tras actualizar el plugin.
- **Revisión de `/code-simplify` (2026-11-10):** un hallazgo **Crítico** de seguridad (el nombre de sesión podía salirse de su carpeta) resuelto con tests; varios Importante resueltos (arranque que fallaba a medias, tecla de marca que se apagaba si el personaje aún no existía, «Terminar» que paraba un Play ajeno, topes de límites, `slots=True`) y los demás registrados en `CHANGELOG.md` → «Conocido».
- **Hallazgos al probar contra el editor real:** `unreal_playtest` no podía arrancar Play ni leer el log porque los argumentos de `StartPIE` y `GetLogEntries` no eran los del esquema real; el editor simulado de los tests aceptaba cualquier argumento. Regla: probar contra el editor real.

## Límites conocidos y lo que NO está comprobado

- **Microsoft Store:** REQ-90 exige una matriz de 12 casos de ropa con la app instalada (MSIX), un juego empaquetado y un vídeo; **no se ha ejecutado**. La versión se publica en GitHub y la ficha de la Store está preparada, pero el envío a Partner Center debe decidirlo la persona responsable sabiendo que ese criterio sigue abierto.
- **Prendas ceñidas** (camisetas, pantalones): experimentales; el aspecto no siempre es presentable aunque las métricas den `ok`.
- **Ventana de la app:** el botón, el desplegable y el panel de grabación se han probado con tests y con la persona usuaria en dos rondas; la tarjeta del plan editable y el historial no existen todavía.
- **F10** (la tecla por defecto desde la última ronda) no se ha vuelto a pulsar en una grabación real tras el cambio de F9.
- **Solo PIE o una partida ya abierta en el editor**: no hay grabación en paquetes ni con visor VR.
