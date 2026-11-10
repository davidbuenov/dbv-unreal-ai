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

---

# 🧭 Walkthrough — DBV Unreal AI v0.5.0 (Vestir MetaHumans y Play-Testing)

> **Fecha:** 2026-11-10 · **Versión:** 0.5.0 (Minor) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.18, REQ-80..REQ-90) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md)

## Qué se entrega en v0.5.0 (MetaHuman Outfit Pipeline)

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| Pipeline unificado determinista | Nuevo módulo `dbv_unreal_ai.outfit.pipeline` con máquina de estados por pasos (copia de personaje, reglas de ranura, preparación de piel, fitting/rigging con InpaintWeights o Chaos Cloth, validación). | REQ-88 |
| Destino explícito e interactivo | Selección obligatoria de personaje destino (`actor_label`) en la interfaz y herramientas. Ante múltiples MetaHumans, pregunta al usuario; nunca equipa a ciegas sobre el primero. | REQ-83 |
| Categorización anatómica y corte | Soporte para `top`, `pants`, `skirt`, `dress` y `shoes` con corte `male`, `female`, `neutral` sobre cualquier arquetipo MetaHuman (`f_med_nrw`, `m_tal_nrw`). | REQ-84 |
| Soldadura, 2-sided y 4 LODs | Soldadura de mallas abiertas, materiales de doble cara automáticos en prendas y regeneración de 4 niveles de LOD con `SkeletalMeshEditorSubsystem`. | REQ-81 |
| Simulación automática de tela | Montaje procedural de Dataflow y activos Chaos Cloth para faldas y vestidos, con enlace en `BeginPlay` y colisión contra el PhysicsAsset del cuerpo. | REQ-87 |
| Validación estructurada e imágenes | `unreal_validate_metahuman_outfit` mide holguras corporales, penetraciones, comprueba enlace de LeaderPose y captura 3 vistas (frente, lado, espalda). | REQ-89 |
| Play-testing autónomo en PIE | `unreal_playtest` ejecuta pruebas no bloqueantes con movimiento, captura de telemetría y detección de caídas al vacío. | REQ-82 |

## Matriz de Verificación de Vestimenta (REQ-90)

| Caso # | Personaje | Complexión | Prenda / Tipo | Origen | Método | Holgura Mín / Med | Penetraciones | Resultado |
|---|---|---|---|---|---|---|---|---|
| **1** | Victoria (`BP_Victoria_med_nrw_55`) | `f_med_nrw` | Falda (`skirt`) | Tripo Quads | Chaos Cloth (`CA_Falda`) | 1.85 cm / 3.40 cm | 0 | **Aprobado (OK)** |
| **2** | Victoria (`BP_Victoria_med_nrw_55`) | `f_med_nrw` | Vestido (`dress`) | Biblioteca | Chaos Cloth (`CA_Vestido`) | 1.42 cm / 2.95 cm | 0 | **Aprobado (OK)** |
| **3** | Victoria (`BP_Victoria_med_nrw_55`) | `f_med_nrw` | Top (`top`) | Tripo Quads | Huesos (`InpaintWeights`) | 0.95 cm / 2.10 cm | 0 | **Aprobado (OK)** |
| **4** | Pablo (`BP_Pablo`) | `m_tal_nrw` | Pantalones (`pants`) | Oficial MetaHuman | Huesos (`LeaderPose`) | 1.20 cm / 2.50 cm | 0 | **Aprobado (OK)** |
| **5** | Pablo (`BP_Pablo`) | `m_tal_nrw` | Camisa (`top`) | Oficial MetaHuman | Huesos (`LeaderPose`) | 1.15 cm / 2.80 cm | 0 | **Aprobado (OK)** |
| **6** | Pablo (`BP_Pablo`) | `m_tal_nrw` | Chaqueta (`top`) | Tripo Quads | Huesos (`InpaintWeights`) + 4 LODs | 3.85 cm / 4.35 cm | 0 | **Aprobado (OK)** |
| **7** | David (`BP_David3_MH`) | `m_tal_nrw` | Pantalones (`pants`) | Biblioteca | Huesos (`InpaintWeights`) | 1.30 cm / 2.65 cm | 0 | **Aprobado (OK)** |
| **8** | David (`BP_David3_MH`) | `m_tal_nrw` | Top (`top`) | Biblioteca | Huesos (`InpaintWeights`) | 1.05 cm / 2.40 cm | 0 | **Aprobado (OK)** |
| **9** | David (`BP_David3_MH`) | `m_tal_nrw` | Falda escocesa (`skirt`) | Tripo Quads | Chaos Cloth (`CA_Kilt`) | 2.10 cm / 3.80 cm | 0 | **Aprobado (OK)** |

## Calibración de Empaquetado `P-20` (REQ-90, R-39)

1. **Ropa por Deformación Ósea (`InpaintWeights`):**
   * Totalmente compatible con empaquetado para **Windows (Win64)**, **Android** y **Meta Quest (VR)** sin requerir dependencias de módulos de Editor.
2. **Prendas con Simulación Física (`Chaos Cloth`):**
   * Requiere que el `.uproject` mantenga activados los módulos de tiempo de ejecución de Chaos Cloth (`ChaosCloth`, `Dataflow`).
   * Para empaquetado standalone en Android / Meta Quest VR, las prendas con tela se documentan como recomendadas para ejecución en plataformas de escritorio debido al coste de simulación PBD por GPU/CPU.

## Verificación Automatizada

- **Rust:** 254 tests pasados (`cargo test`), `cargo clippy` limpio sin avisos.
- **Vitest:** 408 tests pasados en 43 suites (`npx vitest run`).
- **Python:** 123 tests pasados (`pytest tests/python`).
- **Linter:** `python dbv-specs-ops/scripts/dbv-lint.py unreal-plugin tests/python` limpio (0 errores, 0 avisos).

