# 🧭 Walkthrough — DBV Unreal AI v0.4.1

> **Fecha:** 2026-11-07 · **Versión:** 0.4.1 (Patch, desde la 0.4.0) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.16) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md) · **Notas:** [`releases/v0.4.1.md`](./releases/v0.4.1.md)

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA (modelos locales, de nube o agentes por suscripción como Claude Code, Codex o Antigravity) a proyectos abiertos de Unreal Engine 5.8 por su servidor MCP. La IA actúa en el editor **con permiso**, genera assets con Tripo, mundos fotorrealistas con World Labs, y efectos de sonido y audio, y lo traslada todo al nivel de Unreal Engine.

## Qué se entrega en v0.4.1

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| Visibilidad de saldo ElevenLabs & Créditos locales | Consulta en vivo de suscripción y caracteres restantes (`character_count`, `character_limit`) a la API de ElevenLabs con visualización en cabecera de Generar (soporta los 10k gratuitos mensuales y maneja amigablemente la falta de permiso `user_read`), e indicador de «Saldo local: ∞ créditos» para el microservicio local de audio. | REQ-77 |
| Optimización de caché de balances | Retención en memoria de las cuotas consultadas para Tripo, World Labs y ElevenLabs evitando peticiones repetitivas de red al cambiar de pestaña, con invalidación y refresco reactivo tras completar cualquier generación. | REQ-77 |
| Panel completo de síntesis & Presets para audio local | Chips rápidos de videojuegos (`Laser Blaster`, `Puerta Hidráulica`, `Trueno`, `Retro 8-bit`, `Espada`), campo de prompt negativo con chips acústicos (`Sin música ni voz`, `Sin ruido ni distorsión`, `Seco / Para Unreal`), sliders sincronizados para duración, pasos de difusión (`steps`) y escala de guía (`cfg_scale`), más semilla aleatoria determinista (`seed`). | REQ-76 |
| Aislamiento de plugins DBV Unreal AI para empaquetado | `DBVUnrealAICpp.uplugin` declara sus módulos C++ como `"Type": "Editor"` con `"TargetAllowList": ["Editor"]`, y `prepare.rs` (`enable_plugins`) limita `DBVUnrealAI` y `DBVUnrealAICpp` al Editor en `.uproject`, permitiendo empaquetar juegos standalone (Android / Meta Quest VR) sin errores de dependencias de editor. | Fix |
| Resiliencia de Claude Code en Windows | Auto-inyección de la variable de entorno `CLAUDE_CODE_EXECUTABLE` al llamar al adaptador ACP de Claude Code para resolver el binario nativo en Windows y propagación de errores detallados. | Fix |
| Corrección de estado local | Corrección en `generateView.js` y `settingsView.js` evaluando `st?.value?.ready` en lugar de `st?.ready` para la detección del microservicio de audio local. | REQ-77 |

## Cómo probarlo

1. `npm install` y `npm run tauri dev`.
2. Ve a **Generar** → pestaña **Sonidos (SFX)**:
   - Con **Local (`dbv-audio-service`)** seleccionado: prueba los chips de presets (ej. `🔫 Laser Blaster`), los filtros negativos (ej. `🏜️ Seco / Para Unreal`) y ajusta sliders de duración, pasos o CFG.
   - Observa el indicador **«Saldo local: ∞ créditos»**.
   - Con **ElevenLabs** seleccionado: introduce una clave con permiso `user_read` y comprueba que se muestra el saldo real (ej. `Saldo: 10.000 car.`); si la clave no tiene `user_read`, se muestra un mensaje informativo permitiendo generar con normalidad.
3. En **Proyectos de Unreal**: prepara un proyecto y comprueba que en el `.uproject` los plugins `DBVUnrealAI` y `DBVUnrealAICpp` incluyen `"TargetAllowList": ["Editor"]`, permitiendo empaquetar para Android/Meta Quest sin errores.

## Verificación

- **Automática (2026-11-07):**
  - Rust: 243 tests pasados (`cargo test`), `cargo clippy` sin advertencias.
  - Vitest: 386 tests pasados en 43 suites (`npx vitest run`).
  - Python: 48 tests pasados (`pytest tests/python`).
  - Linter: `python dbv-specs-ops/scripts/dbv-lint.py unreal-plugin tests/python` limpio (0 errores, 0 avisos).
  - Trazabilidad Spec-to-Test: 100% (92 requisitos con ID, 90 con tests).
