# Respuestas grabadas del editor real (UEFN 43.10)

Grabadas el 2026-11-20 en el spike P-14 contra `DBVUnrealAITest`, un proyecto UEFN 43.10 vacío. El servidor MCP del
editor estaba en `127.0.0.1:8000/mcp`. Son la entrada de los tests de REQ-122, REQ-123, REQ-126, REQ-127 y REQ-128.

- `initialize.response.json`: **`serverInfo` llega vacío** (`name`, `title` y `version` en blanco). Lo que distingue a
  UEFN son las `instructions` (mencionan UEFN, Fortnite y LUF) y los toolsets `Valkyrie*`.
- `tools_list.response.json`: solo las 3 metaherramientas (`list_toolsets`, `describe_toolset`, `call_tool`), como en UE
  5.8. `describe_toolset` sigue en singular, no como en 5.9. Las herramientas se llaman por su nombre corto
  (`BuildAll`), no por el completo (`VerseToolset.VerseToolset.BuildAll`).
- `list_toolsets.response.json`: los 63 toolsets.
- `describe_toolset.*.response.json`: Verse, Session, Device, Entity, Level, StaticMesh (importa FBX, OBJ y glTF),
  SoundWave (wav, ogg, mp3…), Logs, RevisionControl, AIAssistant, PythonScripting y AgentSkill.
- `call_tool.VerseToolset.BuildAll.disk-file-with-error.response.json`: un `.verse` escrito en disco con el editor
  abierto entra en `BuildAll`. El error trae `code`, `message`, `filePath` y `span`, con líneas contadas desde 0.
- `call_tool.VerseToolset.BuildAll.ok.response.json`: compilación correcta (`returnValue: []`).
- `call_tool.DeviceToolset.*`: el dispositivo compilado aparece en el catálogo como `/<módulo>/_Verse.<clase>`, y
  `PlaceDevice` lo coloca en el nivel.
- `call_tool.SessionToolset.*`: `StartSession` (~2 min, no responde hasta que acaba), `PushChanges` con `bVerseOnly`
  (~15 s, vuelve a ejecutar `OnBegin`), `StopGame`/`StartGame`, `GetVerseOutput` (con cursor, fichero y línea) y
  `StopSession`.
- `spike.modal-dialog.json`: un diálogo abierto durante una llamada MCP no bloquea nada, porque UEFN lo contesta solo
  con la respuesta por defecto. Solo queda constancia en el log (`Message dialog closed, result: …`).

Las rutas `C:/Users/<usuario>/` sustituyen a la carpeta del usuario. No contienen claves ni datos personales.
