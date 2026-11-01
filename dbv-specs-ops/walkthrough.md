# 🧭 Walkthrough — DBV Unreal AI v0.3.0

> **Fecha:** 2026-11-01 · **Versión:** 0.3.0 (Minor, desde la 0.2.4) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.12 y §3.13) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md) · **Notas:** [`releases/v0.3.0.md`](./releases/v0.3.0.md)

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA (modelos locales, de nube o agentes por suscripción como Claude Code, Codex o Antigravity) a proyectos abiertos de Unreal Engine 5.8 por su servidor MCP. La IA actúa en el editor **con permiso**, genera assets con Tripo y mundos con World Labs, y lo lleva todo al nivel.

## Qué se entrega en v0.3.0

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| Código del proyecto | La IA propone cambios solo en `Source/`, `Content/Python/` y `Config/*.ini` (diff revisable, nunca `.uasset` ni `Plugins/`). Al aceptar, la app compila: Live Coding si solo cambian `.cpp`; si cambian cabeceras, cierra el editor, compila con UBT y lo reabre. Los errores vuelven a la IA | REQ-22 |
| Recetas guiadas | «📋 Recetas»: nivel de prueba, personaje de Tripo animado, caminar por un mundo de World Labs, MetaHuman con Convai. Paso a paso, con permiso, y comprobación final en el editor; también como AgentSkills del editor | REQ-23 |
| Guías de Unreal sin red | `search_unreal_guides` busca (BM25) en una copia local de las guías de UE 5.8 y cita la URL de cada fragmento | REQ-25 |
| Gaussian Splatting por GPU | Render propio (módulo `DBVUnrealAICppRender`): subida única, proyección y ordenación en GPU, armónicos esféricos de grado 1 a 3, 10 M de splats a ~57–70 fps, sin parpadeo en Play | REQ-61, REQ-62, REQ-63 |
| Mundos de World Labs | Colocados a escala métrica con el suelo en Z = 0, colisión alineada y `PlayerStart` sobre el terreno. Cliente según la API real (`WLT-Api-Key`, `world_prompt`, `remaining_credits`); clave, tope y saldo propios; `worldlabs_balance` para agentes; tarjeta con tiempo transcurrido (la API no da porcentaje) | REQ-18 |
| Agentes más ligeros | Las consultas del MCP de la app no piden permiso; `describe_toolset` compacto y tope de resultados (79 % menos texto en el guion fijo); contador de herramientas al pie de cada respuesta | REQ-12, REQ-51 |
| Convai y zonas | Detección del plugin, reglas en la conversación, personajes en Ajustes, tutorial F1; lugares con nombre en el nivel | REQ-55..REQ-59, REQ-57 |
| Generar | Logos de cada servicio y «Mejorar con IA»: una idea corta → prompt elaborado, siempre fuera de la conversación (con un agente, en una sesión ACP aparte sin herramientas) | REQ-65, REQ-66 |
| Chino simplificado | Interfaz, ayuda, conversación y recetas en 中文 | REQ-64 |
| Biblioteca y diagnóstico | Biblioteca en `Documentos/DBV Unreal AI/Library` (visible fuera del MSIX), migración sin borrar; log local rotativo sin claves; aviso cuando un `.mcp.json` da a los agentes un acceso directo al editor, con qué hacer | REQ-53, RNF-08, REQ-52 |

## Cómo probarlo

1. `npm install` y `npm run tauri dev` (en la raíz). La app es de escritorio: no hay scripts `start`/`stop`.
2. Abre un proyecto de Unreal 5.8 y conéctalo desde **Proyecto** («Preparar» si hace falta).
3. **Ajustes:** conecta una IA; pon las claves de Tripo y de World Labs (cada una con su saldo y su tope).
4. **Generar → Escenario 3D:** escribe una idea, pulsa «✨ Mejorar con IA» y luego Generar; un mundo tarda unos 5 minutos.
5. **Conversación:** pide un cambio de C++ («crea un actor que gire») y acepta la propuesta: la app compila. Prueba también «📋 Recetas».
6. **Biblioteca → Enviar a Unreal** un mundo: el splat queda a escala, caminable en Play.

## Verificación

- **Automática (en verde el 2026-11-01, antes del tag):** Rust 233 (`npm test`), Vitest 366 en 43 ficheros (`npm run test:ai`), Python 47 (`npm run test:python`), `dbv-lint` sin errores, `cargo clippy --all-targets` sin avisos y banco e2e correcto (`npm run test:e2e`).
- **En vivo con el usuario (UE 5.8.4, `DBVUnrealAITest1`):**
  - Render GPU de splats: la iglesia de 9.975.598 splats en viewport, Play y captura de la app; desierto de World Labs caminable con un personaje.
  - Código: la IA creó `ADemoActor`, la app cerró el editor, compiló con UBT y lo reabrió; un cambio de `.cpp` compiló con Live Coding.
  - Recetas: las cuatro comprobaciones y `ListSkills` con las cuatro recetas en el editor.
  - World Labs: dos mundos `marble-1.1` reales (uno desde un agente y otro desde Generar, 309 s), saldo visible en la app.
  - «Mejorar con IA» con Antigravity sin salir de Generar; chino revisado por el usuario.
- **Sin verificar:** evals con un modelo real en esta versión, revisión nativa del chino, macOS y Linux en máquina real.

## Revisión de código

- `/code-simplify` de la v0.3.0 y de su ampliación: 0 Críticos; los Importantes resueltos están en el CHANGELOG (pie del turno perdido al reabrir, consolas visibles en Windows, `Build.bat` con ventana…).
- Correcciones de las pruebas del usuario el 2026-11-01, con test de regresión cada una: World Labs con la clave y el tope de Tripo, cliente de World Labs contra una API supuesta, «Mejorar con IA» que saltaba a la conversación, tarjeta del mundo en «0 %» y aviso de accesos duplicados que confundía.

## Pendiente para la siguiente versión

- Reanudar un trabajo huérfano de World Labs (hoy, si la app se cierra a mitad, el mundo se genera pero no se descarga).
- Canales de distribución (Homebrew, Microsoft Store, Uptodown) y actualizaciones automáticas.
- Evals con un modelo real y revisión nativa del chino.
