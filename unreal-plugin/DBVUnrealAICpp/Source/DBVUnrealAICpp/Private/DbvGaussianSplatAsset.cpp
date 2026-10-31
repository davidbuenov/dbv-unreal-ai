// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.
//
// @spec REQ-61, REQ-62, REQ-63

#include "DbvGaussianSplatAsset.h"
#include "DbvPlyReader.h"
#include "DbvSpzReader.h"
#include "DbvSplatPack.h"

UDbvGaussianSplatAsset::UDbvGaussianSplatAsset()
{
}

bool UDbvGaussianSplatAsset::LoadSpzFile(const FString& FilePath, FString& OutErrorMessage)
{
	SourceFilePath = FilePath;
	const bool bSuccess = FDbvSpzReader::LoadFromFile(FilePath, Header, Data, OutErrorMessage);
	if (bSuccess)
	{
		FinishLoad();
	}
	return bSuccess;
}

bool UDbvGaussianSplatAsset::LoadPlyFile(const FString& FilePath, FString& OutErrorMessage)
{
	SourceFilePath = FilePath;
	const bool bSuccess = FDbvPlyReader::LoadFromFile(FilePath, Header, Data, OutErrorMessage);
	if (bSuccess)
	{
		FinishLoad();
	}
	return bSuccess;
}

void UDbvGaussianSplatAsset::FinishLoad()
{
	BoundingBox = Header.BoundingBox;
	// D36: si no cabe en el presupuesto de VRAM, primero se quitan los SH y luego se eligen los más visibles.
	DbvSplatPack::FitBudget(Data, DbvSplatPack::VramBudgetBytes());
	NumKept = Data.Num();
	SHDegreeKept = Data.SHDegree;
}
