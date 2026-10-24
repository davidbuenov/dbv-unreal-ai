// =============================================================================
// DBV Unreal AI — Módulo del toolset C++: registro explícito
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Lección de MCPCLiente (proyecto EventoUnrealMCP): el ToolsetRegistry NO descubre un
// UToolsetDefinition por reflexión. Hay que registrarlo en OnPostEngineInit (en
// StartupModule el subsistema aún no existe), como hace DBVMCPFirst.cpp.

#include "DBVUnrealAICppToolset.h"
#include "Misc/CoreDelegates.h"
#include "Modules/ModuleManager.h"
#include "ToolsetRegistry/UToolsetRegistry.h"

class FDBVUnrealAICppModule : public IModuleInterface
{
public:
	virtual void StartupModule() override
	{
		FCoreDelegates::GetOnPostEngineInit().AddRaw(this, &FDBVUnrealAICppModule::OnPostEngineInit);
	}

	virtual void ShutdownModule() override
	{
		FCoreDelegates::GetOnPostEngineInit().RemoveAll(this);
		if (UToolsetRegistry::IsAvailable())
		{
			UToolsetRegistry::UnregisterToolsetClass(UDBVUnrealAICppToolset::StaticClass());
		}
	}

private:
	void OnPostEngineInit()
	{
		UToolsetRegistry::RegisterToolsetClass(UDBVUnrealAICppToolset::StaticClass());
	}
};

IMPLEMENT_MODULE(FDBVUnrealAICppModule, DBVUnrealAICpp)
