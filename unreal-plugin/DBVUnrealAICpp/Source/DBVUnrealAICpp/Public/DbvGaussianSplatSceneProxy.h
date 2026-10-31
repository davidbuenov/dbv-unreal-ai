// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.
//
// @spec REQ-61
//
// El proxy ya no dibuja: da de alta su nube en el render por GPU (DbvSplatRender) al crear sus recursos y de baja
// al destruirlos. La extensión de vista dibuja todo lo registrado tras el tonemap.

#pragma once

#include "CoreMinimal.h"
#include "PrimitiveSceneProxy.h"
#include "DbvSplatRender.h"

class UDbvGaussianSplatComponent;

class DBVUNREALAICPP_API FDbvGaussianSplatSceneProxy : public FPrimitiveSceneProxy
{
public:
	FDbvGaussianSplatSceneProxy(const UDbvGaussianSplatComponent* InComponent, FDbvSplatGpuCloudRef InCloud);

	//~ Begin FPrimitiveSceneProxy Interface
	virtual void CreateRenderThreadResources(FRHICommandListBase& RHICmdList) override;
	virtual void DestroyRenderThreadResources() override;
	virtual FPrimitiveViewRelevance GetViewRelevance(const FSceneView* View) const override;
	virtual uint32 GetMemoryFootprint() const override { return sizeof(*this) + GetAllocatedSize(); }
	virtual SIZE_T GetTypeHash() const override
	{
		static size_t UniquePointer;
		return reinterpret_cast<SIZE_T>(&UniquePointer);
	}
	//~ End FPrimitiveSceneProxy Interface

private:
	FDbvSplatGpuCloudRef Cloud;
};
