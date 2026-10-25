<p align="center"><img src="src/assets/logo.png" alt="Unreal Dev Community Málaga" width="128" height="128"></p>

# DBV Unreal AI

**🇪🇸 Español · [🇬🇧 English](./README.en.md)**

> Copiloto de IA nativo para Unreal Engine 5.8: conecta cualquier IA (local o en la nube) a tu proyecto abierto, lo prepara si hace falta y genera assets con Tripo directamente en el editor.

## 📑 Índice

- [Descárgalo e instálalo](#install)
  - [Windows](#install-windows)
  - [macOS](#install-macos)
  - [Linux](#install-linux)
  - [Próximamente: Homebrew, Microsoft Store y Uptodown](#install-soon)
- [Sobre el proyecto](#about)
- [Estado](#status)
- [Cómo funciona](#how)
- [Requisitos](#requirements)
- [Para desarrolladores](#usage)
- [Usarlo desde otros agentes (MCP)](#mcp)
- [Qué IA usar](./docs/IA.md)
- [Tests](#tests)
- [Estructura del proyecto](#structure)
- [Changelog](#changelog)
- [Licencia](#license)

---

<a name="install"></a>

## 🚀 Descárgalo e instálalo

**No necesitas Rust, Node.js ni ninguna herramienta de programación.** Descarga el instalador de tu sistema desde la **[última Release de GitHub](https://github.com/davidbuenov/dbv-unreal-ai/releases/latest)**: se compila automáticamente en cada versión para Windows, macOS (Intel y Apple Silicon) y Linux.

Solo necesitas **Unreal Engine 5.8** para trabajar con un proyecto y, si quieres conversar, una IA (local, con clave de API o tu suscripción): la guía [`docs/IA.md`](./docs/IA.md) explica cada opción. La clave de [Tripo](https://www.tripo3d.ai) es opcional y solo hace falta para generar modelos 3D.

> **Actualizaciones:** la app todavía no se actualiza sola. Para pasar a una versión nueva, descarga e instala el instalador nuevo desde Releases; la biblioteca, las conexiones y las claves se conservan.

<a name="install-windows"></a>

### 🪟 Windows

**[⬇️ Descarga desde Releases](https://github.com/davidbuenov/dbv-unreal-ai/releases/latest)** uno de estos dos (Windows 10 y 11, 64 bits):

- **`DBV.Unreal.AI_x.y.z_x64-setup.exe`** — el recomendado: doble clic y listo.
- **`DBV.Unreal.AI_x.y.z_x64_en-US.msi`** — el mismo programa en formato MSI, pensado para instalaciones gestionadas (empresas, aulas).

El instalador no está firmado con un certificado de código, así que la primera vez Windows SmartScreen avisa de que «Windows protegió su PC». Pulsa **Más información → Ejecutar de todas formas**. El motor de la interfaz (WebView2) ya viene con Windows 10 y 11; si faltara, el instalador lo descarga.

<a name="install-macos"></a>

### 🍎 macOS

**[⬇️ Descarga `DBV.Unreal.AI_x.y.z_universal.dmg` desde Releases](https://github.com/davidbuenov/dbv-unreal-ai/releases/latest)** — un único binario universal para Intel y Apple Silicon. Ábrelo y arrastra **DBV Unreal AI** a **Aplicaciones**.

No está firmado ni notarizado (requiere el Apple Developer Program, de pago), así que macOS lo bloquea la primera vez («no se puede abrir porque su desarrollador no puede verificarse»). Para abrirlo:

- Clic derecho (o `Ctrl` + clic) sobre **DBV Unreal AI** en Aplicaciones → **Abrir** → confirma en el diálogo. Solo hace falta la primera vez.
- O, desde la Terminal: `xattr -cr "/Applications/DBV Unreal AI.app"` antes de abrirlo.

El fichero `DBV.Unreal.AI_universal.app.tar.gz` de la Release es la misma app comprimida; para instalar basta con el `.dmg`.

<a name="install-linux"></a>

### 🐧 Linux

**[⬇️ Descarga desde Releases](https://github.com/davidbuenov/dbv-unreal-ai/releases/latest)** el paquete de tu distribución (x86_64):

- **`.deb` (Debian, Ubuntu, Linux Mint y derivadas):** `sudo apt install ./DBV.Unreal.AI_x.y.z_amd64.deb` (o doble clic desde el gestor de archivos). Instala también las dependencias que falten, como WebKitGTK.
- **`.rpm` (Fedora, openSUSE, RHEL y derivadas):** `sudo dnf install ./DBV.Unreal.AI-x.y.z-1.x86_64.rpm` (en openSUSE, `sudo zypper install`).
- **`.AppImage` (cualquier distribución):** dale permisos de ejecución (`chmod +x DBV.Unreal.AI_x.y.z_amd64.AppImage`) y ejecútalo. Es portátil, no necesita instalación.

<a name="install-soon"></a>

### 🔜 Próximamente

Estos canales aún no están disponibles; llegarán en una próxima versión. Mientras tanto, usa los instaladores de Releases de arriba.

- **🍺 Homebrew (macOS):** `brew install --cask` desde un tap propio que se actualice solo con cada Release.
- **🛒 Microsoft Store (Windows):** paquete firmado por la Store, sin aviso de SmartScreen y con actualizaciones automáticas.
- **🟢 Uptodown:** descarga directa de la última versión para Windows y macOS.

---

<a name="about"></a>

## 📌 Sobre el proyecto

**DBV Unreal AI** (`dbv-unreal-ai`) es una aplicación de escritorio nativa (Windows, macOS y Linux) que conversa contigo y actúa sobre el proyecto de Unreal Engine 5.8 que tengas abierto. Usa la IA que prefieras —un modelo local (Ollama, LM Studio), una API en la nube o tu suscripción de Claude Code, Codex o Copilot (Gemini CLI, con acceso de pago)— y la conecta con el editor a través de su servidor MCP integrado. Si el proyecto aún no está preparado, la aplicación lo prepara por ti. Genera modelos 3D con Tripo, los guarda en una biblioteca local y los lleva al editor con un clic.

**Construido con:** Rust, Tauri v2, HTML/CSS/JavaScript sin bundler, Unreal Engine 5.8 (MCP + Python).

---

<a name="status"></a>

## 🚧 Estado

**v0.1.0** (2026-10-24, primera versión). Funciona y está verificado en vivo contra Unreal 5.8.2:

- conexión con el editor por MCP, diagnóstico y preparación del proyecto con diff y «Revertir»;
- conversación con IA local, en la nube o agentes de suscripción, con las herramientas del editor;
- **nada que modifique el proyecto se ejecuta sin tu permiso**, y el Python se comprueba contra el motor antes de pedírtelo;
- Tripo texto → modelo, biblioteca local y «Enviar a Unreal» (importación glTF con materiales y texturas);
- servidor MCP propio para usar todo lo anterior desde otros agentes.

También en v0.1.0, construido y probado contra un Tripo simulado (**falta probarlo con la API real de Tripo y con el motor**):

- **toda la API de Tripo** para la IA y los agentes: texto o imagen → modelo, variantes, rig, 101 animaciones (bailes, emociones, deportes…), convertir, retopología, texturizar, segmentar, multivista, imágenes y saldo de la cuenta;
- **consola** de cada trabajo con la petición exacta que se envía a Tripo y su progreso, también en la conversación cuando genera un agente;
- **visor 3D** en la biblioteca y en la conversación;
- **imágenes y documentos de referencia** adjuntos a la conversación;
- biblioteca con **importación** de tus modelos, **carpeta configurable** y nombres cortos con sentido;
- personajes de Tripo en Unreal como Skeletal Mesh reproduciendo su animación.

Hoja de ruta en [`SPECIFICATIONS.md`](./dbv-specs-ops/docs/SPECIFICATIONS.md).

---

<a name="how"></a>

## 🧭 Cómo funciona

1. **Proyecto.** La app detecta los editores de Unreal abiertos y se conecta sola al servidor MCP del editor (`127.0.0.1:8000/mcp`). Si al proyecto le falta algo, el diagnóstico lo dice punto por punto.
2. **Preparar.** «Preparar proyecto…» enseña el diff de cada fichero antes de tocarlo y guarda una copia de seguridad: activa los plugins `ModelContextProtocol`, `ToolsetRegistry`, `PythonScriptPlugin`, `EditorToolset`, `AutomationTestToolset` y `SlateInspectorToolset`, enciende el arranque automático del servidor MCP e instala el **toolset de DBV Unreal AI** (un plugin de solo contenido en Python: no hay que compilar). Con el editor cerrado; al abrirlo de nuevo, queda conectado.
3. **Conversación.** La IA descubre las herramientas del editor (más de 280 en un proyecto preparado), consulta la escena y actúa. Las consultas se ejecutan solas; lo que modifica el proyecto aparece en una tarjeta con el detalle completo y espera tu «Permitir». El Python se valida antes contra la API real de tu versión del motor: si usa algo que no existe o está obsoleto, la IA lo corrige sin molestarte. Todo queda en el registro de acciones (exportable a Markdown).
4. **Generar.** Describe un objeto o elige una imagen de referencia, pide de 1 a 4 variantes y acepta el coste: la tarjeta da el total según la tabla de precios de Tripo y enseña exactamente lo que se envía. Cada trabajo tiene su consola y su progreso; el modelo se descarga a la biblioteca.
5. **Biblioteca.** Cada asset dice qué es y de cuál sale. Un modelo se puede **riggear**, un rig se puede **animar** (hasta 5 animaciones de una vez) y todo se ve en 3D. «Enviar a Unreal» lo importa en `/Game/DBV/Generated` con su material y textura y lo coloca delante de la cámara, en la carpeta `DBV/Generated` del Outliner; un personaje llega como Skeletal Mesh moviéndose. Puedes importar tus propios modelos y elegir dónde vive la biblioteca.

---

<a name="requirements"></a>

## 🧰 Requisitos

- **Unreal Engine 5.8** (instalado con el launcher de Epic o compilado).
- Para desarrollar (no hace falta para usar los instaladores de [Releases](#install)): [Rust](https://rustup.rs) (stable), Node.js 20+ y las dependencias de sistema de Tauri v2 (WebView2 en Windows, WebKitGTK en Linux). Python 3.10+ para los tests del toolset.
- Opcional: una IA (Ollama/LM Studio, una clave de API o un agente de suscripción) y una clave de API de [Tripo](https://www.tripo3d.ai) para generar modelos. Las claves se guardan en el almacén de credenciales del sistema, nunca en ficheros.

---

<a name="usage"></a>

## 🧑‍💻 Para desarrolladores

Para ejecutar la app desde el código fuente (si solo quieres usarla, mejor el [instalador](#install)):

```bash
npm install
npm run tauri dev
```

Instaladores (en `src-tauri/target/release/bundle/`):

```bash
npm run tauri build
```

Al subir un tag `vX.Y.Z`, GitHub Actions compila Windows, macOS (universal) y Linux y los adjunta a una Release en borrador, que se publica a mano.

---

¿Qué IA conectar (local, con clave de API o tu suscripción) y qué necesitas para cada una? Lo explica la guía [`docs/IA.md`](./docs/IA.md).

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

La vista **Ajustes** te da este bloque con la ruta exacta. Ofrece el puente al editor (con la validación del Python), la biblioteca (listar, importar, renombrar, «Enviar a Unreal») y **toda la API de Tripo**: `generate_3d_model`, `image_to_3d_model`, `rig_model`, `animate_model`, `tripo_run` (cualquier otra operación), `tripo_operations` (catálogo con precios), `tripo_balance` y `generation_jobs`. Lo que cuesta créditos exige `confirm_cost: true` tras decirle el precio al usuario; los trabajos corren en segundo plano y la app enseña su progreso.

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

Las notas de cada versión están en dos ficheros, [`dbv-specs-ops/releases/vX.Y.Z.md`](./dbv-specs-ops/releases/) (español) y `vX.Y.Z.en.md` (inglés), y la Release de GitHub las lleva las dos.

---

<a name="license"></a>

## 📄 Licencia

MIT — ver [LICENSE](./LICENSE).

Copyright (c) 2026 David Bueno Vallejo y David Bueno Carmona

---

> 🛠️ Construido con **[dbv-specs-ops](https://github.com/davidbuenov/dbv-specs-ops)** — el framework SDD para desarrollo asistido por IA.
> Creado por [David Bueno Vallejo](https://github.com/davidbuenov) — libre y gratuito.
