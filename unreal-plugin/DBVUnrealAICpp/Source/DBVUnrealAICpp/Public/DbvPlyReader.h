// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.
//
// @spec REQ-62, REQ-63

#pragma once

#include "CoreMinimal.h"
#include "DbvGaussianSplatTypes.h"
#include "DbvSplatRender.h"

/**
 * Reader for binary little-endian 3D Gaussian Splatting .ply files. It writes the GPU format directly
 * (DbvSplatPack), spherical harmonics included, and reads the file in chunks (no 2 GB limit).
 */
class DBVUNREALAICPP_API FDbvPlyReader
{
public:
	/** Reads a .ply from disk, `VerticesPerChunk` vertices at a time (small values are for tests). */
	static bool LoadFromFile(const FString& FilePath, FDbvSplatHeader& OutHeader, FDbvSplatGpuData& Out, FString& OutErrorMessage, int64 VerticesPerChunk = 65536);

	/** Parses a .ply already in memory. */
	static bool LoadFromMemory(const TArray<uint8>& FileData, FDbvSplatHeader& OutHeader, FDbvSplatGpuData& Out, FString& OutErrorMessage);
};
