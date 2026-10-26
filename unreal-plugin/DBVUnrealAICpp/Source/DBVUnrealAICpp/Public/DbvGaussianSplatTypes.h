// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#pragma once

#include "CoreMinimal.h"
#include "DbvGaussianSplatTypes.generated.h"

/**
 * Packed representation of a single 3D Gaussian Splat point in CPU memory.
 */
USTRUCT(BlueprintType)
struct DBVUNREALAICPP_API FDbvGaussianPoint
{
	GENERATED_BODY()

	/** Center position in local space (cm in Unreal, or raw units) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat")
	FVector3f Position = FVector3f::ZeroVector;

	/** Scale factors along principal axes */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat")
	FVector3f Scale = FVector3f::OneVector;

	/** Orientation quaternion (W, X, Y, Z) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat")
	FVector4f Rotation = FVector4f(1.0f, 0.0f, 0.0f, 0.0f);

	/** Base diffuse color (SH degree 0, converted from RGB/DC coefficients) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat")
	FLinearColor Color = FLinearColor::White;

	/** Splat opacity in [0.0, 1.0] */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gaussian Splat")
	float Opacity = 1.0f;

	/** Optional higher-order spherical harmonics coefficients (quantized or raw) */
	TArray<float> HigherOrderSH;

	/** Computes the 3D Covariance Matrix Sigma = R * S * S^T * R^T */
	void ComputeCovariance3D(float OutCov[6]) const
	{
		// Quaternion to 3x3 rotation matrix
		const float W = Rotation.X;
		const float X = Rotation.Y;
		const float Y = Rotation.Z;
		const float Z = Rotation.W;

		const float R00 = 1.0f - 2.0f * (Y * Y + Z * Z);
		const float R01 = 2.0f * (X * Y - W * Z);
		const float R02 = 2.0f * (X * Z + W * Y);

		const float R10 = 2.0f * (X * Y + W * Z);
		const float R11 = 1.0f - 2.0f * (X * X + Z * Z);
		const float R12 = 2.0f * (Y * Z - W * X);

		const float R20 = 2.0f * (X * Z - W * Y);
		const float R21 = 2.0f * (Y * Z + W * X);
		const float R22 = 1.0f - 2.0f * (X * X + Y * Y);

		// Diagonal scale matrix squared
		const float S0 = Scale.X * Scale.X;
		const float S1 = Scale.Y * Scale.Y;
		const float S2 = Scale.Z * Scale.Z;

		// Sigma = R * S^2 * R^T
		// Symmetric matrix stored as 6 floats: (0,0), (0,1), (0,2), (1,1), (1,2), (2,2)
		OutCov[0] = R00 * R00 * S0 + R01 * R01 * S1 + R02 * R02 * S2;
		OutCov[1] = R00 * R10 * S0 + R01 * R11 * S1 + R02 * R12 * S2;
		OutCov[2] = R00 * R20 * S0 + R01 * R21 * S1 + R02 * R22 * S2;
		OutCov[3] = R10 * R10 * S0 + R11 * R11 * S1 + R12 * R12 * S2;
		OutCov[4] = R10 * R20 * S0 + R11 * R21 * S1 + R12 * R22 * S2;
		OutCov[5] = R20 * R20 * S0 + R21 * R21 * S1 + R22 * R22 * S2;
	}
};

/**
 * Header data read from .spz / .ply files
 */
USTRUCT(BlueprintType)
struct DBVUNREALAICPP_API FDbvSplatHeader
{
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
namespace DbvGaussianSplatCoords
{
	/** Recommended transformation when placing World Labs / OpenCV .spz in Unreal Engine */
	static const FRotator OpenCVToUnrealRotation = FRotator(0.0f, 0.0f, 90.0f); // Roll = 90.0 deg
	static const FVector OpenCVToUnrealScale = FVector(100.0, 100.0, 100.0);    // 1m = 100cm

	/** Direct vertex coordinate swap if baked into vertices instead of Actor transform */
	FORCEINLINE FVector3f ConvertOpenCVPointToUnreal(const FVector3f& InCV)
	{
		// OpenCV: +X right, +Y down, +Z forward
		// Unreal: +X forward, +Y right, +Z up
		return FVector3f(InCV.Z * 100.0f, InCV.X * 100.0f, -InCV.Y * 100.0f);
	}
}
