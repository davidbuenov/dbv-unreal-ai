// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#pragma once

#include "CoreMinimal.h"
#include "UObject/NoExportTypes.h"
#include "DbvGaussianSplatTypes.h"
#include "DbvGaussianSplatAsset.generated.h"

/**
 * Unreal Engine asset holding 3D Gaussian Splatting point cloud data.
 */
UCLASS(BlueprintType)
class DBVUNREALAICPP_API UDbvGaussianSplatAsset : public UObject
{
	GENERATED_BODY()

public:
	UDbvGaussianSplatAsset();

	/** File path of the imported source (.spz or .ply) */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Gaussian Splat")
	FString SourceFilePath;

	/** Splat header metadata */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Gaussian Splat")
	FDbvSplatHeader Header;

	/** Bounding box in local coordinate space */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Gaussian Splat")
	FBox BoundingBox = FBox(EForceInit::ForceInit);

	/** Most splats kept for the GPU renderer (REQ-61). S63 replaces it with the VRAM budget (D36). */
	static constexpr int32 MaxDrawnSplats = 16 * 1024 * 1024;

	/** Number of Gaussian splats kept for drawing (at most MaxDrawnSplats) */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	int32 GetNumSplats() const { return Points.Num(); }

	/** Number of Gaussian splats in the source file */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	int32 GetTotalSplats() const { return Header.NumPoints; }

	/** Loads an .spz file into this asset */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	bool LoadSpzFile(const FString& FilePath, FString& OutErrorMessage);

	/** Loads a .ply file into this asset */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	bool LoadPlyFile(const FString& FilePath, FString& OutErrorMessage);

	/** Access to the raw splat points */
	const TArray<FDbvGaussianPoint>& GetPoints() const { return Points; }

private:
	/** Above MaxDrawnSplats, keeps the most visible splats of the whole scene (not the first ones of the file). */
	void KeepMostVisible();

	/** Internal storage of splat points */
	TArray<FDbvGaussianPoint> Points;
};
