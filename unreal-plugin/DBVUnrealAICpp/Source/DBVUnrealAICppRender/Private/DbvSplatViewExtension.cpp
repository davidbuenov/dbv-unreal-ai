// =============================================================================
// DBV Unreal AI — Extensión de vista: dibuja los splats tras el tonemap
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61
//
// D33: los splats se mezclan sobre la imagen ya tonemapeada (espacio de pantalla, como se entrenan los 3DGS), así
// que su color no lo cambian la exposición ni el tonemapper. La profundidad se prueba en el píxel contra SceneDepth.

#include "DbvSplatViewExtension.h"
#include "DbvSplatRender.h"
#include "DbvSplatShaders.h"
#include "PostProcess/PostProcessMaterialInputs.h"
#include "PrimitiveSceneProxy.h"
#include "RenderGraphUtils.h"
#include "ScreenPass.h"
#include "SceneTexturesConfig.h"
#include "CommonRenderResources.h"
#include "PipelineStateCache.h"
#include "HAL/IConsoleManager.h"

DECLARE_GPU_STAT_NAMED(DbvSplats, TEXT("DBV Splats"));

static TAutoConsoleVariable<int32> CVarDbvSplatIndirectDraw(
	TEXT("dbv.Splat.IndirectDraw"),
	1,
	TEXT("1: draw only the visible splats counted on the GPU (indirect). 0: draw every splat (for diagnosis)."),
	ECVF_RenderThreadSafe);

namespace
{
	struct FDbvSplatEntry
	{
		const FPrimitiveSceneProxy* Proxy = nullptr;
		FDbvSplatGpuCloudRef Cloud;
	};

	/** Lo registrado; solo se toca en el hilo de render. */
	TArray<FDbvSplatEntry>& Entries()
	{
		static TArray<FDbvSplatEntry> Registered;
		return Registered;
	}

	BEGIN_SHADER_PARAMETER_STRUCT(FDbvSplatDrawParameters, )
		SHADER_PARAMETER_STRUCT_INCLUDE(FDbvSplatVS::FParameters, VS)
		SHADER_PARAMETER_STRUCT_INCLUDE(FDbvSplatPS::FParameters, PS)
		RENDER_TARGET_BINDING_SLOTS()
	END_SHADER_PARAMETER_STRUCT()

	/** Nubes de la escena de la vista que se ven en ella, de la más lejana a la más cercana. */
	TArray<FDbvSplatEntry> VisibleEntries(const FSceneView& View)
	{
		TArray<TPair<double, FDbvSplatEntry>> Found;
		for (const FDbvSplatEntry& Entry : Entries())
		{
			const bool bSameScene = View.Family && View.Family->Scene == &Entry.Proxy->GetScene();
			if (bSameScene && Entry.Cloud.IsValid() && Entry.Cloud->GetNumSplats() > 0 && Entry.Proxy->IsShown(&View))
			{
				const double Distance = FVector::DistSquared(Entry.Proxy->GetBounds().Origin, View.ViewMatrices.GetViewOrigin());
				Found.Emplace(Distance, Entry);
			}
		}
		Found.Sort([](const TPair<double, FDbvSplatEntry>& A, const TPair<double, FDbvSplatEntry>& B) { return A.Key > B.Key; });
		TArray<FDbvSplatEntry> Result;
		for (const TPair<double, FDbvSplatEntry>& Pair : Found)
		{
			Result.Add(Pair.Value);
		}
		return Result;
	}

