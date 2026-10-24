# Changelog — DBV Unreal AI

**[🇪🇸 Español](./CHANGELOG.md) · 🇬🇧 English**

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

> This file mirrors [`CHANGELOG.md`](./CHANGELOG.md) (Spanish). Both are kept in sync: **every changelog entry is written in both languages, in the same commit.**

---

## [Unreleased]

### Added

- Project bootstrap with the dbv-specs-ops v3.1.0 SDD framework: project identity, activation files for AI assistants, MIT license and initial README.
- Desktop app base from dbv-tauri-starter 0.3.0: Tauri v2, three-tier AI subsystem (local, cloud with OS keychain, ACP agents), with the DBV Unreal AI identity (`com.davidbuenov.dbv-unreal-ai`).
- Unreal Engine 5.8 connection (REQ-01..REQ-05): detection of engine installs (even when Epic's launcher list does not name them), `.uproject` projects and running editors; editor MCP client (`Mcp-Session-Id` session, tool search, text `list_toolsets`, double-encoded results), verified live against UE 5.8.2; readiness check and "Prepare" with diff, backup and byte-exact "Revert"; refuses to prepare while the project's editor is open.
- DBV Unreal AI toolset for Unreal (REQ-06, REQ-25, REQ-26): content-only Python project plugin with `run_python` (undo transaction, stdout/stderr/warnings), `validate_python` (finds missing or deprecated APIs without running the code), `describe_python_api` (reference from the running engine), `get_project_info` and `get_scene_summary`; registered, exercised and unregistered live in the editor. C++ variant as a template that delegates to the same Python package (not compiled yet).
- Unreal tools for the assistant (REQ-10, REQ-12, REQ-26): Epic's server meta-tools (`list_toolsets`, `describe_toolset`, `call_tool`) plus own shortcuts; read/modify classification by verb with an allow-list (anything unknown asks); approval card showing the full Python code; Python is validated against the running engine before asking, and missing names go back to the model to fix.
- Tripo v3 and library (REQ-15, REQ-17, RNF-04): text-to-model with mandatory cost confirmation and a per-session credit cap, immediate GLB download (links expire after 5 minutes) and a local library shared across projects. Image-to-model pending verification with a real key.
- Send to Unreal (REQ-19): Interchange import through the DBV toolset `import_model` (Epic's `import_file` forces FBX and rejects GLB) and placement in front of the camera under `DBV/Generated`; verified live with a real Tripo GLB (correct scale, material and texture).
- Own MCP server (REQ-20): `dbv-unreal-ai --mcp [--port n] [--project path]` over stdio, no network port; bridge to the editor MCP (with mandatory Python validation), library, send to Unreal and Tripo generation (requires `confirm_cost: true` and honours the cap); verified live against the editor. The C++ toolset template switches to `snake_case` names to share the contract with the Python variant.
- DBV Unreal AI UI (REQ-08, REQ-14, REQ-24, DESIGN.md): sidebar (Project · Conversation · Generate · Library · Settings), Unreal connection pill, automatic connection to the editor running the MCP server, readiness check explaining each item, "Prepare" with a per-file diff and "Revert", action log exportable to Markdown, cost card with focus on "Cancel", dark theme by default plus light, ES/EN. The AI assistant loads when the Conversation opens, with the Unreal and generation tools and the MCP servers for ACP agents.
- CI and release (ADR-V010-001 D1): verification on every push (Rust, Vitest, Python toolset, linter) and a draft Release with Windows, universal macOS and Linux binaries when a tag is pushed.

### Fixed

- The connection pill went back to "No editor" when the AI assistant loaded: `applyTranslations()` rewrote its text because it carried `data-i18n` (caught by the headless Edge UI bench).
- The preparation diff did not mark lines skipped at the start and the end with "…".
