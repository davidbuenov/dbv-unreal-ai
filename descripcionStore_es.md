# Ficha de Microsoft Store — Español (España)

> Cada campo de Partner Center («Descripción de Store») está en su propio bloque: en DBV MD Reader, pulsa **Copiar** en el bloque y pégalo en su casilla. Las características y las palabras clave van una por bloque porque en el formulario cada una es una casilla. Entre paréntesis, los caracteres del texto y el límite del formulario.

---

## Descripción *

(3728 / 10000)

```text
DBV Unreal AI es un copiloto de inteligencia artificial nativo para Unreal Engine 5.8 y 5.9: ligero, rápido, local por defecto y respetuoso con tu privacidad — sin suscripciones forzosas, sin publicidad y sin telemetría invasiva.

Conecta directamente con tu editor de Unreal Engine en marcha mediante su servidor nativo Model Context Protocol (MCP) HTTP. Permite crear, modificar y analizar escenas 3D mediante lenguaje natural, generar mundos y objetos con IA y ejecutar secuencias de comandos automatizadas de forma reversible y segura.

Todo el flujo de trabajo con el motor es 100% local. El asistente de IA es flexible y opcional: puedes usar modelos locales con Ollama o LM Studio (sin salir de tu máquina), conectar tus suscripciones existentes a través de agentes CLI (Claude Code, Google Antigravity, GitHub Copilot, Codex, Gemini, Cursor, OpenCode) o usar claves de API en la nube (Anthropic, OpenAI, Google AI Studio, OpenRouter) con control total de costes.

Características principales:
• Comunicación nativa bidireccional con Unreal Engine 5.8 y 5.9 mediante MCP HTTP (puerto 8000)
• Contenido de Fab en Unreal Engine 5.8.4 y 5.9: busca y mira cada producto con su miniatura y su enlace antes de instalarlo; solo lo gratuito o lo que ya tienes y compatible con tu editor, Quixel Megascans incluidos
• Caja negra: graba el juego sin tocar el proyecto, marca momentos con una tecla y analiza con datos para depurar fallos que solo ocurren al jugar
• Vestir MetaHumans desde la app con una herramienta determinista: vestidos con tela (Chaos Cloth) ajustados al cuerpo, camisetas y pantalones ajustados conservando su material, y prendas guardadas en la Biblioteca para reutilizarlas
• Efectos de sonido con un microservicio local, ElevenLabs y Freesound, e importación directa a Unreal
• Asistente de IA con tres modalidades: Locales (offline), Suscripción (agentes CLI) y API Cloud
• Soporte integrado de Google Antigravity CLI, Claude Code, GitHub Copilot, Codex, Gemini CLI, Cursor y OpenCode
• Niveles de autonomía y aviso cuando la IA espera tu aprobación
• Generación y colocación de mundos fotorrealistas con World Labs Marble (Gaussian Splatting .spz y colisión .glb)
• Generación, auto-rigging y animación de modelos 3D con Tripo3D v3
• Gaussian Splatting propio en C++, dibujado por la GPU con color según el ángulo: escenas de 10 millones de splats sin plugins comerciales (en la 5.9, también con el plugin GaussianSplat de Epic)
• La IA edita el código C++ del proyecto (siempre como cambios que revisas) y la app lo compila con Live Coding o Unreal Build Tool
• Recetas guiadas paso a paso con comprobación final en el editor y búsqueda en las guías de Unreal sin conexión
• Personajes conversacionales de Convai y zonas con nombre en el nivel
• Previsualización interactiva en 3D con Three.js e iluminación de estudio
• Preparación automática de proyectos de Unreal Engine con copias de seguridad reversibles
• Inspección y captura del viewport en vivo para comprensión contextual visual
• Pega (Ctrl+V) o arrastra imágenes y documentos a la conversación, con miniatura, para que la IA los vea y los use con Tripo
• Deshacer seguro de turnos y acciones con un solo clic en Unreal Engine
• Sistema de Memoria y Plan del Proyecto independiente y versionable en disco
• Detección automática de múltiples instancias y proyectos de Unreal Engine en paralelo
• Centro de Ayuda interactivo (F1) con guías de Unreal Engine 5.8, atajos y la guía completa para conectar cada IA, disponible sin conexión
• Interfaz en español, inglés y chino simplificado, con diseño cálido, modo oscuro y temas de alto contraste
• Totalmente privado: tus proyectos y niveles no salen de tu equipo
• Licencia de código abierto MIT
```

