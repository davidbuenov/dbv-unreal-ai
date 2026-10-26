// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#pragma once

#include "CoreMinimal.h"
#include "Components/MeshComponent.h"
#include "DbvGaussianSplatTypes.h"
#include "DbvGaussianSplatAsset.h"
#include "DbvGaussianSplatComponent.generated.h"

class FDbvGaussianSplatSceneProxy;

/**
 * Primitive component rendering 3D Gaussian Splats with depth buffer writing
 * and camera-facing anisotropic projection.
 */
UCLASS(ClassGroup = (Rendering), meta = (BlueprintSpawnableComponent), Category = "DBV Unreal AI")
class DBVUNREALAICPP_API UDbvGaussianSplatComponent : public UMeshComponent
{
	GENERATED_BODY()

public:
	UDbvGaussianSplatComponent(const FObjectInitializer& ObjectInitializer);

	/** Splat cloud asset containing point data */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat")
	TObjectPtr<UDbvGaussianSplatAsset> SplatAsset;

	/** File path to load automatically at startup (.spz or .ply) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat")
	FString SourceFilePath;

	/** Whether to write into the Scene Depth buffer for mutual occlusion with meshes */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat|Rendering")
	bool bEnableDepthWrite = true;

	/** Alpha threshold above which depth is written to SceneDepth (YaGS lesson: ~0.1f) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat|Rendering", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float AlphaDepthThreshold = 0.1f;

	/** Multiplier for splat size (1.0 = nominal) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat|Rendering", meta = (ClampMin = "0.01", ClampMax = "5.0"))
	float SplatScaleMultiplier = 1.0f;

	/** Automatically configure actor transform for OpenCV / World Labs coordinate system (Roll=90°, Scale=100) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat|Transform")
	bool bAutoApplyOpenCVTransform = true;

	/** Loads an .spz or .ply file directly from disk */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	bool LoadFile(const FString& InFilePath);

	//~ Begin UPrimitiveComponent Interface
	virtual FPrimitiveSceneProxy* CreateSceneProxy() override;
	virtual FBoxSphereBounds CalcBounds(const FTransform& LocalToWorld) const override;
	//~ End UPrimitiveComponent Interface

	//~ Begin UMeshComponent Interface
	virtual int32 GetNumMaterials() const override { return 1; }
	//~ End UMeshComponent Interface

	virtual void OnRegister() override;

private:
	/** Applies the verified World Labs transform (Roll 90, Scale 100) if requested */
	void ApplyOpenCVTransformIfNeeded();
};
