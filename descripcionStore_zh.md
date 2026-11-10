# Ficha de Microsoft Store — 中文（简体） / Chinese (Simplified, zh-CN)

> Copia cada campo tal cual a la sección correspondiente de Partner Center ("Descripción de Store") en el idioma **Chinese (Simplified) / 中文(中国)**. Los límites de caracteres son los que muestra el propio formulario. Las **notas para la certificación** no van aquí: son un solo campo por envío y se usan las de [`descripcionStore_en.md`](./descripcionStore_en.md). Traducción hecha con IA: conviene que la revise una persona nativa.

---

## 描述 / Description *

DBV Unreal AI 是一款面向 Unreal Engine 5.8 的原生 AI 助手：轻量、快速、默认在本地运行，并尊重你的隐私——没有强制订阅、没有广告，也没有侵入性的遥测。

它通过 Unreal 编辑器自带的 Model Context Protocol（MCP）HTTP 服务器，直接连接正在运行的编辑器。你可以用自然语言创建、修改和分析 3D 场景，用 AI 生成世界和物体，并以可撤销、安全的方式执行自动化脚本。

与引擎相关的整个工作流程都在本地完成。AI 助手灵活且可选：你可以使用 Ollama 或 LM Studio 运行本地模型（数据不离开你的电脑），通过命令行智能体使用你已有的订阅（Claude Code、Google Antigravity、GitHub Copilot、Codex、Gemini），或使用云端 API 密钥（Anthropic、OpenAI、Google AI Studio、OpenRouter），并完全掌控费用。

主要功能：
• 通过 MCP HTTP（端口 8000）与 Unreal Engine 5.8 原生双向通信
• 黑匣子：在不修改项目的情况下记录游戏，用按键标记时刻，并用数据分析，以调试只在游玩时出现的问题
• 在应用中用确定性工具为 MetaHuman 穿衣（宽松服装、带布料的裙子和连衣裙；紧身服装为实验性功能）
• 使用本地微服务、ElevenLabs 和 Freesound 生成音效，并直接导入 Unreal
• 三种 AI 助手模式：本地（离线）、订阅（命令行智能体）和云端 API
• 内置支持 Google Antigravity CLI、Claude Code、GitHub Copilot、Codex 和 Gemini CLI
• 使用 World Labs Marble 生成并放置照片级真实感世界（Gaussian Splatting .spz 与 .glb 碰撞网格）
• 使用 Tripo3D v3 生成 3D 模型、自动绑定骨骼并制作动画
• 自研 C++ Gaussian Splatting，由 GPU 绘制并支持随视角变化的颜色：无需商业插件即可显示 1000 万个 splat 的场景
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

---

## 此版本的新增功能 / What's new in this version

v0.4.0：
• 声音特效 (SFX) 生成与搜索：「生成」中新增「声音」选项卡，支持本地微服务 (Stable Audio Open 1.0)、ElevenLabs 和交互式 Freesound 音频库，具备音频直接试听和按需添加至资源库功能。
• 原生导入音频至 Unreal Engine（音频的「发送至 Unreal」）：直接导入为 USoundWave / SoundCue 资产，并提供 MCP 工具供智能体将音频绑定至场景 Actor。
• 实时多模态视觉：对话框新增「📷 截取视口」按钮，支持多模态模型即时查看并分析当前编辑器画面。
• 可配置的版权归属：在「项目设置」中自定义作者版权，同时固定保留 DBV Unreal AI 和 dbv-specs-ops 的来源署名。
• 具有弹性的 Unreal Engine MCP 端口自动探测：连接时自动扫描 8000..8005 端口。
• 扩展的帮助中心：新增 Tripo、World Labs 和声音专属帮助选项卡，支持一键直达。

---

## 产品功能 / Product features (hasta 20)

1. 通过 MCP HTTP 原生连接 Unreal Engine 5.8
2. AI 助手支持本地、订阅和云端 API 三种方式
3. 兼容 Google Antigravity CLI、Claude Code 和 Copilot
4. 使用 Gaussian Splats (.spz) 生成 3D 世界
5. 使用 Tripo3D 生成角色并制作动画
6. 自研 GPU Gaussian Splatting，无需商业插件
7. 带影棚布光的交互式 3D 查看器
8. 自动且可还原地准备 Unreal 项目
9. 截取视口以理解场景上下文
10. 在界面中一键撤销整轮操作
11. 保存在磁盘上、可版本化的项目记忆
12. 支持多个 Unreal 实例同时运行
13. 内置帮助中心，按 F1 即可打开
14. 全局快捷键：F1、Ctrl+Shift+I、Esc、Ctrl+S
15. 默认 100% 私密、本地运行
16. 界面支持简体中文、英语和西班牙语
17. 可将图片和文档粘贴或拖入对话
18. 基于 MIT 许可证开源
19. AI 修改并编译项目的 C++ 代码
20. 引导配方与离线 Unreal 指南

---

## 搜索词 / Search terms (hasta 7)

1. Unreal Engine
2. 虚幻引擎
3. AI 助手
4. 3D 生成式 AI
5. Model Context Protocol
6. Gaussian Splatting
7. 游戏开发

---

## 简短描述 / Short description (hasta 270 caracteres) *

面向 Unreal Engine 5.8 的原生 AI 助手。通过 MCP HTTP 连接编辑器，协助创建和修改 3D 场景，生成照片级真实感世界和带动画的角色，默认在本地运行并尊重你的隐私。
