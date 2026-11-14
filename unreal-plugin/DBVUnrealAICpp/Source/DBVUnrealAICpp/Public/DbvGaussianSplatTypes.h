// =============================================================================
// DBV Unreal AI — Tipos comunes de datos para 3D Gaussian Splatting
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

#pragma once

#include "CoreMinimal.h"
#include "DbvGaussianSplatTypes.generated.h"

/**
 * Header data read from .spz / .ply files
 */
USTRUCT(BlueprintType)
struct DBVUNREALAICPP_API FDbvSplatHeader {
  GENERATED_BODY()

  UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Gaussian Splat")
  int32 NumPoints = 0;

  UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Gaussian Splat")
  int32 SHDegree = 0;

  UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Gaussian Splat")
  int32 FractionalBits = 0;

  UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Gaussian Splat")
  FBox BoundingBox = FBox(EForceInit::ForceInit);
};

/**
 * Coordinate space conventions and conversion utilities
 */
namespace DbvGaussianSplatCoords {
/** Transform that places World Labs / OpenCV .spz in Unreal Engine: together
 * they give ConvertOpenCVPointToUnreal, (z, x, -y) * 100.
 * OpenCV is right-handed and Unreal left-handed, so a rotation alone cannot do
 * it: one axis has to be negated. The old Roll = 90 with a positive scale showed
 * every world mirrored left to right (seen on 2026-11-14 against the World Labs
 * thumbnail and Epic's GaussianSplat plugin). @spec REQ-18, REQ-114 */
static const FRotator OpenCVToUnrealRotation =
    FRotator(0.0f, -90.0f, 90.0f); // Pitch 0, Yaw -90, Roll 90
static const FVector OpenCVToUnrealScale =
    FVector(-100.0, 100.0, 100.0); // 1m = 100cm, X negated (handedness)

/** Direct vertex coordinate swap if baked into vertices instead of Actor
 * transform */
FORCEINLINE FVector3f ConvertOpenCVPointToUnreal(const FVector3f &InCV) {
  // OpenCV: +X right, +Y down, +Z forward
  // Unreal: +X forward, +Y right, +Z up
  return FVector3f(InCV.Z * 100.0f, InCV.X * 100.0f, -InCV.Y * 100.0f);
}
} // namespace DbvGaussianSplatCoords
