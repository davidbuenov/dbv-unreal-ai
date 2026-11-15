# Ficha de Microsoft Store — 中文（简体） / Chinese (Simplified, zh-CN)

> Partner Center 的每个字段都放在单独的代码块中（语言 **Chinese (Simplified) / 中文(中国)**）：在 DBV MD Reader 中点击代码块的 **复制** 按钮，再粘贴到对应的输入框。功能和搜索词每个一块，因为表单中每一项都是单独的输入框。括号中是字符数和表单上限。**认证说明**不在这里：每次提交只有一个字段，使用 [`descripcionStore_en.md`](./descripcionStore_en.md) 中的内容。此翻译由 AI 完成，建议由母语人士审阅。

---

## 描述 / Description *

(1684 / 10000)

```text
DBV Unreal AI 是一款面向 Unreal Engine 5.8 和 5.9 的原生 AI 助手：轻量、快速、默认在本地运行，并尊重你的隐私——没有强制订阅、没有广告，也没有侵入性的遥测。

它通过 Unreal 编辑器自带的 Model Context Protocol（MCP）HTTP 服务器，直接连接正在运行的编辑器。你可以用自然语言创建、修改和分析 3D 场景，用 AI 生成世界和物体，并以可撤销、安全的方式执行自动化脚本。

与引擎相关的整个工作流程都在本地完成。AI 助手灵活且可选：你可以使用 Ollama 或 LM Studio 运行本地模型（数据不离开你的电脑），通过命令行智能体使用你已有的订阅（Claude Code、Google Antigravity、GitHub Copilot、Codex、Gemini、Cursor、OpenCode），或使用云端 API 密钥（Anthropic、OpenAI、Google AI Studio、OpenRouter），并完全掌控费用。

主要功能：
• 通过 MCP HTTP（端口 8000）与 Unreal Engine 5.8 和 5.9 原生双向通信
• Unreal Engine 5.8.4 和 5.9 中的 Fab 内容：安装前可查看每个产品的缩略图和链接；只安装免费或已拥有且与你的编辑器兼容的内容，包括 Quixel Megascans
• 黑匣子：在不修改项目的情况下记录游戏，用按键标记时刻，并用数据分析，以调试只在游玩时出现的问题
• 在应用中用确定性工具为 MetaHuman 穿衣：贴合身体的布料连衣裙（Chaos Cloth）、保留材质的贴身衬衫和裤子，以及可保存到资源库以便重复使用的服装
• 使用本地微服务、ElevenLabs 和 Freesound 生成音效，并直接导入 Unreal
• 三种 AI 助手模式：本地（离线）、订阅（命令行智能体）和云端 API
• 内置支持 Google Antigravity CLI、Claude Code、GitHub Copilot、Codex、Gemini CLI、Cursor 和 OpenCode
• 自主级别，以及 AI 等待你批准时的提醒
• 使用 World Labs Marble 生成并放置照片级真实感世界（Gaussian Splatting .spz 与 .glb 碰撞网格）
• 使用 Tripo3D v3 生成 3D 模型、自动绑定骨骼并制作动画
• 自研 C++ Gaussian Splatting，由 GPU 绘制并支持随视角变化的颜色：无需商业插件即可显示 1000 万个 splat 的场景（在 5.9 中也可使用 Epic 的 GaussianSplat 插件）
• AI 可修改项目的 C++ 代码（始终以供你审阅的更改形式提出），应用会用 Live Coding 或 Unreal Build Tool 进行编译
• 分步引导的配方，并在编辑器中做最终检查；可离线搜索 Unreal 指南
• Convai 对话角色，以及关卡中带名称的区域
• 基于 Three.js 的交互式 3D 预览与影棚级布光
• 自动准备 Unreal Engine 项目，并保留可还原的备份
• 实时检查并截取视口，帮助 AI 理解场景
• 可将图片和文档粘贴（Ctrl+V）或拖入对话，并显示缩略图，AI 能看到它们并配合 Tripo 使用
• 在 Unreal Engine 中一键安全撤销整轮操作
• 独立、可版本化的项目记忆与计划，保存在磁盘上
• 自动检测多个同时运行的 Unreal Engine 实例和项目
• 交互式帮助中心（F1）：Unreal Engine 5.8 指南、快捷键以及连接每种 AI 的完整指南，可离线使用
• 界面支持简体中文、英语和西班牙语，设计温暖，提供深色模式和高对比度主题
• 完全私密：你的项目和关卡不会离开你的电脑
• MIT 开源许可证
```

