// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#include "DbvGaussianSplatActor.h"

ADbvGaussianSplatActor::ADbvGaussianSplatActor()
{
	PrimaryActorTick.bCanEverTick = false;

	SplatComponent = CreateDefaultSubobject<UDbvGaussianSplatComponent>(TEXT("SplatComponent"));
	RootComponent = SplatComponent;
}

bool ADbvGaussianSplatActor::LoadSpz(const FString& FilePath)
{
	if (SplatComponent)
	{
		return SplatComponent->LoadFile(FilePath);
	}
	return false;
}

bool ADbvGaussianSplatActor::LoadPly(const FString& FilePath)
{
	if (SplatComponent)
	{
		return SplatComponent->LoadFile(FilePath);
	}
	return false;
}
