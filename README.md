<p align="center"><img src="src/assets/logo.png" alt="Unreal Dev Community Málaga" width="128" height="128"></p>

# DBV Unreal AI

**🇪🇸 Español · [🇬🇧 English](./README.en.md)**

> Copiloto de IA nativo para Unreal Engine 5.8: conecta cualquier IA (local o en la nube) a tu proyecto abierto, lo prepara si hace falta y genera assets con Tripo directamente en el editor.

## 📑 Índice

- [Sobre el proyecto](#about)
- [Estado](#status)
- [Cómo funciona](#how)
- [Requisitos](#requirements)
- [Cómo ejecutar](#usage)
- [Usarlo desde otros agentes (MCP)](#mcp)
- [Tests](#tests)
- [Estructura del proyecto](#structure)
- [Changelog](#changelog)
- [Licencia](#license)

---

<a name="about"></a>

## 📌 Sobre el proyecto

**DBV Unreal AI** (`dbv-unreal-ai`) es una aplicación de escritorio nativa (Windows, macOS y Linux) que conversa contigo y actúa sobre el proyecto de Unreal Engine 5.8 que tengas abierto. Usa la IA que prefieras —un modelo local (Ollama, LM Studio), una API en la nube o tu suscripción de Claude Code, Codex, Gemini CLI o Copilot— y la conecta con el editor a través de su servidor MCP integrado. Si el proyecto aún no está preparado, la aplicación lo prepara por ti. Genera modelos 3D con Tripo, los guarda en una biblioteca local y los lleva al editor con un clic.

**Construido con:** Rust, Tauri v2, HTML/CSS/JavaScript sin bundler, Unreal Engine 5.8 (MCP + Python).

---

<a name="status"></a>

## 🚧 Estado

**v0.1 en desarrollo** (sin publicar). Funciona y está verificado en vivo contra Unreal 5.8.2:

- conexión con el editor por MCP, diagnóstico y preparación del proyecto con diff y «Revertir»;
- conversación con IA local, en la nube o agentes de suscripción, con las herramientas del editor;
- **nada que modifique el proyecto se ejecuta sin tu permiso**, y el Python se comprueba contra el motor antes de pedírtelo;
- Tripo texto → modelo, biblioteca local y «Enviar a Unreal» (importación glTF con materiales y texturas);
- servidor MCP propio para usar todo lo anterior desde otros agentes.

Pendiente de v0.1: imagen → modelo con Tripo (falta verificar la API v3), compilar la variante C++ del toolset. Hoja de ruta en [`SPECIFICATIONS.md`](./dbv-specs-ops/docs/SPECIFICATIONS.md).

---

<a name="how"></a>

## 🧭 Cómo funciona

1. **Proyecto.** La app detecta los editores de Unreal abiertos y se conecta sola al servidor MCP del editor (`127.0.0.1:8000/mcp`). Si al proyecto le falta algo, el diagnóstico lo dice punto por punto.
2. **Preparar.** «Preparar proyecto…» enseña el diff de cada fichero antes de tocarlo y guarda una copia de seguridad: activa los plugins `ModelContextProtocol`, `ToolsetRegistry`, `PythonScriptPlugin`, `EditorToolset`, `AutomationTestToolset` y `SlateInspectorToolset`, enciende el arranque automático del servidor MCP e instala el **toolset de DBV Unreal AI** (un plugin de solo contenido en Python: no hay que compilar). Con el editor cerrado; al abrirlo de nuevo, queda conectado.
3. **Conversación.** La IA descubre las herramientas del editor (más de 280 en un proyecto preparado), consulta la escena y actúa. Las consultas se ejecutan solas; lo que modifica el proyecto aparece en una tarjeta con el detalle completo y espera tu «Permitir». El Python se valida antes contra la API real de tu versión del motor: si usa algo que no existe o está obsoleto, la IA lo corrige sin molestarte. Todo queda en el registro de acciones (exportable a Markdown).
4. **Generar.** Describe un objeto, acepta el coste en créditos de Tripo y el modelo se descarga a la biblioteca.
5. **Biblioteca.** «Enviar a Unreal» lo importa en `/Game/DBV/Generated` con su material y textura y lo coloca delante de la cámara, en la carpeta `DBV/Generated` del Outliner.

---

<a name="requirements"></a>

## 🧰 Requisitos

- **Unreal Engine 5.8** (instalado con el launcher de Epic o compilado).
- Para desarrollar: [Rust](https://rustup.rs) (stable), Node.js 20+ y las dependencias de sistema de Tauri v2 (WebView2 en Windows, WebKitGTK en Linux). Python 3.10+ para los tests del toolset.
- Opcional: una IA (Ollama/LM Studio, una clave de API o un agente de suscripción) y una clave de API de [Tripo](https://www.tripo3d.ai) para generar modelos. Las claves se guardan en el almacén de credenciales del sistema, nunca en ficheros.

---

<a name="usage"></a>

## ▶️ Cómo ejecutar

```bash
npm install
npm run tauri dev
```

Instaladores (en `src-tauri/target/release/bundle/`):

```bash
npm run tauri build
```

Al subir un tag `vX.Y.Z`, GitHub Actions compila Windows, macOS (universal) y Linux y los adjunta a una Release en borrador.

---

<a name="mcp"></a>

## 🔌 Usarlo desde otros agentes (MCP)

El propio ejecutable es un servidor MCP por stdio (sin puerto de red). En Claude Code, Cursor o Gemini CLI:

```json
{
  "mcpServers": {
    "dbv-unreal-ai": { "command": "<ruta>/dbv-unreal-ai", "args": ["--mcp", "--port", "8000"] }
  }
}
```

La vista **Ajustes** te da este bloque con la ruta exacta. Ofrece el puente al editor (con la validación del Python), la biblioteca, «Enviar a Unreal» y la generación con Tripo (que exige `confirm_cost: true`).

---

<a name="tests"></a>

## 🧪 Tests

```bash
npm run test:all     # Rust + Vitest + toolset Python + linter
npm run test:e2e     # interfaz en Edge headless (el motor de WebView2), con el backend simulado
```

Contra un editor real abierto (solo lectura): `DBV_UNREAL_MCP_PORT=8000 cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored en_vivo`.

---

<a name="structure"></a>

## 📂 Estructura del proyecto

```text
/
├── src/                     # Interfaz (HTML/CSS/JS sin bundler)
│   ├── app/                 #   vistas: Proyecto, Generar, Biblioteca, Ajustes, registro
│   ├── unreal/              #   herramientas de Unreal para la IA, permisos, tarjeta de aprobación, envío
│   ├── generation/          #   herramientas de generación y biblioteca para la IA
│   └── ai/                  #   asistente de IA de tres niveles (de dbv-tauri-starter)
├── src-tauri/src/           # Backend en Rust
│   ├── unreal/              #   descubrimiento, cliente MCP del editor, preparación
│   ├── generation/          #   cliente de Tripo v3 y trabajos de generación
│   ├── library/             #   biblioteca local de assets
│   ├── mcp_server.rs        #   servidor MCP propio (--mcp)
│   └── ai/                  #   proveedores, ACP, llavero del sistema
├── unreal-plugin/           # Toolset que se instala en el proyecto (Python; plantilla C++)
├── tests/                   # Vitest, tests del toolset en Python y respuestas reales del editor
├── scripts/e2e/             # Banco de interfaz
└── dbv-specs-ops/           # Especificaciones, arquitectura, task.md, memory.md y CHANGELOG.md
```

---

<a name="changelog"></a>

## 📋 Changelog

Ver [CHANGELOG.md](./dbv-specs-ops/CHANGELOG.md).

---

<a name="license"></a>

## 📄 Licencia

MIT — ver [LICENSE](./LICENSE).

Copyright (c) 2026 David Bueno Vallejo y David Bueno Carmona

---

> 🛠️ Construido con **[dbv-specs-ops](https://github.com/davidbuenov/dbv-specs-ops)** — el framework SDD para desarrollo asistido por IA.
> Creado por [David Bueno Vallejo](https://github.com/davidbuenov) — libre y gratuito.
