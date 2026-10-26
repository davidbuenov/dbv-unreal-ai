// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#include "DbvGaussianSplatComponent.h"
#include "DbvGaussianSplatSceneProxy.h"
#include "GameFramework/Actor.h"

UDbvGaussianSplatComponent::UDbvGaussianSplatComponent(const FObjectInitializer& ObjectInitializer)
	: Super(ObjectInitializer)
{
	PrimaryComponentTick.bCanEverTick = false;
	SetCastShadow(false);
	bHasCustomNavigableGeometry = EHasCustomNavigableGeometry::No;
}

void UDbvGaussianSplatComponent::OnRegister()
{
	Super::OnRegister();

	if (!SourceFilePath.IsEmpty() && !SplatAsset)
	{
		LoadFile(SourceFilePath);
	}
}

bool UDbvGaussianSplatComponent::LoadFile(const FString& InFilePath)
{
	SourceFilePath = InFilePath;

	if (!SplatAsset)
	{
		SplatAsset = NewObject<UDbvGaussianSplatAsset>(this, TEXT("InternalSplatAsset"));
	}

	FString ErrorMsg;
	bool bSuccess = false;

	if (InFilePath.EndsWith(TEXT(".spz"), ESearchCase::IgnoreCase))
	{
		bSuccess = SplatAsset->LoadSpzFile(InFilePath, ErrorMsg);
	}
	else if (InFilePath.EndsWith(TEXT(".ply"), ESearchCase::IgnoreCase))
	{
		bSuccess = SplatAsset->LoadPlyFile(InFilePath, ErrorMsg);
	}
	else
	{
		ErrorMsg = FString::Printf(TEXT("Unsupported file format: %s (must be .spz or .ply)"), *InFilePath);
	}

	if (bSuccess)
	{
		ApplyOpenCVTransformIfNeeded();
		UpdateBounds();
		MarkRenderStateDirty();
	}
	else
	{
		UE_LOG(LogTemp, Error, TEXT("UDbvGaussianSplatComponent::LoadFile failed: %s"), *ErrorMsg);
	}

	return bSuccess;
}

void UDbvGaussianSplatComponent::ApplyOpenCVTransformIfNeeded()
{
	if (!bAutoApplyOpenCVTransform)
	{
		return;
	}

	AActor* Owner = GetOwner();
	if (Owner && Owner->GetRootComponent() == this)
	{
		// OpenCV coordinate system to Unreal Engine:
		// Roll = 90.0 degrees, Scale = 100x (1 meter = 100 cm)
		Owner->SetActorRotation(DbvGaussianSplatCoords::OpenCVToUnrealRotation);
		Owner->SetActorScale3D(DbvGaussianSplatCoords::OpenCVToUnrealScale);
	}
}

FPrimitiveSceneProxy* UDbvGaussianSplatComponent::CreateSceneProxy()
{
	if (!SplatAsset || SplatAsset->GetNumSplats() == 0)
	{
		return nullptr;
	}

	return new FDbvGaussianSplatSceneProxy(this);
}

FBoxSphereBounds UDbvGaussianSplatComponent::CalcBounds(const FTransform& LocalToWorld) const
{
	if (SplatAsset && SplatAsset->BoundingBox.IsValid)
	{
		return FBoxSphereBounds(SplatAsset->BoundingBox).TransformBy(LocalToWorld);
	}

	return FBoxSphereBounds(FVector::ZeroVector, FVector(100.0f), 100.0f).TransformBy(LocalToWorld);
}
