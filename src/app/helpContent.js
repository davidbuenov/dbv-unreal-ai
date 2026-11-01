// =============================================================================
// DBV Unreal AI — Contenido del Centro de Ayuda por idioma
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-46, REQ-59, REQ-64
//
// Una tabla por idioma (antes eran ramas `isEs ? … : …`, que no admiten un tercer idioma). Lo que falte en un
// idioma se toma del inglés. El chino lo tradujo la IA: conviene que lo revise una persona nativa. La pestaña
// «Guía de IAs» no está aquí: es la guía común de las apps DBV (`src/ai/guide.js`).

const PLUGINS = 'ModelContextProtocol, ToolsetRegistry, PythonScriptPlugin, EditorToolset';

/** Plantilla de la pestaña de Unreal. */
const ue = (s) => `
        <div class="help-section">
          <h4>${s.title}</h4>
          <p>${s.intro}</p>
          <div class="help-box">
            <strong>${s.plugins}</strong>
            <p class="mono" style="margin-top: 4px;">${PLUGINS}</p>
          </div>
          <p>${s.prepare}</p>
        </div>
      `;

/** Plantilla de la pestaña de Convai (REQ-59). */
const convai = (s) => `
        <div class="help-section">
          <h4>${s.title}</h4>
          <p>${s.intro}</p>
          <div class="help-box">
            <strong>${s.needTitle}</strong>
            <ol>${s.needs.map((item) => `<li>${item}</li>`).join('')}</ol>
          </div>
          <h4>${s.stepsTitle}</h4>
          <ol>${s.steps.map((item) => `<li>${item}</li>`).join('')}</ol>
          <h4>${s.issuesTitle}</h4>
          <ul>${s.issues.map((item) => `<li>${item}</li>`).join('')}</ul>
        </div>`;

/** Plantilla de la pestaña de atajos. */
const shortcuts = (s) => `
        <div class="help-section">
          <h4>${s.title}</h4>
          <table class="help-table">
            <thead>
              <tr><th>${s.shortcut}</th><th>${s.action}</th></tr>
            </thead>
            <tbody>
              <tr><td><span class="help-kbd">F1</span></td><td>${s.help}</td></tr>
              <tr><td><span class="help-kbd">${s.shift}</span> / <span class="help-kbd">Cmd+Shift+I</span></td><td>${s.chat}</td></tr>
              <tr><td><span class="help-kbd">Esc</span></td><td>${s.esc}</td></tr>
              <tr><td><span class="help-kbd">Ctrl+S</span> / <span class="help-kbd">Cmd+S</span></td><td>${s.save}</td></tr>
            </tbody>
          </table>
        </div>
      `;

const LINK = 'target="_blank" rel="noopener noreferrer" style="color: var(--accent);"';

/** Plantilla de la pestaña de enlaces. */
const links = (s) => `
        <div class="help-section">
          <h4>${s.title}</h4>
          <p>${s.intro}</p>
          <div class="help-box" style="display: grid; gap: 8px;">
            <div>🌐 <a href="https://github.com/davidbuenov/dbv-unreal-ai" ${LINK}>${s.repo}</a></div>
            <div>🤖 <a href="https://antigravity.google/product/antigravity-cli" ${LINK}>Google Antigravity CLI</a></div>
            <div>⚡ <a href="https://github.com/google-gemini/gemini-cli" ${LINK}>Gemini CLI (GitHub)</a></div>
            <div>🎨 <a href="https://www.tripo3d.ai" ${LINK}>Tripo 3D Studio</a></div>
            <div>🌍 <a href="https://www.worldlabs.ai" ${LINK}>World Labs (Marble)</a></div>
            <div>🗣️ <a href="https://convai.com" ${LINK}>Convai (Playground)</a></div>
          </div>
        </div>
      `;

