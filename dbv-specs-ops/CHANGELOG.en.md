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
- "Copy diagnostics" in Settings (RNF-08): version, system, Unreal installations and open editors, without keys or prompts.
- Evals with a real model against the real editor (`npm run eval:unreal -- --model qwen3:8b --toolset unreal-plugin/DBVUnrealAI/Content/Python`): the same loop and tools as the app, with the ground truth computed by querying the editor. With `qwen3:8b`: 5/5 in three runs and zero modifications without approval.
- When an API is deprecated, the validation and the API reference give the equivalent editor subsystem call (`unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()`), and the system prompt explains the Unreal 5.8 subsystem pattern.
- Tagged tests for every v0.1 requirement (REQ-11 with MCP servers in `session/new`, REQ-13, RNF-01, RNF-05 with measured latency, RNF-06, RNF-07, RNF-IA.2).

### Fixed

- The connection pill went back to "No editor" when the AI assistant loaded: `applyTranslations()` rewrote its text because it carried `data-i18n` (caught by the headless Edge UI bench).
- The preparation diff did not mark lines skipped at the start and the end with "…".
- With an 8B model, counting actors with Python ran out of its 12 steps: the deprecated-API warning ("use the subsystem") did not say which one and the model could not find the alternative (caught by the eval; from 14 editor calls down to 4).
- [Critical] "Prepare" left the project half-changed and impossible to revert if a write failed (the backup manifest was written last). It is now all-or-nothing: manifest before writing and automatic rollback on failure.
- [Critical] A Tripo job that failed instantly (bad key) left the UI "Queued…" until the timeout: the event arrived before anyone listened. The listener is now registered before starting and keeps what arrives.
- [Important] The session credit cap was consumed even when Tripo rejected the request without creating a task; now only jobs that create a task count (new "Sent to Tripo…" stage), also in the own MCP server.
- [Important] Model downloads only accept HTTPS (they used to accept `http://127.0.0.1`, meant for tests).
- [Important] An expired MCP session has its own error type instead of being recognised by its message text.
- [Important] `#![deny(clippy::unwrap_used, clippy::expect_used)]` crate-wide (except startup) and the single-`return` rule applied to the MCP client, Tripo, the generation job, preparation, the own MCP server and the Unreal tools.
- The window stayed black with `npm run tauri dev`: `index.html` started with a UTF-8 BOM (inherited from the starter) and Tauri decides the content type from the first bytes, so the WebView downloaded it instead of rendering it. The installer would have been affected too. BOM removed (also from `main.js`) plus a regression test requiring the file to start with `<!doctype html>`. It was hidden because the Edge benches set the type from the file extension.
- With a subscription agent (Claude Code…) the AI still introduced itself as "a Markdown documents assistant": the Unreal prompt only reached local and cloud models. The agent now gets the Unreal one with the exact tool names of its MCP server (a test cross-checks them against `mcp_server.rs`), without the `purpose` instruction, which belongs to our permission card.
- Text and behaviours of the base document editor were left over: conversation suggestions ("Summarise this document", "Check the spelling"), cloud notice, tools hint, file tool descriptions, `get_outline` (meaningless without an open document) and the empty "Context:" bar. All of it is now Unreal-specific, in Spanish and English.
- The proposal check applied Markdown rules (unclosed code blocks, `$$`, links) to any file: a `.py`, `.ini` or `.cpp` with `a[0](x)` or triple quotes gave false errors. They now only apply to `.md`/`.markdown`.
- `run_python` returns the value of the last expression, like a console. Models write the result that way; the code ran fine but `result: null` came back and they thought it had failed.
- Concrete hints when code fails, instead of similar names from a deprecated class: a missing member is looked up (also approximately) in the editor subsystems (`EditorLevelLibrary.get_current_level` → `LevelEditorSubsystem`); `get_editor_subsystem(unreal.EditorLevelLibrary)` is rejected before running; a runtime `AttributeError` (`world.get_all_actors()`) brings `EditorActorSubsystem.get_all_level_actors()`; the API reference accepts expressions with calls.
- If the model writes Unreal Python in its reply without calling `unreal_run_python`, the app reminds it once (nothing had run and the user saw no approval card).
- The Eval uses the same system prompt as the app (`--prompt full`) and can be compared with the short one (`--prompt domain`).
