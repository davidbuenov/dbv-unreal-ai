# DBV Unreal AI

**🇪🇸 Español · [🇬🇧 English](./README.en.md)**

> Copiloto de IA nativo para Unreal Engine 5.8: conecta cualquier IA (local o en la nube) a tu proyecto abierto, lo prepara si hace falta y genera assets con Tripo y World Labs directamente en el editor.

## 📑 Índice

- [Sobre el proyecto](#about)
- [Estado](#status)
- [Requisitos](#requirements)
- [Cómo ejecutar](#usage)
- [Estructura del proyecto](#structure)
- [Changelog](#changelog)
- [Licencia](#license)

---

<a name="about"></a>

## 📌 Sobre el proyecto

**DBV Unreal AI** (`dbv-unreal-ai`) es una aplicación de escritorio nativa (Windows, macOS y Linux) que conversa contigo y actúa sobre el proyecto de Unreal Engine 5.8 que tengas abierto. Usa la IA que prefieras —un modelo local, una API en la nube o un agente de suscripción— y la conecta con el editor a través del protocolo MCP. Si el proyecto aún no está preparado para recibir órdenes de una IA, la aplicación lo prepara por ti. Además integra servicios de generación 3D como Tripo y World Labs para enviar los resultados directamente a Unreal y mantenerlos accesibles en una biblioteca de assets.

**Construido con:** Rust, Tauri v2, HTML/CSS/JavaScript, Unreal Engine 5.8 (MCP + Python + C++).

---

<a name="status"></a>

## 🚧 Estado

En fase de especificación. Ver [`dbv-specs-ops/docs/SPECIFICATIONS.md`](./dbv-specs-ops/docs/SPECIFICATIONS.md).

---

<a name="requirements"></a>

## 🧰 Requisitos

- [Rust](https://rustup.rs) (stable) y las dependencias de sistema de Tauri v2 (WebView2 en Windows, WebKitGTK en Linux; nada adicional en macOS).
- Node.js 20+ (solo para la CLI de Tauri y los tests).
- Unreal Engine 5.8 con un proyecto abierto en el editor.

---

<a name="usage"></a>

## ▶️ Cómo ejecutar

```bash
npm install
npm run tauri dev
```

Para generar los instaladores:

```bash
npm run tauri build
# Resultado: src-tauri/target/release/bundle/
```

---

<a name="structure"></a>

## 📂 Estructura del proyecto

```text
/
├── src/              # Frontend (HTML/CSS/JS sin bundler)
├── src-tauri/        # Backend nativo en Rust (Tauri v2)
├── tests/            # Tests
├── dbv-specs-ops/    # Especificaciones, arquitectura, task.md, memory.md y CHANGELOG.md
└── README.md         # Este fichero
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