---

## Novedades de esta versión

(1029 / 1500)

```text
v0.6.2:
• Corregido: Claude Code y Codex no arrancaban si Node.js estaba instalado en su carpeta por defecto (C:\Program Files\nodejs). Si veías «"C:\Program" no se reconoce como un comando interno o externo», ya está resuelto.

v0.6.1:
• Fab en Unreal Engine 5.8.4: pide «quiero un coche» y verás cada producto con su miniatura y su enlace a fab.com antes de instalarlo, en la conversación o en la nueva vista Fab.
• Solo se instala lo gratuito o lo que ya tienes en tu biblioteca de Fab (nada se compra desde la app) y solo lo compatible con tu versión del editor, Quixel Megascans incluidos. Lo instalado queda como referencia en la Biblioteca.
• La app encuentra el motor aunque el proyecto use un GUID o un motor registrado, y si no lo encuentra te deja elegir su carpeta.
• El plugin C++ viene ya compilado para Unreal 5.8 y la 5.9 Preview: Fab y los splats funcionan sin Visual Studio.
• La vista Proyecto explica cada paso con su solución y tiene «Copiar diagnóstico» para pedir ayuda, sin claves ni tu nombre de usuario.
```

---

## Características del producto (hasta 20)

**1.** (56 / 200)

```text
Conexión nativa con Unreal Engine 5.8 y 5.9 vía MCP HTTP
```

**2.** (61 / 200)

```text
Asistente de IA con opciones Locales, Suscripción y API Cloud
```

**3.** (67 / 200)

```text
Compatible con Claude Code, Antigravity, Copilot, Cursor y OpenCode
```

**4.** (50 / 200)

```text
Generación de mundos 3D con Gaussian Splats (.spz)
```

**5.** (48 / 200)

```text
Generación y animación de personajes con Tripo3D
```

**6.** (58 / 200)

```text
Gaussian Splatting propio por GPU, sin plugins comerciales
```

**7.** (47 / 200)

```text
Visor 3D interactivo con iluminación de estudio
```

**8.** (55 / 200)

```text
Preparación automática y reversible de proyectos Unreal
```

**9.** (56 / 200)

```text
Captura del viewport para análisis contextual de escenas
```

**10.** (45 / 200)

```text
Deshacer turnos con un clic desde la interfaz
```

**11.** (52 / 200)

```text
Memoria persistente del proyecto versionada en disco
```

**12.** (53 / 200)

```text
Soporte de múltiples instancias de Unreal en paralelo
```

**13.** (45 / 200)

```text
Centro de Ayuda integrado con atajo rápido F1
```

**14.** (46 / 200)

```text
Atajos globales: F1, Ctrl+Mayús+I, Esc, Ctrl+S
```

**15.** (59 / 200)

```text
Contenido de Fab con miniatura y enlace, solo gratis o tuyo
```

**16.** (48 / 200)

```text
Interfaz en español, inglés y chino simplificado
```

**17.** (57 / 200)

```text
Pegar o arrastrar imágenes y documentos a la conversación
```

**18.** (32 / 200)

```text
Código abierto bajo licencia MIT
```

**19.** (48 / 200)

```text
La IA edita y compila el código C++ del proyecto
```

**20.** (46 / 200)

```text
Recetas guiadas y guías de Unreal sin conexión
```

---

## Términos de búsqueda / Palabras clave (hasta 7)

**1.** (13 / 40)

```text
Unreal Engine
```

**2.** (15 / 40)

```text
Unreal Engine 5
```

**3.** (10 / 40)

```text
AI Copilot
```

**4.** (16 / 40)

```text
3D Generative AI
```

**5.** (22 / 40)

```text
Model Context Protocol
```

**6.** (18 / 40)

```text
Gaussian Splatting
```

**7.** (16 / 40)

```text
Game Development
```

---

