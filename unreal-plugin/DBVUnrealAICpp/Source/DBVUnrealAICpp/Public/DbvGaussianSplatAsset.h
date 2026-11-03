// =============================================================================
// DBV Unreal AI — Asset empaquetado de nube de Gaussian Splats
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61, REQ-62, REQ-63

#pragma once

#include "CoreMinimal.h"
#include "UObject/NoExportTypes.h"
#include "DbvGaussianSplatTypes.h"
#include "DbvSplatRender.h"
#include "DbvGaussianSplatAsset.generated.h"

/**
 * A loaded Gaussian Splatting cloud, already packed for the GPU (D37). The component moves the data to the GPU
 * and keeps only the counts here. Above the VRAM budget (D36) it drops the harmonics first and then keeps the most
 * visible splats; the points are never serialized (the component reloads them from the file).
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

	/** Number of Gaussian splats kept for drawing */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	int32 GetNumSplats() const { return NumKept; }

	/** Number of Gaussian splats in the source file */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	int32 GetTotalSplats() const { return Header.NumPoints; }

	/** Degree of the spherical harmonics in the source file */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	int32 GetFileSHDegree() const { return Header.SHDegree; }

	/** Degree of the spherical harmonics drawn (0 = base colour only) */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	int32 GetSHDegree() const { return SHDegreeKept; }

	/** Loads an .spz file into this asset */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	bool LoadSpzFile(const FString& FilePath, FString& OutErrorMessage);

	/** Loads a .ply file into this asset */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	bool LoadPlyFile(const FString& FilePath, FString& OutErrorMessage);

	/** Hands the packed splats over (to the GPU); the counts stay. */
	FDbvSplatGpuData TakeData() { return MoveTemp(Data); }

private:
	/** Common end of a successful load: selection of the most visible splats if needed. */
	void FinishLoad();

	FDbvSplatGpuData Data;
	int32 NumKept = 0;
	int32 SHDegreeKept = 0;
};
