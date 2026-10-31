// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#include "DbvGaussianSplatComponent.h"
#include "DbvGaussianSplatSceneProxy.h"
#include "GameFramework/Actor.h"
#include "Misc/Paths.h"

namespace
{
	/**
	 * Nubes ya subidas a la GPU, por fichero y escala. Un duplicado (Play, copiar y pegar) o el nivel reabierto
	 * reutilizan la del original en vez de volver a leer el fichero (2,35 GB en la iglesia). @spec REQ-61
	 */
	TMap<FString, TWeakPtr<FDbvSplatGpuCloud, ESPMode::ThreadSafe>>& CloudCache()
	{
		static TMap<FString, TWeakPtr<FDbvSplatGpuCloud, ESPMode::ThreadSafe>> Cache;
		return Cache;
	}

	FString CacheKey(const FString& Path, float ScaleMultiplier)
	{
		return FString::Printf(TEXT("%s|%g"), *FPaths::ConvertRelativePathToFull(Path), ScaleMultiplier);
	}
}

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

	// Un duplicado para Play o un nivel reabierto trae la ruta y el asset, pero no la nube de la GPU (no se guarda).
	if (!SourceFilePath.IsEmpty() && !GpuCloud.IsValid())
	{
		GpuCloud = CloudCache().FindRef(CacheKey(SourceFilePath, SplatScaleMultiplier)).Pin();
		if (GpuCloud.IsValid())
		{
			UpdateBounds();
			MarkRenderStateDirty();
		}
		else
		{
			// Volver a cargar no debe deshacer lo que el usuario movió o giró.
			AActor* Owner = GetOwner();
			const FTransform Kept = Owner ? Owner->GetActorTransform() : FTransform::Identity;
			LoadFile(SourceFilePath);
			if (Owner)
			{
				Owner->SetActorTransform(Kept);
			}
		}
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
	LastLoadError = bSuccess ? FString() : ErrorMsg;

	if (bSuccess)
	{
		ApplyOpenCVTransformIfNeeded();
		BuildGpuCloud();
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
	if (!GpuCloud.IsValid() || GpuCloud->GetNumSplats() == 0)
	{
		return nullptr;
	}

	return new FDbvGaussianSplatSceneProxy(this, GpuCloud);
}

void UDbvGaussianSplatComponent::BuildGpuCloud()
{
	ReleaseGpuCloud();
	if (!SplatAsset || SplatAsset->GetNumSplats() == 0)
	{
		return;
	}

	// D34: posición y opacidad, covarianza 3D en espacio local y color base RGB8. @spec REQ-61
	const TArray<FDbvGaussianPoint>& Points = SplatAsset->GetPoints();
	FDbvSplatGpuData Data;
	Data.PosOpacity.SetNumUninitialized(Points.Num());
	Data.Cov.SetNumUninitialized(Points.Num() * 2);
	for (int32 i = 0; i < Points.Num(); ++i)
	{
		const FDbvGaussianPoint& Pt = Points[i];
		float Cov[6];
		FDbvGaussianPoint Scaled = Pt;
		Scaled.Scale *= SplatScaleMultiplier;
		// Los .ply de 3DGS guardan el cuaternión sin normalizar.
		const FVector4f& Q = Pt.Rotation;
		const float Length = FMath::Sqrt(Q.X * Q.X + Q.Y * Q.Y + Q.Z * Q.Z + Q.W * Q.W);
		Scaled.Rotation = Length > UE_SMALL_NUMBER ? Q / Length : FVector4f(1.0f, 0.0f, 0.0f, 0.0f);
		Scaled.ComputeCovariance3D(Cov);
		const FColor Rgb = FLinearColor(Pt.Color.R, Pt.Color.G, Pt.Color.B).QuantizeRound();
		const uint32 PackedRgb = uint32(Rgb.R) | (uint32(Rgb.G) << 8) | (uint32(Rgb.B) << 16);
		Data.PosOpacity[i] = FVector4f(Pt.Position, Pt.Opacity);
		Data.Cov[i * 2] = FVector4f(Cov[0], Cov[1], Cov[2], Cov[3]);
		Data.Cov[i * 2 + 1] = FVector4f(Cov[4], Cov[5], FMath::AsFloat(PackedRgb), 0.0f);
		Data.Bounds += FVector(Pt.Position);
	}

	// La nube se comparte (original, duplicados de Play, proxies): su recurso se libera en el hilo de render solo
	// cuando desaparece la última referencia.
	GpuCloud = FDbvSplatGpuCloudRef(new FDbvSplatGpuCloud(MoveTemp(Data)), [](FDbvSplatGpuCloud* Cloud)
	{
		ENQUEUE_RENDER_COMMAND(DbvReleaseSplatCloud)([Cloud](FRHICommandListImmediate&)
		{
			Cloud->ReleaseResource();
			delete Cloud;
		});
	});
	BeginInitResource(GpuCloud.Get());
	CloudCache().Add(CacheKey(SourceFilePath, SplatScaleMultiplier), GpuCloud);
}

void UDbvGaussianSplatComponent::ReleaseGpuCloud()
{
	// Si era la última referencia, el borrador de BuildGpuCloud libera el recurso en el hilo de render.
	GpuCloud.Reset();
}

void UDbvGaussianSplatComponent::BeginDestroy()
{
	ReleaseGpuCloud();
	Super::BeginDestroy();
}

FBoxSphereBounds UDbvGaussianSplatComponent::CalcBounds(const FTransform& LocalToWorld) const
{
	if (GpuCloud.IsValid() && GpuCloud->GetLocalBounds().IsValid)
	{
		return FBoxSphereBounds(GpuCloud->GetLocalBounds()).TransformBy(LocalToWorld);
	}
	if (SplatAsset && SplatAsset->BoundingBox.IsValid)
	{
		return FBoxSphereBounds(SplatAsset->BoundingBox).TransformBy(LocalToWorld);
	}

	return FBoxSphereBounds(FVector::ZeroVector, FVector(100.0f), 100.0f).TransformBy(LocalToWorld);
}
