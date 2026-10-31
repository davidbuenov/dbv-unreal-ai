// =============================================================================
// DBV Unreal AI — Empaquetado de splats en el formato de la GPU
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-61, REQ-62

#include "DbvSplatPack.h"
#include "HAL/IConsoleManager.h"
#include "RHI.h"

#include <algorithm>

static TAutoConsoleVariable<int32> CVarDbvSplatVRAMBudgetMB(
	TEXT("dbv.Splat.VRAMBudgetMB"),
	0,
	TEXT("VRAM budget per Gaussian Splatting cloud in MB (0 = 40% of the dedicated video memory). Applied when a splat is loaded."),
	ECVF_Default);

namespace DbvSplatPack
{
	void Reset(FDbvSplatGpuData& Data, int32 Num, int32 SHDegree)
	{
		Data = FDbvSplatGpuData();
		Data.SHDegree = FMath::Clamp(SHDegree, 0, 3);
		Data.SHStride = (RestCoefficients(Data.SHDegree) * 3 + 1) / 2;
		Data.PosOpacity.SetNumZeroed(Num);
		Data.Cov.SetNumZeroed(Num * 2);
		Data.SH.SetNumZeroed(Num * Data.SHStride);
	}

	void Set(FDbvSplatGpuData& Data, int32 Index, const FVector3f& Position, const FVector3f& Scale, const FVector4f& QuatWXYZ, float Opacity, const FVector3f& Rgb)
	{
		const float Length = FMath::Sqrt(QuatWXYZ.X * QuatWXYZ.X + QuatWXYZ.Y * QuatWXYZ.Y + QuatWXYZ.Z * QuatWXYZ.Z + QuatWXYZ.W * QuatWXYZ.W);
		const FVector4f Q = Length > UE_SMALL_NUMBER ? QuatWXYZ / Length : FVector4f(1.0f, 0.0f, 0.0f, 0.0f);
		const float W = Q.X, X = Q.Y, Y = Q.Z, Z = Q.W;

		// Σ = R · S² · Rᵀ, simétrica: (0,0) (0,1) (0,2) (1,1) (1,2) (2,2).
		const float R[3][3] = {
			{ 1.0f - 2.0f * (Y * Y + Z * Z), 2.0f * (X * Y - W * Z), 2.0f * (X * Z + W * Y) },
			{ 2.0f * (X * Y + W * Z), 1.0f - 2.0f * (X * X + Z * Z), 2.0f * (Y * Z - W * X) },
			{ 2.0f * (X * Z - W * Y), 2.0f * (Y * Z + W * X), 1.0f - 2.0f * (X * X + Y * Y) },
		};
		const float S2[3] = { Scale.X * Scale.X, Scale.Y * Scale.Y, Scale.Z * Scale.Z };
		auto Sigma = [&R, &S2](int32 A, int32 B) { return R[A][0] * R[B][0] * S2[0] + R[A][1] * R[B][1] * S2[1] + R[A][2] * R[B][2] * S2[2]; };

		const FColor Color = FLinearColor(Rgb.X, Rgb.Y, Rgb.Z).QuantizeRound();
		const uint32 PackedRgb = uint32(Color.R) | (uint32(Color.G) << 8) | (uint32(Color.B) << 16);
		Data.PosOpacity[Index] = FVector4f(Position, FMath::Clamp(Opacity, 0.0f, 1.0f));
		Data.Cov[Index * 2] = FVector4f(Sigma(0, 0), Sigma(0, 1), Sigma(0, 2), Sigma(1, 1));
		Data.Cov[Index * 2 + 1] = FVector4f(Sigma(1, 2), Sigma(2, 2), FMath::AsFloat(PackedRgb), 0.0f);
		Data.Bounds += FVector(Position);
	}

	void SetSH(FDbvSplatGpuData& Data, int32 Index, int32 Coefficient, const FVector3f& Value)
	{
		const float Channels[3] = { Value.X, Value.Y, Value.Z };
		for (int32 Channel = 0; Channel < 3; ++Channel)
		{
			const int32 Half = Coefficient * 3 + Channel;
			uint32& Word = Data.SH[Index * Data.SHStride + Half / 2];
			const uint32 Bits = FFloat16(Channels[Channel]).Encoded;
			const uint32 Shift = (Half % 2) * 16;
			Word = (Word & ~(0xFFFFu << Shift)) | (Bits << Shift);
		}
	}

