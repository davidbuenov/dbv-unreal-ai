// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#include "DbvGaussianSplatAsset.h"
#include "DbvSpzReader.h"
#include "DbvPlyReader.h"

#include <algorithm>

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
		KeepMostVisible();
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
		KeepMostVisible();
	}
	return bSuccess;
}

void UDbvGaussianSplatAsset::KeepMostVisible()
{
	if (Points.Num() <= MaxDrawnSplats)
	{
		return;
	}

	// Lo que más se ve de lejos: opacidad por la mayor área proyectable del elipsoide. Se elige en tiempo lineal
	// (nth_element) y se conserva el orden del fichero, para que la escena entera aparezca, menos densa, en vez de
	// solo sus primeros splats. @spec REQ-54
	TArray<float> Scores;
	Scores.SetNumUninitialized(Points.Num());
	for (int32 i = 0; i < Points.Num(); ++i)
	{
		const FVector3f& S = Points[i].Scale;
		Scores[i] = Points[i].Opacity * FMath::Max3(S.X * S.Y, S.Y * S.Z, S.X * S.Z);
	}
	TArray<int32> Order;
	Order.SetNumUninitialized(Points.Num());
	for (int32 i = 0; i < Order.Num(); ++i)
	{
		Order[i] = i;
	}
	std::nth_element(Order.GetData(), Order.GetData() + MaxDrawnSplats, Order.GetData() + Order.Num(),
		[&Scores](int32 A, int32 B) { return Scores[A] > Scores[B]; });
	Order.SetNum(MaxDrawnSplats);
	Order.Sort();

	TArray<FDbvGaussianPoint> Kept;
	Kept.Reserve(MaxDrawnSplats);
	for (const int32 Index : Order)
	{
		Kept.Add(MoveTemp(Points[Index]));
	}
	Points = MoveTemp(Kept);
}
