# 🤖 Usar una IA con DBV Unreal AI

> 🌐 **Español** · [English](./IA.en.md) · [← Volver al README](../README.md)

La IA en **DBV Unreal AI** es **opcional**. Sin conectar ninguna, la aplicación detecta tus editores de Unreal, prepara el proyecto, genera modelos con Tripo desde la vista **Generar** y gestiona la biblioteca. La IA añade la **Conversación**: le pides algo sobre tu proyecto y actúa en el editor con tu permiso. Esta guía explica **qué necesitas** según la modalidad de IA que prefieras.

## Lo esencial en una tabla

| Quiero usar… | Qué necesito tener o instalar | ¿Sale algo de mi equipo? | ¿Tiene coste? |
| --- | --- | --- | --- |
| **Una IA local** (Ollama, LM Studio, llama.cpp…) | El programa servidor y al menos un modelo descargado | **No** | No (usa tu CPU/GPU y memoria) |
| **Claude, ChatGPT, Gemini u OpenRouter con clave de API** | Una clave de API del proveedor | Sí, al proveedor elegido | Sí, por uso, según sus tarifas |
| **Mi suscripción mensual** (Claude Pro/Max, ChatGPT Plus, Copilot) | El **agente oficial** instalado y con sesión iniciada: Claude Code, Codex o GitHub Copilot CLI | Sí, al servicio del agente | Cubierto por tu suscripción |

> ⚠️ **Una suscripción no es una clave de API.** Pagar Claude Pro, ChatGPT Plus o Gemini no te da una clave de API (se contratan y facturan aparte, en las consolas de desarrollador). Si tienes una suscripción, la vía recomendada es el **agente instalado** (tercera fila).

Para conectar una IA: **Ajustes → Conectar una IA**, o abre la **Conversación** (`Ctrl+Mayús+I`, `Cmd+Shift+I` en macOS) y, si no hay ninguna conectada, se abre el asistente de conexión.

> 🎨 **Tripo y World Labs son aparte.** Generar modelos 3D y mundos fotorrealistas usa tu clave de Tripo o World Labs (Ajustes) y gasta créditos de tu cuenta correspondiente, sea cual sea la IA de la conversación.

---

## 1. Una IA local (nada sale de tu equipo)

La opción más privada: tus datos no salen de la máquina y no hay coste por uso. La velocidad y la calidad dependen de tu equipo y del tamaño del modelo. Para que la IA use las herramientas del editor, elige un modelo que admita herramientas (*tools*), por ejemplo `qwen3:8b`.

