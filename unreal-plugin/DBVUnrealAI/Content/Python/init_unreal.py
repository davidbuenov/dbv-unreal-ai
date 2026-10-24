# =============================================================================
# DBV Unreal AI — Arranque del toolset en el editor
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Unreal ejecuta este fichero al arrancar el editor porque el plugin está activo.

Igual que ``EditorToolset`` de Epic: registrar el toolset aquí basta para que el
servidor MCP (``ModelContextProtocol``) lo exponga en ``list_toolsets``. Si el proyecto
usa la variante C++ (``DBVUnrealAICpp``), es ella la que registra y aquí no se hace nada,
para no duplicar las herramientas.
"""

import unreal

if hasattr(unreal, "DBVUnrealAICppToolset"):
    unreal.log("DBV Unreal AI: variante C++ activa; el toolset Python no se registra")
else:
    from dbv_unreal_ai import toolset

    if toolset.register():
        unreal.log("DBV Unreal AI: toolset registrado")
    else:
        unreal.log_warning("DBV Unreal AI: ToolsetRegistry no disponible; el toolset no se registró")
