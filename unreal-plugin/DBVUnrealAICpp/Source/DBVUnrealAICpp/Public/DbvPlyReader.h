// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#pragma once

#include "CoreMinimal.h"
#include "DbvGaussianSplatTypes.h"

/**
 * Fast native reader for standard 3D Gaussian Splatting binary .ply files.
 */
class DBVUNREALAICPP_API FDbvPlyReader
{
public:
	/**
	 * Reads a binary .ply file from disk and parses 3DGS attributes.
	 *
	 * @param FilePath Absolute path to the .ply file
	 * @param OutHeader Header metadata
	 * @param OutPoints Array of unpacked Gaussian splats
	 * @param OutErrorMessage Populated with details if parsing fails
	 * @return True on success, False otherwise
	 */
	static bool LoadFromFile(
		const FString& FilePath,
		FDbvSplatHeader& OutHeader,
		TArray<FDbvGaussianPoint>& OutPoints,
		FString& OutErrorMessage
	);

	/**
	 * Parses binary .ply data from an in-memory buffer.
	 */
	static bool LoadFromMemory(
		const TArray<uint8>& FileData,
		FDbvSplatHeader& OutHeader,
		TArray<FDbvGaussianPoint>& OutPoints,
		FString& OutErrorMessage
	);
};
