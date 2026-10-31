// =============================================================================
// DBV Unreal AI — Reglas de compilación del render de splats por GPU
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61
//
// Módulo aparte porque los shaders de un plugin se registran en PostConfigInit, y el módulo principal depende de
// Python y del ToolsetRegistry, que no deben cargarse tan pronto.

using UnrealBuildTool;

public class DBVUnrealAICppRender : ModuleRules
{
	public DBVUnrealAICppRender(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
		PublicDependencyModuleNames.AddRange(new string[] {
			"Core",
			"CoreUObject",
			"Engine",
			"RenderCore",
			"RHI"
		});
		PrivateDependencyModuleNames.AddRange(new string[] {
			"Renderer",
			"Projects"
		});
	}
}