### Ollama
1. Descarga e instala [Ollama](https://ollama.com/download).
2. Descarga un modelo desde la terminal, por ejemplo:
   ```bash
   ollama pull qwen3:8b
   ```
3. En DBV Unreal AI, abre **Conectar una IA**. Si Ollama está en marcha, aparece con una marca verde y el botón **Usar**.
4. Elige el modelo, pulsa **Probar conexión** y **Guardar**.

### LM Studio
1. Instala [LM Studio](https://lmstudio.ai), descarga el modelo e inicia su servidor local (*Local Server*, puerto `1234` por defecto).
2. DBV Unreal AI detecta el servidor solo.

---

## 2. En la nube, con tu clave de API

Para la máxima potencia de los modelos de frontera con una **clave de API**:

| Proveedor | Dónde se consigue la clave |
| --- | --- |
| **Anthropic (Claude)** | Consola de Anthropic |
| **OpenAI (ChatGPT)** | Plataforma de OpenAI |
| **Google (Gemini)** | Google AI Studio |
| **OpenRouter** | openrouter.ai |

1. Abre **Conectar una IA → Añadir una IA en la nube (clave de API)…**
2. Elige el proveedor y pega tu clave.
3. Pulsa **Probar conexión**, elige el modelo y **Guardar**.

- **Custodia segura:** la clave se guarda en el almacén de credenciales de tu sistema (Administrador de credenciales de Windows, Llavero de macOS o Secret Service en Linux). Nunca en ficheros de texto ni en el navegador. La de Tripo, igual.
- **Aviso de nube:** antes del primer envío a un proveedor en la nube, la app te recuerda que tu petición y el contexto del proyecto salen hacia él.

---

## 3. Con tu suscripción: agentes instalados (ACP)

Si tienes **Claude Pro/Max, ChatGPT Plus o GitHub Copilot**, puedes conectar su herramienta oficial de línea de comandos mediante el protocolo ACP:

| Agente | Qué debe estar instalado | Cómo se ejecuta |
| --- | --- | --- |
| **Claude Code** | Claude Code y [Node.js](https://nodejs.org) | `npx -y @agentclientprotocol/claude-agent-acp` |
| **Google Antigravity CLI** | [Antigravity CLI](https://antigravity.google/product/antigravity-cli) (`agy`) | `agy_acp_server` o `agy --acp` |
| **Codex (OpenAI)** | Codex y Node.js | `npx -y @zed-industries/codex-acp` |
| **GitHub Copilot CLI** | GitHub Copilot CLI | `copilot --acp` |
| **Gemini CLI** | [Gemini CLI](https://github.com/google-gemini/gemini-cli) (para API / Vertex AI) | `gemini --acp` |

> ℹ️ **Google Antigravity CLI y Gemini CLI.** Google utiliza [Antigravity CLI](https://antigravity.google/product/antigravity-cli) para suscripciones de Google One, Gemini Advanced y cuentas gratuitas. Por su parte, [Gemini CLI](https://github.com/google-gemini/gemini-cli) se reserva para accesos de pago directo por uso (clave de API, Vertex AI o Gemini Code Assist). DBV Unreal AI soporta ambos según el tipo de cuenta que poseas.

**Pasos:**
1. Instala el agente y comprueba que está en el `PATH` de tu terminal.
2. Ábrelo una vez en la terminal para iniciar sesión con tu cuenta.
3. En DBV Unreal AI, abre **Conectar una IA**: el agente aparece disponible para **Conectar**.

- **El agente ya llega conectado a Unreal:** la app le pasa su propio servidor MCP (`dbv-unreal-ai`), con las herramientas del editor, la biblioteca y toda la API de Tripo.
- **Control de permisos:** cada vez que el agente quiere usar una herramienta, leer o modificar ficheros, DBV Unreal AI te pide confirmación (Permitir una vez, Permitir siempre en esta conversación o Denegar).
- **Lo que genera se ve:** si el agente lanza un trabajo de Tripo, la conversación enseña su tarjeta con el progreso y la consola de lo que se envía.
- **Punto de restauración:** al acabar cada turno, la app muestra qué ficheros cambiaron en disco, con **Deshacer** en bloque o por fichero.

---

## 4. Lo propio de Unreal y de Tripo

- **Nada se modifica sin tu permiso.** Las consultas (listar actores, leer la escena) van solas; lo que cambia el proyecto aparece en una tarjeta con el detalle completo (el código Python entero, los parámetros) y espera tu «Permitir». El Python se comprueba antes contra la API real de tu versión del motor.
- **Nada se gasta sin tu permiso.** Cada operación de Tripo (generar, riggear, animar, convertir…) enseña antes su coste según la tabla de precios de Tripo y exactamente lo que se enviará. El tope de créditos por sesión se fija en Ajustes.
- **Imágenes y documentos de referencia.** Con 📎 en la conversación puedes adjuntar imágenes y documentos. Un modelo con visión ve la imagen; uno sin visión recibe su nombre y su ruta, y la app te avisa. En los dos casos la IA puede usar la imagen para generar un modelo con Tripo.
- **Desde fuera de la app.** Ajustes → **Usar DBV Unreal AI desde otros agentes (MCP)** te da el bloque de configuración para Claude Code, Cursor o Gemini CLI con la ruta exacta del ejecutable.
