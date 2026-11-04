# 🧭 Walkthrough — DBV Unreal AI v0.4.0

> **Fecha:** 2026-11-04 · **Versión:** 0.4.0 (Minor, desde la 0.3.1) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.15) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md) · **Notas:** [`releases/v0.4.0.md`](./releases/v0.4.0.md)

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA (modelos locales, de nube o agentes por suscripción como Claude Code, Codex o Antigravity) a proyectos abiertos de Unreal Engine 5.8 por su servidor MCP. La IA actúa en el editor **con permiso**, genera assets con Tripo, mundos fotorrealistas con World Labs, y efectos de sonido y audio, y lo traslada todo al nivel de Unreal Engine.

## Qué se entrega en v0.4.0

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| Generación de audio & SFX | Nueva pestaña «Sonidos (SFX)» en Generar: microservicio local (`dbv-audio-service`, Stable Audio Open 1.0 en `http://127.0.0.1:8765`), ElevenLabs Text-to-SFX vía API oficial (`/v1/sound-generation`) y catálogo interactivo de Freesound con listado de resultados coincidentes, preescucha nativa HTML5 y descarga selectiva. Guardado en biblioteca (`kind: "sound"`). | REQ-71 |
| Importación de audio a Unreal | «Enviar a Unreal» importa `.wav` directamente a `/Game/DBV/Audio/` como `USoundWave` (o `SoundCue` con bucle). Herramienta MCP `unreal_import_sound` para asociar audio a componentes de actores o reproducir en localizaciones 3D. | REQ-72 |
| Visión multimodal en tiempo real | Botón «📷 Capturar viewport» en la barra de mensajes de la IA que toma captura inmediata del editor de Unreal con `CaptureViewport`, la adjunta con miniatura y la envía a los modelos o agentes con visión. | REQ-73 |
| Autoría de Copyright configurable | Ajustes del Proyecto permite definir el titular de Copyright del usuario (`Copyright (c) 2026 <Usuario>`), inyectando su titularidad en los ficheros generados por la IA mientras se preserva inmutable la atribución de origen a `DBV Unreal AI` y `dbv-specs-ops`. | REQ-74 |
| Autodetección resiliente del puerto MCP | Si el puerto guardado en `.ini` falla o devuelve 404, la app sondea automáticamente el rango `8000..8005` antes de emitir error, sincronizándose al vuelo con el editor abierto. | REQ-75 |
| Centro de Ayuda ampliado | Nuevas secciones de ayuda para Tripo 3D, World Labs y Audio & SFX (en español, inglés y chino) accesibles con botones contextuales directos desde Generar y Ajustes. | REQ-46 |

## Cómo probarlo

1. `npm install` y `npm run tauri dev`.
2. Ve a **Generar** → pestaña **Sonidos (SFX)**.
   - Prueba **Freesound**: busca un sonido (ej. «grua»), escucha los resultados con el reproductor HTML5 integrado y pulsa «+ Añadir a la biblioteca».
   - Prueba **ElevenLabs**: introduce un prompt, duración y bucle, y genera el efecto.
   - Revisa en la **Biblioteca** el audio guardado y pulsa «Enviar a Unreal» para importarlo como asset al proyecto.
3. En la conversación con la IA, con Unreal abierto, pulsa **«📷 Capturar viewport»** para adjuntar la escena actual y razonar sobre ella con modelos multimodales.
4. En **Ajustes**, configura el titular de autoría del proyecto y genera un script o asset para comprobar las 6 líneas de cabecera estándar.

## Verificación

- **Automática (2026-11-04):**
  - Rust: 238 tests pasados (`cargo test`), `cargo clippy` sin advertencias.
  - Vitest: 381 tests pasados en 43 suites (`npx vitest run`).
  - Python: 48 tests pasados (`pytest tests/python`).
  - Linter: `python dbv-specs-ops/scripts/dbv-lint.py --changed` limpio (0 errores).
  - Trazabilidad Spec-to-Test: 100% (90 requisitos con ID, 88 con tests, 0 requisitos nuevos sin test).
