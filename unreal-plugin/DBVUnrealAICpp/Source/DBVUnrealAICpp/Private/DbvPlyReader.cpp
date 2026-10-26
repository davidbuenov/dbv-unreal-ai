// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#include "DbvPlyReader.h"
#include "Misc/FileHelper.h"
#include "HAL/PlatformFileManager.h"

DEFINE_LOG_CATEGORY_STATIC(LogDbvPlyReader, Log, All);

bool FDbvPlyReader::LoadFromFile(
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

bool FDbvPlyReader::LoadFromMemory(
	const TArray<uint8>& FileData,
	FDbvSplatHeader& OutHeader,
	TArray<FDbvGaussianPoint>& OutPoints,
	FString& OutErrorMessage)
{
	if (FileData.Num() < 64)
	{
		OutErrorMessage = TEXT("File data is too small to be a valid PLY file");
		return false;
	}

	// Find "end_header\n"
	const FString HeaderSearch = TEXT("end_header");
	int32 EndHeaderIdx = -1;
	const int32 MaxSearch = FMath::Min(FileData.Num(), 8192);

	for (int32 i = 0; i < MaxSearch - 10; ++i)
	{
		if (FMemory::Memcmp(FileData.GetData() + i, "end_header", 10) == 0)
		{
			EndHeaderIdx = i + 10;
			// Skip newline characters (\r\n or \n)
			while (EndHeaderIdx < FileData.Num() && 
			      (FileData[EndHeaderIdx] == '\r' || FileData[EndHeaderIdx] == '\n'))
			{
				EndHeaderIdx++;
			}
			break;
		}
	}

	if (EndHeaderIdx < 0)
	{
		OutErrorMessage = TEXT("Could not find end_header in PLY file");
		return false;
	}

	// Parse header string
	const FString HeaderStr(EndHeaderIdx, reinterpret_cast<const char*>(FileData.GetData()));
	TArray<FString> Lines;
	HeaderStr.ParseIntoArrayLines(Lines);

	int32 VertexCount = 0;
	TArray<FString> PropertyNames;

	for (const FString& Line : Lines)
	{
		if (Line.StartsWith(TEXT("element vertex ")))
		{
			FString CountStr = Line.RightChop(15).TrimStartAndEnd();
			VertexCount = FCString::Atoi(*CountStr);
		}
		else if (Line.StartsWith(TEXT("property float ")))
		{
			FString PropName = Line.RightChop(15).TrimStartAndEnd();
			PropertyNames.Add(PropName);
		}
	}

	if (VertexCount <= 0)
	{
		OutErrorMessage = TEXT("PLY header did not specify a valid vertex count");
		return false;
	}

	const int32 BytesPerVertex = PropertyNames.Num() * sizeof(float);
	const int32 ExpectedDataSize = EndHeaderIdx + VertexCount * BytesPerVertex;

	if (FileData.Num() < ExpectedDataSize)
	{
		OutErrorMessage = FString::Printf(TEXT("PLY file truncated: expected %d bytes, got %d"),
			ExpectedDataSize, FileData.Num());
		return false;
	}

	// Map property indices
	const int32 IdxX = PropertyNames.IndexOfByKey(TEXT("x"));
	const int32 IdxY = PropertyNames.IndexOfByKey(TEXT("y"));
	const int32 IdxZ = PropertyNames.IndexOfByKey(TEXT("z"));
	const int32 IdxDC0 = PropertyNames.IndexOfByKey(TEXT("f_dc_0"));
	const int32 IdxDC1 = PropertyNames.IndexOfByKey(TEXT("f_dc_1"));
	const int32 IdxDC2 = PropertyNames.IndexOfByKey(TEXT("f_dc_2"));
	const int32 IdxOpacity = PropertyNames.IndexOfByKey(TEXT("opacity"));
	const int32 IdxScale0 = PropertyNames.IndexOfByKey(TEXT("scale_0"));
	const int32 IdxScale1 = PropertyNames.IndexOfByKey(TEXT("scale_1"));
	const int32 IdxScale2 = PropertyNames.IndexOfByKey(TEXT("scale_2"));
	const int32 IdxRot0 = PropertyNames.IndexOfByKey(TEXT("rot_0"));
	const int32 IdxRot1 = PropertyNames.IndexOfByKey(TEXT("rot_1"));
	const int32 IdxRot2 = PropertyNames.IndexOfByKey(TEXT("rot_2"));
	const int32 IdxRot3 = PropertyNames.IndexOfByKey(TEXT("rot_3"));

	OutHeader.NumPoints = VertexCount;
	OutHeader.FractionalBits = 0;
	OutHeader.BoundingBox.Init();

	OutPoints.Empty(VertexCount);
	OutPoints.SetNumUninitialized(VertexCount);

	const float* FloatData = reinterpret_cast<const float*>(FileData.GetData() + EndHeaderIdx);
	constexpr float SH_C0 = 0.28209479177387814f;

	for (int32 i = 0; i < VertexCount; ++i)
	{
		const float* Vert = FloatData + i * PropertyNames.Num();
		FDbvGaussianPoint& Pt = OutPoints[i];

		// Position
		Pt.Position.X = (IdxX >= 0) ? Vert[IdxX] : 0.0f;
		Pt.Position.Y = (IdxY >= 0) ? Vert[IdxY] : 0.0f;
		Pt.Position.Z = (IdxZ >= 0) ? Vert[IdxZ] : 0.0f;
		OutHeader.BoundingBox += FVector(Pt.Position);

		// Scale (exp of log-scale)
		Pt.Scale.X = (IdxScale0 >= 0) ? FMath::Exp(Vert[IdxScale0]) : 1.0f;
		Pt.Scale.Y = (IdxScale1 >= 0) ? FMath::Exp(Vert[IdxScale1]) : 1.0f;
		Pt.Scale.Z = (IdxScale2 >= 0) ? FMath::Exp(Vert[IdxScale2]) : 1.0f;

		// Rotation (Quaternion)
		Pt.Rotation.X = (IdxRot0 >= 0) ? Vert[IdxRot0] : 1.0f;
		Pt.Rotation.Y = (IdxRot1 >= 0) ? Vert[IdxRot1] : 0.0f;
		Pt.Rotation.Z = (IdxRot2 >= 0) ? Vert[IdxRot2] : 0.0f;
		Pt.Rotation.W = (IdxRot3 >= 0) ? Vert[IdxRot3] : 0.0f;

		// Opacity (sigmoid)
		const float RawOpacity = (IdxOpacity >= 0) ? Vert[IdxOpacity] : 0.0f;
		Pt.Opacity = 1.0f / (1.0f + FMath::Exp(-RawOpacity));

		// Base diffuse color from SH0
		const float DC0 = (IdxDC0 >= 0) ? Vert[IdxDC0] : 0.0f;
		const float DC1 = (IdxDC1 >= 0) ? Vert[IdxDC1] : 0.0f;
		const float DC2 = (IdxDC2 >= 0) ? Vert[IdxDC2] : 0.0f;

		Pt.Color.R = FMath::Clamp(0.5f + SH_C0 * DC0, 0.0f, 1.0f);
		Pt.Color.G = FMath::Clamp(0.5f + SH_C0 * DC1, 0.0f, 1.0f);
		Pt.Color.B = FMath::Clamp(0.5f + SH_C0 * DC2, 0.0f, 1.0f);
		Pt.Color.A = Pt.Opacity;
	}

	UE_LOG(LogDbvPlyReader, Log, TEXT("Successfully parsed PLY file: %d splats, bounds %s"),
		VertexCount, *OutHeader.BoundingBox.ToString());

	return true;
}
