// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#include "DbvGaussianSplatAsset.h"
#include "DbvSpzReader.h"
#include "DbvPlyReader.h"

UDbvGaussianSplatAsset::UDbvGaussianSplatAsset()
{
}

bool UDbvGaussianSplatAsset::LoadSpzFile(const FString& FilePath, FString& OutErrorMessage)
{
	Points.Empty();
	SourceFilePath = FilePath;

	const bool bSuccess = FDbvSpzReader::LoadFromFile(FilePath, Header, Points, OutErrorMessage);
	if (bSuccess)
	{
		BoundingBox = Header.BoundingBox;
	}
	return bSuccess;
}

bool UDbvGaussianSplatAsset::LoadPlyFile(const FString& FilePath, FString& OutErrorMessage)
{
	Points.Empty();
	SourceFilePath = FilePath;

	const bool bSuccess = FDbvPlyReader::LoadFromFile(FilePath, Header, Points, OutErrorMessage);
	if (bSuccess)
	{
		BoundingBox = Header.BoundingBox;
	}
	return bSuccess;
}
