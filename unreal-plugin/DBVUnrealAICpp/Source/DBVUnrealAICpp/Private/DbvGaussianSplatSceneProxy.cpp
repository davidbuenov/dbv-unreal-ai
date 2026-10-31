// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.
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
