# 🤖 Usar una IA con DBV Unreal AI

> 🌐 **Español** · [English](./IA.en.md) · [← Volver al README](../README.md)

El asistente de IA de **DBV Unreal AI** es **opcional**. Sin configurar nada, la aplicación funciona exactamente igual, de forma local. Esta guía explica **qué necesitas tener en tu ordenador** según la IA que quieras usar.

## Lo esencial en una tabla

| Quiero usar… | Qué necesito instalar o tener | ¿Sale algo de mi equipo? | ¿Cuesta dinero? |
| --- | --- | --- | --- |
| **Una IA local** (Ollama, LM Studio, llama.cpp…) | El programa servidor y al menos un modelo descargado | No | No (gasta tu CPU/GPU y disco) |
| **Claude, ChatGPT, Gemini u OpenRouter con clave de API** | Una clave de API de ese proveedor | Sí, al proveedor | Sí, por uso, según la tarifa del proveedor |
| **Mi suscripción** (Claude Pro/Max, ChatGPT Plus, Gemini / Google One, Copilot) | El **agente** de ese servicio instalado y con la sesión iniciada: Claude Code, Codex, Gemini CLI, Google Antigravity o GitHub Copilot CLI | Sí, al servicio del agente | Lo cubre tu suscripción |

> ⚠️ **Una suscripción no es una clave de API.** Pagar Claude Pro, ChatGPT Plus o Gemini no te da una clave de API (se contratan por separado). Con una suscripción, la vía es el **agente instalado** (tercera fila).

Todo se configura en la pantalla **Conectar una IA**, que se abre con el botón de IA de la aplicación. Al abrirla, DBV Unreal AI **detecta lo que ya tienes** (Ollama, LM Studio y los agentes) y lo marca con ✓; lo que no encuentra aparece con un botón **Cómo instalarlo**. Esta detección solo ocurre al abrir esa pantalla, nunca al arrancar la aplicación.

La pantalla tiene tres bloques, uno por cada fila de la tabla: **IAs Locales**, **IAs por Suscripción (Agentes CLI)** e **IAs en la nube con clave de API**.

---

## 1. Una IA local (nada sale de tu equipo)

Es la opción más privada: tus documentos no abandonan tu ordenador y no hay coste por uso. A cambio, la calidad depende del tamaño del modelo y de tu hardware.

### Ollama