## Descripción corta (hasta 270 caracteres) *

(253 / 270)

```text
Copiloto nativo de inteligencia artificial para Unreal Engine 5.8 y 5.9. Conecta vía MCP HTTP, asiste en la creación y modificación de escenas 3D, genera mundos fotorrealistas y personajes animados, respetando tu privacidad y funcionando de forma local.
```

---

## Notas para la certificación

(3630)

```text
La aplicación se comunica de forma local con el editor de Unreal Engine 5.8 o 5.9 a través de un servidor HTTP Model Context Protocol en localhost (puerto 8000).

Las funciones de Inteligencia Artificial son 100% opcionales y están desactivadas por defecto:
- La app funciona plenamente sin IA configurada (preparación de proyectos, inspección, visualización 3D y biblioteca de assets).
- Si el usuario decide conectar una IA, puede elegir entre IAs 100% locales (Ollama o LM Studio sin salida a red), agentes CLI con sus propias credenciales, o modelos en la nube mediante su propia clave privada custodiada en el sistema.
- Ningún dato ni proyecto sale del equipo sin la acción y consentimiento explícito del usuario.
- Los agentes por suscripción (Claude Code, Codex, Gemini CLI, Google Antigravity, GitHub Copilot CLI, Cursor, OpenCode) son programas aparte que el usuario instala y en los que inicia sesión por su cuenta; la app solo los lanza, con la carpeta del proyecto como directorio de trabajo, cuando el usuario conecta uno. Si un agente pide iniciar sesión, puede abrir el navegador del sistema para que el usuario entre en ese servicio.
- Cuando la IA cambia el código C++ del proyecto y el usuario acepta el cambio, la app compila con las herramientas de Unreal instaladas por el usuario (Live Coding a través del editor, o Unreal Build Tool con el editor cerrado) y vuelve a abrir el editor. Solo se compila el proyecto que el usuario tiene abierto.
- La app guarda su biblioteca de assets en Documentos\DBV Unreal AI\Library y un registro local rotativo, sin claves ni datos personales.
- Las imágenes que el usuario pega (Ctrl+V) o arrastra a la conversación se guardan en la carpeta temporal del sistema (como máximo las 20 últimas) para que la IA pueda usarlas por su ruta; solo llegan a una IA en la nube si el usuario ha conectado una y envía el mensaje.
- La «Caja negra» (opcional) solo lee el juego que el usuario ejecuta en su editor y guarda sus grabaciones en la carpeta del proyecto de Unreal del usuario (Saved\DBVUnrealAI\Sessions, con un máximo de 20 sesiones); no envía nada a la red. Mientras graba, desactiva en memoria la opción del editor «Usar menos CPU en segundo plano» y la restaura al terminar.
- Vestir un MetaHuman (opcional) crea los assets de la prenda dentro del proyecto de Unreal del usuario (Content/DBV/MetaHumanOutfits) y una copia del Blueprint del personaje; el original no se modifica. Todo el cálculo es local, sin red.
- Con Unreal Engine 5.9, la app puede usar las herramientas que publica el propio editor, incluidas las de Unreal Canvas si el usuario lo tiene activo; no genera contenido a través de Canvas.
- Contenido de Fab (opcional): la app usa la sesión de Fab que el usuario abre en su propio editor con el plugin oficial de Epic (botón Fab del Content Browser); el token de esa sesión no sale del editor. Solo descarga a su proyecto de Unreal contenido gratuito o que el usuario ya tiene en su biblioteca de Fab; nunca compra nada ni acepta licencias por él. Las miniaturas se cargan de media.fab.com y se guarda una por producto instalado en la biblioteca de la app.
- Para encontrar el motor de Unreal, la app solo lee (no escribe) la clave del registro HKCU\Software\Epic Games\Unreal Engine\Builds. Si el usuario elige a mano la carpeta del motor, se guarda en Documentos\DBV Unreal AI\engine-overrides.json.
- «Preparar proyecto», cuando el usuario lo acepta, copia en la carpeta Plugins del proyecto de Unreal del usuario el plugin de la app, que incluye dos DLL ya compiladas para su versión del editor (sin ellas habría que compilarlo con Visual Studio).
```
