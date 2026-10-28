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
• Centro de Ayuda interactivo (F1) con guías de Unreal Engine 5.8, atajos y catálogo de IAs
• Interfaz cuidada con diseño cálido, modo oscuro y temas de alto contraste
• Totalmente privado: tus proyectos y niveles no salen de tu equipo
• Licencia de código abierto MIT

---

## Novedades de esta versión

v0.2.1:
• Clasificación tripartita clara en la conexión a IAs: IAs Locales (100% offline), IAs por Suscripción (Agentes CLI) e IAs en la nube con clave de API, con distintivos de estado visuales y reactivos en vivo.
• Soporte e integración formal de Google Antigravity CLI (agy) para suscripciones de Google One y Gemini Advanced, conviviendo con Gemini CLI para acceso directo por API.
• Nuevo Centro de Ayuda integrado accesible con el botón (?) en cabecera y el atajo global F1, con guías de Unreal Engine 5.8, catálogo comparativo de IAs y atajos.
• Modal «Acerca de» renovado con diseño DBV Typst Editor, ficha técnica del entorno y enlaces oficiales.
• Preparación integral de empaquetado MSIX para distribución en Microsoft Store.

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