	FVector3f GetSH(const FDbvSplatGpuData& Data, int32 Index, int32 Coefficient)
	{
		float Channels[3];
		for (int32 Channel = 0; Channel < 3; ++Channel)
		{
			const int32 Half = Coefficient * 3 + Channel;
			FFloat16 Value;
			Value.Encoded = uint16((Data.SH[Index * Data.SHStride + Half / 2] >> ((Half % 2) * 16)) & 0xFFFF);
			Channels[Channel] = Value.GetFloat();
		}
		return FVector3f(Channels[0], Channels[1], Channels[2]);
	}

	int32 KeepMostVisible(FDbvSplatGpuData& Data, int32 MaxSplats)
	{
		const int32 Num = Data.Num();
		if (Num <= MaxSplats)
		{
			return Num;
		}

		// Opacidad por tamaño (traza de la covarianza): lo que más se ve de toda la escena, elegido en tiempo
		// lineal y en el orden del fichero. @spec REQ-61, REQ-63
		TArray<float> Scores;
		Scores.SetNumUninitialized(Num);
		for (int32 i = 0; i < Num; ++i)
		{
			const FVector4f& A = Data.Cov[i * 2];
			const FVector4f& B = Data.Cov[i * 2 + 1];
			Scores[i] = Data.PosOpacity[i].W * (A.X + A.W + B.Y);
		}
		TArray<int32> Order;
		Order.SetNumUninitialized(Num);
		for (int32 i = 0; i < Num; ++i)
		{
			Order[i] = i;
		}
		std::nth_element(Order.GetData(), Order.GetData() + MaxSplats, Order.GetData() + Num, [&Scores](int32 L, int32 R) { return Scores[L] > Scores[R]; });
		Order.SetNum(MaxSplats);
		Order.Sort();

		FDbvSplatGpuData Kept;
		Kept.SHDegree = Data.SHDegree;
		Kept.SHStride = Data.SHStride;
		Kept.PosOpacity.Reserve(MaxSplats);
		Kept.Cov.Reserve(MaxSplats * 2);
		Kept.SH.Reserve(MaxSplats * Data.SHStride);
		for (const int32 i : Order)
		{
			Kept.PosOpacity.Add(Data.PosOpacity[i]);
			Kept.Cov.Add(Data.Cov[i * 2]);
			Kept.Cov.Add(Data.Cov[i * 2 + 1]);
			for (int32 w = 0; w < Data.SHStride; ++w)
			{
				Kept.SH.Add(Data.SH[i * Data.SHStride + w]);
			}
			Kept.Bounds += FVector(FVector3f(Data.PosOpacity[i]));
		}
		Data = MoveTemp(Kept);
		return MaxSplats;
	}

	int64 GpuBytesPerSplat(int32 SHStride)
	{
		// Subido: posición y opacidad (16), covarianza y color (32) y SH (4 por uint). Por vista: proyectado (24),
		// dos claves y dos valores de la ordenación (16) e índices ordenados (4).
		return 16 + 32 + int64(SHStride) * 4 + 24 + 16 + 4;
	}

	int64 VramBudgetBytes()
	{
		const int64 Configured = int64(CVarDbvSplatVRAMBudgetMB.GetValueOnAnyThread()) * 1024 * 1024;
		FTextureMemoryStats Stats;
		RHIGetTextureMemoryStats(Stats);
		const int64 Dedicated = Stats.DedicatedVideoMemory > 0 ? Stats.DedicatedVideoMemory : int64(4) * 1024 * 1024 * 1024;
		return Configured > 0 ? Configured : Dedicated * 2 / 5;
	}

	void FitBudget(FDbvSplatGpuData& Data, int64 BudgetBytes)
	{
		if (Data.Num() * GpuBytesPerSplat(Data.SHStride) > BudgetBytes && Data.SHStride > 0)
		{
			Data.SH.Empty();
			Data.SHDegree = 0;
			Data.SHStride = 0;
		}
		const int64 MaxSplats = BudgetBytes / GpuBytesPerSplat(Data.SHStride);
		KeepMostVisible(Data, int32(FMath::Clamp<int64>(MaxSplats, 1, MAX_int32)));
	}

	void ScaleSize(FDbvSplatGpuData& Data, float Factor)
	{
		const float Factor2 = Factor * Factor;
		for (int32 i = 0; i < Data.Num(); ++i)
		{
			FVector4f& A = Data.Cov[i * 2];
			FVector4f& B = Data.Cov[i * 2 + 1];
			A *= Factor2;
			B.X *= Factor2;
			B.Y *= Factor2;
		}
	}
}
