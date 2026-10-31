// =============================================================================
// DBV Unreal AI — Empaquetado de splats en el formato de la GPU
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61, REQ-62
//
// Los lectores (.ply, .spz) escriben directamente aquí (D37): posición y opacidad, covarianza 3D ya calculada,
// color base RGB8 y los armónicos esféricos de grado 1 a 3 en half, ordenados por coeficiente y canal
// (k0.r, k0.g, k0.b, k1.r…). Todo en la convención de los .ply de 3DGS (x derecha, y abajo, z delante).

#pragma once

#include "CoreMinimal.h"
#include "DbvSplatRender.h"

namespace DbvSplatPack
{
	/** Coeficientes de SH por canal sin contar el DC: 0, 3, 8 o 15. */
	inline int32 RestCoefficients(int32 SHDegree)
	{
		const int32 Degree = FMath::Clamp(SHDegree, 0, 3);
		return (Degree + 1) * (Degree + 1) - 1;
	}

	/** Prepara `Data` para `Num` splats con armónicos de grado `SHDegree`. */
	DBVUNREALAICPPRENDER_API void Reset(FDbvSplatGpuData& Data, int32 Num, int32 SHDegree);

	/**
	 * Escribe el splat `Index`. `QuatWXYZ` puede venir sin normalizar (los .ply de 3DGS lo guardan así); `Scale` es
	 * lineal; `Rgb` es el color base en [0, 1].
	 */
	DBVUNREALAICPPRENDER_API void Set(FDbvSplatGpuData& Data, int32 Index, const FVector3f& Position, const FVector3f& Scale, const FVector4f& QuatWXYZ, float Opacity, const FVector3f& Rgb);

	/** Escribe el coeficiente `Coefficient` (0..RestCoefficients-1) de los SH del splat `Index`, en RGB. */
	DBVUNREALAICPPRENDER_API void SetSH(FDbvSplatGpuData& Data, int32 Index, int32 Coefficient, const FVector3f& Value);

	/** Lee el coeficiente que escribió SetSH (para tests). */
	DBVUNREALAICPPRENDER_API FVector3f GetSH(const FDbvSplatGpuData& Data, int32 Index, int32 Coefficient);

	/** Color base desde el coeficiente DC de los SH: 0,5 + C0 · dc, recortado a [0, 1]. */
	inline FVector3f RgbFromDC(const FVector3f& DC)
	{
		constexpr float SH_C0 = 0.28209479177387814f;
		return FVector3f(FMath::Clamp(0.5f + SH_C0 * DC.X, 0.0f, 1.0f), FMath::Clamp(0.5f + SH_C0 * DC.Y, 0.0f, 1.0f), FMath::Clamp(0.5f + SH_C0 * DC.Z, 0.0f, 1.0f));
	}

	/** Se queda con los `MaxSplats` más visibles (opacidad × tamaño), en el orden del fichero. Devuelve cuántos quedan. */
	DBVUNREALAICPPRENDER_API int32 KeepMostVisible(FDbvSplatGpuData& Data, int32 MaxSplats);

	/** Multiplica el tamaño de todos los splats (la covarianza por `Factor`²). */
	DBVUNREALAICPPRENDER_API void ScaleSize(FDbvSplatGpuData& Data, float Factor);
}