1. Instala [Ollama](https://ollama.com/download).
2. Descarga un modelo, por ejemplo: `ollama pull qwen2.5:3b` (o `llama3`).
3. Comprueba qué tienes instalado con `ollama list`.
4. Abre **Conectar una IA**. Si Ollama está en marcha, aparece con ✓ y un botón **Usar**. Elige el modelo de la lista, pulsa **Probar conexión** y **Guardar**.

Ollama escucha en `http://localhost:11434`. La aplicación habla con él por su **API nativa** (`/api/chat`) y le indica ella misma el tamaño de contexto, porque Ollama usa por defecto uno pequeño (2 048–4 096 tokens) y recorta en silencio lo que no cabe.

### LM Studio

1. Instala [LM Studio](https://lmstudio.ai), descarga un modelo y **arranca su servidor local** (pestaña *Developer / Local Server*).
2. Escucha en `http://localhost:1234`; la aplicación lo detecta igual que a Ollama.

### Tu propio servidor compatible con OpenAI (llama.cpp, vLLM, Jan…)

Cualquier servidor que ofrezca la API de OpenAI sirve. El caso típico es `llama-server` de [llama.cpp](https://github.com/ggml-org/llama.cpp) con un modelo `.gguf` que tú mismo has descargado. **Este servidor no se detecta solo: hay que añadirlo a mano.**

1. **Conectar una IA → Añadir una IA en la nube (clave de API)…** *(el botón se llama así, pero abre un formulario con todos los proveedores)*.
2. En **Proveedor**, elige **Servidor compatible con OpenAI (local o propio)**.
   - ⚠️ **No elijas «Ollama»** para un servidor que no es Ollama: ese modo usa un protocolo distinto y la aplicación no entenderá la respuesta. Y al **editar** una conexión el proveedor no se puede cambiar: hay que crear una nueva.
3. Rellena el formulario:

   | Campo | Valor |
   | --- | --- |
   | Nombre | el que quieras |
   | Dirección | `http://127.0.0.1:8080/v1` (puerto de tu servidor, **terminada en `/v1`**) |
   | Clave | vacía, salvo que arrancaras el servidor con `--api-key` |
   | Modelo | el `id` que devuelve `curl http://127.0.0.1:8080/v1/models` |
   | Contexto (tokens) | el `n_ctx` del servidor dividido entre sus slots (ver abajo) |
   | Herramientas | *Detectar* |

4. **Probar conexión** y **Guardar**. Marca la conexión como activa y elígela en el selector del panel.

Los datos del formulario (modelo, contexto, herramientas) se consultan con unos comandos: ver [Averiguar los datos del formulario](#averiguar-los-datos-del-formulario).

**Tres trampas habituales con llama.cpp:**

- **El contexto no es `n_ctx_train`.** `/v1/models` muestra `n_ctx_train`, el máximo con el que se entrenó el modelo, no el que tiene tu servidor. El valor real es el flag `-c` con el que arrancaste.
- **Los *slots* reparten el contexto.** Con varios slots (`-np`), el contexto se divide entre ellos; con `-np 4` y `-c 32768`, cada petición dispone de 8 192. Para dárselo entero a una conversación: `-np 1`.
- **Los modelos que «piensan» pueden parecer mudos.** Algunos modelos escriben primero su razonamiento en un campo aparte (`reasoning_content`) y dejan la respuesta (`content`) vacía hasta terminar. La aplicación muestra `content`, así que ves una respuesta vacía mientras el modelo consume cientos o miles de tokens. Arranca el servidor con `--reasoning-budget 0` para desactivar el razonamiento.

Ejemplo de arranque razonable:

```bash
llama-server -m modelo.gguf -c 32768 -np 1 --reasoning-budget 0 --port 8080
```

### Qué esperar de un modelo local

- **Herramientas.** Con un modelo que las admite, la IA consulta por su cuenta los documentos de la carpeta y propone cambios que tú revisas antes de aplicarlos. Sin herramientas, el asistente sigue sirviendo para conversar y para trabajar sobre el contexto que se le envía, pero hace menos por sí mismo. El campo *Herramientas* del formulario las detecta; si falla la detección, puedes forzarlas a *Sí* o *No*.
- **Tamaño.** Los modelos de 2–3 mil millones de parámetros (que ocupan unos 2–3 GB) responden rápido en un portátil, pero se quedan cortos en propuestas largas o complejas. Si las respuestas salen flojas, prueba antes con un modelo mayor que tocar la configuración.
- **Contexto.** Un contexto de 4 096 tokens es el mínimo y se queda corto con documentos largos o imágenes. 8 192 es cómodo para trabajar; 32 768, holgado.
- **Memoria.** Un modelo más grande o un contexto mayor necesitan más RAM o VRAM. Empieza pequeño y sube.

---

## Averiguar los datos del formulario

Para conectar una IA local el formulario pide tres cosas: **modelo**, **contexto** y si admite **herramientas**. Se consultan en tu propio equipo con los comandos de abajo, y el resultado es lo que hay que escribir en cada campo.

> En **PowerShell** usa `curl.exe` (con la extensión): `curl` a secas es un alias de otro comando y no devuelve lo mismo. En CMD, macOS y Linux vale `curl`.

### Ollama (puerto 11434)

| Quiero saber… | Comando | Campo del formulario |
| --- | --- | --- |
| Qué modelos tengo descargados | `ollama list` | **Modelo**: la columna `NAME`, completa (por ejemplo `qwen2.5:3b`; `llama3:latest` también vale como `llama3`) |
| Lo mismo, por la API | `curl http://localhost:11434/api/tags` | — |
| Qué modelos están cargados en memoria ahora | `ollama ps` | — |
| Detalles de un modelo (arquitectura, parámetros, contexto máximo, cuantización) | `ollama show qwen2.5:3b` | **Contexto**: el máximo que muestra es el del modelo; elige un valor igual o menor que quepa en tu memoria |
| Descargar un modelo nuevo | `ollama pull qwen2.5:3b` | — |

La aplicación **fija el contexto por su cuenta** en Ollama, así que el campo *Contexto* es el que tú decides, no el que tenga Ollama configurado. Si lo dejas vacío, usa un valor prudente (4 096).

### LM Studio (puerto 1234)

Con su servidor local arrancado:

```bash
curl http://localhost:1234/v1/models      # el campo "id" de cada modelo → campo Modelo
lms ls                                    # modelos descargados (si tienes instalada su herramienta `lms`)
```

El contexto se fija en LM Studio al cargar el modelo (ajuste *Context Length*); copia ese número en el formulario.

### Servidor propio con llama.cpp (`llama-server`)

Con el servidor arrancado (el puerto por defecto es `8080`):

```bash
curl http://127.0.0.1:8080/v1/models      # "data" → "id": nombre del modelo → campo Modelo
curl http://127.0.0.1:8080/props          # contexto, slots y capacidades
```

Y en PowerShell, ya masticado:

```powershell
# Nombre del modelo → campo Modelo
(curl.exe -s http://127.0.0.1:8080/v1/models | ConvertFrom-Json).data.id

# Contexto, slots, herramientas e imágenes
$p = curl.exe -s http://127.0.0.1:8080/props | ConvertFrom-Json
"n_ctx=$($p.default_generation_settings.n_ctx)  slots=$($p.total_slots)  tools=$($p.chat_template_caps.supports_tools)  vision=$($p.modalities.vision)"
```

| Dato que devuelve | Qué significa | Campo del formulario |
| --- | --- | --- |
| `id` en `/v1/models` | Nombre del modelo (suele ser el fichero `.gguf`) | **Modelo** |
| `n_ctx` en `/props` | Contexto **total** del servidor (el flag `-c`) | **Contexto (tokens)**, dividido entre `total_slots` |
| `total_slots` en `/props` | Slots en paralelo: cada petición recibe `n_ctx / total_slots` (con 4 slots y 32 768, 8 192). Con `-np 1`, todo el contexto | Divide `n_ctx` entre este número (ver «Tres trampas habituales», arriba) |
| `supports_tools` en `/props` | Si el modelo admite herramientas | **Herramientas**: déjalo en *Detectar*; fuérzalo a *No* si ves que falla |
| `vision` en `/props` | Si el servidor lleva visión (`--mmproj`) | — |
| `n_ctx_train` en `/v1/models` | Contexto con el que se **entrenó** el modelo: **no es el del servidor** | No lo uses |

### El botón «Probar conexión»

Es la comprobación definitiva: si el servidor responde y la dirección es correcta, devuelve la lista real de modelos. Para Ollama y LM Studio, esa lista sale en el desplegable del campo **Modelo**; para un servidor propio, comprueba que el nombre que escribes es uno de los que devuelve `/v1/models`.

---

## 2. En la nube, con tu clave de API

Para usar un modelo de un proveedor con **clave de API**:

| Proveedor | Dónde se consigue la clave | Dirección (ya preconfigurada) |
| --- | --- | --- |
| Anthropic (Claude) | consola de Anthropic | `https://api.anthropic.com/v1` |
| OpenAI (ChatGPT) | plataforma de OpenAI | `https://api.openai.com/v1` |
| Google (Gemini) | Google AI Studio | `https://generativelanguage.googleapis.com/v1beta/openai` |
| OpenRouter | openrouter.ai | `https://openrouter.ai/api/v1` |

**Conectar una IA → Añadir una IA en la nube (clave de API)…** → elige el proveedor → pega la clave → **Probar conexión** → elige el modelo de la lista que devuelve el propio proveedor → **Guardar**.

- **La clave** se guarda en el **almacén de credenciales del sistema** (Administrador de credenciales de Windows, Llavero de macOS, Secret Service en Linux), nunca en un fichero ni en `localStorage`. Al editar una conexión el campo de la clave queda vacío: dejarlo así conserva la guardada. Eliminar la conexión borra también la clave.
- **Privacidad:** para responder, se envía al proveedor el contexto (documento abierto, selección y lo que adjuntes). **La primera vez por proyecto y proveedor, la aplicación te pregunta** antes de enviar nada. Puedes quitar cualquier elemento del contexto desde el propio panel.
- **Coste:** lo factura el proveedor por uso.

---

## 3. Con tu suscripción: agentes instalados

Si pagas **Claude Pro/Max, ChatGPT Plus, Gemini / Google One o GitHub Copilot**, no necesitas clave de API: DBV Unreal AI puede hablar con el **agente de línea de comandos** de ese servicio, que usa tu cuenta. La aplicación **no ve ni guarda tus credenciales**: el inicio de sesión lo hace el propio agente. La conexión usa el protocolo abierto [ACP (Agent Client Protocol)](https://agentclientprotocol.com).

| Agente | Qué debe estar instalado | Cómo lo lanza la aplicación |
| --- | --- | --- |
| **Claude Code** | Claude Code y **Node.js** (incluye `npx`) | `npx -y @agentclientprotocol/claude-agent-acp` |
| **Codex** (OpenAI) | Codex y **Node.js** (incluye `npx`) | `npx -y @agentclientprotocol/codex-acp` |
| **Gemini CLI** | Gemini CLI | `gemini --acp` |
| **Google Antigravity** | Antigravity CLI (`agy`) **y su servidor ACP** (`agy_acp_server`, se instala aparte: [ver abajo](#google-antigravity-instalar-su-servidor-acp)) | `agy_acp_server` |
| **GitHub Copilot CLI** | Copilot CLI (**no basta** el comando `copilot` que trae VS Code: [ver abajo](#github-copilot-el-comando-copilot-de-vs-code-no-es-la-cli)) | `copilot --acp` |

Enlaces de instalación: [Claude Code](https://docs.anthropic.com/en/docs/claude-code/setup) · [Codex](https://github.com/openai/codex) · [Gemini CLI](https://github.com/google-gemini/gemini-cli) · [Antigravity CLI](https://antigravity.google/product/antigravity-cli) · [Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli) · [Node.js](https://nodejs.org).

**Pasos:**

1. Instala el agente (y Node.js si lo necesita) para que quede en el `PATH` de tu sistema.
2. **Ábrelo una vez en una terminal** e inicia sesión con tu cuenta. La aplicación no puede iniciar sesión por ti.
3. Abre **Conectar una IA**. El agente aparece en **IAs por Suscripción** con ✓ *instalado* y un botón **Conectar**.
4. La primera vez puede tardar: para Claude Code y Codex se descarga su adaptador con `npx`.
5. Si el agente pide **iniciar sesión** al conectarse (Antigravity y Codex pueden pedirlo la primera vez), la aplicación se lo solicita con el método habitual del agente (por ejemplo, *Log in with Google* o *Login with ChatGPT*): si se abre el navegador, complétalo allí. La sesión la guarda el propio agente.
6. Opcional: con el agente conectado, el botón **⚙️** te deja elegir **modelo** y **nivel de razonamiento**. La lista se pide al propio agente la primera vez que abres la pantalla; **🔄** la vuelve a pedir. Junto al nombre del agente verás un resumen del modelo y el nivel elegidos.

### GitHub Copilot: el comando `copilot` de VS Code no es la CLI

VS Code con Copilot pone en el `PATH` un comando `copilot` que **solo es un lanzador**: ejecuta la CLI de Copilot si está instalada y, si no, avisa *«GitHub Copilot CLI was not found»*. La aplicación lo sabe y, si detrás no hay CLI, muestra Copilot como *no instalado*. Instala la CLI de verdad con cualquiera de estas vías ([guía oficial](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli)):

| Sistema | Orden |
| --- | --- |
| Windows | `winget install GitHub.Copilot` |
| macOS y Linux | `brew install --cask copilot-cli` |
| Cualquiera (Node.js 22 o posterior) | `npm install -g @github/copilot` |

Después ábrela una vez con `copilot` en una terminal, inicia sesión con tu cuenta de GitHub y vuelve a abrir **Conectar una IA**.

### Google Antigravity: instalar su servidor ACP

La CLI de Antigravity (`agy`) **no habla ACP por sí sola**: lo hace un programa aparte, **`agy_acp_server`**, que Google publica en el [registro ACP](https://github.com/agentclientprotocol/registry/blob/main/antigravity-acp/agent.json). Si solo tienes `agy`, la aplicación muestra Antigravity como *instalado, pero sin su servidor ACP* y no se puede conectar.

1. **Instala [Antigravity CLI](https://antigravity.google/product/antigravity-cli)**, ábrelo una vez con `agy` en una terminal e inicia sesión con tu cuenta de Google.
2. **Descarga el servidor ACP** para tu sistema (versión 1.4.4; la más reciente siempre está en el [registro](https://github.com/agentclientprotocol/registry/blob/main/antigravity-acp/agent.json)):

   | Sistema | Descarga |
   | --- | --- |
   | Windows x64 | [agy-acp-server-1.4.4-windows-x86_64.zip](https://dl.google.com/agy-extensions/releases/windows/agy-acp-server-1.4.4-windows-x86_64.zip) |
   | Windows ARM | [agy-acp-server-1.4.4-windows-arm64.zip](https://dl.google.com/agy-extensions/releases/windows/agy-acp-server-1.4.4-windows-arm64.zip) |
   | macOS Apple Silicon | [agy-acp-server-1.4.4-darwin-arm64.zip](https://dl.google.com/agy-extensions/releases/macos/agy-acp-server-1.4.4-darwin-arm64.zip) |
   | macOS Intel | [agy-acp-server-1.4.4-darwin-x86_64.zip](https://dl.google.com/agy-extensions/releases/macos/agy-acp-server-1.4.4-darwin-x86_64.zip) |
   | Linux x64 | [agy-acp-server-1.4.4-linux-x86_64.zip](https://dl.google.com/agy-extensions/releases/linux/agy-acp-server-1.4.4-linux-x86_64.zip) |
   | Linux ARM | [agy-acp-server-1.4.4-linux-arm64.zip](https://dl.google.com/agy-extensions/releases/linux/agy-acp-server-1.4.4-linux-arm64.zip) |

3. **Descomprime todo el zip** (trae más de un fichero y deben quedar juntos) en esta carpeta, donde la aplicación lo busca:

   | Sistema | Carpeta |
   | --- | --- |
   | Windows | `%LOCALAPPDATA%\agy\acp-server` |
   | macOS y Linux | `~/.local/share/agy-acp-server` |

   En **Windows**, desde PowerShell (Windows x64; para ARM cambia `x86_64` por `arm64`):

   ```powershell
   $zip = "$env:TEMP\agy-acp-server.zip"
   Invoke-WebRequest https://dl.google.com/agy-extensions/releases/windows/agy-acp-server-1.4.4-windows-x86_64.zip -OutFile $zip
   Expand-Archive $zip -DestinationPath "$env:LOCALAPPDATA\agy\acp-server" -Force
   ```

   En **macOS o Linux** (cambia el nombre del zip por el de tu sistema):

   ```bash
   mkdir -p ~/.local/share/agy-acp-server && cd ~/.local/share/agy-acp-server
   curl -LO https://dl.google.com/agy-extensions/releases/macos/agy-acp-server-1.4.4-darwin-arm64.zip
   unzip -o agy-acp-server-*.zip && chmod +x agy_acp_server.par
   ```

4. Vuelve a abrir **Conectar una IA**: Antigravity aparece con ✓ *instalado* y el botón **Conectar**.
5. La primera vez que le escribas, el servidor pide **iniciar sesión** y la aplicación se lo solicita con tu cuenta de Google (*Log in with Google*): si se abre el navegador, complétalo allí.

> Si prefieres otra carpeta, vale cualquiera que esté en el `PATH`. Los adaptadores de terceros que «envuelven» `agy` no se admiten: usan un modo de `agy` sin documentar y Google no ha aclarado si sus condiciones lo permiten.

**Cómo funciona:**

- La aplicación lanza el agente **con la carpeta del documento como directorio de trabajo**; se cierra al cerrar el documento o la aplicación.
- **Permisos:** cada vez que el agente pide permiso para algo (ejecutar una orden, editar, leer), la aplicación te lo muestra con lo que pide. Puedes **Permitir una vez**, **Permitir siempre en esta conversación** o **Denegar**. Nada se aprueba solo, y lo que quede fuera de la carpeta se deniega.
- **Seguridad de tus ficheros:** antes de cada turno, la aplicación guarda un **punto de restauración** de los ficheros de texto de la carpeta. Lo que el agente cambie directamente en disco aparece al terminar bajo el título **«El agente cambió N documentos en disco»**, con sus diferencias y **Deshacer** por fichero o en bloque.

---

## Privacidad de un vistazo

| Vía | Qué sale de tu equipo |
| --- | --- |
| IA local | Nada. La conversación no abandona tu ordenador. |
| API en la nube | El contexto que se envía, al proveedor. Se pregunta la primera vez por proyecto. |
| Agente con suscripción | Lo que el agente envíe a su servicio. La aplicación no controla su red; los permisos sí pasan por ella. |

Las herramientas de los modelos directos (local y API) **no acceden a la red ni ejecutan programas**: solo leen la carpeta y proponen cambios que tú revisas trozo a trozo antes de aplicarlos, con **Deshacer**.

## Problemas frecuentes

| Síntoma | Causa probable | Solución |
| --- | --- | --- |
| Escribo y **no responde nada** (servidor propio) | Proveedor en *Ollama* en lugar de *compatible con OpenAI* | Crea una conexión nueva con el proveedor correcto |
| Respuesta vacía y muchos tokens «recibidos» | El modelo razona y no llega a escribir `content` | Arranca el servidor con `--reasoning-budget 0` |
| «Servidor local apagado» | Ollama, LM Studio o tu servidor no están en marcha | Arráncalo y vuelve a **Probar conexión** |
| Error de dirección o 404 con un servidor propio | Falta `/v1` al final de la dirección | `http://127.0.0.1:PUERTO/v1` |
| Clave no válida | Clave mal pegada o de otro proveedor | Vuelve a pegarla; una suscripción no sirve como clave |
| Un agente no aparece o no conecta | No está en el `PATH`, falta Node.js o no has iniciado sesión | Instálalo, ábrelo una vez en una terminal y abre de nuevo **Conectar una IA** |
| El agente dice que necesita Node.js | Claude Code y Codex usan `npx` | Instala [Node.js](https://nodejs.org) |
| Copilot aparece *no instalado* aunque `copilot` existe, o «GitHub Copilot CLI was not found» | Solo está el lanzador `copilot` de VS Code, sin la CLI | Instala la CLI: [instrucciones](#github-copilot-el-comando-copilot-de-vs-code-no-es-la-cli) |
| Antigravity aparece *sin su servidor ACP*, o «Antigravity necesita su servidor ACP» | Solo está instalada la CLI `agy` | Instala `agy_acp_server`: [instrucciones](#google-antigravity-instalar-su-servidor-acp) |
| Codex responde «Internal error» | Una versión antigua de la aplicación, con el adaptador `@zed-industries/codex-acp`, que no entiende los modelos actuales | Actualiza la aplicación: la versión actual usa `@agentclientprotocol/codex-acp` |
| El modelo olvida el principio del documento | Contexto demasiado pequeño | Sube el contexto del servidor y el campo **Contexto (tokens)** |

<!-- dbv-ia:fin-guia-comun · Todo lo anterior sale de dbv-specs-ops/docs/templates/IA.template.md (no lo edites aquí: cambia la plantilla y sincroniza con scripts/sync-ai.mjs). Lo que pongas debajo es propio de esta aplicación y se conserva. -->

## Lo propio de DBV Unreal AI (Unreal y Tripo)

- **El agente ya llega conectado a Unreal:** la app le pasa su propio servidor MCP (`dbv-unreal-ai`), con las herramientas del editor, la biblioteca y toda la API de Tripo.
- **Lo que genera se ve:** si el agente lanza un trabajo de Tripo, la conversación enseña su tarjeta con el progreso y la consola de lo que se envía.
- **Nada se modifica sin tu permiso.** Las consultas (listar actores, leer la escena) van solas; lo que cambia el proyecto aparece en una tarjeta con el detalle completo (el código Python entero, los parámetros) y espera tu «Permitir». El Python se comprueba antes contra la API real de tu versión del motor.
- **Nada se gasta sin tu permiso.** Cada operación de Tripo (generar, riggear, animar, convertir…) enseña antes su coste según la tabla de precios de Tripo y exactamente lo que se enviará. El tope de créditos por sesión se fija en Ajustes.
- **Imágenes y documentos de referencia.** Con 📎 en la conversación puedes adjuntar imágenes y documentos. Un modelo con visión ve la imagen; uno sin visión recibe su nombre y su ruta, y la app te avisa. En los dos casos la IA puede usar la imagen para generar un modelo con Tripo.
- **Desde fuera de la app.** Ajustes → **Usar DBV Unreal AI desde otros agentes (MCP)** te da el bloque de configuración para Claude Code, Cursor o Gemini CLI con la ruta exacta del ejecutable.
- **Gastar pocos tokens (v0.3.0).** Todo lo que devuelve una herramienta se queda en la conversación y el agente lo vuelve a enviar en cada paso. Por eso la app devuelve a la IA un formato compacto de los toolsets (una línea por herramienta), corta los resultados enormes y diciendo cómo pedir menos, y hace capturas más ligeras. Al pie de cada respuesta verás «Herramientas: N llamadas · X KB (~Y tokens, aprox.)». Para gastar menos, pide cosas concretas («cuántas luces hay», no «descríbeme todo») y elige en ⚙️ un modelo y un nivel de razonamiento acordes a la tarea.
- **Un agente que ve el editor dos veces.** Si la carpeta del proyecto tiene un `.mcp.json` (u otra configuración de agente) que apunta al servidor MCP del editor, como el que escribe el asistente «Set Up AI Coding» de Convai, la vista Proyecto te avisa. Ese acceso directo duplica las herramientas, gasta más y se salta la comprobación del Python y tus aprobaciones. La app no toca esos ficheros: si quieres evitarlo, quita ese servidor o no añadas las instrucciones de Convai.
- **Recordar varias cosas a la vez.** «Recordar» y la IA pueden proponer varias entradas de memoria en una sola tarjeta: marcas las que quieras guardar, las editas y descartas el resto.

### Convai: personajes que hablan

Con el plugin **Convai** (desde Fab, UE 5.8 o posterior) un MetaHuman puede conversar contigo con voz, mover la boca y hacer acciones. La app no necesita ninguna clave de Convai: el plugin se autentica solo cuando inicias sesión en su ventana dentro del editor, y la app usa sus herramientas desde el mismo servidor MCP del editor.

- **Comprobarlo:** la vista Proyecto tiene una sección **Convai** que dice si el plugin está activo, si el editor publica sus herramientas y sus guías, y si has iniciado sesión. Avisa también si `Config/DefaultEngine.ini`, donde el plugin guarda la clave, está en git: no lo subas a un repositorio público.
- **Registrar personajes:** en **Ajustes → Convai** añade cada personaje con un nombre y su Character ID del Playground. La ayuda (F1 → Convai) tiene el tutorial completo.
- **Montarlo:** pide «Haz que BP_VictoriaNew sea Victoria» (el nombre registrado). La IA lee antes las guías de Convai y te pide el **Character ID** del personaje, que creas en el Playground de [convai.com](https://convai.com) (nunca se lo inventa). Cada paso pide tu permiso. Estas acciones **guardan Blueprints**: «Deshacer este turno» puede no revertirlas, así que conviene tener el proyecto en control de versiones.
- **Zonas del nivel:** pide «define las zonas de este nivel». La IA propone sitios con nombre y descripción («la cocina: la mesa larga y el horno de leña»), los revisas en una tarjeta y, al aceptar, se crean como marcadores en la carpeta `DBV/Zones` del Outliner. Con Convai, el personaje los conoce y puede ir allí («llévame a la cocina»). Se pueden deshacer.
- **Hablar con el personaje** se hace en **Play**, dentro de Unreal y con el micrófono, no en la app.

### Novedades de la v0.3.0 para la IA

- **Editar y compilar el código del proyecto.** La IA puede proponer cambios en el C++ (`Source/`, también `*.Build.cs`), el Python (`Content/Python/`) y los `.ini` de `Config/` de tu proyecto, como diffs que revisas antes de que se escriba nada. Nunca toca `.uasset` ni `.umap`, nada fuera del proyecto ni en `Plugins/`. Al aceptar un cambio de C++, la app compila: con **Live Coding** si solo cambian `.cpp`, o, si cambian cabeceras o reglas del módulo, te pide cerrar el editor, compila con UnrealBuildTool y te ofrece reabrirlo. Si no compila, los errores vuelven a la IA solos como mucho dos veces seguidas. Un agente que usa el `--mcp` desde fuera tiene `compile_project_code`.
- **Recetas.** El botón **📋 Recetas** de la conversación ofrece tareas guiadas: *Nivel de prueba*, *Personaje de Tripo con animaciones*, *Caminar por un mundo de World Labs*, *MetaHuman que habla con Convai* y *Diagnosticar un fallo en ejecución* (mide lo que pasa mientras juegas, lo cruza con lo que notas y confirma la causa antes de tocar nada). La IA las hace paso a paso, pidiendo permiso para cada cambio, y al final comprueba en el editor qué está y qué falta (`recipe_check`). Un agente conectado al editor las ve también como Agent Skills (`ListSkills`).
- **Guías de Unreal sin red.** En **Ajustes → Conocimiento** puedes elegir una copia local de las guías de Unreal 5.8 (el formato de MCPCLiente). La IA busca en ella con `search_unreal_guides` sin conexión y cita la URL de cada fragmento. La app no descarga ni trae documentación de Epic.
- **«Mejorar con IA» en Generar.** Junto al prompt de Tripo y de World Labs, convierte una idea corta en una descripción elaborada para cada servicio. No genera nada: revisas el texto y pulsas Generar. También con un agente: va aparte, sin pasar por la conversación, y el texto vuelve al campo.
- **Menos permisos con los agentes.** Las consultas del servidor de la app (ver la escena, listar herramientas, comprobar una receta…) ya no piden permiso: la conversación lo indica con «✓ Consulta, sin pedir permiso». Lo que modifica, ejecuta Python o gasta créditos sigue preguntando.
- **Chino.** La interfaz, la ayuda y las respuestas de la IA pueden ir en chino simplificado (中文).

### Novedades de la v0.3.1 para la IA

- **Cada conversación, su propia sesión.** Con un agente (Claude Code, Codex, Antigravity…), crear una conversación nueva, cambiar de una a otra o borrarla cierra la sesión del agente y abre una limpia. Una conversación nueva ya no arrastra lo que se habló en otra ni gasta su cuota, y si reabres una antigua, el agente recibe un resumen acotado de lo que se dijo. El contador de tokens del pie es de la conversación que tienes abierta.
- **El modelo que ves es el que se usa.** Si la conexión guardaba un nombre genérico («claude», «default»), la app lo cambia por el modelo que muestra la interfaz en vez de dejar que el agente use el de su configuración global.
- **Pegar y soltar imágenes.** `Ctrl+V` en el cuadro de texto con una captura, o arrastrar imágenes y documentos de texto sobre la conversación. Las imágenes (PNG, JPG, WebP, hasta 10 MB) salen con su miniatura; si el modelo tiene visión la ve, y como se guardan en la carpeta temporal del sistema (las 20 últimas), la IA puede usar su ruta con Tripo. Un fichero que no es imagen ni texto se rechaza con un aviso.
- **También los `.md`.** Además de `Source/`, `Content/Python/` y `Config/*.ini`, la IA puede proponer cambios en el `README.md` y los demás `.md` de la raíz del proyecto de Unreal y en los de `docs/`, con el mismo diff que revisas antes de que se escriba nada. En el resto de carpetas, los `.md` siguen rechazados.
- **Cabecera de DBV en lo que crea.** Todo fichero nuevo que la IA cree en `Source/`, `Content/Python/` o en los `.md` permitidos empieza con la cabecera estándar de 6 líneas de DBV Unreal AI (nombre, copyright, licencia MIT y dbv-specs-ops).
- **Mundos de World Labs que no se pierden.** Si cierras la app, o un agente cierra su sesión, con un mundo a medias, al volver a abrirla se reanuda y se descarga a la biblioteca. Si aún no has guardado la clave de World Labs, queda «en curso» hasta que la pongas.

### Novedades de la v0.5.0 para la IA: Vestir MetaHumans y Play-Testing Autónomo

- **Vestir un MetaHuman con una sola herramienta (`unreal_dress_metahuman`).** La IA utiliza un pipeline determinista único (`outfit/pipeline.py`) en vez de inventar código con `run_python`. El proceso ejecuta 5 pasos ordenados:
  1. *Copia segura del personaje:* genera `BP_<Nombre>_DBV` para preservar el Blueprint original intacto.
  2. *Reglas de ranura:* retira o enmascara prendas que chocan (por ejemplo, oculta `Legs` con un vestido largo).
  3. *Piel limpia y recorte anatómico:* sustituye texturas de ropa interior pintada y recorta triángulos corporales que puedan asomar.
  4. *Ajuste y deformación esquelética:* transfiere pesos de huesos (`INPAINT_WEIGHTS`) para tops, pantalones y calzado, o monta tela física con Chaos Cloth para faldas y vestidos.
  5. *Validación final:* mide holguras, penetraciones y bordes abiertos.
- **Destino explícito del MetaHuman (`REQ-83`).** La IA nunca adivina ni equipa a ciegas sobre «el primero del nivel». Consulta `unreal_list_metahumans`: si hay un solo personaje, lo selecciona y te informa; si hay varios, te pregunta antes de equipar; si no hay ninguno en el nivel, se limita a importar la prenda a la biblioteca.
- **Tipos de prenda y corte (`REQ-84`).** Soporta cinco categorías con ajuste anatómico específico: `top`, `pants`, `skirt`, `dress` y `shoes`, distinguiendo complexiones (`male`, `female`, `neutral`) en cualquier arquetipo MetaHuman (`f_med_nrw`, `m_tal_nrw`, etc.).
- **Materiales de dos caras, soldadura y 4 LODs (`REQ-81`).** Las prendas se sellan con soldadura de vértices, configuran materiales con `override_two_sided=True` para evitar espaldas o cuellos invisibles, y generan automáticamente 4 niveles de LOD sincronizados con el cuerpo MetaHuman.
- **Tela automática con Chaos Cloth (`REQ-87`).** Para faldas y vestidos, construye proceduralmente el grafo Dataflow y el activo Cloth Asset enlazando `ChaosClothComponent` en `BeginPlay`, permitiendo vuelo dinámico con el movimiento del personaje en Play.
- **Validación automática e informe (`unreal_validate_metahuman_outfit` / `REQ-89`).** Mide holgura corporal mínima y mediana, comprueba colisiones físicas y captura el resultado en 3 ángulos (frente, lado y espalda). Si el informe devuelve avisos o penetraciones, la IA te los explica con total transparencia en lugar de darlos por buenos.
- **Límites de empaquetado (`P-20` / `R-39`).** La vestimenta basada en deformación ósea (`InpaintWeights`) empaqueta y funciona directamente en Windows, Android y Meta Quest VR. La simulación de tela requiere que el proyecto incluya los plugins en tiempo de ejecución de Chaos Cloth.
- **Play-testing autónomo en PIE (`unreal_playtest` / `REQ-82`).** Permite ejecutar pruebas de juego en Play-In-Editor sin bloquear el hilo principal de Unreal, registrando telemetría, capturas y detectando fallos como caídas al vacío (`fell_out_of_world`).
