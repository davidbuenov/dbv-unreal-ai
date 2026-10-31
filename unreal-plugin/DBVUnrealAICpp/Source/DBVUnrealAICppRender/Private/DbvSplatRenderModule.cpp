// =============================================================================
// DBV Unreal AI — Módulo del render de splats: shaders y extensión de vista
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61
//
// Carga en PostConfigInit para registrar la carpeta de shaders del plugin antes de que se compilen los globales. La
// extensión de vista necesita GEngine, así que se crea en OnPostEngineInit.

#include "DbvSplatViewExtension.h"
#include "Interfaces/IPluginManager.h"
#include "Misc/CoreDelegates.h"
#include "Misc/Paths.h"
#include "Modules/ModuleManager.h"
#include "ShaderCore.h"

class FDBVUnrealAICppRenderModule : public IModuleInterface
{
public:
	virtual void StartupModule() override
	{
		const TSharedPtr<IPlugin> Plugin = IPluginManager::Get().FindPlugin(TEXT("DBVUnrealAICpp"));
		if (Plugin.IsValid())
		{
			AddShaderSourceDirectoryMapping(TEXT("/Plugin/DBVUnrealAICpp"), FPaths::Combine(Plugin->GetBaseDir(), TEXT("Shaders")));
		}
		FCoreDelegates::GetOnPostEngineInit().AddRaw(this, &FDBVUnrealAICppRenderModule::OnPostEngineInit);
	}

	virtual void ShutdownModule() override
	{
		FCoreDelegates::GetOnPostEngineInit().RemoveAll(this);
		ViewExtension.Reset();
	}

private:
	void OnPostEngineInit()
	{
		ViewExtension = FSceneViewExtensions::NewExtension<FDbvSplatViewExtension>();
	}

	TSharedPtr<FDbvSplatViewExtension, ESPMode::ThreadSafe> ViewExtension;
};

IMPLEMENT_MODULE(FDBVUnrealAICppRenderModule, DBVUnrealAICppRender)
