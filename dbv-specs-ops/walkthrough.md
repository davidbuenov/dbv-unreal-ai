# 🧭 Walkthrough — DBV Unreal AI v0.5.1

> **Fecha:** 2026-11-12 · **Versión:** 0.5.1 (Patch, desde la 0.5.0) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.18, §3.21) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md) · **Notas:** [`releases/v0.5.1.md`](./releases/v0.5.1.md)

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA (modelos locales, de nube o agentes por suscripción como Claude Code, Codex o Antigravity) a un proyecto abierto de Unreal Engine 5.8, con permiso antes de actuar, generación 3D (Tripo, World Labs), audio y un servidor MCP propio.

## Qué se entrega en v0.5.1

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| **Vestidos con tela** | `unreal_dress_metahuman` con `dress`: el vestido de Tripo se ajusta al cuerpo (`tailor.fit_dress`: escala por pecho y cintura, sisa a la axila, mangas a los brazos, tapas del sólido y muñones fuera, 1 cm de holgura mínima), se orienta por la silueta y se monta con Chaos Cloth con un mapa de `MaxDistance` propio: talle y mangas sujetos, falda suelta. Material de Tripo, ranuras `Torso` y `Legs` vacías. | REQ-84, REQ-87, REQ-88 |
| **Validación de tela** | La holgura de un vestido se mide con la distancia con signo de cada vértice al cuerpo completo; antes salía siempre `error`. Capturas con el personaje entero. | REQ-89 |
| **`flip_orientation`** | Parámetro opcional para girar 180° un vestido que la detección dejó al revés; en el MCP, en la IA de la app y en el puente C++. | REQ-87, REQ-88 |
| **Prendas a la Biblioteca** | `unreal_export_outfit` guarda en la Biblioteca la prenda que lleva un MetaHuman, con su manifiesto, para volver a ponerla sin reajustar en la misma complexión. | REQ-100 |
| **El sastre** | Tops y pantalones de Tripo deformados sobre el cuerpo conservando malla, UV y material (sustituye a la carcasa). | REQ-81, REQ-98 |
| **Pipeline de vestir** | Faldas y vestidos ya se pueden equipar (claves y firmas que no coincidían), un fallo no destruye prendas anteriores, no quedan restos ni éxitos simulados. | REQ-85, REQ-87, REQ-88 |

## Cómo probarlo

1. `npm install` y `npm run tauri dev`. Abre un proyecto de Unreal 5.8 con un MetaHuman en el nivel y pulsa **Preparar** (copia el plugin v0.5.1).
2. En la Biblioteca, elige un vestido generado con Tripo, **Enviar a Unreal**, tipo `dress`, y el MetaHuman de destino (o pídeselo a la IA, que usará `unreal_dress_metahuman`).
3. Tarda alrededor de un minuto. El informe debe decir `validation_result: ok` y traer tres capturas (frente, lado y espalda) en `Saved/DBVUnrealAI/outfits/captures/<actor>/`.
4. Comprueba en el visor que el vestido está bien puesto. Si saliera girado, repite con `flip_orientation: true`.
5. Dale a **Play** con una animación de andar: el talle sigue al cuerpo y la falda se mueve.

## Verificación

- **Automática (2026-11-12):** Python 208 tests (`npm run test:python`), Rust 338 (`cargo test`), Vitest 427, `dbv-lint --changed` sin errores y Clippy sin avisos. Tests nuevos: `test_outfit_tailor_dress.py` (perfil, tapas, piezas sueltas, sujeción, orientación por silueta y `flip_orientation`) y `test_outfit_cloth_dress.py` (mapa de `MaxDistance`, holgura de tela y pasos del pipeline).
- **En vivo (UE 5.8, `DBVMCPFirst`):** `Victoria_VestidoAzul` (`f_med_nrw`) con `vestido_azul_marino`: 5 pasos, validación `ok`, 0 % de vértices dentro de la piel, holgura mediana 4,5 cm, frente decidido por la silueta. Revisado y aprobado por el usuario en el visor.
- **Revisión de `/code-simplify` (2026-11-12):** sin hallazgos Crítico. Un Importante resuelto (mallas del ajuste que quedaban sueltas si fallaba la tela) y otro como límite conocido (el pelo en las capturas).

## Límites conocidos y lo que NO está comprobado

- **Solo un vestido y un cuerpo probados en vivo** (`f_med_nrw`). Falta un vestido en Pablo o David (`m_tal_nrw`) y el resto de la matriz de REQ-90; la Microsoft Store sigue pendiente de esa matriz.
- **Las faldas (`skirt`) no se ajustan:** se montan sobre la malla tal como llega.
- **El pelo no se ve en las capturas de validación**; en el visor y en Play sí.
- **Un vestido sin curva lumbar clara se orienta por el escote** y puede salir girado: se corrige con `flip_orientation`.
- **Piel limpia y recorte del cuerpo** (REQ-86) siguen desactivados en el pipeline, a la espera de decisión.
- **Plantillas de tela del plugin** con referencias al proyecto de pruebas: en otro proyecto cargan con avisos de referencias que faltan.
