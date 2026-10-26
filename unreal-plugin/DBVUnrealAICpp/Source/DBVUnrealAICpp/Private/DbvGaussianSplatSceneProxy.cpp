// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

#include "DbvGaussianSplatSceneProxy.h"
#include "DbvGaussianSplatComponent.h"
#include "SceneManagement.h"
#include "DynamicMeshBuilder.h"
#include "Materials/Material.h"

FDbvGaussianSplatSceneProxy::FDbvGaussianSplatSceneProxy(const UDbvGaussianSplatComponent* InComponent)
	: FPrimitiveSceneProxy(InComponent)
	, bEnableDepthWrite(InComponent->bEnableDepthWrite)
	, AlphaDepthThreshold(InComponent->AlphaDepthThreshold)
	, SplatScaleMultiplier(InComponent->SplatScaleMultiplier)
{
	if (InComponent->SplatAsset)
	{
		Splats = InComponent->SplatAsset->GetPoints();
	}

	SplatMaterial = InComponent->GetMaterial(0);
	if (!SplatMaterial)
	{
		SplatMaterial = UMaterial::GetDefaultMaterial(MD_Surface);
	}
}

FDbvGaussianSplatSceneProxy::~FDbvGaussianSplatSceneProxy()
{
}

void FDbvGaussianSplatSceneProxy::GetDynamicMeshElements(
	const TArray<const FSceneView*>& Views,
	const FSceneViewFamily& ViewFamily,
	uint32 VisibilityMap,
	FMeshElementCollector& Collector) const
{
	if (Splats.Num() == 0)
	{
		return;
	}

	for (int32 ViewIndex = 0; ViewIndex < Views.Num(); ++ViewIndex)
	{
		if (!(VisibilityMap & (1 << ViewIndex)))
		{
			continue;
		}

		const FSceneView* View = Views[ViewIndex];
		const FVector ViewLocation = View->ViewMatrices.GetViewOrigin();
		const FVector CamForward = View->GetViewDirection();
		const FVector CamRight = View->GetViewRight();
		const FVector CamUp = View->GetViewUp();

		// Index array for depth sorting (argsort)
		const int32 NumToDraw = FMath::Min(Splats.Num(), 65536 * 4); // Safe batch cap for immediate dynamic mesh
		TArray<int32> SortedIndices;
		SortedIndices.SetNumUninitialized(NumToDraw);
		for (int32 i = 0; i < NumToDraw; ++i)
		{
			SortedIndices[i] = i;
		}

		// Sort back-to-front relative to camera plane (Painters algorithm for translucent blending)
		const FMatrix& LocalToWorldMatrix = GetLocalToWorld();
		SortedIndices.Sort([this, &ViewLocation, &LocalToWorldMatrix](int32 A, int32 B)
		{
			const FVector WorldPosA = LocalToWorldMatrix.TransformPosition(FVector(Splats[A].Position));
			const FVector WorldPosB = LocalToWorldMatrix.TransformPosition(FVector(Splats[B].Position));
			const float DistSqA = FVector::DistSquared(WorldPosA, ViewLocation);
			const float DistSqB = FVector::DistSquared(WorldPosB, ViewLocation);
			return DistSqA > DistSqB;
		});

		FDynamicMeshBuilder MeshBuilder(Collector.GetFeatureLevel());

		// Build camera-facing billboard quads for sorted splats
		for (int32 Idx = 0; Idx < NumToDraw; ++Idx)
		{
			const int32 SplatIdx = SortedIndices[Idx];
			const FDbvGaussianPoint& Pt = Splats[SplatIdx];

			// Splat radius in local space scaled by user multiplier
			const float Radius = (Pt.Scale.X + Pt.Scale.Y + Pt.Scale.Z) * 0.3333f * SplatScaleMultiplier;
			const FVector Center = FVector(Pt.Position);

			const FVector HalfRight = CamRight * Radius;
			const FVector HalfUp = CamUp * Radius;

			const FVector P0 = Center - HalfRight - HalfUp;
			const FVector P1 = Center + HalfRight - HalfUp;
			const FVector P2 = Center + HalfRight + HalfUp;
			const FVector P3 = Center - HalfRight + HalfUp;

			const FVector2f UV0(0.0f, 0.0f);
			const FVector2f UV1(1.0f, 0.0f);
			const FVector2f UV2(1.0f, 1.0f);
			const FVector2f UV3(0.0f, 1.0f);

			const FColor VertColor = Pt.Color.ToFColor(true);

			const int32 V0 = MeshBuilder.AddVertex(FVector3f(P0), UV0, FVector3f(CamRight), FVector3f(CamUp), FVector3f(CamForward), VertColor);
			const int32 V1 = MeshBuilder.AddVertex(FVector3f(P1), UV1, FVector3f(CamRight), FVector3f(CamUp), FVector3f(CamForward), VertColor);
			const int32 V2 = MeshBuilder.AddVertex(FVector3f(P2), UV2, FVector3f(CamRight), FVector3f(CamUp), FVector3f(CamForward), VertColor);
			const int32 V3 = MeshBuilder.AddVertex(FVector3f(P3), UV3, FVector3f(CamRight), FVector3f(CamUp), FVector3f(CamForward), VertColor);

			MeshBuilder.AddTriangle(V0, V1, V2);
			MeshBuilder.AddTriangle(V0, V2, V3);
		}

		MeshBuilder.GetMesh(LocalToWorldMatrix, SplatMaterial->GetRenderProxy(), SDPG_World, false, false, ViewIndex, Collector);
	}
}

FPrimitiveViewRelevance FDbvGaussianSplatSceneProxy::GetViewRelevance(const FSceneView* View) const
{
	FPrimitiveViewRelevance Result;
	Result.bDrawRelevance = IsShown(View);
	Result.bDynamicRelevance = true;
	Result.bShadowRelevance = false;
	Result.bVelocityRelevance = IsMovable();
	if (SplatMaterial)
	{
		SplatMaterial->GetRelevance_Concurrent(GetScene().GetShaderPlatform()).SetPrimitiveViewRelevance(Result);
	}
	return Result;
}

uint32 FDbvGaussianSplatSceneProxy::GetMemoryFootprint() const
{
	return sizeof(*this) + Splats.GetAllocatedSize();
}
