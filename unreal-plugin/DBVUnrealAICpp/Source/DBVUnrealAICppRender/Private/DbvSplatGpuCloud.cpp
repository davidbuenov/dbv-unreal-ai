// =============================================================================
// DBV Unreal AI — Nube de splats en la GPU: subida única, proyección y orden
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61
//
// La proyección y la ordenación van en un pase inmediato con búferes propios y transiciones a mano, como hace el
// motor con su propia ordenación (GPUSortManager.cpp). SortGPUBuffers es la radix de las partículas: claves de 16 bits
// (4 pasadas de 4 bits) y valores de 32.

#include "DbvSplatRender.h"
#include "DbvSplatShaders.h"
#include "GPUSort.h"
#include "GlobalShader.h"
#include "RenderGraphUtils.h"
#include "RHICommandList.h"
#include "RHIBufferInitializer.h"

IMPLEMENT_GLOBAL_SHADER(FDbvSplatProjectCS, "/Plugin/DBVUnrealAICpp/Private/DbvSplat.usf", "ProjectCS", SF_Compute);
IMPLEMENT_GLOBAL_SHADER(FDbvSplatVS, "/Plugin/DBVUnrealAICpp/Private/DbvSplat.usf", "SplatVS", SF_Vertex);
IMPLEMENT_GLOBAL_SHADER(FDbvSplatPS, "/Plugin/DBVUnrealAICpp/Private/DbvSplat.usf", "SplatPS", SF_Pixel);

namespace
{
	/** Bytes de FDbvProjected en el shader. */
	constexpr uint32 ProjectedStride = 24;

	/** Bits útiles de la clave de profundidad. */
	constexpr uint32 SortKeyMask = 0xFFFF;

	template <typename T>
	FBufferRHIRef UploadStructured(FRHICommandListBase& RHICmdList, const TCHAR* Name, const TArray<T>& Values)
	{
		const uint32 Bytes = Values.Num() * sizeof(T);
		const FRHIBufferCreateDesc Desc = FRHIBufferCreateDesc::CreateStructured(Name, Bytes, sizeof(T))
			.AddUsage(EBufferUsageFlags::ShaderResource | EBufferUsageFlags::Static)
			.SetInitialState(ERHIAccess::SRVMask)
			.SetInitActionInitializer();
		FRHIBufferInitializer Initializer = RHICmdList.CreateBufferInitializer(Desc);
		Initializer.WriteData(Values.GetData(), Bytes);
		return Initializer.Finalize();
	}

	FBufferRHIRef CreateUintBuffer(FRHICommandListBase& RHICmdList, const TCHAR* Name, int32 Count)
	{
		const FRHIBufferCreateDesc Desc = FRHIBufferCreateDesc::CreateVertex(Name, Count * sizeof(uint32))
			.AddUsage(EBufferUsageFlags::ShaderResource | EBufferUsageFlags::UnorderedAccess)
			.SetInitialState(ERHIAccess::UAVCompute);
		return RHICmdList.CreateBuffer(Desc);
	}

	FShaderResourceViewRHIRef UintSRV(FRHICommandListBase& RHICmdList, FRHIBuffer* Buffer)
	{
		return RHICmdList.CreateShaderResourceView(Buffer, FRHIViewDesc::CreateBufferSRV().SetType(FRHIViewDesc::EBufferType::Typed).SetFormat(PF_R32_UINT));
	}

	FUnorderedAccessViewRHIRef UintUAV(FRHICommandListBase& RHICmdList, FRHIBuffer* Buffer)
	{
		return RHICmdList.CreateUnorderedAccessView(Buffer, FRHIViewDesc::CreateBufferUAV().SetType(FRHIViewDesc::EBufferType::Typed).SetFormat(PF_R32_UINT));
	}
}

FDbvSplatGpuCloud::FDbvSplatGpuCloud(FDbvSplatGpuData&& InData)
	: Data(MoveTemp(InData))
{
	NumSplats = Data.Num();
	LocalBounds = Data.Bounds;
	SHDegree = Data.SHStride > 0 ? Data.SHDegree : 0;
	SHStride = Data.SHStride;
}

