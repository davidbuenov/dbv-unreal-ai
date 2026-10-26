// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#pragma once

#include "CoreMinimal.h"
#include "DbvGaussianSplatTypes.h"

/**
 * Fast native reader and decompressor for Scaniverse / World Labs .spz files.
 */
class DBVUNREALAICPP_API FDbvSpzReader
{
public:
	/**
	 * Reads an .spz file from disk, decompresses the payload, and fills the point array.
	 * 
	 * @param FilePath Absolute path to the .spz file
	 * @param OutHeader Header metadata populated from the file
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
	 * Decompresses and parses an .spz payload directly from a memory buffer.
	 *
	 * @param CompressedData Raw compressed bytes (gzip or raw SPSP)
	 * @param OutHeader Header metadata
	 * @param OutPoints Array of unpacked Gaussian splats
	 * @param OutErrorMessage Populated on failure
	 * @return True on success, False otherwise
	 */
	static bool LoadFromMemory(
		const TArray<uint8>& CompressedData,
		FDbvSplatHeader& OutHeader,
		TArray<FDbvGaussianPoint>& OutPoints,
		FString& OutErrorMessage
	);

private:
	/** Internal helper to unpack 24-bit signed fixed point integer */
	static FORCEINLINE int32 UnpackInt24(const uint8* Ptr)
	{
		int32 Value = (static_cast<int32>(Ptr[0])) |
		              (static_cast<int32>(Ptr[1]) << 8) |
		              (static_cast<int32>(Ptr[2]) << 16);
		// Sign extend from 24 bits to 32 bits
		if (Value & 0x00800000)
		{
			Value |= 0xFF000000;
		}
		return Value;
	}
};
