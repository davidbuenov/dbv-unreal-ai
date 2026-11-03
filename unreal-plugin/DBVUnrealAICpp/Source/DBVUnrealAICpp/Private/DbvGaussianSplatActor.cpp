// =============================================================================
// DBV Unreal AI — Implementación del actor de Gaussian Splatting
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

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
