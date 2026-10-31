// =============================================================================
// DBV Unreal AI — Render de splats por GPU: nube subida y registro de proxies
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61
//
// El componente empaqueta los splats (D34), crea una FDbvSplatGpuCloud y la sube UNA vez (BeginInitResource). Su
// proxy de escena se da de alta en el registro al crear sus recursos y de baja al destruirlos; la extensión de vista
// dibuja lo registrado en la escena de cada vista, tras el tonemap (D33).

#pragma once

#include "CoreMinimal.h"
#include "RenderResource.h"
#include "RHIResources.h"

class FPrimitiveSceneProxy;
class FRHICommandListImmediate;

/** Splats empaquetados en el formato de la GPU, en el espacio local del componente. */
struct FDbvSplatGpuData
{
	/** xyz local y opacidad en [0, 1]. */
	TArray<FVector4f> PosOpacity;
	/** Dos por splat: (c00, c01, c02, c11) y (c12, c22, color RGB8 como float, 0). */
	TArray<FVector4f> Cov;
	/** Caja de los centros, en espacio local. */
	FBox Bounds = FBox(EForceInit::ForceInit);

	int32 Num() const { return PosOpacity.Num(); }
};

/** Parámetros de una vista para proyectar una nube. */
struct FDbvSplatViewParams
{
	FMatrix44f LocalToView;
	FVector4f ProjectionTerms;
	FVector4f OutputRect;
	FVector2f DepthRange;
};

/** Una nube de splats en la GPU, con sus búferes de trabajo por vista (se reutilizan de una vista a otra). */
class DBVUNREALAICPPRENDER_API FDbvSplatGpuCloud : public FRenderResource
{
public:
	explicit FDbvSplatGpuCloud(FDbvSplatGpuData&& InData);

	virtual void InitRHI(FRHICommandListBase& RHICmdList) override;
	virtual void ReleaseRHI() override;
	virtual FString GetFriendlyName() const override { return TEXT("FDbvSplatGpuCloud"); }

	int32 GetNumSplats() const { return NumSplats; }
	const FBox& GetLocalBounds() const { return LocalBounds; }

	/** Proyecta para una vista y ordena de atrás adelante (hilo de render). */
	void ProjectAndSort(FRHICommandListImmediate& RHICmdList, const FDbvSplatViewParams& View, ERHIFeatureLevel::Type InFeatureLevel) const;

	FRHIShaderResourceView* GetProjectedSRV() const { return ProjectedSRV; }
	FRHIShaderResourceView* GetSortedIndicesSRV() const { return IndicesSRV; }

private:
	FDbvSplatGpuData Data;
	int32 NumSplats = 0;
	FBox LocalBounds;

	FBufferRHIRef PosBuffer, CovBuffer, ProjectedBuffer, IndicesBuffer;
	FShaderResourceViewRHIRef PosSRV, CovSRV, ProjectedSRV, IndicesSRV;
	FUnorderedAccessViewRHIRef ProjectedUAV, IndicesUAV;
	FBufferRHIRef KeyBuffers[2], ValueBuffers[2];
	FShaderResourceViewRHIRef KeySRVs[2], ValueSRVs[2];
	FUnorderedAccessViewRHIRef KeyUAVs[2], ValueUAVs[2];
};

using FDbvSplatGpuCloudRef = TSharedPtr<FDbvSplatGpuCloud, ESPMode::ThreadSafe>;

namespace DbvSplatRender
{
	/** Da de alta un proxy con su nube (hilo de render). */
	DBVUNREALAICPPRENDER_API void Register(const FPrimitiveSceneProxy* Proxy, FDbvSplatGpuCloudRef Cloud);

	/** Da de baja un proxy (hilo de render). */
	DBVUNREALAICPPRENDER_API void Unregister(const FPrimitiveSceneProxy* Proxy);
}
