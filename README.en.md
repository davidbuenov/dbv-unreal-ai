# dbv-unreal-ai

**[🇪🇸 Español](./README.md) · 🇬🇧 English**

> Native AI copilot for Unreal Engine 5.8: connect any AI (local or cloud) to your open project, prepare it when needed and generate assets with Tripo and World Labs straight into the editor.

## 📑 Table of Contents

- [About](#about)
- [Status](#status)
- [Requirements](#requirements)
- [How to run](#usage)
- [Project structure](#structure)
- [Changelog](#changelog)
- [License](#license)

---

<a name="about"></a>

## 📌 About

**dbv-unreal-ai** is a native desktop application (Windows, macOS and Linux) that talks with you and acts on the Unreal Engine 5.8 project you have open. It uses the AI you prefer —a local model, a cloud API or a subscription agent— and connects it to the editor through the MCP protocol. If the project is not yet ready to take orders from an AI, the application prepares it for you. It also integrates 3D generation services such as Tripo and World Labs to send results straight into Unreal and keep them available in an asset library.

**Built with:** Rust, Tauri v2, HTML/CSS/JavaScript, Unreal Engine 5.8 (MCP + Python + C++).

---

<a name="status"></a>

## 🚧 Status

In the specification phase. See [`dbv-specs-ops/docs/SPECIFICATIONS.md`](./dbv-specs-ops/docs/SPECIFICATIONS.md).

---

<a name="requirements"></a>

## 🧰 Requirements

- [Rust](https://rustup.rs) (stable) and the Tauri v2 system dependencies (WebView2 on Windows, WebKitGTK on Linux; nothing extra on macOS).
- Node.js 20+ (only for the Tauri CLI and tests).
- Unreal Engine 5.8 with a project open in the editor.

---

<a name="usage"></a>

## ▶️ How to run

```bash
npm install
npm run tauri dev
```

To build the installers:

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/
```

---

<a name="structure"></a>

## 📂 Project structure

```text
/
├── src/              # Frontend (HTML/CSS/JS, no bundler)
├── src-tauri/        # Native Rust backend (Tauri v2)
├── tests/            # Tests
├── dbv-specs-ops/    # Specifications, architecture, task.md, memory.md and CHANGELOG.md
└── README.md         # Spanish README
```

---

<a name="changelog"></a>

## 📋 Changelog

See [CHANGELOG.en.md](./dbv-specs-ops/CHANGELOG.en.md).

---

<a name="license"></a>

## 📄 License

MIT — see [LICENSE](./LICENSE).

Copyright (c) 2026 David Bueno Vallejo

---

> 🛠️ Built with **[dbv-specs-ops](https://github.com/davidbuenov/dbv-specs-ops)** — the SDD framework for AI-assisted development.
> Created by [David Bueno Vallejo](https://github.com/davidbuenov) — free and open source.
