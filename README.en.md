<p align="center"><img src="src/assets/logo.png" alt="Unreal Dev Community Málaga" width="128" height="128"></p>

# DBV Unreal AI

**[🇪🇸 Español](./README.md) · 🇬🇧 English · [🌐 Website](https://davidbuenov.github.io/dbv-unreal-ai/en/)**

> Native AI copilot for Unreal Engine 5.8: connect any AI (local or cloud) to your open project, prepare it when needed and generate assets with Tripo and World Labs straight into the editor.

<p align="center">
  <a href="https://www.unrealengine.com" target="_blank"><img src="https://img.shields.io/badge/Unreal_Engine-5.8-0E1128?style=for-the-badge&logo=unrealengine&logoColor=white" alt="Unreal Engine 5.8"></a>
  <a href="https://www.worldlabs.ai" target="_blank"><img src="https://img.shields.io/badge/World_Labs-Marble_3DGS-1A1A1A?style=for-the-badge&logoColor=white" alt="World Labs Marble"></a>
  <a href="https://www.tripo3d.ai" target="_blank"><img src="https://img.shields.io/badge/Tripo_3D-AI_Generation-6366F1?style=for-the-badge&logoColor=white" alt="Tripo 3D"></a>
</p>

## 📑 Table of Contents

- [Download and install](#install)
  - [Microsoft Store (Windows)](#install-msstore)
  - [Windows (Releases)](#install-windows)
  - [macOS](#install-macos)
  - [Linux](#install-linux)
  - [Coming soon: Homebrew and Uptodown](#install-soon)
- [About](#about)
- [Compatible Applications & Ecosystem](#ecosystem)
- [Status](#status)
- [How it works](#how)
- [Requirements](#requirements)
- [For developers](#usage)
- [Use it from other agents (MCP)](#mcp)
- [Which AI to use](./docs/IA.en.md)
- [Tests](#tests)
- [Project structure](#structure)
- [Changelog](#changelog)
- [License](#license)

---

<a name="install"></a>

## 🚀 Download and install

**You don't need Rust, Node.js or any programming tool.** Download the installer for your system from the **[latest GitHub Release](https://github.com/davidbuenov/dbv-unreal-ai/releases/latest)**: it is built automatically for every version for Windows, macOS (Intel and Apple Silicon) and Linux.

You only need **Unreal Engine 5.8** to work on a project and, to chat, an AI (local, with an API key or your subscription): the [`docs/IA.en.md`](./docs/IA.en.md) guide explains each option. [Tripo](https://www.tripo3d.ai) and [World Labs](https://platform.worldlabs.ai) keys are optional and only needed to generate 3D models and worlds.

> **Updates:** the app does not update itself yet. To move to a new version, download and run the new installer from Releases; your library, connections and keys are kept.

<a name="install-msstore"></a>

### 🛒 Microsoft Store (Windows - Official & Recommended)

You can now install **DBV Unreal AI** for free directly from the Microsoft Store, officially signed by Microsoft with no SmartScreen warnings and automatic updates:

<p align="center">
  <a href="https://apps.microsoft.com/detail/9n5ls7zkgth5?hl=en-EN&gl=EN" target="_blank">
    <img src="https://get.microsoft.com/images/en-us%20dark.svg" alt="Free download and install on Windows | Microsoft Store" height="56">
  </a>
</p>

- 🇬🇧 **English:** [dbv-unreal-ai - Free download and install on Windows | Microsoft Store](https://apps.microsoft.com/detail/9n5ls7zkgth5?hl=en-EN&gl=EN)
- 🇪🇸 **Español:** [DBV Unreal AI: descarga e instalación gratuitas en Windows | Microsoft Store](https://apps.microsoft.com/detail/9n5ls7zkgth5?hl=es-ES&gl=ES)

<a name="install-windows"></a>

### 🪟 Windows (Manual installers from Releases)

If you prefer not to use the store, **[⬇️ Download from Releases](https://github.com/davidbuenov/dbv-unreal-ai/releases/latest)** one of these two (Windows 10 and 11, 64-bit):

- **`DBV.Unreal.AI_x.y.z_x64-setup.exe`** — the recommended one: double-click and you're done.
- **`DBV.Unreal.AI_x.y.z_x64_en-US.msi`** — the same program as an MSI package, meant for managed installs (companies, classrooms).

The installer is not signed with a code-signing certificate, so the first time Windows SmartScreen warns that "Windows protected your PC". Click **More info → Run anyway**. The UI engine (WebView2) already ships with Windows 10 and 11; if it were missing, the installer downloads it.

<a name="install-macos"></a>

### 🍎 macOS

**[⬇️ Download `DBV.Unreal.AI_x.y.z_universal.dmg` from Releases](https://github.com/davidbuenov/dbv-unreal-ai/releases/latest)** — a single universal binary for Intel and Apple Silicon. Open it and drag **DBV Unreal AI** to **Applications**.

It is neither signed nor notarized (that requires the paid Apple Developer Program), so macOS blocks it the first time ("cannot be opened because the developer cannot be verified"). To open it:

- Right-click (or `Ctrl` + click) **DBV Unreal AI** in Applications → **Open** → confirm in the dialog. Only needed the first time.
- Or, from the Terminal: `xattr -cr "/Applications/DBV Unreal AI.app"` before opening it.

The `DBV.Unreal.AI_universal.app.tar.gz` file in the Release is the same app compressed; the `.dmg` is all you need to install.

<a name="install-linux"></a>

### 🐧 Linux

**[⬇️ Download from Releases](https://github.com/davidbuenov/dbv-unreal-ai/releases/latest)** the package for your distribution (x86_64):

- **`.deb` (Debian, Ubuntu, Linux Mint and derivatives):** `sudo apt install ./DBV.Unreal.AI_x.y.z_amd64.deb` (or double-click it in the file manager). It also installs any missing dependencies, such as WebKitGTK.
- **`.rpm` (Fedora, openSUSE, RHEL and derivatives):** `sudo dnf install ./DBV.Unreal.AI-x.y.z-1.x86_64.rpm` (on openSUSE, `sudo zypper install`).
- **`.AppImage` (any distribution):** make it executable (`chmod +x DBV.Unreal.AI_x.y.z_amd64.AppImage`) and run it. It is portable, no installation needed.

<a name="install-soon"></a>

### 🔜 Coming soon

These channels are not available yet; they will arrive in an upcoming version. In the meantime, use the Microsoft Store or the Releases installers above.

- **🍺 Homebrew (macOS):** `brew install --cask` from a dedicated tap that updates itself with every Release.
- **🟢 Uptodown:** direct download of the latest version for Windows and macOS.

---

<a name="about"></a>

## 📌 About

**DBV Unreal AI** (`dbv-unreal-ai`) is a native desktop application (Windows, macOS and Linux) that talks with you and acts on the Unreal Engine 5.8 project you have open. It uses the AI you prefer —a local model (Ollama, LM Studio), a cloud API or your Claude Code, Codex or Copilot subscription (Gemini CLI, with paid access)— and connects it to the editor through its built-in MCP server. If the project is not ready yet, the app prepares it for you. It generates 3D models with Tripo and photorealistic worlds with World Labs Marble, keeps them in a local library and brings them into the editor with one click.

**Built with:** Rust, Tauri v2, bundler-free HTML/CSS/JavaScript, Unreal Engine 5.8 (MCP + Python + C++).

---

<a name="ecosystem"></a>

## 🔗 Compatible Applications & Connected Ecosystem

**DBV Unreal AI** serves as a native bridge and control center between your game development environment and leading spatial intelligence and 3D generative AI platforms:

| Icon | Application / Platform | Connection Type in DBV Unreal AI | Integrated Capabilities |
| :---: | :--- | :--- | :--- |
| <a href="https://www.unrealengine.com"><img src="docs/assets/unreal-engine.svg" width="40" height="40" alt="Unreal Engine 5.8"></a> | **[Unreal Engine 5.8](https://www.unrealengine.com)** | **Native (Local)**<br>MCP HTTP on port 8000+ & Python/C++ | • Real-time bidirectional connection & editor self-diagnostics.<br>• Static Python validation with diff and mandatory approval card.<br>• Free open-source C++ `UDbvGaussianSplatComponent` for 3DGS.<br>• Multi-instance support & single-click turn-based transaction rollback. |
| <a href="https://www.worldlabs.ai"><img src="docs/assets/world-labs.svg" width="40" height="40" alt="World Labs"></a> | **[World Labs (Marble)](https://www.worldlabs.ai)** | **Cloud API / Secure Keyring**<br>Key stored in OS credential vault | • Photorealistic persistent world generation with `marble-1.1`.<br>• Gaussian Splat (`.spz`) and physics collision (`.glb`) downloads.<br>• In-app inspection via local Three.js 3D viewer.<br>• "Send to Unreal" button for synchronized, one-click level placement. |
| <a href="https://www.tripo3d.ai"><img src="docs/assets/tripo3d.svg" width="40" height="40" alt="Tripo3D"></a> | **[Tripo3D](https://www.tripo3d.ai)** | **Cloud API / Secure Keyring**<br>Full 18-operation surface | • 3D mesh generation from text prompts or reference images.<br>• Visible credit balance & intelligent polygon budgeting.<br>• Automated skeletal rigging & animated action clips.<br>• Direct import as Static Mesh, Skeletal Mesh, and Animation Sequence. |

---

<a name="status"></a>

## 🚧 Status

**v0.2.1** (2026-10-28). Minor release focused on usability, transparent AI connectivity, built-in help, and store preparation:

- **Clear tripartite AI classification (REQ-43):** three distinct categories (Local AIs 100% offline, Subscription AIs via CLI agents, and Cloud AIs with API key) with live reactive status badges (`✓ Active` / `✓ Connected`);
- **Official Google Antigravity CLI support (REQ-44):** integrated `agy` agent support for Google One and Gemini Advanced subscriptions without API keys;
- **Integrated Help Center (REQ-46):** instant help via the header (?) button or global **`F1`** shortcut, featuring Unreal Engine 5.8 setup guides, AI comparison tables, and shortcuts;
- **Redesigned "About" dialog (REQ-45):** platform telemetry, project links, and environment diagnostics matching DBV Typst Editor aesthetics;
- **Microsoft Store packaging:** complete MSIX configuration with `@choochmeque/tauri-windows-bundle` and anti-placeholder asset verification;
- **Photorealistic world generation with World Labs Marble (REQ-18):** generation from text or image (`marble-1.1`), credit estimation, automatic download of `.spz` Gaussian splats, `.glb` physical collider meshes, and scale metadata into the library;
- **Native C++ 3D Gaussian Splatting component (REQ-41, REQ-42):** `DBVUnrealAICpp` module with `UDbvGaussianSplatComponent` and synchronized placement of `.spz` splats and collision meshes into the level via "Send to Unreal";
- **One-click assistant turn undo (REQ-40):** «↩️ Undo this turn» button in conversation rolling back editor transactions in Unreal Engine via Slate post-tick callback;
- **Multiple Unreal Engine instances (REQ-07):** concurrent editor detection across ports (8000, 8001…) with identity verification and dynamic project switcher;
- **Interactive character rig attachment in animations (REQ-16, REQ-31):** 3D viewer binds animation clip tracks in real time to any library character rig;
- **Persistent project memory and interactive planning (REQ-34..REQ-37):** management of `dbv-unreal-ai/memory.md` and `task.md` with in-editor done criteria verification and interactive Plan view;
- **Viewport visual capture (REQ-13):** `unreal_capture_viewport` for multimodal models with metric grid and nearby actor labels;
- **Live verified Tripo integration (REQ-15, REQ-16):** models, rigs and animations tested against the real Tripo API and imported into the editor as Skeletal Mesh and Animation Sequence;
- MCP connection with the editor, project readiness check and preparation with diff and "Revert";
- conversation with local, cloud or subscription AIs with explicit approval for modifications and proactive Python validation;
- own MCP server (`dbv-unreal-ai --mcp`) to use all tools from external agents.

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
- For development (not needed to use the [Releases](#install) installers): [Rust](https://rustup.rs) (stable), Node.js 20+ and the Tauri v2 system dependencies (WebView2 on Windows, WebKitGTK on Linux). Python 3.10+ for the toolset tests.
- Optional: an AI (Ollama/LM Studio, an API key or a subscription agent) and a [Tripo](https://www.tripo3d.ai) API key to generate models. Keys are stored in the system credential store, never in files.

---

<a name="usage"></a>

## 🧑‍💻 For developers

To run the app from source (if you only want to use it, the [installer](#install) is easier):

```bash
npm install
npm run tauri dev
```

Installers (in `src-tauri/target/release/bundle/`):

```bash
npm run tauri build
```

Pushing a `vX.Y.Z` tag makes GitHub Actions build Windows, macOS (universal) and Linux and attach them to a draft Release, which is published by hand.

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
