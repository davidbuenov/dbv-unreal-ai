# 🤖 Using an AI with DBV Unreal AI

> 🌐 [Español](./IA.md) · **English** · [← Back to the README](../README.en.md)

The AI in **DBV Unreal AI** is **optional**. Without connecting one, the app detects your Unreal editors, prepares the project, generates models with Tripo from the **Generate** view and manages the library. The AI adds the **Conversation**: you ask for something about your project and it acts in the editor with your permission. This guide explains **what you need** depending on the kind of AI you prefer.

## The essentials in one table

| I want to use… | What I need to install or have | Does anything leave my machine? | Does it cost money? |
| --- | --- | --- | --- |
| **A local AI** (Ollama, LM Studio, llama.cpp…) | The server application and at least one downloaded model | **No** | No (uses your own CPU/GPU and RAM) |
| **Claude, ChatGPT, Gemini or OpenRouter with an API key** | An API key from that provider | Yes, to the selected provider | Yes, pay-per-use at provider rates |
| **My monthly subscription** (Claude Pro/Max, ChatGPT Plus, Copilot) | The official **installed agent** signed in: Claude Code, Codex or Copilot CLI | Yes, to the agent's service | Covered by your subscription |

> ⚠️ **A subscription is not an API key.** Subscribing to Claude Pro, ChatGPT Plus or Gemini does not give you an API key (those are bought and billed separately on developer consoles). With a subscription, the recommended route is the **installed agent** (third row).

To connect an AI: **Settings → Connect an AI**, or open the **Conversation** (`Ctrl+Shift+I`, `Cmd+Shift+I` on macOS) and, if none is connected, the connection assistant opens.

> 🎨 **Tripo and World Labs are separate.** Generating 3D models and photorealistic worlds uses your Tripo or World Labs key (Settings) and spends credits of your corresponding account, whatever AI the conversation uses.

---

## 1. A local AI (nothing leaves your machine)

The most private option: your data never leaves the machine and there is no per-use cost. Speed and quality depend on your hardware and on the model size. For the AI to use the editor tools, pick a model that supports tools, for example `qwen3:8b`.

### Ollama
1. Download and install [Ollama](https://ollama.com/download).
2. Pull a model from the terminal, for example:
   ```bash
   ollama pull qwen3:8b
   ```
3. In DBV Unreal AI, open **Connect an AI**. If Ollama is running, it shows a green check and a **Use** button.
4. Choose the model, click **Test connection** and **Save**.

### LM Studio
1. Install [LM Studio](https://lmstudio.ai), download a model and start its local server (*Local Server*, port `1234` by default).
2. DBV Unreal AI detects the server automatically.

---

## 2. In the cloud, with your API key

For the full power of frontier models with an **API key**:

| Provider | Where to get the key |
| --- | --- |
| **Anthropic (Claude)** | Anthropic Console |
| **OpenAI (ChatGPT)** | OpenAI Platform |
| **Google (Gemini)** | Google AI Studio |
| **OpenRouter** | openrouter.ai |

1. Open **Connect an AI → Add a cloud AI (API key)…**
2. Choose the provider and paste your key.
3. Click **Test connection**, choose the model and **Save**.

- **Secure storage:** the key is kept in your system's credential store (Windows Credential Manager, macOS Keychain or Secret Service on Linux). Never in text files or in the browser. The Tripo key too.
- **Cloud notice:** before the first message to a cloud provider, the app reminds you that your request and the project context go to it.

---

## 3. With your subscription: installed agents (ACP)

If you have **Claude Pro/Max, ChatGPT Plus or GitHub Copilot**, you can connect its official command-line tool through the Agent Client Protocol (ACP):

| Agent | Requirements | Launch command |
| --- | --- | --- |
| **Claude Code** | Claude Code and [Node.js](https://nodejs.org) | `npx -y @agentclientprotocol/claude-agent-acp` |
| **Google Antigravity CLI** | [Antigravity CLI](https://antigravity.google/product/antigravity-cli) (`agy`) | `agy_acp_server` or `agy --acp` |
| **Codex (OpenAI)** | Codex and Node.js | `npx -y @zed-industries/codex-acp` |
| **GitHub Copilot CLI** | GitHub Copilot CLI | `copilot --acp` |
| **Gemini CLI** | [Gemini CLI](https://github.com/google-gemini/gemini-cli) (for API / Vertex AI) | `gemini --acp` |

> ℹ️ **Google Antigravity CLI and Gemini CLI.** Google uses [Antigravity CLI](https://antigravity.google/product/antigravity-cli) for Google One, Gemini Advanced, and free accounts. Meanwhile, [Gemini CLI](https://github.com/google-gemini/gemini-cli) is reserved for pay-per-use access (API key, Vertex AI, or Gemini Code Assist). DBV Unreal AI supports both according to your account type.

**Steps:**
1. Install the agent and make sure it is in your `PATH`.
2. Open it once in a terminal to sign in with your account.
3. In DBV Unreal AI, open **Connect an AI**: the agent appears ready to **Connect**.

- **The agent arrives already connected to Unreal:** the app gives it its own MCP server (`dbv-unreal-ai`), with the editor tools, the library and the whole Tripo API.
- **Permission control:** every time the agent wants to use a tool, read or change files, DBV Unreal AI asks you first (Allow once, Always allow in this conversation, or Deny).
- **What it generates is visible:** if the agent starts a Tripo job, the conversation shows its card with the progress and the console of what is sent.
- **Restore point:** after each turn, the app shows which files changed on disk, with **Undo** for all of them or one by one.

---

## 4. What is specific to Unreal and Tripo

- **Nothing changes without your permission.** Queries (listing actors, reading the scene) run on their own; anything that changes the project appears in a card with the full detail (the whole Python code, the parameters) and waits for your "Allow". Python is checked first against the real API of your engine version.
- **Nothing is spent without your permission.** Every Tripo operation (generate, rig, animate, convert…) first shows its cost from Tripo's price list and exactly what will be sent. The per-session credit cap is set in Settings.
- **Reference images and documents.** With 📎 in the conversation you can attach images and documents. A vision model sees the image; one without vision gets its name and path, and the app tells you. Either way the AI can use the image to generate a model with Tripo.
- **From outside the app.** Settings → **Use DBV Unreal AI from other agents (MCP)** gives you the configuration block for Claude Code, Cursor or Gemini CLI with the exact path of the executable.
