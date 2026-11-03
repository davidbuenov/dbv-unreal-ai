// =============================================================================
// DBV Unreal AI — Implementación del Scene Proxy de Gaussian Splatting
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61

#include "DbvGaussianSplatSceneProxy.h"
#include "DbvGaussianSplatComponent.h"

FDbvGaussianSplatSceneProxy::FDbvGaussianSplatSceneProxy(const UDbvGaussianSplatComponent* InComponent, FDbvSplatGpuCloudRef InCloud)
	: FPrimitiveSceneProxy(InComponent)
	, Cloud(MoveTemp(InCloud))
{
}

void FDbvGaussianSplatSceneProxy::CreateRenderThreadResources(FRHICommandListBase& RHICmdList)
{
	DbvSplatRender::Register(this, Cloud);
}

void FDbvGaussianSplatSceneProxy::DestroyRenderThreadResources()
{
	DbvSplatRender::Unregister(this);
}

FPrimitiveViewRelevance FDbvGaussianSplatSceneProxy::GetViewRelevance(const FSceneView* View) const
{
	// No hay mallas que dibujar en las pasadas del motor: la extensión de vista pinta la nube tras el tonemap.
	FPrimitiveViewRelevance Result;
	Result.bDrawRelevance = IsShown(View);
	Result.bShadowRelevance = false;
	return Result;
}