---

## 此版本的新增功能 / What's new in this version

(365 / 1500)

```text
v0.6.1:
• Unreal Engine 5.8.4 中的 Fab：说“我想要一辆车”，在安装前即可在对话或新的 Fab 视图中看到每个产品的缩略图和 fab.com 链接。
• 只安装免费内容或已在你的 Fab 库中的内容（应用内不会购买任何东西），并且只安装与你的编辑器版本兼容的内容，包括 Quixel Megascans。已安装的内容会作为引用保存在资源库中。
• 即使项目使用 GUID 或已注册的引擎，应用也能找到引擎；如果找不到，可以手动选择其文件夹。
• C++ 插件现已为 Unreal 5.8 和 5.9 Preview 预先编译：无需 Visual Studio 即可使用 Fab 和 Splat。
• “项目”视图会逐步说明每个步骤及其解决方法，并提供“复制诊断信息”以便求助，不含密钥和你的用户名。
```

---

## 产品功能 / Product features (hasta 20)

**1.** (40 / 200)

```text
通过 MCP HTTP 原生连接 Unreal Engine 5.8 和 5.9
```

**2.** (24 / 200)

```text
AI 助手支持本地、订阅和云端 API 三种方式
```

**3.** (52 / 200)

```text
兼容 Claude Code、Antigravity、Copilot、Cursor 和 OpenCode
```

**4.** (34 / 200)

```text
使用 Gaussian Splats (.spz) 生成 3D 世界
```

**5.** (20 / 200)

```text
使用 Tripo3D 生成角色并制作动画
```

**6.** (32 / 200)

```text
自研 GPU Gaussian Splatting，无需商业插件
```

**7.** (16 / 200)

```text
带影棚布光的交互式 3D 查看器
```

**8.** (19 / 200)

```text
自动且可还原地准备 Unreal 项目
```

**9.** (12 / 200)

```text
截取视口以理解场景上下文
```

**10.** (12 / 200)

```text
在界面中一键撤销整轮操作
```

**11.** (16 / 200)

```text
保存在磁盘上、可版本化的项目记忆
```

**12.** (18 / 200)

```text
支持多个 Unreal 实例同时运行
```

**13.** (16 / 200)

```text
内置帮助中心，按 F1 即可打开
```

**14.** (32 / 200)

```text
全局快捷键：F1、Ctrl+Shift+I、Esc、Ctrl+S
```

**15.** (24 / 200)

```text
带缩略图和链接的 Fab 内容，仅限免费或已拥有
```

**16.** (16 / 200)

```text
界面支持简体中文、英语和西班牙语
```

**17.** (14 / 200)

```text
可将图片和文档粘贴或拖入对话
```

**18.** (12 / 200)

```text
基于 MIT 许可证开源
```

**19.** (18 / 200)

```text
AI 修改并编译项目的 C++ 代码
```

**20.** (17 / 200)

```text
引导配方与离线 Unreal 指南
```

---

## 搜索词 / Search terms (hasta 7)

**1.** (13 / 40)

```text
Unreal Engine
```

**2.** (4 / 40)

```text
虚幻引擎
```

**3.** (5 / 40)

```text
AI 助手
```

**4.** (9 / 40)

```text
3D 生成式 AI
```

**5.** (22 / 40)

```text
Model Context Protocol
```

**6.** (18 / 40)

```text
Gaussian Splatting
```

**7.** (4 / 40)

```text
游戏开发
```

---

## 简短描述 / Short description (hasta 270 caracteres) *

(102 / 270)

```text
面向 Unreal Engine 5.8 和 5.9 的原生 AI 助手。通过 MCP HTTP 连接编辑器，协助创建和修改 3D 场景，生成照片级真实感世界和带动画的角色，默认在本地运行并尊重你的隐私。
```
