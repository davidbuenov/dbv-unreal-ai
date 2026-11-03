// =============================================================================
// DBV Unreal AI — Actor de Gaussian Splatting en niveles de Unreal Engine
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "DbvGaussianSplatComponent.h"
#include "DbvGaussianSplatActor.generated.h"

/**
 * Placeable Actor in Unreal Engine levels representing a 3D Gaussian Splatting scene.
 */
UCLASS(BlueprintType, Category = "DBV Unreal AI")
class DBVUNREALAICPP_API ADbvGaussianSplatActor : public AActor
{
	GENERATED_BODY()

public:
	ADbvGaussianSplatActor();

	/** The underlying Gaussian Splatting rendering component */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Gaussian Splat", meta = (AllowPrivateAccess = "true"))
	TObjectPtr<UDbvGaussianSplatComponent> SplatComponent;

	/** Load an .spz file directly from disk */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	bool LoadSpz(const FString& FilePath);

	/** Load a .ply file directly from disk */
	UFUNCTION(BlueprintCallable, Category = "Gaussian Splat")
	bool LoadPly(const FString& FilePath);
};
