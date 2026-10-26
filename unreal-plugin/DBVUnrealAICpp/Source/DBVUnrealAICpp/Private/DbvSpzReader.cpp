// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#include "DbvSpzReader.h"
#include "Misc/FileHelper.h"
#include "Misc/Compression.h"
#include "HAL/PlatformFileManager.h"

DEFINE_LOG_CATEGORY_STATIC(LogDbvSpzReader, Log, All);

bool FDbvSpzReader::LoadFromFile(
	const FString& FilePath,
	FDbvSplatHeader& OutHeader,
	TArray<FDbvGaussianPoint>& OutPoints,
	FString& OutErrorMessage)
{
	if (!FPlatformFileManager::Get().GetPlatformFile().FileExists(*FilePath))
	{
		OutErrorMessage = FString::Printf(TEXT("File does not exist: %s"), *FilePath);
		return false;
	}

	TArray<uint8> FileData;
	if (!FFileHelper::LoadFileToArray(FileData, *FilePath))
	{
		OutErrorMessage = FString::Printf(TEXT("Failed to read file: %s"), *FilePath);
		return false;
	}

	return LoadFromMemory(FileData, OutHeader, OutPoints, OutErrorMessage);
}

bool FDbvSpzReader::LoadFromMemory(
	const TArray<uint8>& CompressedData,
	FDbvSplatHeader& OutHeader,
	TArray<FDbvGaussianPoint>& OutPoints,
	FString& OutErrorMessage)
{
	if (CompressedData.Num() < 16)
	{
		OutErrorMessage = TEXT("Payload is too small to be a valid .spz file");
		return false;
	}

	const uint8* RawBytes = CompressedData.GetData();
	TArray<uint8> DecompressedBuffer;

	// Check if this is a GZIP stream (Magic 0x1F, 0x8B)
	const bool bIsGzip = (RawBytes[0] == 0x1F && RawBytes[1] == 0x8B);
	// Check if this is a ZLIB stream (Magic 0x78)
	const bool bIsZlib = (RawBytes[0] == 0x78 && (RawBytes[1] == 0x9C || RawBytes[1] == 0x01 || RawBytes[1] == 0xDA));

	if (bIsGzip || bIsZlib)
	{
		FName CompressionFormat = bIsGzip ? NAME_Gzip : NAME_Zlib;

		// Extract expected uncompressed size from GZIP trailer if possible
		int32 UncompressedSize = 0;
		if (bIsGzip && CompressedData.Num() >= 10)
		{
			const int32 TrailerOffset = CompressedData.Num() - 4;
			UncompressedSize = static_cast<int32>(RawBytes[TrailerOffset]) |
			                   (static_cast<int32>(RawBytes[TrailerOffset + 1]) << 8) |
			                   (static_cast<int32>(RawBytes[TrailerOffset + 2]) << 16) |
			                   (static_cast<int32>(RawBytes[TrailerOffset + 3]) << 24);
		}

		// Fallback buffer size estimate if uncompressed size is unavailable
		if (UncompressedSize <= 0 || UncompressedSize > 1024 * 1024 * 1024)
		{
			UncompressedSize = CompressedData.Num() * 6; // Typical compression ratio for SPZ
		}

		DecompressedBuffer.SetNumUninitialized(UncompressedSize);

		bool bSuccess = FCompression::UncompressMemory(
			CompressionFormat,
			DecompressedBuffer.GetData(),
			UncompressedSize,
			CompressedData.GetData(),
			CompressedData.Num()
		);

		if (!bSuccess)
		{
			// Try with larger buffer if it failed due to truncation
			UncompressedSize = CompressedData.Num() * 12;
			DecompressedBuffer.SetNumUninitialized(UncompressedSize);
			bSuccess = FCompression::UncompressMemory(
				CompressionFormat,
				DecompressedBuffer.GetData(),
				UncompressedSize,
				CompressedData.GetData(),
				CompressedData.Num()
			);
		}

		if (!bSuccess)
		{
			OutErrorMessage = TEXT("Failed to decompress SPZ gzip/zlib stream via FCompression");
			return false;
		}

		RawBytes = DecompressedBuffer.GetData();
	}

	// Validate SPSP magic header
	// 'S', 'P', 'S', 'P' (0x50535053) or 'S', 'P', 'Z', '\0'
	const uint32 Magic = *reinterpret_cast<const uint32*>(RawBytes);
	constexpr uint32 MagicSPSP = 0x50535053; // "SPSP"
	constexpr uint32 MagicSPZ0 = 0x005A5053; // "SPZ\0"

	int32 HeaderOffset = 0;
	if (Magic == MagicSPSP || Magic == MagicSPZ0)
	{
		HeaderOffset = 4;
	}

	// Parse SPZ header
	const uint32 Version = *reinterpret_cast<const uint32*>(RawBytes + HeaderOffset);
	HeaderOffset += 4;

	const uint32 NumPoints = *reinterpret_cast<const uint32*>(RawBytes + HeaderOffset);
	HeaderOffset += 4;

	const uint8 SHDegree = RawBytes[HeaderOffset];
	HeaderOffset += 1;

	const uint8 FractionalBits = RawBytes[HeaderOffset];
	HeaderOffset += 1;

	const uint8 Flags = RawBytes[HeaderOffset];
	HeaderOffset += 1;

	// Reserved byte
	HeaderOffset += 1;

	if (NumPoints == 0 || NumPoints > 50000000)
	{
		OutErrorMessage = FString::Printf(TEXT("Invalid point count in SPZ header: %u"), NumPoints);
		return false;
	}

	OutHeader.NumPoints = static_cast<int32>(NumPoints);
	OutHeader.SHDegree = static_cast<int32>(SHDegree);
	OutHeader.FractionalBits = static_cast<int32>(FractionalBits);
	OutHeader.BoundingBox.Init();

	OutPoints.Empty(OutHeader.NumPoints);
	OutPoints.SetNumUninitialized(OutHeader.NumPoints);

	const float PosScale = 1.0f / static_cast<float>(1 << FractionalBits);

	// Positions (3 bytes per coord = 9 bytes per point)
	const uint8* PosPtr = RawBytes + HeaderOffset;
	HeaderOffset += OutHeader.NumPoints * 9;

	// Scales (3 bytes per point in log scale)
	const uint8* ScalePtr = RawBytes + HeaderOffset;
	HeaderOffset += OutHeader.NumPoints * 3;

	// Rotations (4 bytes quaternion: W, X, Y, Z normalized)
	const uint8* RotPtr = RawBytes + HeaderOffset;
	HeaderOffset += OutHeader.NumPoints * 4;

	// Alphas (1 byte per point)
	const uint8* AlphaPtr = RawBytes + HeaderOffset;
	HeaderOffset += OutHeader.NumPoints;

	// Colors / SH0 (3 bytes RGB per point)
	const uint8* ColorPtr = RawBytes + HeaderOffset;
	HeaderOffset += OutHeader.NumPoints * 3;

	// Unpack points into OutPoints array
	for (int32 i = 0; i < OutHeader.NumPoints; ++i)
	{
		FDbvGaussianPoint& Pt = OutPoints[i];

		// Position (24-bit fixed point)
		const int32 RawX = UnpackInt24(PosPtr + i * 9 + 0);
		const int32 RawY = UnpackInt24(PosPtr + i * 9 + 3);
		const int32 RawZ = UnpackInt24(PosPtr + i * 9 + 6);

		Pt.Position.X = static_cast<float>(RawX) * PosScale;
		Pt.Position.Y = static_cast<float>(RawY) * PosScale;
		Pt.Position.Z = static_cast<float>(RawZ) * PosScale;

		OutHeader.BoundingBox += FVector(Pt.Position);

		// Scale (exp of log scale)
		const float LogSx = (static_cast<float>(ScalePtr[i * 3 + 0]) / 16.0f) - 10.0f;
		const float LogSy = (static_cast<float>(ScalePtr[i * 3 + 1]) / 16.0f) - 10.0f;
		const float LogSz = (static_cast<float>(ScalePtr[i * 3 + 2]) / 16.0f) - 10.0f;

		Pt.Scale.X = FMath::Exp(LogSx);
		Pt.Scale.Y = FMath::Exp(LogSy);
		Pt.Scale.Z = FMath::Exp(LogSz);

		// Rotation (Quaternion W, X, Y, Z in [-1, 1])
		Pt.Rotation.X = (static_cast<float>(RotPtr[i * 4 + 0]) / 127.5f) - 1.0f;
		Pt.Rotation.Y = (static_cast<float>(RotPtr[i * 4 + 1]) / 127.5f) - 1.0f;
		Pt.Rotation.Z = (static_cast<float>(RotPtr[i * 4 + 2]) / 127.5f) - 1.0f;
		Pt.Rotation.W = (static_cast<float>(RotPtr[i * 4 + 3]) / 127.5f) - 1.0f;

		// Opacity (sigmoid or linear)
		const float RawAlpha = static_cast<float>(AlphaPtr[i]) / 255.0f;
		Pt.Opacity = RawAlpha;

		// Base diffuse color (SH0 DC)
		Pt.Color.R = static_cast<float>(ColorPtr[i * 3 + 0]) / 255.0f;
		Pt.Color.G = static_cast<float>(ColorPtr[i * 3 + 1]) / 255.0f;
		Pt.Color.B = static_cast<float>(ColorPtr[i * 3 + 2]) / 255.0f;
		Pt.Color.A = Pt.Opacity;
	}

	UE_LOG(LogDbvSpzReader, Log, TEXT("Successfully parsed SPZ file: %d splats, SH degree %d, bounds %s"),
		OutHeader.NumPoints, OutHeader.SHDegree, *OutHeader.BoundingBox.ToString());

	return true;
}
