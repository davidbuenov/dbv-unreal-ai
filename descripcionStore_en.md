# Microsoft Store Listing — English (United States)

> Copy each field directly into the corresponding section of Partner Center ("Store listing"). Character limits shown match Partner Center constraints.

---

## Description *

DBV Unreal AI is a native artificial intelligence copilot for Unreal Engine 5.8: lightweight, fast, local-first, and privacy-respecting — with no forced subscriptions, no ads, and no tracking telemetry.

It connects directly to your running Unreal Engine editor using its native Model Context Protocol (MCP) HTTP server. Create, inspect, and modify 3D scenes using natural language, generate photorealistic worlds and 3D assets with AI, and run automated workflows safely and reversibly.

All core interactions with your engine run 100% locally on your machine. The AI assistant is flexible and completely optional: use local models with Ollama or LM Studio (100% offline), connect your active subscriptions via CLI agents (Claude Code, Google Antigravity, GitHub Copilot, Codex, Gemini), or provide cloud API keys (Anthropic, OpenAI, Google AI Studio, OpenRouter) with complete cost control.

Key Features:
• Bidirectional native communication with Unreal Engine 5.8 via MCP HTTP (port 8000)
• AI assistant with three distinct tiers: Local (offline), Subscription (CLI agents), and Cloud API
• Integrated support for Google Antigravity CLI, Claude Code, GitHub Copilot, Codex, and Gemini CLI
• Photorealistic world generation and placement with World Labs Marble (Gaussian Splatting .spz & collision .glb)
• 3D character generation, auto-rigging, and animation with Tripo3D v3
• Open-source native C++ Gaussian Splatting component (UDbvGaussianSplatComponent) with zero proprietary paywalls
• Interactive 3D asset preview powered by Three.js with studio lighting
• Automatic, reversible Unreal Engine project preparation with one-click backups
• Live viewport inspection and capture for visual context awareness
• Safe, one-click turn and transaction undo directly within Unreal Engine
• Persistent, disk-versioned Project Memory and Task Planning system
• Automatic detection of multiple Unreal Engine instances and parallel projects
• Interactive Help Center (F1) with Unreal Engine 5.8 guides, shortcuts, and the full guide to connect each AI, available offline
• Warm, polished interface with dark mode and high-contrast support
• Completely private: your projects, levels, and code never leave your machine
• Open-source under the MIT license

---

## What's new in this version

v0.2.3:
• Every subscription agent works: Google Antigravity (through its official agy_acp_server and sign-in with your Google account), Codex, GitHub Copilot and Claude Code.
• Guided sign-in: if the agent asks for it, the app requests it with your account and warns you that a browser may open.
• Full guide to connect each AI inside the app (F1 → AI Guide and from "Connect an AI"), offline and with a Copy button for every install command.
• If an agent is installed but something is missing (the Antigravity server or the Copilot CLI), the app says so and takes you to the instructions.
• Clearer assistant panel: context chips are no longer cut off.

---

## Product features (up to 20)

1. Native Unreal Engine 5.8 connection via MCP HTTP
2. AI assistant with Local, Subscription, and Cloud API options
3. Compatible with Google Antigravity CLI, Claude Code, Copilot
4. Photorealistic 3D world generation with Gaussian Splats (.spz)
5. 3D character generation and animation with Tripo3D
6. Open-source C++ Gaussian Splatting component
7. Interactive 3D preview with studio lighting
8. Automatic, reversible Unreal Engine project preparation
9. Live viewport capture for contextual visual analysis
10. One-click turn and action undo inside Unreal Engine
11. Persistent, file-based project memory and planning
12. Parallel detection for multiple Unreal Engine instances
13. Interactive Help Center with global F1 shortcut
14. Global keyboard shortcuts: F1, Ctrl+Shift+I, Esc, Ctrl+S
15. 100% private and local by default
16. Bilingual interface (Spanish and English)
17. Zero hidden telemetry or background tracking
18. Open source under the MIT License

---

## Search terms / Keywords (up to 7)

1. Unreal Engine
2. Unreal Engine 5
3. AI Copilot
4. 3D Generative AI
5. Model Context Protocol
6. Gaussian Splatting
7. Game Development

---

## Short description (up to 270 characters) *

Native AI copilot for Unreal Engine 5.8. Connects via MCP HTTP to assist with 3D scene creation, world building, and animated asset generation, prioritizing local execution, open standards, and user privacy.

---

## Notes for certification

The application communicates locally with the Unreal Engine 5.8 editor via a local Model Context Protocol HTTP server on localhost (port 8000).

Artificial Intelligence features are 100% optional and disabled by default:
- The app is completely functional without AI configured (project preparation, viewport inspection, 3D library and asset management).
- If the user chooses to connect an AI, they can choose between 100% local models (Ollama or LM Studio with no network access), CLI agents using existing user subscriptions, or cloud models using their own private API key securely stored in the OS credential manager.
- No user data or project files leave the computer without explicit user action and consent.
- Subscription agents (Claude Code, Codex, Gemini CLI, Google Antigravity, GitHub Copilot CLI) are separate programs the user installs and signs in to on their own; the app only launches them, with the project folder as working directory, when the user connects one. If an agent asks to sign in, it may open the system browser for the user to log in to that service.