	/** Matrices y rangos de una nube para esta vista, en píxeles de la textura de salida. */
	FDbvSplatViewParams MakeViewParams(const FSceneView& View, const FPrimitiveSceneProxy& Proxy, const FIntRect& OutputRect)
	{
		const FMatrix LocalToView = Proxy.GetLocalToWorld() * View.ViewMatrices.GetWorldToView();
		const FMatrix& Projection = View.ViewMatrices.GetViewToClipNoAA();
		const FBoxSphereBounds& Bounds = Proxy.GetBounds();
		const double CenterDepth = View.ViewMatrices.GetWorldToView().TransformPosition(Bounds.Origin).Z;
		const double MinDepth = FMath::Max(10.0, CenterDepth - Bounds.SphereRadius);
		const double MaxDepth = FMath::Max(MinDepth * 1.01, CenterDepth + Bounds.SphereRadius);

		FDbvSplatViewParams Params;
		Params.LocalToView = FMatrix44f(LocalToView);
		Params.ProjectionTerms = FVector4f(Projection.M[0][0], Projection.M[1][1], Projection.M[2][0], Projection.M[2][1]);
		Params.OutputRect = FVector4f(OutputRect.Min.X, OutputRect.Min.Y, OutputRect.Width(), OutputRect.Height());
		Params.DepthRange = FVector2f(MinDepth, 1.0 / FMath::Loge(MaxDepth / MinDepth));
		Params.CameraLocal = FVector3f(Proxy.GetLocalToWorld().InverseTransformPosition(View.ViewMatrices.GetViewOrigin()));
		return Params;
	}

	void AddDrawPass(FRDGBuilder& GraphBuilder, const FSceneView& View, const FDbvSplatEntry& Entry, FScreenPassRenderTarget Output, FRDGTextureRef SceneDepth)
	{
		const FIntPoint Extent = Output.Texture->Desc.Extent;
		const FIntRect Rect = Output.ViewRect;
		FGlobalShaderMap* ShaderMap = GetGlobalShaderMap(View.GetFeatureLevel());
		TShaderMapRef<FDbvSplatVS> VertexShader(ShaderMap);
		TShaderMapRef<FDbvSplatPS> PixelShader(ShaderMap);

		FDbvSplatDrawParameters* Parameters = GraphBuilder.AllocParameters<FDbvSplatDrawParameters>();
		Parameters->VS.Projected = Entry.Cloud->GetProjectedSRV();
		Parameters->VS.SortedIndices = Entry.Cloud->GetSortedIndicesSRV();
		Parameters->VS.TargetSizeAndInv = FVector4f(Extent.X, Extent.Y, 1.0f / Extent.X, 1.0f / Extent.Y);
		Parameters->PS.View = View.ViewUniformBuffer;
		Parameters->PS.SceneDepthTexture = SceneDepth;
		Parameters->PS.PointSampler = TStaticSamplerState<SF_Point, AM_Clamp, AM_Clamp, AM_Clamp>::GetRHI();
		Parameters->PS.OutputRect = FVector4f(Rect.Min.X, Rect.Min.Y, Rect.Width(), Rect.Height());
		Parameters->RenderTargets[0] = FRenderTargetBinding(Output.Texture, ERenderTargetLoadAction::ELoad);

		const int32 NumSplats = Entry.Cloud->GetNumSplats();
		FRHIBuffer* DrawArgs = CVarDbvSplatIndirectDraw.GetValueOnRenderThread() != 0 ? Entry.Cloud->GetDrawArgsBuffer() : nullptr;
		GraphBuilder.AddPass(
			RDG_EVENT_NAME("DbvSplatDraw (%d)", NumSplats),
			Parameters,
			ERDGPassFlags::Raster,
			[Parameters, VertexShader, PixelShader, Extent, DrawArgs, NumSplats](FRHICommandList& RHICmdList)
			{
				RHICmdList.SetViewport(0.0f, 0.0f, 0.0f, Extent.X, Extent.Y, 1.0f);
				FGraphicsPipelineStateInitializer Pso;
				RHICmdList.ApplyCachedRenderTargets(Pso);
				// Alfa premultiplicado, de atrás adelante; el alfa de destino no se toca.
				Pso.BlendState = TStaticBlendState<CW_RGBA, BO_Add, BF_One, BF_InverseSourceAlpha, BO_Add, BF_Zero, BF_One>::GetRHI();
				Pso.RasterizerState = TStaticRasterizerState<FM_Solid, CM_None>::GetRHI();
				Pso.DepthStencilState = TStaticDepthStencilState<false, CF_Always>::GetRHI();
				Pso.BoundShaderState.VertexDeclarationRHI = GEmptyVertexDeclaration.VertexDeclarationRHI;
				Pso.BoundShaderState.VertexShaderRHI = VertexShader.GetVertexShader();
				Pso.BoundShaderState.PixelShaderRHI = PixelShader.GetPixelShader();
				Pso.PrimitiveType = PT_TriangleList;
				SetGraphicsPipelineState(RHICmdList, Pso, 0);
				SetShaderParameters(RHICmdList, VertexShader, VertexShader.GetVertexShader(), Parameters->VS);
				SetShaderParameters(RHICmdList, PixelShader, PixelShader.GetPixelShader(), Parameters->PS);
				// Solo los visibles: la proyección los cuenta en la GPU y quedan los primeros tras ordenar.
				if (DrawArgs)
				{
					RHICmdList.DrawPrimitiveIndirect(DrawArgs, 0);
				}
				else
				{
					RHICmdList.DrawPrimitive(0, 2, NumSplats);
				}
			});
	}
}

