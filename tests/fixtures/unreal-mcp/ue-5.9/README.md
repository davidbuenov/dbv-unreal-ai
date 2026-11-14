# Respuestas grabadas del editor real (UE 5.9 MegaJam Preview)

Grabadas el 2026-11-13 contra `PruebaUnreal59` (UE 5.9.0-60351404, proyecto C++ creado en 5.9, toolset C++ DBV) con el
servidor MCP del editor en `127.0.0.1:8000/mcp`. Son la entrada de los tests de compatibilidad con 5.9 (REQ-109,
REQ-112, REQ-116).

- `list_toolsets.response.json`: los toolsets del proyecto (`AllToolsets`, `AllMetaHumanToolsets`, el nuestro).
- `describe_toolsets.EditorToolset.EditorAppToolset.response.json`: en 5.9 `describe_toolset` (singular) ya no existe;
  `describe_toolsets` recibe una lista y responde `{"toolsets": {"<nombre>": {...}}}`, con `annotations.accessMode`
  (`read`, `write`, `execute`) en cada herramienta. Ya no tiene `CaptureViewport`.
- `call_tool.SceneCaptureToolset.CaptureEditorViewport.response.json`: la captura de 5.9, con rejilla y etiquetas. La
  imagen llega como bloque de contenido `image` y `returnValue.image` es una referencia `urn:toolresult:`. **El PNG se ha
  sustituido por uno de 1×1** para que el fichero no pese 900 KB.

No contienen claves ni datos personales.
