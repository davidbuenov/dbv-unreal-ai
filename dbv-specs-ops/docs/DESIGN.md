# 🎨 Sistema de Diseño: DBV Unreal AI

> **Fase:** `/spec` (Especificación Visual)
> **Estado:** Borrador v0.2.1
> **Última Revisión:** 2026-10-28 (componentes v0.2 y división de IAs v0.2.1)
> **Aplica a:** la aplicación de escritorio (Tauri v2, Vanilla JS/CSS sin bundler).

---

> 📐 Inspirado en el estándar **[design.md](https://github.com/google-labs-code/design.md)** de Google Labs.

---

```yaml
version: alpha
name: "DBV Unreal AI"
description: "Herramienta profesional de escritorio, densa y oscura, que se siente vecina del editor de Unreal: grises neutros, un único acento azul de acción y colores semánticos reservados para el estado de la conexión, el destino de la IA y el coste."

# Tema por defecto: oscuro (como el editor de Unreal). El claro es un tema completo, no un modo secundario.
colors:            # Tema claro
  primary:      "#0969da"
  secondary:    "#57606a"
  accent:       "#0969da"
  neutral:      "#ffffff"
  surface:      "#f6f8fa"
  on-primary:   "#ffffff"
  on-surface:   "#1f2328"
  on-neutral:   "#57606a"
  error:        "#cf222e"
  success:      "#1a7f37"
  warning:      "#9a6700"

dark:              # Tema oscuro (por defecto)
  primary:      "#4c9aff"
  secondary:    "#8b949e"
  accent:       "#4c9aff"
  neutral:      "#121314"   # lienzo, cercano al gris del editor de Unreal
  surface:      "#1b1d1f"   # paneles y tarjetas
  on-primary:   "#0b0c0d"
  on-surface:   "#e6e8eb"
  on-neutral:   "#8b949e"
  error:        "#f85149"
  success:      "#3fb950"
  warning:      "#d29922"

# Tokens semánticos propios del producto (cada tema decide su valor)
semantic:
  ai-local:       { dark: "#3fb950", light: "#1a7f37" }   # verde: nada sale del equipo
  ai-cloud:       { dark: "#d29922", light: "#9a6700" }   # ámbar: sale a un proveedor
  ai-agent:       { dark: "#a371f7", light: "#8250df" }   # violeta: agente por ACP
  cost:           { dark: "#e3b341", light: "#7d4e00" }   # créditos / gasto
  unreal-online:  { dark: "#3fb950", light: "#1a7f37" }
  unreal-needs-prep: { dark: "#d29922", light: "#9a6700" }
  unreal-offline: { dark: "#6e7681", light: "#8c959f" }
  action-modify:  { dark: "#f0883e", light: "#bc4c00" }   # borde de tarjetas de aprobación

typography:
  heading:    { fontFamily: "system-ui stack", fontSize: 20px, fontWeight: 600, lineHeight: 1.25 }
  subheading: { fontFamily: "system-ui stack", fontSize: 15px, fontWeight: 600, lineHeight: 1.3 }
  body:       { fontFamily: "system-ui stack", fontSize: 13px, fontWeight: 400, lineHeight: 1.55 }
  label:      { fontFamily: "system-ui stack", fontSize: 12px, fontWeight: 500, letterSpacing: "0.01em" }
  caption:    { fontFamily: "system-ui stack", fontSize: 11px, fontWeight: 400 }
  mono:       { fontFamily: "mono stack",      fontSize: 12px, fontWeight: 400 }

rounded:
  none: 0px
  sm:   4px
  md:   6px
  lg:   8px
  full: 9999px

spacing:
  xs:  4px
  sm:  8px
  md:  12px
  lg:  16px
  xl:  24px
  xxl: 48px

components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor:       "{colors.on-primary}"
    typography:      "{typography.label}"
    rounded:         "{rounded.md}"
    padding:         "6px 14px"
  button-secondary:
    backgroundColor: "transparent"
    textColor:       "{colors.on-surface}"
    rounded:         "{rounded.md}"
    padding:         "6px 14px"
    border:          "1px solid {colors.secondary}"
  card:
    backgroundColor: "{colors.surface}"
    rounded:         "{rounded.lg}"
    padding:         "{spacing.lg}"
  approval-card:
    backgroundColor: "{colors.surface}"
    rounded:         "{rounded.lg}"
    padding:         "{spacing.md}"
    borderLeft:      "3px solid {semantic.action-modify}"
  status-pill:
    rounded:         "{rounded.full}"
    padding:         "2px 10px"
    typography:      "{typography.caption}"
  input:
    backgroundColor: "{colors.neutral}"
    textColor:       "{colors.on-surface}"
    rounded:         "{rounded.md}"
    padding:         "8px 10px"
    border:          "1px solid {colors.secondary}"
  input-focus:
    border:          "2px solid {colors.accent}"
```

---

## Visión General

La aplicación se usa **al lado del editor de Unreal**, a menudo en un segundo monitor o en media pantalla, durante horas. Debe sentirse como una herramienta más del editor, no como una web: oscura por defecto, densa, tranquila y rápida. El color no decora: dice **dónde está conectada** (Unreal), **a dónde van los datos** (local / nube / agente), **qué va a cambiar** (acciones de modificación) y **cuánto cuesta** (créditos).

---

## 🎨 Colores

- **Accent (`#4c9aff` oscuro / `#0969da` claro):** la única señal de acción: botón primario, foco, selección, enlaces. Un solo botón primario por vista.
- **Neutral / Surface:** grises casi neutros (ligera dominante fría, como el editor de Unreal) para lienzo y paneles. Nunca negro puro.
- **Semánticos de producto** (`semantic.*`): se usan **solo** para su significado.
  - `ai-local` / `ai-cloud` / `ai-agent` en la etiqueta de destino de cada petición (`RF-IA-03.2`).
  - `unreal-*` en la píldora de conexión con el editor.
  - `action-modify` en el borde de las tarjetas de aprobación (REQ-12).
  - `cost` en todo importe de créditos o tokens.
  - Patrón heredado de dbv-typst-editor: si dos cosas deben distinguirse, el token es semántico y cada tema decide su valor.
- **Error / Success / Warning:** feedback del sistema, nunca decoración.

### Modo Oscuro

- **Estrategia:** el oscuro es el tema por defecto (`:root`). El claro redefine todos los tokens en `[data-theme="light"]`. Todo componente se verifica en ambos. Se respeta `prefers-color-scheme` en el primer arranque.

---

## ✍️ Tipografía

- **Pilas del sistema, sin fuentes de red** (la app funciona sin conexión con modelos locales):
  - `--font-ui: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;`
  - `--font-mono: 'JetBrains Mono', 'Cascadia Code', Consolas, 'SF Mono', Menlo, monospace;`
- **Escala de escritorio, no de web:** 20 px (título de vista, una vez por pantalla), 15 px (títulos de panel y modal), 13 px (cuerpo y chat), 12 px (controles, etiquetas), 11 px (metadatos).
- **Monoespaciada = literal y copiable:** código Python, rutas `/Game/...`, nombres de herramientas, `task_id`, puertos y versiones van siempre en `--font-mono`.

---

## 🧩 Componentes Clave

### Estructura de la ventana

- **Barra lateral izquierda (iconos + etiqueta):** Proyecto · Conversación · **Plan** (v0.2) · Generar · Biblioteca · Ajustes.
- **Barra superior:** píldora de conexión con Unreal (proyecto, versión, puerto, estado), selector de IA con su etiqueta de destino, y gasto acumulado de la sesión (`cost`).
- **Centro:** la vista activa (conversación por defecto).
- **Panel derecho plegable:** contexto activo (fichas removibles) y registro de acciones de la sesión (REQ-14).

### Tarjeta de aprobación (REQ-12)

- Borde izquierdo `action-modify`. Muestra: herramienta, proyecto destino, resumen en una línea y el detalle (código Python con resaltado, o diff, o asset y ruta destino).
- Acciones: **Permitir** (primario), **Permitir siempre en esta conversación** (secundario), **Denegar** (texto). El código completo se ve sin pulsar nada más si ocupa ≤ 20 líneas; si es más largo, plegado con «ver todo».

### Tarjeta de coste (REQ-15, REQ-18, RNF-04)

- Importe en `cost`, proveedor, parámetros clave y saldo si la API lo da. Confirmar es el botón primario; nunca se confirma con Intro por accidente (el foco inicial va a «Cancelar»).

### Lista de preparación (REQ-04)

- Una fila por comprobación con ✓/✗, explicación en una línea y «¿Por qué?» desplegable. El botón «Preparar» muestra antes el diff de los ficheros.

### Tarjeta de asset (REQ-17)

- Miniatura cuadrada, nombre, proveedor, fecha, coste. Acciones al pasar el cursor: Enviar a Unreal (primario), abrir carpeta, regenerar, borrar.

### Mensajes de la conversación

- Usuario alineado a la derecha sobre `surface`; IA a la izquierda sin fondo. Las llamadas a herramientas se muestran como filas compactas plegables (icono, nombre en mono, duración, resultado).

### Decisiones de diseño v0.2 (2026-10-26)

- **Vista Plan (REQ-36):** dos pestañas, **Tareas** y **Memoria**. Tareas: el objetivo arriba (subheading), lista de casillas con el título en body, el criterio debajo en caption y en `--font-mono` si es una comprobación (REQ-37), y el coste estimado en `cost` alineado a la derecha. Abajo, «Siguiente paso» y el total estimado. Acciones por fila al pasar el cursor: «Hacer esta tarea» (primario de la fila), editar, borrar. Arriba, un solo botón primario: «Planificar». Estado de una comprobación: ✓ `success`, ✗ `error` con el motivo, «pendiente de comprobar» en `unreal-offline`. Memoria: `memory.md` en Markdown seguro, de solo lectura, con «Abrir carpeta» y «Editar en mi editor». Sin memoria activada, la vista explica qué es en tres líneas y ofrece «Activar memoria del proyecto».
- **Tarjeta «¿Guardar en la memoria del proyecto?» (REQ-35):** misma forma que la tarjeta de aprobación, con borde izquierdo `accent` (no `action-modify`: no cambia el editor). Muestra la sección de destino y la entrada en un campo editable; acciones Guardar (primario), Editar, Descartar (texto). El foco inicial va a Descartar, como en la tarjeta de coste.
- **Propuesta de plan (REQ-36):** la lista propuesta se ve como la vista Tareas, pero cada fila es editable y se puede quitar; el pie muestra el total estimado en `cost` y «Aceptar plan» (primario).
- **Selector de proyecto (REQ-07):** la píldora de conexión se convierte en un menú cuando hay más de un editor conectado (proyecto, puerto y estado de cada uno). El proyecto activo se repite en la cabecera de la conversación para que nunca haya duda de a cuál se dirige.
- **Captura del viewport (REQ-13):** miniatura de 160 px de ancho dentro de la fila de la herramienta; al pulsarla se abre a tamaño completo.
- **Deshacer este turno (REQ-40):** botón secundario pequeño al pie de la respuesta de la IA, solo si el turno modificó el editor. Si no puede deshacer con seguridad, muestra el aviso en `warning` en esa misma línea, sin modal.
- **Actualización disponible (REQ-39):** franja discreta bajo la barra superior (no un modal) con la versión, «Ver novedades», «Actualizar» (primario) y «Más tarde». Las notas se muestran en Markdown seguro.
- **MCPs de terceros (REQ-21):** sección en Ajustes con una fila por servidor (nombre, tipo, estado, número de herramientas) y un interruptor por proyecto. El comando de un servidor stdio se ve completo en `--font-mono` en el diálogo de confirmación.

### Decisiones de diseño v0.2.1 (2026-10-28)

- **Modal «Conectar una IA» estructurado (REQ-43):**
  - Se sustituye la lista plana por **tres secciones o tarjetas visuales claramente jerarquizadas**:
    1. **Suscripción (Agentes de terminal / ACP):** borde o acento en `ai-agent` (`#a371f7`). Incluye Claude Code, Antigravity CLI, GitHub Copilot CLI, OpenAI Codex y Gemini CLI. Cada elemento muestra estado («instalado» / «no instalado») y acciones («Conectar» / «Cómo instalarlo»).
    2. **Claves de API en la nube (Pago por uso):** borde o acento en `ai-cloud` (`#d29922`). Acceso directo al formulario de configuración de claves en el llavero (OpenAI, Anthropic, Gemini API, OpenRouter).
    3. **IAs Locales (Offline y Privacidad):** borde o acento en `ai-local` (`#3fb950`). Ollama y LM Studio con detección de puerto en ejecución («en marcha» / «no está en marcha»).
- **Píldoras y badges de categoría (REQ-43):**
  - En la sección **«Tus conexiones»**, cada fila muestra una píldora semántica con color de fondo atenuado y texto de alto contraste:
    - `[Suscripción]` con color `ai-agent`.
    - `[API Cloud]` con color `ai-cloud`.
    - `[Local]` con color `ai-local`.
- **Integración visual de Google Antigravity CLI (REQ-44):**
  - Se incluye en el bloque de Suscripción con su icono distintivo y enlace directo a la web oficial `https://antigravity.google/product/antigravity-cli`.
  - El texto informativo clarifica: *«Para usar tu suscripción de Google (Google One / Gemini Advanced), usa Google Antigravity CLI. Gemini CLI se reserva para claves de API, Vertex AI o Gemini Code Assist.»*
- **Modal «Acerca de» enriquecido (REQ-45):**
  - Card central con logo circular de 64x64 px, título en `heading`, versión en `caption`, bloque de enlaces sociales/web con iconos SVG de 16x16 px, y tarjeta interna de datos del sistema (`surface` elevado) con formato clave-valor en `--font-mono`.
- **Centro de Ayuda integrado (REQ-46):**
  - Modal con botón `?` en el header y atajo `F1`.
  - Estructura con pestañas superiores compactas o índice lateral para navegar entre: Unreal Engine 5.8, Conectar IAs, Atajos de teclado y Enlaces a la documentación. Textos legibles en `body` y bloques de código de comandos en `mono`.
- **Configuración de Modelos y Razonamiento en Agentes CLI (REQ-47):**
  - Botón de ajustes ⚙️ (`ai-agent-settings-btn`) situado en cada agente conectado o activo en el modal de conexión.
  - Al pulsarlo, despliega un panel integrado (`ai-agent-settings-panel`) con:
    - Selector desplegable de modelo poblado dinámicamente mediante consulta al CLI (con Gemini 3.8 Flash como base para Antigravity).
    - Selector de nivel de razonamiento / pensamiento (*reasoning effort*): Desactivado, Bajo, Medio, Alto.
    - Botón de confirmación y guardado reactivo sin cerrar el modal.

### Decisiones de diseño v0.3.0 (2026-10-31)

- **Tarjeta de memoria con varias entradas (REQ-50):** la misma tarjeta de REQ-35, con borde `accent`, pero con una fila por entrada: casilla (marcada por defecto), sección en `caption`, texto en un campo editable de una línea y motivo debajo en `caption` `text-muted`. Una entrada repetida sale desmarcada con «ya está» en `text-muted`. Una con forma de secreto, deshabilitada y con su motivo en `error`. Pie con «Guardar las marcadas (n)» (primario, deshabilitado con 0) y «Descartar todo» (texto). El foco inicial va a «Descartar todo».
- **Contador de herramientas del turno (REQ-51):** una línea en `caption` `text-muted` al pie de la respuesta, junto a «Deshacer este turno»: «Herramientas: 7 llamadas · 48 KB (~12 k tokens, aprox.)». Pasa a `warning` por encima de 200 KB en un turno y su `title` explica cómo reducirlo. Sin modal ni color nuevo.
- **Aviso de acceso duplicado (REQ-52):** franja `warning` en la vista Proyecto, encima del diagnóstico, con el fichero encontrado en `--font-mono`, una frase con la consecuencia y «Cómo evitarlo» (abre la ayuda). En la conversación, la misma frase una sola vez al abrir la sesión del agente.
- **Sección Convai en la vista Proyecto (REQ-55):** bloque con el mismo formato que la lista de preparación (REQ-04): ✓/✗ por comprobación (plugin, UE ≥ 5.8, toolsets, guías, sesión iniciada, clave versionada en git) con su explicación. Si no está instalado, se muestra plegado como «opcional» con un enlace a la documentación. La clave nunca se muestra, ni siquiera enmascarada: solo «sesión iniciada en el plugin».
- **Propuesta de zonas (REQ-57):** igual que la propuesta de plan (REQ-36): una fila editable por zona (nombre, descripción, posición en metros en `--font-mono`) que se puede quitar, y «Crear zonas (n)» como primario. Las zonas que ya existían llevan la etiqueta «actualizar».
- **Ruta de la biblioteca (REQ-53):** en Ajustes, la ruta real en `--font-mono` con «Abrir carpeta». Si se migró desde la ubicación antigua, una nota en `caption`: «Se copiaron n assets de la ubicación anterior, que se conserva».

### Decisiones de diseño de la ampliación v0.3.0 (2026-10-31)

- **Splats en la conversación (REQ-63):** al enviar un splat, la tarjeta de resultado añade una línea en `caption`: «9.975.598 splats · SH grado 3». Si no se dibujan todos o se reducen los armónicos, la línea pasa a `warning` con la nota del informe. No hay controles de render en la app: se ajustan en el actor dentro de Unreal.
- **Diff de código (REQ-22):** una tarjeta por propuesta con una sección plegable por fichero. Cabecera con la ruta en `--font-mono` y `+n −m`; cuerpo con el diff unificado en `--font-mono`, líneas añadidas sobre `success` suave y quitadas sobre `error` suave. Acciones por fichero: «Aplicar» y «Descartar»; en el pie, «Aplicar todo (n)» (primario). Después, una línea de estado de la compilación: «Compilando con Live Coding…», «Compilado ✓» o los errores en `--font-mono` (plegados si son más de 10 líneas), más «Revertir» (texto).
- **Menú Recetas (REQ-23):** un botón «Recetas» junto a las sugerencias de la conversación abre una lista corta (nombre, una línea de qué hace y qué necesita). Elegir una escribe su prompt en la caja, sin enviarlo, para que la persona lo revise. Las que necesitan algo que falta (Convai, Tripo) salen deshabilitadas con el motivo.
- **Guías sin red (REQ-25):** en Ajustes, una pestaña nueva **Conocimiento** (después de Biblioteca; REQ-60 pasa a ocho pestañas), con el estado del índice («sin construir» o «n páginas, fecha»), «Construir índice» con su explicación de qué se descarga y de dónde, y «Borrar índice».
- **Chino (REQ-64):** el selector de idioma ofrece «中文»; con una fuente del sistema para CJK en la pila de `--font-sans` (`"Microsoft YaHei", "PingFang SC", "Noto Sans CJK SC"`), sin incrustar fuentes.

### Decisiones de diseño v0.4.1 (2026-11-05)

- **Panel interactivo de Parámetros de Síntesis de Audio (REQ-76):**
  - En la pestaña «Sonidos (SFX)» de la vista Generar, al seleccionar el proveedor Local (`dbv-audio-service`):
    - **Fila de chips rápidos de juego (Quick Presets):** píldoras interactivas compactas arriba del prompt (`🔫 Laser`, `🚪 Puerta`, `⚡ Trueno`, `🕹️ Retro`, `⚔️ Espada`) que rellenan de un clic tanto el prompt descriptivo como los valores óptimos de duración, steps y escala CFG.
    - **Campo de Prompt Negativo con filtros de exclusión:** caja de texto con placeholder guiado y botones chip rápidos (`🚫 Sin música`, `🔇 Sin ruido`, `🏜️ Seco / Para Unreal`, `✖ Limpiar`) para configurar acústicas puras sin necesidad de escribir términos técnicos en inglés a mano.
    - **Controles de Síntesis en paralelo:**
      - **Duración:** barra deslizante (slider) + campo numérico sincronizados con pistas contextuales según el rango temporal.
      - **Pasos (Steps):** slider + campo numérico (10–100) con guía de calidad.
      - **Escala CFG:** slider + campo numérico (1.0–15.0, paso 0.5) con guía de adherencia al prompt.
      - **Semilla (Seed):** campo numérico con botón secundario `🎲 Aleatorio`.
    - **Adaptación contextual:** al cambiar al proveedor ElevenLabs o Freesound, todos los controles específicos de difusión local se ocultan automáticamente, restaurando la interfaz mínima y adecuada a cada proveedor.

---

## ✨ Movimiento e Interacción

- `--transition: 120ms ease` para hover, foco y despliegues; nada de animaciones de entrada.
- El streaming no anima: el texto aparece tal cual llega.
- Indicador de «el editor está ocupado» (RNF-03) con un pulso discreto en la píldora de conexión, sin bloquear la interfaz.
- **Reducción de movimiento:** con `prefers-reduced-motion` se eliminan pulsos y transiciones.
- **Accesibilidad (RNF-07):** todo control alcanzable por teclado, foco visible de 2 px con `accent`, objetivos ≥ 28 px en escritorio, contraste AA comprobado en ambos temas.

---

**Instrucción para la IA:** Lee y respeta los tokens y decisiones definidos en este fichero. Si necesitas crear un componente no definido aquí, extrapola coherentemente desde los tokens existentes y registra la nueva decisión como "Decisión de Diseño" en este mismo archivo con fecha y justificación.
