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
 * Reader for Niantic .spz files (World Labs, Scaniverse), versions 2 and 3, gzip-compressed. It writes the GPU
 * format directly (DbvSplatPack), converting from the SPZ axes (RUB) to the .ply ones (RDF) so both share the
 * same actor transform.
 */
class DBVUNREALAICPP_API FDbvSpzReader
{
public:
	/** Reads an .spz from disk. */
	static bool LoadFromFile(const FString& FilePath, FDbvSplatHeader& OutHeader, FDbvSplatGpuData& Out, FString& OutErrorMessage);

	/** Parses an .spz payload (gzip-compressed, or already decompressed) from memory. */
	static bool LoadFromMemory(const TArray<uint8>& Payload, FDbvSplatHeader& OutHeader, FDbvSplatGpuData& Out, FString& OutErrorMessage);
};
