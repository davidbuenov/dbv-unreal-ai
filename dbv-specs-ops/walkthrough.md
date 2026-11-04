# 🧭 Walkthrough — DBV Unreal AI v0.3.1

> **Fecha:** 2026-11-03 · **Versión:** 0.3.1 (Patch, desde la 0.3.0) · **Autores:** David Bueno Vallejo y David Bueno Carmona
> **Especificación:** [`docs/SPECIFICATIONS.md`](./docs/SPECIFICATIONS.md) (§3.14) · **Changelog:** [`CHANGELOG.md`](./CHANGELOG.md) · **Notas:** [`releases/v0.3.1.md`](./releases/v0.3.1.md)

## Qué es

Una aplicación de escritorio (Tauri v2: Windows, macOS, Linux) que conecta cualquier IA (modelos locales, de nube o agentes por suscripción como Claude Code, Codex o Antigravity) a proyectos abiertos de Unreal Engine 5.8 por su servidor MCP. La IA actúa en el editor **con permiso**, genera assets con Tripo y mundos con World Labs, y lo lleva todo al nivel.

## Qué se entrega en v0.3.1

Nace de probar la v0.3.0 publicada: una conversación nueva con Claude Code mostraba `3429404 tokens enviados`, respondía como Opus pese a tener «Sonnet» seleccionado, contestaba a una pregunta de la conversación anterior y gastaba ~30 % de la cuota en una consulta corta.

| Área | Qué hace | Requisitos |
| --- | --- | --- |
| Sesión por conversación | Crear, cambiar o borrar una conversación cierra la sesión ACP y abre una limpia; al reabrir una antigua se le pasa un resumen acotado (`historyMessages`). Se conservan `images` y `agentInfo` entre turnos | REQ-67 |
| Modelo por defecto real | Un nombre genérico (`claude`, `default`, vacío) se resuelve al modelo que muestra la interfaz (`claude-sonnet-5-5`, `gpt-6-luna`, `gemini-4-argon`) en el front y en `ANTHROPIC_MODEL`, para no caer en el de `~/.claude/settings.json` | REQ-67 |
| Tokens por conversación | El contador del pie es el de la conversación abierta y empieza vacío en una nueva | REQ-67 |
| Imágenes pegadas y soltadas | `Ctrl+V` o arrastrar imágenes y texto al panel; se guardan en `%TEMP%/dbv-unreal-ai-clipboard` (≤ 10 MB, las 20 últimas) y se adjuntan con miniatura, `base64` y ruta real para Tripo. Un binario que no es imagen se rechaza | REQ-68 |
| `.md` editables | La IA propone cambios en los `.md` de la raíz del proyecto de Unreal y en `docs/**`, con el mismo diff; `Plugins/` queda fuera del escaneo de documentos | REQ-22 |
| Cabecera estándar DBV | Los ficheros del plugin C++ y `main.rs` la llevan; `unrealPrompt` la exige en todo fichero nuevo de la IA | REQ-69 |
| Huérfanos de World Labs | `resume_orphans(context, keys)` reanuda un mundo a medias con la clave de cada servicio y lo descarga a la biblioteca; sin clave, se queda «en curso» | REQ-70, REQ-27 |

## Cómo probarlo

1. `npm install` y `npm run tauri dev` (en la raíz). La app es de escritorio: no hay scripts `start`/`stop`.
2. Conecta un agente (p. ej. Claude Code) y escribe en una conversación. Pulsa **＋**: el contador del pie queda vacío y la nueva conversación no sabe de la anterior.
3. Haz una captura, pulsa `Ctrl+V` en el cuadro de texto y comprueba la miniatura; arrastra también un `.md` y una imagen al panel. Suelta algo fuera del panel: no debe pasar nada.
4. Pide «actualiza el README.md del proyecto»: llega como diff para aceptar.
5. Lanza un mundo de World Labs, cierra la app a mitad y ábrela: el trabajo se reanuda y el mundo acaba en la biblioteca.

## Verificación

- **Automática (2026-11-03, tras `/code-simplify` y el cambio de versión):** Rust 235 (+5 ignorados), Vitest 371 en 43 ficheros, Python en verde, `dbv-lint` sin errores, `cargo clippy --all-targets` sin avisos, banco e2e correcto y `sync-ai --check` sin diferencias con `dbv-tauri-starter`.
- **MSIX de la Store:** `dbv-unreal-ai_0.3.1.0.msixbundle` generado con `npm run tauri:windows:build`; su `AppxManifest.xml` declara `davidbuenov.DBVUnrealAI`, `0.3.1.0`, x64, `runFullTrust` e `internetClient` y el alias `dbv-unreal-ai.exe`; los assets no son placeholders. Pasos de Partner Center en [`docs/MICROSOFT_STORE.md`](./docs/MICROSOFT_STORE.md) §7.
- **Sin verificar:** arrastrar y soltar en la ventana real de Windows (y con el MSIX instalado) tras desactivar `dragDropEnabled`; pegar una imagen dentro del MSIX (virtualización de `AppData`); evals con un modelo real; macOS y Linux en máquina real.

## Revisión de código

- `/code-simplify` de la v0.3.1: 1 Crítico resuelto (el `drop` HTML5 nunca llegaba en Windows por el arrastre nativo de Tauri) y 2 Importantes (un binario soltado se adjuntaba como texto; `resume_orphans(context)` daba la clave de Tripo a World Labs). Detalle en el CHANGELOG.
- Refactor sin cambio de comportamiento: base64 del crate `base64`, `JobContext::{tripo_client, worldlabs_client}`, `generation::service_keys`, `usageText`/`addUsage` en `aiApp.js`, `pastedAttachments` fuera de `start()`.
- En `/ship`: la especificación de REQ-70 pasó a describir lo que hace el código (sin clave, el huérfano se suelta y espera) en vez de «se marca como fallido».

## Pendiente para la siguiente versión

- **Decidir REQ-69:** la IA pone el copyright de los autores de la app y la licencia MIT en los ficheros nuevos del proyecto **del usuario**; en una app de la Store quizá deba llevar el nombre y la licencia del propio usuario.
- La triple resolución del modelo por defecto (`connectWizard.js`, `acpSession.js`, `acp.rs`) se simplificaría en `dbv-tauri-starter`, de donde se sincronizan esos ficheros.
- Canales de distribución (Homebrew, Uptodown), actualizaciones automáticas, evals con un modelo real y revisión nativa del chino.