namespace DbvSplatRender
{
	void Register(const FPrimitiveSceneProxy* Proxy, FDbvSplatGpuCloudRef Cloud)
	{
		check(IsInRenderingThread());
		Unregister(Proxy);
		Entries().Add({ Proxy, MoveTemp(Cloud) });
	}

	void Unregister(const FPrimitiveSceneProxy* Proxy)
	{
		check(IsInRenderingThread());
		Entries().RemoveAll([Proxy](const FDbvSplatEntry& Entry) { return Entry.Proxy == Proxy; });
	}
}

void FDbvSplatViewExtension::SubscribeToPostProcessingPass(EPostProcessingPass Pass, const FSceneView& View, FPostProcessingPassDelegateArray& InOutPassCallbacks, bool bIsPassEnabled)
{
	const bool bPerspective = View.ViewMatrices.IsPerspectiveProjection();
	if (Pass == EPostProcessingPass::Tonemap && bIsPassEnabled && bPerspective && Entries().Num() > 0)
	{
		InOutPassCallbacks.Add(FPostProcessingPassDelegate::CreateRaw(this, &FDbvSplatViewExtension::PostTonemap));
	}
}

FScreenPassTexture FDbvSplatViewExtension::PostTonemap(FRDGBuilder& GraphBuilder, const FSceneView& View, const FPostProcessMaterialInputs& Inputs)
{
	const TArray<FDbvSplatEntry> Visible = VisibleEntries(View);
	FScreenPassTexture Result;
	if (Visible.Num() == 0 || !Inputs.SceneTextures.SceneTextures)
	{
		Result = Inputs.ReturnUntouchedSceneColorForPostProcessing(GraphBuilder);
	}
	else
	{
		const FScreenPassTexture SceneColor = FScreenPassTexture::CopyFromSlice(GraphBuilder, Inputs.GetInput(EPostProcessMaterialInput::SceneColor));
		FScreenPassRenderTarget Output = Inputs.OverrideOutput;
		if (Output.IsValid())
		{
			AddDrawTexturePass(GraphBuilder, View, SceneColor, Output);
		}
		else
		{
			Output = FScreenPassRenderTarget(SceneColor, ERenderTargetLoadAction::ELoad);
		}

		RDG_EVENT_SCOPE_STAT(GraphBuilder, DbvSplats, "DbvSplats");
		FRDGTextureRef SceneDepth = Inputs.SceneTextures.SceneTextures->GetParameters()->SceneDepthTexture;
		const ERHIFeatureLevel::Type FeatureLevel = View.GetFeatureLevel();
		for (const FDbvSplatEntry& Entry : Visible)
		{
			const FDbvSplatViewParams Params = MakeViewParams(View, *Entry.Proxy, Output.ViewRect);
			FDbvSplatGpuCloudRef Cloud = Entry.Cloud;
			AddPass(GraphBuilder, RDG_EVENT_NAME("DbvSplatProjectSort"), [Cloud, Params, FeatureLevel](FRHICommandListImmediate& RHICmdList)
			{
				Cloud->ProjectAndSort(RHICmdList, Params, FeatureLevel);
			});
			AddDrawPass(GraphBuilder, View, Entry, Output, SceneDepth);
		}
		Result = FScreenPassTexture(Output);
	}
	return Result;
}
