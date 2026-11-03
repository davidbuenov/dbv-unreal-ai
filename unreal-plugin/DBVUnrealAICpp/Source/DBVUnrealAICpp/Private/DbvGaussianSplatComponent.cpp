// =============================================================================
// DBV Unreal AI — Implementación del componente de Gaussian Splatting
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

#include "DbvGaussianSplatComponent.h"
#include "DbvGaussianSplatSceneProxy.h"
#include "DbvSplatPack.h"
#include "GameFramework/Actor.h"
#include "Misc/Paths.h"

namespace {
/**
 * Nubes ya subidas a la GPU, por fichero y escala. Un duplicado (Play, copiar y
 * pegar) o el nivel reabierto reutilizan la del original en vez de volver a
 * leer el fichero (2,35 GB en la iglesia). @spec REQ-61
 */
TMap<FString, TWeakPtr<FDbvSplatGpuCloud, ESPMode::ThreadSafe>> &CloudCache() {
  static TMap<FString, TWeakPtr<FDbvSplatGpuCloud, ESPMode::ThreadSafe>> Cache;
  return Cache;
}

FString CacheKey(const FString &Path, float ScaleMultiplier) {
  return FString::Printf(
      TEXT("%s|%g"), *FPaths::ConvertRelativePathToFull(Path), ScaleMultiplier);
}
} // namespace

UDbvGaussianSplatComponent::UDbvGaussianSplatComponent(
    const FObjectInitializer &ObjectInitializer)
    : Super(ObjectInitializer) {
  PrimaryComponentTick.bCanEverTick = false;
  SetCastShadow(false);
  bHasCustomNavigableGeometry = EHasCustomNavigableGeometry::No;
}

void UDbvGaussianSplatComponent::OnRegister() {
  Super::OnRegister();

  // Un duplicado para Play o un nivel reabierto trae la ruta y el asset, pero
  // no la nube de la GPU (no se guarda).
  if (!SourceFilePath.IsEmpty() && !GpuCloud.IsValid()) {
    GpuCloud = CloudCache()
                   .FindRef(CacheKey(SourceFilePath, SplatScaleMultiplier))
                   .Pin();
    if (GpuCloud.IsValid()) {
      UpdateBounds();
      MarkRenderStateDirty();
    } else {
      // Volver a cargar no debe deshacer lo que el usuario movió o giró.
      AActor *Owner = GetOwner();
      const FTransform Kept =
          Owner ? Owner->GetActorTransform() : FTransform::Identity;
      LoadFile(SourceFilePath);
      if (Owner) {
        Owner->SetActorTransform(Kept);
      }
    }
  }
}

bool UDbvGaussianSplatComponent::LoadFile(const FString &InFilePath) {
  SourceFilePath = InFilePath;

  if (!SplatAsset) {
    SplatAsset =
        NewObject<UDbvGaussianSplatAsset>(this, TEXT("InternalSplatAsset"));
  }

  FString ErrorMsg;
  bool bSuccess = false;

  if (InFilePath.EndsWith(TEXT(".spz"), ESearchCase::IgnoreCase)) {
    bSuccess = SplatAsset->LoadSpzFile(InFilePath, ErrorMsg);
  } else if (InFilePath.EndsWith(TEXT(".ply"), ESearchCase::IgnoreCase)) {
    bSuccess = SplatAsset->LoadPlyFile(InFilePath, ErrorMsg);
  } else {
    ErrorMsg = FString::Printf(
        TEXT("Unsupported file format: %s (must be .spz or .ply)"),
        *InFilePath);
  }
  LastLoadError = bSuccess ? FString() : ErrorMsg;

  if (bSuccess) {
    ApplyOpenCVTransformIfNeeded();
    BuildGpuCloud();
    UpdateBounds();
    MarkRenderStateDirty();
  } else {
    UE_LOG(LogTemp, Error,
           TEXT("UDbvGaussianSplatComponent::LoadFile failed: %s"), *ErrorMsg);
  }

  return bSuccess;
}

void UDbvGaussianSplatComponent::ApplyOpenCVTransformIfNeeded() {
  if (!bAutoApplyOpenCVTransform) {
    return;
  }

  AActor *Owner = GetOwner();
  if (Owner && Owner->GetRootComponent() == this) {
    // OpenCV coordinate system to Unreal Engine:
    // Roll = 90.0 degrees, Scale = 100x (1 meter = 100 cm)
    Owner->SetActorRotation(DbvGaussianSplatCoords::OpenCVToUnrealRotation);
    Owner->SetActorScale3D(DbvGaussianSplatCoords::OpenCVToUnrealScale);
  }
}

FPrimitiveSceneProxy *UDbvGaussianSplatComponent::CreateSceneProxy() {
  if (!GpuCloud.IsValid() || GpuCloud->GetNumSplats() == 0) {
    return nullptr;
  }

  return new FDbvGaussianSplatSceneProxy(this, GpuCloud);
}

void UDbvGaussianSplatComponent::BuildGpuCloud() {
  ReleaseGpuCloud();
  if (!SplatAsset || SplatAsset->GetNumSplats() == 0) {
    return;
  }

  // D34/D37: el lector ya dejó los splats en el formato de la GPU; aquí solo se
  // aplica el tamaño elegido.
  // @spec REQ-61, REQ-62
  FDbvSplatGpuData Data = SplatAsset->TakeData();
  if (!FMath::IsNearlyEqual(SplatScaleMultiplier, 1.0f)) {
    DbvSplatPack::ScaleSize(Data, SplatScaleMultiplier);
  }

  // La nube se comparte (original, duplicados de Play, proxies): su recurso se
  // libera en el hilo de render solo cuando desaparece la última referencia.
  GpuCloud = FDbvSplatGpuCloudRef(
      new FDbvSplatGpuCloud(MoveTemp(Data)), [](FDbvSplatGpuCloud *Cloud) {
        ENQUEUE_RENDER_COMMAND(DbvReleaseSplatCloud)(
            [Cloud](FRHICommandListImmediate &) {
              Cloud->ReleaseResource();
              delete Cloud;
            });
      });
  BeginInitResource(GpuCloud.Get());
  CloudCache().Add(CacheKey(SourceFilePath, SplatScaleMultiplier), GpuCloud);
}

void UDbvGaussianSplatComponent::ReleaseGpuCloud() {
  // Si era la última referencia, el borrador de BuildGpuCloud libera el recurso
  // en el hilo de render.
  GpuCloud.Reset();
}

void UDbvGaussianSplatComponent::BeginDestroy() {
  ReleaseGpuCloud();
  Super::BeginDestroy();
}

FBoxSphereBounds
UDbvGaussianSplatComponent::CalcBounds(const FTransform &LocalToWorld) const {
  if (GpuCloud.IsValid() && GpuCloud->GetLocalBounds().IsValid) {
    return FBoxSphereBounds(GpuCloud->GetLocalBounds())
        .TransformBy(LocalToWorld);
  }
  if (SplatAsset && SplatAsset->BoundingBox.IsValid) {
    return FBoxSphereBounds(SplatAsset->BoundingBox).TransformBy(LocalToWorld);
  }

  return FBoxSphereBounds(FVector::ZeroVector, FVector(100.0f), 100.0f)
      .TransformBy(LocalToWorld);
}
