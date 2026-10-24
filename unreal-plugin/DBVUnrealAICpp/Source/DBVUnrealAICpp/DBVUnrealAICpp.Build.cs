// =============================================================================
// DBV Unreal AI — Reglas de compilación del toolset C++
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

using UnrealBuildTool;

public class DBVUnrealAICpp : ModuleRules
{
	public DBVUnrealAICpp(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
		PublicDependencyModuleNames.AddRange(new string[] { "Core", "CoreUObject", "Engine", "ToolsetRegistry" });
		PrivateDependencyModuleNames.AddRange(new string[] { "PythonScriptPlugin" });
	}
}
