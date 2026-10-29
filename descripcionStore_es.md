# Ficha de Microsoft Store — Español (España)

> Copia cada campo tal cual a la sección correspondiente de Partner Center ("Descripción de Store"). Los límites de caracteres indicados son los que muestra el propio formulario.

---

## Descripción *

DBV Unreal AI es un copiloto de inteligencia artificial nativo para Unreal Engine 5.8: ligero, rápido, local por defecto y respetuoso con tu privacidad — sin suscripciones forzosas, sin publicidad y sin telemetría invasiva.

Conecta directamente con tu editor de Unreal Engine en marcha mediante su servidor nativo Model Context Protocol (MCP) HTTP. Permite crear, modificar y analizar escenas 3D mediante lenguaje natural, generar mundos y objetos con IA y ejecutar secuencias de comandos automatizadas de forma reversible y segura.

Todo el flujo de trabajo con el motor es 100% local. El asistente de IA es flexible y opcional: puedes usar modelos locales con Ollama o LM Studio (sin salir de tu máquina), conectar tus suscripciones existentes a través de agentes CLI (Claude Code, Google Antigravity, GitHub Copilot, Codex, Gemini) o usar claves de API en la nube (Anthropic, OpenAI, Google AI Studio, OpenRouter) con control total de costes.

Características principales:
• Comunicación nativa bidireccional con Unreal Engine 5.8 mediante MCP HTTP (puerto 8000)
• Asistente de IA con tres modalidades: Locales (offline), Suscripción (agentes CLI) y API Cloud
• Soporte integrado de Google Antigravity CLI, Claude Code, GitHub Copilot, Codex y Gemini CLI
• Generación y colocación de mundos fotorrealistas con World Labs Marble (Gaussian Splatting .spz y colisión .glb)
• Generación, auto-rigging y animación de modelos 3D con Tripo3D v3
• Componente nativo libre en C++ de Gaussian Splatting (UDbvGaussianSplatComponent) sin dependencias comerciales
• Previsualización interactiva en 3D con Three.js e iluminación de estudio
• Preparación automática de proyectos de Unreal Engine con copias de seguridad reversibles
• Inspección y captura del viewport en vivo para comprensión contextual visual
• Deshacer seguro de turnos y acciones con un solo clic en Unreal Engine
• Sistema de Memoria y Plan del Proyecto independiente y versionable en disco
• Detección automática de múltiples instancias y proyectos de Unreal Engine en paralelo
• Centro de Ayuda interactivo (F1) con guías de Unreal Engine 5.8, atajos y la guía completa para conectar cada IA, disponible sin conexión
• Interfaz cuidada con diseño cálido, modo oscuro y temas de alto contraste
• Totalmente privado: tus proyectos y niveles no salen de tu equipo
• Licencia de código abierto MIT

---

## Novedades de esta versión

v0.2.3:
• Todos los agentes por suscripción funcionan: Google Antigravity (con su servidor oficial agy_acp_server e inicio de sesión con tu cuenta de Google), Codex, GitHub Copilot y Claude Code.
• Inicio de sesión guiado: si el agente lo pide, la app lo solicita con tu cuenta y te avisa de que puede abrirse el navegador.
• Guía completa para conectar cada IA dentro de la app (F1 → Guía de IAs y desde «Conectar una IA»), sin conexión y con botón Copiar para cada orden de instalación.
• Si un agente está instalado pero le falta algo (el servidor de Antigravity o la CLI de Copilot), la app lo dice y te lleva a las instrucciones.
• Panel del asistente más claro: las etiquetas de contexto ya no se cortan.

---

## Características del producto (hasta 20)

1. Conexión nativa con Unreal Engine 5.8 vía MCP HTTP
2. Asistente de IA con opciones Locales, Suscripción y API Cloud
3. Compatible con Google Antigravity CLI, Claude Code y Copilot
4. Generación de mundos 3D con Gaussian Splats (.spz)
5. Generación y animación de personajes con Tripo3D
6. Componente libre de Gaussian Splatting en C++
7. Visor 3D interactivo con iluminación de estudio
8. Preparación automática y reversible de proyectos Unreal
9. Captura del viewport para análisis contextual de escenas
10. Deshacer turnos con un clic desde la interfaz
11. Memoria persistente del proyecto versionada en disco
12. Soporte de múltiples instancias de Unreal en paralelo
13. Centro de Ayuda integrado con atajo rápido F1
14. Atajos globales: F1, Ctrl+Mayús+I, Esc, Ctrl+S
15. 100% privado y local por defecto
16. Interfaz bilingüe (español e inglés)
17. Sin telemetría oculta ni recopilación de datos
18. Código abierto bajo licencia MIT

---

## Términos de búsqueda / Palabras clave (hasta 7)

1. Unreal Engine
2. Unreal Engine 5
3. AI Copilot
4. 3D Generative AI
5. Model Context Protocol
6. Gaussian Splatting
7. Game Development

---

## Descripción corta (hasta 270 caracteres) *

Copiloto nativo de inteligencia artificial para Unreal Engine 5.8. Conecta vía MCP HTTP, asiste en la creación y modificación de escenas 3D, genera mundos fotorrealistas y personajes animados, respetando tu privacidad y funcionando de forma local.

---

## Notas para la certificación

La aplicación se comunica de forma local con el editor de Unreal Engine 5.8 a través de un servidor HTTP Model Context Protocol en localhost (puerto 8000).

Las funciones de Inteligencia Artificial son 100% opcionales y están desactivadas por defecto:
- La app funciona plenamente sin IA configurada (preparación de proyectos, inspección, visualización 3D y biblioteca de assets).
- Si el usuario decide conectar una IA, puede elegir entre IAs 100% locales (Ollama o LM Studio sin salida a red), agentes CLI con sus propias credenciales, o modelos en la nube mediante su propia clave privada custodiada en el sistema.
- Ningún dato ni proyecto sale del equipo sin la acción y consentimiento explícito del usuario.
- Los agentes por suscripción (Claude Code, Codex, Gemini CLI, Google Antigravity, GitHub Copilot CLI) son programas aparte que el usuario instala y en los que inicia sesión por su cuenta; la app solo los lanza, con la carpeta del proyecto como directorio de trabajo, cuando el usuario conecta uno. Si un agente pide iniciar sesión, puede abrir el navegador del sistema para que el usuario entre en ese servicio.