void FDbvSplatGpuCloud::InitRHI(FRHICommandListBase& RHICmdList)
{
	if (NumSplats <= 0)
	{
		return;
	}

	PosBuffer = UploadStructured(RHICmdList, TEXT("DbvSplatPosOpacity"), Data.PosOpacity);
	CovBuffer = UploadStructured(RHICmdList, TEXT("DbvSplatCov"), Data.Cov);
	PosSRV = RHICmdList.CreateShaderResourceView(PosBuffer, FRHIViewDesc::CreateBufferSRV().SetTypeFromBuffer(PosBuffer));
	CovSRV = RHICmdList.CreateShaderResourceView(CovBuffer, FRHIViewDesc::CreateBufferSRV().SetTypeFromBuffer(CovBuffer));
	if (SHStride > 0 && Data.SH.Num() > 0)
	{
		// Armónicos esféricos en half (REQ-62); los usa la proyección para el color según el ángulo.
		SHBuffer = UploadStructured(RHICmdList, TEXT("DbvSplatSH"), Data.SH);
		SHSRV = RHICmdList.CreateShaderResourceView(SHBuffer, FRHIViewDesc::CreateBufferSRV().SetTypeFromBuffer(SHBuffer));
	}
	// Ya están en la GPU: la copia de la CPU sobra.
	Data = FDbvSplatGpuData();

	const FRHIBufferCreateDesc ProjectedDesc = FRHIBufferCreateDesc::CreateStructured(TEXT("DbvSplatProjected"), NumSplats * ProjectedStride, ProjectedStride)
		.AddUsage(EBufferUsageFlags::ShaderResource | EBufferUsageFlags::UnorderedAccess)
		.SetInitialState(ERHIAccess::UAVCompute);
	ProjectedBuffer = RHICmdList.CreateBuffer(ProjectedDesc);
	ProjectedSRV = RHICmdList.CreateShaderResourceView(ProjectedBuffer, FRHIViewDesc::CreateBufferSRV().SetTypeFromBuffer(ProjectedBuffer));
	ProjectedUAV = RHICmdList.CreateUnorderedAccessView(ProjectedBuffer, FRHIViewDesc::CreateBufferUAV().SetTypeFromBuffer(ProjectedBuffer));

	IndicesBuffer = CreateUintBuffer(RHICmdList, TEXT("DbvSplatSortedIndices"), NumSplats);
	IndicesSRV = UintSRV(RHICmdList, IndicesBuffer);
	IndicesUAV = UintUAV(RHICmdList, IndicesBuffer);
	for (int32 i = 0; i < 2; ++i)
	{
		KeyBuffers[i] = CreateUintBuffer(RHICmdList, TEXT("DbvSplatSortKeys"), NumSplats);
		KeySRVs[i] = UintSRV(RHICmdList, KeyBuffers[i]);
		KeyUAVs[i] = UintUAV(RHICmdList, KeyBuffers[i]);
		ValueBuffers[i] = CreateUintBuffer(RHICmdList, TEXT("DbvSplatSortValues"), NumSplats);
		ValueSRVs[i] = UintSRV(RHICmdList, ValueBuffers[i]);
		ValueUAVs[i] = UintUAV(RHICmdList, ValueBuffers[i]);
	}
}

void FDbvSplatGpuCloud::ReleaseRHI()
{
	PosSRV.SafeRelease(); CovSRV.SafeRelease(); SHSRV.SafeRelease(); SHBuffer.SafeRelease(); ProjectedSRV.SafeRelease(); IndicesSRV.SafeRelease();
	ProjectedUAV.SafeRelease(); IndicesUAV.SafeRelease();
	PosBuffer.SafeRelease(); CovBuffer.SafeRelease(); ProjectedBuffer.SafeRelease(); IndicesBuffer.SafeRelease();
	for (int32 i = 0; i < 2; ++i)
	{
		KeySRVs[i].SafeRelease(); KeyUAVs[i].SafeRelease(); KeyBuffers[i].SafeRelease();
		ValueSRVs[i].SafeRelease(); ValueUAVs[i].SafeRelease(); ValueBuffers[i].SafeRelease();
	}
}

void FDbvSplatGpuCloud::ProjectAndSort(FRHICommandListImmediate& RHICmdList, const FDbvSplatViewParams& View, ERHIFeatureLevel::Type InFeatureLevel) const
{
	if (NumSplats <= 0 || !PosBuffer.IsValid())
	{
		return;
	}

	TShaderMapRef<FDbvSplatProjectCS> ComputeShader(GetGlobalShaderMap(InFeatureLevel));
	FDbvSplatProjectCS::FParameters Parameters;
	Parameters.SplatPosOpacity = PosSRV;
	Parameters.SplatCov = CovSRV;
	Parameters.OutProjected = ProjectedUAV;
	Parameters.OutKeys = KeyUAVs[0];
	Parameters.OutValues = IndicesUAV;
	Parameters.LocalToView = View.LocalToView;
	Parameters.ProjectionTerms = View.ProjectionTerms;
	Parameters.OutputRect = View.OutputRect;
	Parameters.DepthRange = View.DepthRange;
	Parameters.NumSplats = NumSplats;

	RHICmdList.Transition({
		FRHITransitionInfo(ProjectedUAV, ERHIAccess::Unknown, ERHIAccess::UAVCompute),
		FRHITransitionInfo(KeyUAVs[0], ERHIAccess::Unknown, ERHIAccess::UAVCompute),
		FRHITransitionInfo(IndicesUAV, ERHIAccess::Unknown, ERHIAccess::UAVCompute),
	});
	FComputeShaderUtils::Dispatch(RHICmdList, ComputeShader, Parameters, FComputeShaderUtils::GetGroupCountWrapped(NumSplats, 256));
	RHICmdList.Transition({
		FRHITransitionInfo(KeyUAVs[0], ERHIAccess::UAVCompute, ERHIAccess::SRVCompute),
		FRHITransitionInfo(IndicesUAV, ERHIAccess::UAVCompute, ERHIAccess::SRVCompute),
	});

	FGPUSortBuffers SortBuffers;
	for (int32 i = 0; i < 2; ++i)
	{
		SortBuffers.RemoteKeySRVs[i] = KeySRVs[i];
		SortBuffers.RemoteKeyUAVs[i] = KeyUAVs[i];
		SortBuffers.RemoteValueSRVs[i] = ValueSRVs[i];
		SortBuffers.RemoteValueUAVs[i] = ValueUAVs[i];
	}
	SortBuffers.FirstValuesSRV = IndicesSRV;
	SortBuffers.FinalValuesUAV = IndicesUAV;
	SortGPUBuffers(RHICmdList, SortBuffers, 0, SortKeyMask, NumSplats, InFeatureLevel);

	RHICmdList.Transition({
		FRHITransitionInfo(IndicesUAV, ERHIAccess::Unknown, ERHIAccess::SRVMask),
		FRHITransitionInfo(ProjectedUAV, ERHIAccess::Unknown, ERHIAccess::SRVMask),
	});
}
