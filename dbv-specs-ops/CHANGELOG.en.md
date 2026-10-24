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
