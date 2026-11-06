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

/** Plantilla de la pestaña de Tripo. */
const tripo = (s) => `
        <div class="help-section">
          <h4>${s.title}</h4>
          <p>${s.intro}</p>
          <div class="help-box">
            <strong>${s.needTitle}</strong>
            <ol>${s.needs.map((item) => `<li>${item}</li>`).join('')}</ol>
          </div>
          <h4>${s.stepsTitle}</h4>
          <ol>${s.steps.map((item) => `<li>${item}</li>`).join('')}</ol>
        </div>`;

/** Plantilla de la pestaña de World Labs. */
const worldlabs = (s) => `
        <div class="help-section">
          <h4>${s.title}</h4>
          <p>${s.intro}</p>
          <div class="help-box">
            <strong>${s.needTitle}</strong>
            <ol>${s.needs.map((item) => `<li>${item}</li>`).join('')}</ol>
          </div>
          <h4>${s.stepsTitle}</h4>
          <ol>${s.steps.map((item) => `<li>${item}</li>`).join('')}</ol>
        </div>`;

/** Plantilla de la pestaña de Audio y Sonidos (REQ-71). */
const audio = (s) => `
        <div class="help-section">
          <h4>${s.title}</h4>
          <p>${s.intro}</p>
          <div class="help-box">
            <strong>${s.needTitle}</strong>
            <ol>${s.needs.map((item) => `<li>${item}</li>`).join('')}</ol>
          </div>
          <h4>${s.providersTitle}</h4>
          <ul>${s.providers.map((item) => `<li>${item}</li>`).join('')}</ul>
          <h4>${s.stepsTitle}</h4>
          <ol>${s.steps.map((item) => `<li>${item}</li>`).join('')}</ol>
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
            <div>🔊 <a href="https://elevenlabs.io/app/developers/api-keys" ${LINK}>ElevenLabs (API Keys)</a></div>
            <div>🎵 <a href="https://freesound.org/apiv2/apply" ${LINK}>Freesound (API Credentials)</a></div>
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
        '<strong>Un agente ve el editor dos veces:</strong> el asistente «Set Up AI Coding» de Convai (o Unreal) puede escribir un <span class="mono">.mcp.json</span> que conecta a los agentes directamente con el editor. La vista <em>Proyecto</em> lo avisa: borra en ese fichero la entrada que termina en <span class="mono">/mcp</span>. Las instrucciones que Convai deja en <span class="mono">CLAUDE.md</span> no molestan y puedes dejarlas.',
        '<strong>Volver a ensamblar el MetaHuman</strong> regenera su Blueprint y borra la configuración de Convai: repite el paso 2.',
        'Las acciones de Convai <strong>guardan Blueprints</strong>: «Deshacer este turno» puede no revertirlas. Ten el proyecto en control de versiones.',
      ],
    },
    tripo: {
      title: 'Modelos 3D con Tripo',
      intro: 'Genera mallas poligonales optimizadas a partir de descripciones de texto o imágenes de referencia con la API de Tripo Studio v3.',
      needTitle: 'Qué necesitas',
      needs: [
        'Una cuenta en <strong>tripo3d.ai</strong> con créditos disponibles.',
        'Tu <strong>API Key</strong> configurada en <em>Ajustes → Tripo</em>. La app comprueba el saldo y respeta tu tope de presupuesto.',
      ],
      stepsTitle: 'Flujo de trabajo',
      steps: [
        'En la vista <em>Generar</em> (pestaña «Texto» o «Imagen»), describe el modelo o adjunta una imagen y pulsa <strong>Comenzar</strong>.',
        'La tarjeta de coste te muestra la estimación de créditos antes de lanzar el trabajo.',
        'El modelo terminado se guarda en la <em>Biblioteca</em> como archivo GLB.',
        'Desde la biblioteca puedes aplicar <strong>Rigging automático</strong>, vincular animaciones compatibles o pulsar <strong>«Enviar a Unreal»</strong> para importarlo como StaticMesh o SkeletalMesh en <span class="mono">/Game/DBV/Models/</span>.',
      ],
    },
    worldlabs: {
      title: 'Mundos y Escenarios con World Labs',
      intro: 'Genera entornos 3D fotorrealistas completos mediante la tecnología Marble de World Labs a partir de un texto o imagen panorámica.',
      needTitle: 'Qué necesitas',
      needs: [
        'Una cuenta en <strong>worldlabs.ai</strong> con acceso a la API Marble.',
        'Tu <strong>API Key</strong> configurada en <em>Ajustes → World Labs</em>.',
        'El proyecto de Unreal preparado con la app: incluye el componente C++ nativo por GPU <span class="mono">UDbvGaussianSplatComponent</span>.',
      ],
      stepsTitle: 'Flujo de trabajo',
      steps: [
        'En la vista <em>Generar</em> (pestaña «Mundo»), escribe la descripción del escenario y pulsa <strong>Comenzar</strong>.',
        'La generación suele tardar unos 5 minutos. Si cierras la app, el trabajo se reanuda automáticamente al volver a abrirla sin perder el mundo generado.',
        'El resultado contiene un archivo de Gaussian Splatting (<span class="mono">.spz</span>) para el fotorrealismo y una malla de colisión simplificada (<span class="mono">.glb</span>) para las físicas.',
        'Pulsa <strong>«Enviar a Unreal»</strong> en la biblioteca para colocar en el nivel el actor de splats y su colisionador físico perfectamente sincronizados.',
      ],
    },
    audio: {
      title: 'Sonidos y Efectos de Sonido (SFX)',
      intro: 'Genera efectos sonoros o busca grabaciones de audio y música para ambientar tu juego, listos para importarse como assets USoundWave en Unreal Engine.',
      needTitle: 'Tres proveedores y cómo configurarlos',
      needs: [
        '<strong>Local (dbv-audio-service):</strong> 100% gratuito, privado y sin cuota de nube. Ejecuta el microservicio en local con el modelo Stable Audio Open 1.0 en <span class="mono">http://127.0.0.1:8765</span> aprovechando tu GPU.',
        '<strong>ElevenLabs SFX:</strong> Efectos de sonido realistas de altísima calidad vía API en la nube (el plan gratuito incluye 10.000 caracteres mensuales). Al crear tu clave en <a href="https://elevenlabs.io/app/developers/api-keys" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">ElevenLabs API Keys</a>, dale permisos completos (<em>Full Access</em> o al menos <em>user_read</em> y <em>sound_generation</em>), copia la clave secreta que empieza por <span class="mono">sk_</span> y pégala en <em>Ajustes → Audio</em>.',
        '<strong>Freesound:</strong> Búsqueda y descarga directa de sonidos y grabaciones reales (licencias libres CC0 y CC-BY). Crea una cuenta gratuita en Freesound, solicita tus credenciales en <a href="https://freesound.org/apiv2/apply" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">Freesound - API credentials</a> (obtendrás un <em>Client secret / Api key</em>) y pégala como token en <em>Ajustes → Audio</em>.',
      ],
      providersTitle: 'Duración y bucles',
      providers: [
        'Tanto con el servicio local como con ElevenLabs puedes elegir la duración exacta (de 0.5 a 30 segundos).',
        'La opción <strong>Bucle continuo</strong> genera el sonido de manera que el inicio y el final encajen perfectamente (ideal para motores, viento o lluvia).',
      ],
      stepsTitle: 'Flujo de trabajo',
      steps: [
        'En la vista <em>Generar</em>, entra en la pestaña <strong>Sonidos</strong> (🎵).',
        'Selecciona el proveedor deseado, escribe lo que quieres escuchar (ej. <em>«pasos en grava mojada»</em>) y pulsa <strong>Comenzar</strong>.',
        'El audio se descarga en tu <em>Biblioteca</em> con un reproductor integrado para preescucha inmediata.',
        'Pulsa <strong>«Enviar a Unreal»</strong> para importarlo a <span class="mono">/Game/DBV/Audio/</span> como asset <span class="mono">USoundWave</span> (o <span class="mono">SoundCue</span> si tiene bucle).',
        'Los agentes de IA también pueden usar la herramienta <span class="mono">unreal_import_sound</span> para asociar sonidos a componentes de actores o eventos del nivel.',
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
        '<strong>An agent sees the editor twice:</strong> the Convai "Set Up AI Coding" assistant (or Unreal) can write a <span class="mono">.mcp.json</span> that connects agents directly to the editor. The <em>Project</em> view warns about it: delete the entry ending in <span class="mono">/mcp</span> in that file. The instructions Convai leaves in <span class="mono">CLAUDE.md</span> do no harm and you can keep them.',
        '<strong>Re-assembling the MetaHuman</strong> regenerates its Blueprint and deletes the Convai setup: repeat step 2.',
        'Convai actions <strong>save Blueprints</strong>: "Undo this turn" may not revert them. Keep the project under version control.',
      ],
    },
    tripo: {
      title: '3D Models with Tripo',
      intro: 'Generate optimized polygonal meshes (PBR, low-poly or quad topologies) from text prompts or reference images using Tripo Studio v3.',
      needTitle: 'What you need',
      needs: [
        'An account at <strong>tripo3d.ai</strong> with available credits.',
        'Your <strong>API Key</strong> configured in <em>Settings → Tripo</em>. The app tracks balance and honors your budget cap.',
      ],
      stepsTitle: 'Workflow',
      steps: [
        'In the <em>Generate</em> view ("Text" or "Image" tab), describe the model or attach an image and click <strong>Start</strong>.',
        'The cost card shows you the estimated credits before launching the job.',
        'The completed model is saved to your <em>Library</em> as a GLB file.',
        'From the library you can apply <strong>Auto Rigging</strong>, link compatible animations, or click <strong>«Send to Unreal»</strong> to import as StaticMesh or SkeletalMesh into <span class="mono">/Game/DBV/Models/</span>.',
      ],
    },
    worldlabs: {
      title: 'Worlds and Environments with World Labs',
      intro: 'Generate complete photorealistic 3D environments using World Labs Marble technology from a prompt or panoramic image.',
      needTitle: 'What you need',
      needs: [
        'An account at <strong>worldlabs.ai</strong> with Marble API access.',
        'Your <strong>API Key</strong> configured in <em>Settings → World Labs</em>.',
        'The Unreal project prepared with the app: includes the native GPU C++ component <span class="mono">UDbvGaussianSplatComponent</span>.',
      ],
      stepsTitle: 'Workflow',
      steps: [
        'In the <em>Generate</em> view ("World" tab), write the environment description and click <strong>Start</strong>.',
        'Generation usually takes around 5 minutes. If you close the app, the job automatically resumes when reopening without losing the generated world.',
        'The result contains a Gaussian Splatting file (<span class="mono">.spz</span>) for photorealism and a simplified collision mesh (<span class="mono">.glb</span>) for physics.',
        'Click <strong>«Send to Unreal»</strong> in the library to place the splat actor and physical collider perfectly synchronized into the level.',
      ],
    },
    audio: {
      title: 'Sound Effects (SFX) and Audio',
      intro: 'Generate sound effects or search real audio recordings to bring your game to life, ready to import as USoundWave assets in Unreal Engine.',
      needTitle: 'Three providers and how to configure them',
      needs: [
        '<strong>Local (dbv-audio-service):</strong> 100% free, private with zero cloud cost. Runs Stable Audio Open 1.0 locally at <span class="mono">http://127.0.0.1:8765</span> using your local GPU.',
        '<strong>ElevenLabs SFX:</strong> Ultra high-quality realistic AI sound effects (free tier includes 10,000 monthly characters). When creating your key at <a href="https://elevenlabs.io/app/developers/api-keys" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">ElevenLabs API Keys</a>, grant full access (<em>Full Access</em> or at least <em>user_read</em> and <em>sound_generation</em>), copy the secret key starting with <span class="mono">sk_</span> and paste it into <em>Settings → Audio</em>.',
        '<strong>Freesound:</strong> Search and download real-world sounds (CC0 and CC-BY licenses). Create a free account at Freesound, request credentials at <a href="https://freesound.org/apiv2/apply" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">Freesound - API credentials</a> (you get a <em>Client secret / Api key</em>), and paste it as token into <em>Settings → Audio</em>.',
      ],
      providersTitle: 'Duration and loops',
      providers: [
        'Both local service and ElevenLabs let you choose the exact duration (from 0.5 to 30 seconds).',
        'The <strong>Seamless loop</strong> option generates the sound so start and end blend seamlessly (ideal for engines, wind or rain).',
      ],
      stepsTitle: 'Workflow',
      steps: [
        'In the <em>Generate</em> view, navigate to the <strong>Sounds</strong> tab (🎵).',
        'Select the provider, type what you want to hear (e.g. <em>"footsteps on wet gravel"</em>) and click <strong>Start</strong>.',
        'The audio is downloaded into your <em>Library</em> with a built-in player for instant preview.',
        'Click <strong>«Send to Unreal»</strong> to import into <span class="mono">/Game/DBV/Audio/</span> as a <span class="mono">USoundWave</span> asset (or <span class="mono">SoundCue</span> if looped).',
        'AI agents can also invoke the <span class="mono">unreal_import_sound</span> tool to attach sounds to actor components or level events.',
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
        '<strong>智能体看到两次编辑器：</strong>Convai 的「Set Up AI Coding」助手（或 Unreal）可能写入一个 <span class="mono">.mcp.json</span>，让智能体直接连接编辑器。<em>项目</em>视图会提示：请在该文件中删除以 <span class="mono">/mcp</span> 结尾的条目。Convai 留在 <span class="mono">CLAUDE.md</span> 中的说明不会造成影响，可以保留。',
        '<strong>重新组装 MetaHuman</strong> 会重新生成其蓝图并删除 Convai 设置：请重复第 2 步。',
        'Convai 的操作会<strong>保存蓝图</strong>：«撤销本轮» 可能无法还原。请把项目放在版本控制中。',
      ],
    },
    tripo: {
      title: '使用 Tripo 的 3D 模型',
      intro: '使用 Tripo Studio v3 API 根据文本提示或参考图像生成优化的多边形网格（PBR、低多边形或四边形拓扑）。',
      needTitle: '你需要',
      needs: [
        '在 <strong>tripo3d.ai</strong> 注册并拥有可用积分的账户。',
        '在<em>设置 → Tripo</em> 中配置你的 <strong>API 密钥</strong>。应用会检查余额并遵守你的预算上限。',
      ],
      stepsTitle: '工作流程',
      steps: [
        '在<em>生成</em>视图（「文本」或「图像」标签页）中，描述模型或附加图像，然后点击<strong>开始</strong>。',
        '费用卡片会在启动任务前显示预估积分。',
        '完成的模型会以 GLB 文件形式保存在<em>库</em>中。',
        '在库中可以应用<strong>自动骨骼绑定</strong>、关联兼容动画，或点击<strong>«发送至 Unreal»</strong>将其作为 StaticMesh 或 SkeletalMesh 导入到 <span class="mono">/Game/DBV/Models/</span>。',
      ],
    },
    worldlabs: {
      title: '使用 World Labs 的世界与环境',
      intro: '使用 World Labs Marble 技术，根据文本或全景图像生成完整逼真的 3D 场景。',
      needTitle: '你需要',
      needs: [
        '在 <strong>worldlabs.ai</strong> 注册并拥有 Marble API 访问权限的账户。',
        '在<em>设置 → World Labs</em> 中配置你的 <strong>API 密钥</strong>。',
        '使用本应用准备好的 Unreal 项目：包含原生 GPU C++ 组件 <span class="mono">UDbvGaussianSplatComponent</span>。',
      ],
      stepsTitle: '工作流程',
      steps: [
        '在<em>生成</em>视图（「世界」标签页）中，输入场景描述并点击<strong>开始</strong>。',
        '生成通常需要约 5 分钟。如果关闭应用，重新打开时会自动继续轮询，不会丢失生成的世界。',
        '结果包含用于照片级写实的 Gaussian Splatting 文件（<span class="mono">.spz</span>）以及用于物理的简化碰撞网格（<span class="mono">.glb</span>）。',
        '在库中点击<strong>«发送至 Unreal»</strong>，即可将 Splat Actor 和碰撞体完全同步放置在关卡中。',
      ],
    },
    audio: {
      title: '音频与音效 (SFX)',
      intro: '生成音效或搜索真实录音以营造游戏氛围，可直接作为 USoundWave 资源导入 Unreal Engine。',
      needTitle: '三种提供商及配置方法',
      needs: [
        '<strong>本地 (dbv-audio-service)：</strong>100% 免费、私密且无需云端费用。在 <span class="mono">http://127.0.0.1:8765</span> 本地运行 Stable Audio Open 1.0，利用本地 GPU。',
        '<strong>ElevenLabs SFX：</strong>逼真的高质量 AI 音效（免费计划每月提供 10,000 字符额度）。在 <a href="https://elevenlabs.io/app/developers/api-keys" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">ElevenLabs API Keys</a> 创建密钥时，建议授予完全权限（<em>Full Access</em> 或至少勾选 <em>user_read</em> 和 <em>sound_generation</em>），复制以 <span class="mono">sk_</span> 开头的密钥并粘贴至<em>设置 → 音频</em>。',
        '<strong>Freesound：</strong>搜索并下载真实录音（CC0 和 CC-BY 许可）。在 Freesound 注册免费账户，于 <a href="https://freesound.org/apiv2/apply" target="_blank" rel="noopener noreferrer" style="color: var(--accent);">Freesound - API credentials</a> 申请凭证（获取 <em>Client secret / Api key</em>），并作为令牌粘贴至<em>设置 → 音频</em>。',
      ],
      providersTitle: '时长与循环',
      providers: [
        '本地服务和 ElevenLabs 均可选择精确时长（0.5 至 30 秒）。',
        '<strong>无缝循环</strong>选项能让音频首尾完美衔接（非常适合引擎声、风声或雨声）。',
      ],
      stepsTitle: '工作流程',
      steps: [
        '在<em>生成</em>视图中，进入<strong>声音</strong>标签页 (🎵)。',
        '选择提供商，输入你想听到的声音描述（例如 <em>«潮湿砾石上的脚步声»</em>），然后点击<strong>开始</strong>。',
        '音频下载至你的<em>库</em>中，配有内置播放器供即时试听。',
        '点击<strong>«发送至 Unreal»</strong>，将其作为 <span class="mono">USoundWave</span> 资源（如果是循环则为 <span class="mono">SoundCue</span>）导入到 <span class="mono">/Game/DBV/Audio/</span>。',
        'AI 智能体也可以调用 <span class="mono">unreal_import_sound</span> 工具将声音关联到 Actor 组件或关卡事件。',
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
    tripo: () => tripo(s.tripo),
    worldlabs: () => worldlabs(s.worldlabs),
    audio: () => audio(s.audio),
    convai: () => convai(s.convai),
    shortcuts: () => shortcuts(s.shortcuts),
    links: () => links(s.links),
  };
  return (pages[tab] ?? pages.ue)();
}

/** Pestañas que existen. */
export const HELP_TABS = ['ue', 'ai', 'tripo', 'worldlabs', 'audio', 'convai', 'shortcuts', 'links'];
