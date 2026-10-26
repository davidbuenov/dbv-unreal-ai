// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#pragma once

#include "CoreMinimal.h"
#include "PrimitiveSceneProxy.h"
#include "DbvGaussianSplatTypes.h"

class UDbvGaussianSplatComponent;

/**
 * Render-thread scene proxy for 3D Gaussian Splats.
 * Implements camera-facing billboard generation, view-space depth sorting (argsort),
 * and depth-buffer writing based on the YaGS architecture lessons.
 */
class DBVUNREALAICPP_API FDbvGaussianSplatSceneProxy : public FPrimitiveSceneProxy
{
public:
	FDbvGaussianSplatSceneProxy(const UDbvGaussianSplatComponent* InComponent);
	virtual ~FDbvGaussianSplatSceneProxy() override;

	//~ Begin FPrimitiveSceneProxy Interface
	virtual void GetDynamicMeshElements(
		const TArray<const FSceneView*>& Views,
		const FSceneViewFamily& ViewFamily,
		uint32 VisibilityMap,
		FMeshElementCollector& Collector
	) const override;

	virtual FPrimitiveViewRelevance GetViewRelevance(const FSceneView* View) const override;
	virtual uint32 GetMemoryFootprint() const override;
	virtual SIZE_T GetTypeHash() const override
	{
		static size_t UniquePointer;
		return reinterpret_cast<SIZE_T>(&UniquePointer);
	}
	//~ End FPrimitiveSceneProxy Interface

private:
	/** Splats snapshot copied for render thread */
	TArray<FDbvGaussianPoint> Splats;

	/** Splat rendering settings */
	bool bEnableDepthWrite = true;
	float AlphaDepthThreshold = 0.1f;
	float SplatScaleMultiplier = 1.0f;

	/** Material interface used to draw the splat quads */
	UMaterialInterface* SplatMaterial = nullptr;
};
