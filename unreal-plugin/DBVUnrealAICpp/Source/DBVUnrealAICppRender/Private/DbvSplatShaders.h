// =============================================================================
// DBV Unreal AI — Shaders globales del render de splats
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61

#pragma once

#include "CoreMinimal.h"
#include "GlobalShader.h"
#include "ShaderParameterStruct.h"
#include "SceneView.h"

class FDbvSplatProjectCS : public FGlobalShader
{
	DECLARE_GLOBAL_SHADER(FDbvSplatProjectCS);
	SHADER_USE_PARAMETER_STRUCT(FDbvSplatProjectCS, FGlobalShader);

	BEGIN_SHADER_PARAMETER_STRUCT(FParameters, )
		SHADER_PARAMETER_SRV(StructuredBuffer<float4>, SplatPosOpacity)
		SHADER_PARAMETER_SRV(StructuredBuffer<float4>, SplatCov)
		SHADER_PARAMETER_SRV(StructuredBuffer<uint>, SplatSH)
		SHADER_PARAMETER_UAV(RWStructuredBuffer<FDbvProjected>, OutProjected)
		SHADER_PARAMETER_UAV(RWBuffer<uint>, OutKeys)
		SHADER_PARAMETER_UAV(RWBuffer<uint>, OutValues)
		SHADER_PARAMETER_UAV(RWBuffer<uint>, OutDrawArgs)
		SHADER_PARAMETER(FMatrix44f, LocalToView)
		SHADER_PARAMETER(FVector4f, ProjectionTerms)
		SHADER_PARAMETER(FVector4f, OutputRect)
		SHADER_PARAMETER(FVector2f, DepthRange)
		SHADER_PARAMETER(FVector3f, CameraLocal)
		SHADER_PARAMETER(uint32, NumSplats)
		SHADER_PARAMETER(uint32, SHDegree)
		SHADER_PARAMETER(uint32, SHStride)
	END_SHADER_PARAMETER_STRUCT()

	static bool ShouldCompilePermutation(const FGlobalShaderPermutationParameters& Parameters)
	{
		return IsFeatureLevelSupported(Parameters.Platform, ERHIFeatureLevel::SM5);
	}
};

class FDbvSplatVS : public FGlobalShader
{
	DECLARE_GLOBAL_SHADER(FDbvSplatVS);
	SHADER_USE_PARAMETER_STRUCT(FDbvSplatVS, FGlobalShader);

	BEGIN_SHADER_PARAMETER_STRUCT(FParameters, )
		SHADER_PARAMETER_SRV(StructuredBuffer<FDbvProjected>, Projected)
		SHADER_PARAMETER_SRV(Buffer<uint>, SortedIndices)
		SHADER_PARAMETER(FVector4f, TargetSizeAndInv)
	END_SHADER_PARAMETER_STRUCT()

	static bool ShouldCompilePermutation(const FGlobalShaderPermutationParameters& Parameters)
	{
		return IsFeatureLevelSupported(Parameters.Platform, ERHIFeatureLevel::SM5);
	}
};

class FDbvSplatPS : public FGlobalShader
{
	DECLARE_GLOBAL_SHADER(FDbvSplatPS);
	SHADER_USE_PARAMETER_STRUCT(FDbvSplatPS, FGlobalShader);

	BEGIN_SHADER_PARAMETER_STRUCT(FParameters, )
		SHADER_PARAMETER_STRUCT_REF(FViewUniformShaderParameters, View)
		SHADER_PARAMETER_RDG_TEXTURE(Texture2D, SceneDepthTexture)
		SHADER_PARAMETER_SAMPLER(SamplerState, PointSampler)
		SHADER_PARAMETER(FVector4f, OutputRect)
	END_SHADER_PARAMETER_STRUCT()

	static bool ShouldCompilePermutation(const FGlobalShaderPermutationParameters& Parameters)
	{
		return IsFeatureLevelSupported(Parameters.Platform, ERHIFeatureLevel::SM5);
	}
};