const TEXTS = {
  es: {
    loading: 'Cargando la guía…',
    ue: {
      title: 'Requisitos de Unreal Engine 5.8',
      intro: 'DBV Unreal AI se comunica de forma bidireccional y nativa mediante el servidor MCP HTTP integrado en el motor (puerto 8000 por defecto).',
      plugins: 'Plugins del motor necesarios:',
      prepare: 'Para conectar un proyecto sin editar archivos manualmente, usa el botón <strong>«Preparar proyecto»</strong> en la vista <em>Proyecto</em>. La app activará los plugins, configurará el arranque automático y realizará una copia de seguridad reversible.',
    },
    convai: {
      title: 'Convai: personajes que hablan',
      intro: 'Con Convai, un MetaHuman de tu nivel conversa contigo por voz (o por texto), mueve la boca y hace acciones: seguirte, ir a un sitio o llevarte a él. DBV Unreal AI lo prepara todo desde la conversación.',
      needTitle: 'Qué necesitas',
      needs: [
        '<strong>Unreal Engine 5.8 o posterior</strong> y el proyecto preparado con la app (vista <em>Proyecto</em> → «Preparar proyecto»).',
        'El <strong>plugin Convai</strong> instalado desde Fab y activado en el proyecto (<em>Edit → Plugins</em>), con la <strong>sesión iniciada</strong> en su ventana dentro del editor. La app no necesita ninguna clave de Convai.',
        'Un <strong>personaje creado en el Playground</strong> de convai.com (personalidad, voz e idioma) y su <strong>Character ID</strong>.',
        'Un <strong>MetaHuman ensamblado</strong> en el nivel: se coloca su Blueprint <span class="mono">BP_&lt;Nombre&gt;</span> (en <span class="mono">/Game/MetaHumans/&lt;Nombre&gt;</span>), no el asset del personaje, que no se puede arrastrar a la escena.',
      ],
      stepsTitle: 'Pasos',
      steps: [
        '<strong>Ajustes → Convai</strong>: añade el personaje con un nombre y su Character ID. La vista <em>Proyecto</em> comprueba que el plugin, sus herramientas y la sesión están bien.',
        'En la conversación pide: <em>«Haz que BP_VictoriaNew sea Victoria con Convai»</em>. La IA lee las guías de Convai, usa el Character ID registrado y te pide permiso en cada paso (componentes, jugador, movimiento y NavMesh).',
        'Pide <em>«Define las zonas de este nivel»</em>: revisas los sitios propuestos y se crean en <span class="mono">DBV/Zones</span>. El personaje los conoce y puede ir a ellos.',
        'Guarda el nivel, pulsa <strong>Play</strong> y habla con el micrófono, o pulsa <strong>Intro</strong> para escribir. Prueba «Sígueme», «Ve al mirador norte» o «Llévame al inicio».',
      ],
      issuesTitle: 'Problemas conocidos',
      issues: [
        '<strong>Brazos y piernas estirados:</strong> abre <span class="mono">metahuman_base_skel</span>, activa <em>Show Retargeting Options</em>, clic derecho en <span class="mono">root</span> → <em>Recursively Set Translation Retargeting Skeleton</em>, pon <span class="mono">pelvis</span> en <em>Animation Scaled</em> y guarda.',
        '<strong>«Llévame a…» y no se mueve:</strong> pide a la IA que mejore la descripción de la acción <em>Escort</em>. Si en tu idioma sigue sin moverse, configura el idioma del personaje en el Playground; «Ve a…» usa <em>Move to</em>, que es más directa.',
        '<strong>Un agente ve el editor dos veces:</strong> el asistente «Set Up AI Coding» de Convai escribe un <span class="mono">.mcp.json</span> y un <span class="mono">CLAUDE.md</span> en el proyecto. La vista <em>Proyecto</em> lo avisa; quítalos si usas la app.',
        '<strong>Volver a ensamblar el MetaHuman</strong> regenera su Blueprint y borra la configuración de Convai: repite el paso 2.',
        'Las acciones de Convai <strong>guardan Blueprints</strong>: «Deshacer este turno» puede no revertirlas. Ten el proyecto en control de versiones.',
      ],
    },
    shortcuts: { title: 'Atajos de Teclado Globales', shortcut: 'Atajo', action: 'Acción', help: 'Abrir este Centro de Ayuda', shift: 'Ctrl+Mayús+I', chat: 'Abrir / alternar panel de Conversación', esc: 'Cerrar cualquier ventana modal activa', save: 'Guardar en vistas de edición' },
    links: { title: 'Enlaces y Documentación Oficial', intro: 'Accede a los recursos y guías detalladas:', repo: 'Repositorio oficial en GitHub' },
  },
  en: {
    loading: 'Loading the guide…',
    ue: {
      title: 'Unreal Engine 5.8 Requirements',
      intro: 'DBV Unreal AI communicates natively and bidirectionally via the built-in MCP HTTP server in the engine (default port 8000).',
      plugins: 'Required Engine Plugins:',
      prepare: 'To prepare a project without manual editing, click <strong>«Prepare project»</strong> in the <em>Project</em> view. The app activates plugins, sets auto-start and creates a reversible backup.',
    },
    convai: {
      title: 'Convai: characters that talk',
      intro: 'With Convai, a MetaHuman in your level talks with you by voice (or text), moves its lips and performs actions: following you, going somewhere or taking you there. DBV Unreal AI sets it all up from the conversation.',
      needTitle: 'What you need',
      needs: [
        '<strong>Unreal Engine 5.8 or later</strong> and the project prepared with the app (<em>Project</em> view → "Prepare project").',
        'The <strong>Convai plugin</strong> installed from Fab and enabled in the project (<em>Edit → Plugins</em>), <strong>signed in</strong> in its window inside the editor. The app needs no Convai key.',
        'A <strong>character created in the Playground</strong> at convai.com (personality, voice and language) and its <strong>Character ID</strong>.',
        'An <strong>assembled MetaHuman</strong> in the level: place its Blueprint <span class="mono">BP_&lt;Name&gt;</span> (in <span class="mono">/Game/MetaHumans/&lt;Name&gt;</span>), not the character asset, which cannot be dragged into the scene.',
      ],
      stepsTitle: 'Steps',
      steps: [
        '<strong>Settings → Convai</strong>: add the character with a name and its Character ID. The <em>Project</em> view checks that the plugin, its tools and the sign-in are fine.',
        'In the conversation ask: <em>"Make BP_VictoriaNew be Victoria with Convai"</em>. The AI reads the Convai guides, uses the registered Character ID and asks your permission at each step (components, player, movement and NavMesh).',
        'Ask <em>"Define the zones of this level"</em>: you review the proposed places and they are created in <span class="mono">DBV/Zones</span>. The character knows them and can walk there.',
        'Save the level, press <strong>Play</strong> and talk with the microphone, or press <strong>Enter</strong> to type. Try "Follow me", "Go to the north lookout" or "Take me to the start".',
      ],
      issuesTitle: 'Known issues',
      issues: [
        '<strong>Stretched arms and legs:</strong> open <span class="mono">metahuman_base_skel</span>, enable <em>Show Retargeting Options</em>, right-click <span class="mono">root</span> → <em>Recursively Set Translation Retargeting Skeleton</em>, set <span class="mono">pelvis</span> to <em>Animation Scaled</em> and save.',
        '<strong>"Take me to…" and it does not move:</strong> ask the AI to improve the description of the <em>Escort</em> action. If it still does not move in your language, set the character language in the Playground; "Go to…" uses <em>Move to</em>, which is more direct.',
        '<strong>An agent sees the editor twice:</strong> the Convai "Set Up AI Coding" assistant writes a <span class="mono">.mcp.json</span> and a <span class="mono">CLAUDE.md</span> in the project. The <em>Project</em> view warns about it; remove them if you use the app.',
        '<strong>Re-assembling the MetaHuman</strong> regenerates its Blueprint and deletes the Convai setup: repeat step 2.',
        'Convai actions <strong>save Blueprints</strong>: "Undo this turn" may not revert them. Keep the project under version control.',
      ],
    },
    shortcuts: { title: 'Global Keyboard Shortcuts', shortcut: 'Shortcut', action: 'Action', help: 'Open this Help Center', shift: 'Ctrl+Shift+I', chat: 'Open / toggle Conversation panel', esc: 'Close any active modal dialog', save: 'Save in edit views' },
    links: { title: 'Official Links and Documentation', intro: 'Access detailed resources and guides:', repo: 'Official GitHub repository' },
  },
  'zh-CN': {
    loading: '正在加载指南…',
    ue: {
      title: 'Unreal Engine 5.8 要求',
      intro: 'DBV Unreal AI 通过引擎内置的 MCP HTTP 服务器（默认端口 8000）与编辑器进行原生的双向通信。',
      plugins: '需要的引擎插件：',
      prepare: '无需手动编辑文件即可连接项目：在<em>项目</em>视图中点击 <strong>«准备项目»</strong>。应用会启用插件、设置自动启动，并创建可撤销的备份。',
    },
    convai: {
      title: 'Convai：会说话的角色',
      intro: '借助 Convai，关卡中的 MetaHuman 可以通过语音（或文字）与你对话、做出口型并执行动作：跟随你、走到某处或带你去那里。DBV Unreal AI 在对话中完成全部设置。',
      needTitle: '你需要',
      needs: [
        '<strong>Unreal Engine 5.8 或更高版本</strong>，以及用本应用准备好的项目（<em>项目</em>视图 → «准备项目»）。',
        '从 Fab 安装并在项目中启用的 <strong>Convai 插件</strong>（<em>Edit → Plugins</em>），并在编辑器内的插件窗口中<strong>登录</strong>。本应用不需要任何 Convai 密钥。',
        '在 convai.com 的 <strong>Playground 中创建的角色</strong>（性格、声音和语言）及其 <strong>Character ID</strong>。',
        '关卡中一个<strong>已组装的 MetaHuman</strong>：放置它的蓝图 <span class="mono">BP_&lt;名称&gt;</span>（位于 <span class="mono">/Game/MetaHumans/&lt;名称&gt;</span>），而不是角色资源本身——后者无法拖入场景。',
      ],
      stepsTitle: '步骤',
      steps: [
        '<strong>设置 → Convai</strong>：用名称和 Character ID 添加角色。<em>项目</em>视图会检查插件、其工具和登录状态是否正常。',
        '在对话中提出：<em>「让 BP_VictoriaNew 成为使用 Convai 的 Victoria」</em>。AI 会阅读 Convai 指南，使用已登记的 Character ID，并在每一步（组件、玩家、移动和 NavMesh）征求你的许可。',
        '提出 <em>「定义这个关卡的区域」</em>：你审核建议的地点，它们会创建在 <span class="mono">DBV/Zones</span> 中。角色知道这些地点并可以走过去。',
        '保存关卡，点击 <strong>Play</strong>，用麦克风说话，或按 <strong>Enter</strong> 输入文字。试试「跟着我」、「去北边的观景台」或「带我去起点」。',
      ],
      issuesTitle: '已知问题',
      issues: [
        '<strong>手臂和腿被拉长：</strong>打开 <span class="mono">metahuman_base_skel</span>，启用 <em>Show Retargeting Options</em>，右键点击 <span class="mono">root</span> → <em>Recursively Set Translation Retargeting Skeleton</em>，把 <span class="mono">pelvis</span> 设为 <em>Animation Scaled</em> 并保存。',
        '<strong>说「带我去…」但它不动：</strong>让 AI 改进 <em>Escort</em> 动作的描述。如果用你的语言仍然不动，请在 Playground 中设置角色的语言；「去…」使用 <em>Move to</em>，更加直接。',
        '<strong>智能体看到两次编辑器：</strong>Convai 的「Set Up AI Coding」助手会在项目中写入 <span class="mono">.mcp.json</span> 和 <span class="mono">CLAUDE.md</span>。<em>项目</em>视图会提示；如果使用本应用，请删除它们。',
        '<strong>重新组装 MetaHuman</strong> 会重新生成其蓝图并删除 Convai 设置：请重复第 2 步。',
        'Convai 的操作会<strong>保存蓝图</strong>：«撤销本轮» 可能无法还原。请把项目放在版本控制中。',
      ],
    },
    shortcuts: { title: '全局快捷键', shortcut: '快捷键', action: '操作', help: '打开此帮助中心', shift: 'Ctrl+Shift+I', chat: '打开 / 切换对话面板', esc: '关闭当前的模态窗口', save: '在编辑视图中保存' },
    links: { title: '链接和官方文档', intro: '访问资源和详细指南：', repo: 'GitHub 官方仓库' },
  },
};

/** Textos de la ayuda en `lang`, con el inglés para lo que falte. */
export function helpTexts(lang) {
  return { ...TEXTS.en, ...(TEXTS[lang] ?? {}) };
}

/** HTML de cada pestaña (salvo la guía de IAs) en `lang`. */
export function helpTab(tab, lang) {
  const s = helpTexts(lang);
  const pages = {
    ue: () => ue(s.ue),
    ai: () => `<p class="hint">${s.loading}</p>`,
    convai: () => convai(s.convai),
    shortcuts: () => shortcuts(s.shortcuts),
    links: () => links(s.links),
  };
  return (pages[tab] ?? pages.ue)();
}

/** Pestañas que existen. */
export const HELP_TABS = ['ue', 'ai', 'convai', 'shortcuts', 'links'];
