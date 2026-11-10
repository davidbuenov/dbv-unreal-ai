# Ficha de Microsoft Store — Español (España)

> Copia cada campo tal cual a la sección correspondiente de Partner Center ("Descripción de Store"). Los límites de caracteres indicados son los que muestra el propio formulario.

---

## Descripción *

DBV Unreal AI es un copiloto de inteligencia artificial nativo para Unreal Engine 5.8: ligero, rápido, local por defecto y respetuoso con tu privacidad — sin suscripciones forzosas, sin publicidad y sin telemetría invasiva.

Conecta directamente con tu editor de Unreal Engine en marcha mediante su servidor nativo Model Context Protocol (MCP) HTTP. Permite crear, modificar y analizar escenas 3D mediante lenguaje natural, generar mundos y objetos con IA y ejecutar secuencias de comandos automatizadas de forma reversible y segura.

Todo el flujo de trabajo con el motor es 100% local. El asistente de IA es flexible y opcional: puedes usar modelos locales con Ollama o LM Studio (sin salir de tu máquina), conectar tus suscripciones existentes a través de agentes CLI (Claude Code, Google Antigravity, GitHub Copilot, Codex, Gemini) o usar claves de API en la nube (Anthropic, OpenAI, Google AI Studio, OpenRouter) con control total de costes.

Características principales:
• Comunicación nativa bidireccional con Unreal Engine 5.8 mediante MCP HTTP (puerto 8000)
• Caja negra: graba el juego sin tocar el proyecto, marca momentos con una tecla y analiza con datos para depurar fallos que solo ocurren al jugar
• Vestir MetaHumans desde la app con una herramienta determinista (prendas holgadas, faldas y vestidos con tela; las prendas ceñidas son experimentales)
• Efectos de sonido con un microservicio local, ElevenLabs y Freesound, e importación directa a Unreal
• Asistente de IA con tres modalidades: Locales (offline), Suscripción (agentes CLI) y API Cloud
• Soporte integrado de Google Antigravity CLI, Claude Code, GitHub Copilot, Codex y Gemini CLI
• Generación y colocación de mundos fotorrealistas con World Labs Marble (Gaussian Splatting .spz y colisión .glb)
• Generación, auto-rigging y animación de modelos 3D con Tripo3D v3
• Gaussian Splatting propio en C++, dibujado por la GPU con color según el ángulo: escenas de 10 millones de splats sin plugins comerciales
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

---

## Novedades de esta versión

v0.4.0:
• Generación y búsqueda de efectos de sonido (SFX): nueva pestaña Sonidos en Generar compatible con microservicio local (Stable Audio Open 1.0), ElevenLabs Text-to-SFX y catálogo interactivo de Freesound con preescucha directa y guardado selectivo en biblioteca.
• Importación directa de audio a Unreal Engine («Enviar a Unreal» de Audio) como assets USoundWave / SoundCue y herramienta MCP para asociar audio a actores y eventos 3D.
• Visión multimodal en tiempo real: nuevo botón «📷 Capturar viewport» en la conversación para que los modelos con visión analicen la escena actual del editor.
• Configuración de Copyright personalizada en Ajustes de Proyecto preservando la atribución inmutable de DBV Unreal AI y dbv-specs-ops.
• Autodetección resiliente del puerto MCP del editor: sondeo automático de puertos 8000..8005 al conectar.
• Centro de Ayuda ampliado con secciones para Tripo, World Labs y Sonidos con accesos contextuales directos.

---

## Características del producto (hasta 20)

1. Conexión nativa con Unreal Engine 5.8 vía MCP HTTP
2. Asistente de IA con opciones Locales, Suscripción y API Cloud
3. Compatible con Google Antigravity CLI, Claude Code y Copilot
4. Generación de mundos 3D con Gaussian Splats (.spz)
5. Generación y animación de personajes con Tripo3D
6. Gaussian Splatting propio por GPU, sin plugins comerciales
7. Visor 3D interactivo con iluminación de estudio
8. Preparación automática y reversible de proyectos Unreal
9. Captura del viewport para análisis contextual de escenas
10. Deshacer turnos con un clic desde la interfaz
11. Memoria persistente del proyecto versionada en disco
12. Soporte de múltiples instancias de Unreal en paralelo
13. Centro de Ayuda integrado con atajo rápido F1
14. Atajos globales: F1, Ctrl+Mayús+I, Esc, Ctrl+S
15. 100% privado y local por defecto
16. Interfaz en español, inglés y chino simplificado
17. Pegar o arrastrar imágenes y documentos a la conversación
18. Código abierto bajo licencia MIT
19. La IA edita y compila el código C++ del proyecto
20. Recetas guiadas y guías de Unreal sin conexión

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
- Cuando la IA cambia el código C++ del proyecto y el usuario acepta el cambio, la app compila con las herramientas de Unreal instaladas por el usuario (Live Coding a través del editor, o Unreal Build Tool con el editor cerrado) y vuelve a abrir el editor. Solo se compila el proyecto que el usuario tiene abierto.
- La app guarda su biblioteca de assets en Documentos\DBV Unreal AI\Library y un registro local rotativo, sin claves ni datos personales.
- Las imágenes que el usuario pega (Ctrl+V) o arrastra a la conversación se guardan en la carpeta temporal del sistema (como máximo las 20 últimas) para que la IA pueda usarlas por su ruta; solo llegan a una IA en la nube si el usuario ha conectado una y envía el mensaje.
