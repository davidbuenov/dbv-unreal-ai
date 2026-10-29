# 🪪 Project Config

> This file is read automatically by the AI at session start.
> If placeholders are detected, the AI will propose a complete setup draft with marked assumptions (`[ASSUMPTION: ...]`) for you to confirm in one single step.

---

## Project Identity

- **Name:** DBV Unreal AI (identificador técnico: `dbv-unreal-ai`)
- **Description:** Copiloto de IA nativo para Unreal Engine 5.8: conecta cualquier IA (local o en la nube) a tu proyecto abierto, lo prepara si hace falta y genera assets con Tripo y World Labs directamente en el editor
- **Author / Company:** David Bueno Vallejo · https://github.com/davidbuenov
- **Co-author:** David Bueno Carmona
- **Copyright line (headers & LICENSE):** Copyright (c) [Year] David Bueno Vallejo and David Bueno Carmona
- **License:** MIT
- **Documentation language:** Español **e inglés, siempre en dos ficheros distintos** (regla del usuario, 2026-10-24): `README.md` / `README.en.md`, `dbv-specs-ops/CHANGELOG.md` / `CHANGELOG.en.md`, notas de cada versión `dbv-specs-ops/releases/vX.Y.Z.md` / `vX.Y.Z.en.md` (`python scripts/release/notes.py vX.Y.Z` en `/ship`) y `docs/IA.md` / `IA.en.md`. Toda entrada nueva se escribe en los dos idiomas en el mismo commit.
- **GitHub Release:** Automático (`.github/workflows/release.yml` al subir el tag): binarios de Windows, macOS y Linux, cuerpo con las notas en inglés y en español, y los dos ficheros de notas adjuntos.
- **Git:** Sí
- **Languages:** Rust, JavaScript, HTML, CSS, Python (scripts dentro de Unreal), C++ (toolset de Unreal)
- **Technologies / Stack:** Tauri v2 (WebView nativo del SO) + Vanilla JS/HTML/CSS sin bundler, partiendo de dbv-tauri-starter; motor de IA de tres niveles (local / nube con llavero del SO / agentes ACP) como en dbv-typst-editor; Unreal Engine 5.8 vía su servidor MCP integrado (ModelContextProtocol + ToolsetRegistry + PythonScriptPlugin) y un toolset C++ de preparación como en DBVMCPFirst; APIs de Tripo 3D y World Labs
- **Agent Readiness (Web):** Not Applicable
- **Desktop AI Assistant (Local/Cloud/ACP):** Yes
- **Framework Version:** 3.2.0


---

## Model Routing Guidelines

To optimize OpEx (Token Burn) and latency, refer to this routing strategy when executing project development tasks:

| Development Phase | Required Reasoning Complexity | Recommended Model Class | Example Models |
| --- | --- | --- | --- |
| `/spec` (Specifications) | Very High | Advanced Reasoning / Frontier Models | Gemini 3.1 Pro, Claude Opus 5, GPT-5.6 Sol |
| `/plan` (Planning / Architecture) | Very High | Advanced Reasoning / Frontier Models | Gemini 3.1 Pro, Claude Opus 5, GPT-5.6 Sol |
| `/build` (Code Implementation) | Medium | Fast, high-accuracy coding models | Gemini 3.5 Flash, Claude Sonnet 5, GPT-5.6 Terra |
| `/test` (Conventional Tests / Evals) | Medium-Low | Fast & cheap models | Gemini 2.5 Flash-Lite, Claude Haiku 5, GPT-5.6 Luna |
| `/code-simplify` (Security & Refactor) | High | Security-conscious reasoning models | Gemini 3.1 Pro, Claude Sonnet 5, GPT-5.6 Sol |
| `/ship` (Documentation, Changelog) | Low | Fast, text-optimized models | Gemini 2.5 Flash-Lite, Claude Haiku 5, GPT-5.6 Luna |

---

## File Header Template

All source files must include a header comment in the appropriate syntax for the language.
Use the fields above to generate it. Always include the framework credit line.

**Example (JavaScript / CSS):**
```
// =============================================================================
// [Project Name] — [Description]
// Copyright (c) [Year] [Author / Company]
// Licensed under the [License] License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
```

**Example (Python):**
```
# =============================================================================
# [Project Name] — [Description]
# Copyright (c) [Year] [Author / Company]
# Licensed under the [License] License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
```

**Example (HTML):**
```
<!--
  [Project Name] — [Description]
  Copyright (c) [Year] [Author / Company]
  Licensed under the [License] License. See LICENSE for details.
  Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
-->
```

**Example (Rust / Java / C# / Go):**
```
// =============================================================================
// [Project Name] — [Description]
// Copyright (c) [Year] [Author / Company]
// Licensed under the [License] License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
```

---

## Fuente de verdad
<!-- Opcional — ver docs/SOURCE_OF_TRUTH.md -->
- Modo: repo

## Guardarraíles activos
<!-- Opcional — ver docs/GUARDRAILS.md -->
- Pre-commit: no
- Linter dbv-lint.py (en /code-simplify): sí
- Branch protection en CI: no

## Release de GitHub
<!-- Opcional — lo rellena /ship la primera vez; ver docs/MASTER_PROMPT.md (Fase 6) -->
- GitHub Release: <automático | manual | no>

## Maintain (Fase 7)
<!-- Opcional, desactivada por defecto — ver docs/MAINTAIN.md -->
- Habilitado: no
- CLI no interactivo: <claude -p | gemini -p | otro>

---

> 🛠️ Framework SDD creado por **[David Bueno Vallejo](https://github.com/davidbuenov)** — libre y gratuito · [dbv-specs-ops](https://github.com/davidbuenov/dbv-specs-ops)
