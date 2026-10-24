<p align="center"><img src="src/assets/logo.png" alt="Unreal Dev Community Málaga" width="128" height="128"></p>

# DBV Unreal AI

**[🇪🇸 Español](./README.md) · 🇬🇧 English**

> Native AI copilot for Unreal Engine 5.8: connect any AI (local or cloud) to your open project, prepare it when needed and generate assets with Tripo straight into the editor.

## 📑 Table of Contents

- [About](#about)
- [Status](#status)
- [How it works](#how)
- [Requirements](#requirements)
- [How to run](#usage)
- [Use it from other agents (MCP)](#mcp)
- [Which AI to use](./docs/IA.en.md)
- [Tests](#tests)
- [Project structure](#structure)
- [Changelog](#changelog)
- [License](#license)

---

<a name="about"></a>

## 📌 About

**DBV Unreal AI** (`dbv-unreal-ai`) is a native desktop application (Windows, macOS and Linux) that talks with you and acts on the Unreal Engine 5.8 project you have open. It uses the AI you prefer —a local model (Ollama, LM Studio), a cloud API or your Claude Code, Codex, Gemini CLI or Copilot subscription— and connects it to the editor through its built-in MCP server. If the project is not ready yet, the app prepares it for you. It generates 3D models with Tripo, keeps them in a local library and brings them into the editor with one click.

**Built with:** Rust, Tauri v2, bundler-free HTML/CSS/JavaScript, Unreal Engine 5.8 (MCP + Python).

---

<a name="status"></a>

## 🚧 Status

**v0.1.0** (2026-10-24, first release). Working and verified live against Unreal 5.8.2:

- MCP connection with the editor, project readiness check and preparation with diff and "Revert";
- conversation with local, cloud or subscription AIs using the editor tools;
- **nothing that modifies the project runs without your permission**, and Python is checked against the engine before asking;
- Tripo text-to-model, local library and "Send to Unreal" (glTF import with materials and textures);
- own MCP server to use all of the above from other agents.

Also in v0.1.0, built and tested against a simulated Tripo (**still to be tested with the real Tripo API and the engine**):

- **the whole Tripo API** for the AI and agents: text or image → model, variants, rig, 101 animations (dances, emotions, sports…), convert, retopology, texture, segmentation, multiview, images and the account balance;
- a **console** for every job with the exact request sent to Tripo and its progress, also in the conversation when an agent generates;
- a **3D viewer** in the library and in the conversation;
- **reference images and documents** attached to the conversation;
- a library that **imports** your own models, with a **configurable folder** and short meaningful names;
- Tripo characters in Unreal as a Skeletal Mesh playing their animation.

Roadmap in [`SPECIFICATIONS.md`](./dbv-specs-ops/docs/SPECIFICATIONS.md).

---

<a name="how"></a>

## 🧭 How it works

1. **Project.** The app detects the open Unreal editors and connects by itself to the editor MCP server (`127.0.0.1:8000/mcp`). If the project is missing something, the readiness check says what, item by item.
2. **Prepare.** "Prepare project…" shows the diff of every file before touching it and saves a backup: it enables the `ModelContextProtocol`, `ToolsetRegistry`, `PythonScriptPlugin`, `EditorToolset`, `AutomationTestToolset` and `SlateInspectorToolset` plugins, turns on the MCP server auto start and installs the **DBV Unreal AI toolset** (a content-only Python plugin: nothing to compile). With the editor closed; once reopened, it is connected.
3. **Conversation.** The AI discovers the editor tools (more than 280 in a prepared project), queries the scene and acts. Queries run directly; anything that modifies the project shows up in a card with the full detail and waits for your "Allow". Python is first validated against the real API of your engine version: if it uses something missing or deprecated, the AI fixes it without bothering you. Everything is kept in the action log (exportable to Markdown).
4. **Generate.** Describe an object or choose a reference image, ask for 1 to 4 variants and accept the cost: the card gives the total from Tripo's price list and shows exactly what is sent. Every job has its console and progress; the model is downloaded into the library.
5. **Library.** Each asset says what it is and where it comes from. A model can be **rigged**, a rig can be **animated** (up to 5 animations at once) and everything can be viewed in 3D. "Send to Unreal" imports it into `/Game/DBV/Generated` with its material and texture and places it in front of the camera, in the `DBV/Generated` Outliner folder; a character arrives as a moving Skeletal Mesh. You can import your own models and choose where the library lives.

---

<a name="requirements"></a>

## 🧰 Requirements

- **Unreal Engine 5.8** (installed with the Epic launcher or built from source).
- For development: [Rust](https://rustup.rs) (stable), Node.js 20+ and the Tauri v2 system dependencies (WebView2 on Windows, WebKitGTK on Linux). Python 3.10+ for the toolset tests.
- Optional: an AI (Ollama/LM Studio, an API key or a subscription agent) and a [Tripo](https://www.tripo3d.ai) API key to generate models. Keys are stored in the system credential store, never in files.

---

<a name="usage"></a>

## ▶️ How to run

```bash
npm install
npm run tauri dev
```

Installers (in `src-tauri/target/release/bundle/`):

```bash
npm run tauri build
```

Pushing a `vX.Y.Z` tag makes GitHub Actions build Windows, macOS (universal) and Linux and attach them to a draft Release.

---

Which AI to connect (local, with an API key or your subscription) and what each one needs: see the guide [`docs/IA.en.md`](./docs/IA.en.md).

---

<a name="mcp"></a>

## 🔌 Use it from other agents (MCP)

The executable itself is a stdio MCP server (no network port). In Claude Code, Cursor or Gemini CLI:

```json
{
  "mcpServers": {
    "dbv-unreal-ai": { "command": "<path>/dbv-unreal-ai", "args": ["--mcp", "--port", "8000"] }
  }
}
```

The **Settings** view gives you this block with the exact path. It offers the bridge to the editor (with Python validation), the library (list, import, rename, "Send to Unreal") and **the whole Tripo API**: `generate_3d_model`, `image_to_3d_model`, `rig_model`, `animate_model`, `tripo_run` (any other operation), `tripo_operations` (catalogue with prices), `tripo_balance` and `generation_jobs`. Anything that costs credits requires `confirm_cost: true` after telling the user the price; jobs run in the background and the app shows their progress.

---

<a name="tests"></a>

## 🧪 Tests

```bash
npm run test:all     # Rust + Vitest + Python toolset + linter
npm run test:e2e     # UI in headless Edge (the WebView2 engine), with a simulated backend
```

Against a real open editor (read-only): `DBV_UNREAL_MCP_PORT=8000 cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored en_vivo`.

---

<a name="structure"></a>

## 📂 Project structure

```text
/
├── src/                     # UI (bundler-free HTML/CSS/JS)
│   ├── app/                 #   views: Project, Generate, Library, Settings, action log
│   ├── unreal/              #   Unreal tools for the AI, permissions, approval card, sending
│   ├── generation/          #   generation and library tools for the AI
│   └── ai/                  #   three-tier AI assistant (from dbv-tauri-starter)
├── src-tauri/src/           # Rust backend
│   ├── unreal/              #   discovery, editor MCP client, preparation
│   ├── generation/          #   Tripo v3 client and generation jobs
│   ├── library/             #   local asset library
│   ├── mcp_server.rs        #   own MCP server (--mcp)
│   └── ai/                  #   providers, ACP, system keychain
├── unreal-plugin/           # Toolset installed into the project (Python; C++ template)
├── tests/                   # Vitest, Python toolset tests and real editor responses
├── scripts/e2e/             # UI test bench
└── dbv-specs-ops/           # Specifications, architecture, task.md, memory.md and CHANGELOG.md
```

---

<a name="changelog"></a>

## 📋 Changelog

See [CHANGELOG.en.md](./dbv-specs-ops/CHANGELOG.en.md).

Each version's release notes are in two files, [`dbv-specs-ops/releases/vX.Y.Z.en.md`](./dbv-specs-ops/releases/) (English) and `vX.Y.Z.md` (Spanish), and the GitHub Release carries both.

---

<a name="license"></a>

## 📄 License

MIT — see [LICENSE](./LICENSE).

Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona

---

> 🛠️ Built with **[dbv-specs-ops](https://github.com/davidbuenov/dbv-specs-ops)** — the SDD framework for AI-assisted development.
> Created by [David Bueno Vallejo](https://github.com/davidbuenov) — free and open source.
