// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.

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
