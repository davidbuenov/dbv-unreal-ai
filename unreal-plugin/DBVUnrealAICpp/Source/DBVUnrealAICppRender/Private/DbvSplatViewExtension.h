// =============================================================================
// DBV Unreal AI — Extensión de vista del render de splats
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61

#pragma once

#include "CoreMinimal.h"
#include "SceneViewExtension.h"

struct FScreenPassTexture;
struct FPostProcessMaterialInputs;

/** Se suscribe a la pasada de tonemap y dibuja encima los splats registrados en la escena de la vista (D33). */
class FDbvSplatViewExtension : public FSceneViewExtensionBase
{
public:
	FDbvSplatViewExtension(const FAutoRegister& AutoRegister) : FSceneViewExtensionBase(AutoRegister) {}

	virtual void SetupViewFamily(FSceneViewFamily& InViewFamily) override {}
	virtual void SetupView(FSceneViewFamily& InViewFamily, FSceneView& InView) override {}
	virtual void BeginRenderViewFamily(FSceneViewFamily& InViewFamily) override {}
	virtual void SubscribeToPostProcessingPass(EPostProcessingPass Pass, const FSceneView& View, FPostProcessingPassDelegateArray& InOutPassCallbacks, bool bIsPassEnabled) override;

private:
	FScreenPassTexture PostTonemap(FRDGBuilder& GraphBuilder, const FSceneView& View, const FPostProcessMaterialInputs& Inputs);
};
