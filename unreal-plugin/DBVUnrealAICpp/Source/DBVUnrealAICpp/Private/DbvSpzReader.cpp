// =============================================================================
// DBV Unreal AI — Implementación del lector de ficheros comprimidos SPZ
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-62, REQ-63
//
// Formato .spz de Niantic (versiones 2 y 3), comprimido con gzip. Cabecera de 16 bytes: magic «NGSP»
// (0x5053474E), versión, número de puntos, grado de SH, bits fraccionarios, flags y un byte reservado. Después, por
// bloques: posiciones (3 × 24 bits en punto fijo), alfas (1 byte), colores (3 bytes, el coeficiente DC), escalas
// (3 bytes, logaritmo), rotaciones (3 bytes en v2; 4 bytes «smallest three» en v3) y SH (1 byte por coeficiente y
// canal). El formato dice que los ejes van en RUB (los .ply de 3DGS, en RDF), pero World Labs, que es de donde llegan
// casi todos los .spz de la app, los guarda ya en RDF (OpenCV): sus visores giran la malla 180°. Con el giro RUB → RDF
// el desierto salía boca abajo (verificado con el usuario el 2026-10-31), así que por defecto no se gira.
// bConvertRubToRdf queda para un .spz que sí siga la convención del formato (Scaniverse).
//
// El lector anterior esperaba «SPSP»/«SPZ\0» y otro orden de bloques: los .spz nunca se habían dibujado bien con el
// actor propio (encontrado el 2026-10-31).

#include "DbvSpzReader.h"
#include "DbvSplatPack.h"
#include "HAL/PlatformFileManager.h"
#include "Misc/Compression.h"
#include "Misc/FileHelper.h"

DEFINE_LOG_CATEGORY_STATIC(LogDbvSpzReader, Log, All);

namespace
{
	constexpr uint32 SpzMagic = 0x5053474E; // "NGSP"
	constexpr int32 SpzHeaderSize = 16;
	constexpr float SpzColorScale = 0.15f;

	/** Girar 180° alrededor de X (RUB → RDF). Desactivado: World Labs guarda los .spz ya en RDF. */
	constexpr bool bConvertRubToRdf = false;

	/** Signo de cada coeficiente de SH (grados 1 a 3) al girar 180° alrededor de X: cambian los impares en y + z. */
	constexpr float RubToRdfSHSign[15] = { -1, -1, 1, -1, 1, 1, -1, 1, -1, 1, -1, -1, 1, -1, 1 };

	int32 UnpackInt24(const uint8* Ptr)
	{
		int32 Value = int32(Ptr[0]) | (int32(Ptr[1]) << 8) | (int32(Ptr[2]) << 16);
		return (Value & 0x00800000) ? (Value | int32(0xFF000000)) : Value;
	}

	/** Cuaternión (w, x, y, z) de una rotación v2: x, y, z en bytes y w deducido. */
	FVector4f UnpackRotationV2(const uint8* Ptr)
	{
		const FVector3f Xyz(Ptr[0] / 127.5f - 1.0f, Ptr[1] / 127.5f - 1.0f, Ptr[2] / 127.5f - 1.0f);
		const float W = FMath::Sqrt(FMath::Max(0.0f, 1.0f - Xyz.SizeSquared()));
		return FVector4f(W, Xyz.X, Xyz.Y, Xyz.Z);
	}

	/** Cuaternión (w, x, y, z) de una rotación v3 «smallest three»: 2 bits para el índice del mayor y 3 × 10 bits. */
	FVector4f UnpackRotationV3(const uint8* Ptr)
	{
		uint32 Packed = uint32(Ptr[0]) | (uint32(Ptr[1]) << 8) | (uint32(Ptr[2]) << 16) | (uint32(Ptr[3]) << 24);
		constexpr uint32 Mask = (1u << 9) - 1u;
		const int32 Largest = int32(Packed >> 30);
		float Xyzw[4] = { 0.0f, 0.0f, 0.0f, 0.0f };
		float SumSquares = 0.0f;
		for (int32 i = 3; i >= 0; --i)
		{
			if (i != Largest)
			{
				const uint32 Magnitude = Packed & Mask;
				const bool bNegative = ((Packed >> 9) & 1u) != 0;
				Packed >>= 10;
				Xyzw[i] = UE_INV_SQRT_2 * float(Magnitude) / float(Mask) * (bNegative ? -1.0f : 1.0f);
				SumSquares += Xyzw[i] * Xyzw[i];
			}
		}
		Xyzw[Largest] = FMath::Sqrt(FMath::Max(0.0f, 1.0f - SumSquares));
		return FVector4f(Xyzw[3], Xyzw[0], Xyzw[1], Xyzw[2]);
	}

	/** Descomprime gzip con el tamaño que dice su cola (ISIZE); un payload sin comprimir se devuelve tal cual. */
	bool Decompress(const TArray<uint8>& Payload, TArray<uint8>& Out, FString& OutError)
	{
		const bool bGzip = Payload.Num() >= 18 && Payload[0] == 0x1F && Payload[1] == 0x8B;
		bool bOk = true;
		if (!bGzip)
		{
			Out = Payload;
		}
		else
		{
			const int32 Tail = Payload.Num() - 4;
			const uint32 Size = uint32(Payload[Tail]) | (uint32(Payload[Tail + 1]) << 8) | (uint32(Payload[Tail + 2]) << 16) | (uint32(Payload[Tail + 3]) << 24);
			bOk = Size > 0 && Size < uint32(MAX_int32);
			if (bOk)
			{
				Out.SetNumUninitialized(int32(Size));
				bOk = FCompression::UncompressMemory(NAME_Gzip, Out.GetData(), Out.Num(), Payload.GetData(), Payload.Num());
			}
			if (!bOk)
			{
				OutError = TEXT("Failed to decompress the .spz gzip stream");
			}
		}
		return bOk;
	}
}

bool FDbvSpzReader::LoadFromFile(const FString& FilePath, FDbvSplatHeader& OutHeader, FDbvSplatGpuData& Out, FString& OutErrorMessage)
{
	TArray<uint8> Payload;
	if (!FFileHelper::LoadFileToArray(Payload, *FilePath))
	{
		OutErrorMessage = FString::Printf(TEXT("File does not exist or cannot be read: %s"), *FilePath);
		return false;
	}
	return LoadFromMemory(Payload, OutHeader, Out, OutErrorMessage);
}

bool FDbvSpzReader::LoadFromMemory(const TArray<uint8>& Payload, FDbvSplatHeader& OutHeader, FDbvSplatGpuData& Out, FString& OutErrorMessage)
{
	TArray<uint8> Raw;
	if (!Decompress(Payload, Raw, OutErrorMessage))
	{
		return false;
	}
	if (Raw.Num() < SpzHeaderSize)
	{
		OutErrorMessage = TEXT("The .spz payload is too small");
		return false;
	}

	const uint8* Bytes = Raw.GetData();
	uint32 Magic, Version, NumPoints;
	FMemory::Memcpy(&Magic, Bytes, 4);
	FMemory::Memcpy(&Version, Bytes + 4, 4);
	FMemory::Memcpy(&NumPoints, Bytes + 8, 4);
	const int32 SHDegree = Bytes[12];
	const int32 FractionalBits = Bytes[13];
	if (Magic != SpzMagic || (Version != 2 && Version != 3))
	{
		OutErrorMessage = FString::Printf(TEXT("Not a supported .spz file (magic 0x%08X, version %u; expected NGSP v2 or v3)"), Magic, Version);
		return false;
	}
	if (NumPoints == 0 || NumPoints > uint32(MAX_int32) || SHDegree > 3)
	{
		OutErrorMessage = FString::Printf(TEXT("Invalid .spz header: %u points, SH degree %d"), NumPoints, SHDegree);
		return false;
	}

	const int64 N = NumPoints;
	const int32 PerChannel = DbvSplatPack::RestCoefficients(SHDegree);
	const int64 RotationBytes = Version == 3 ? 4 : 3;
	const int64 PosOffset = SpzHeaderSize;
	const int64 AlphaOffset = PosOffset + N * 9;
	const int64 ColorOffset = AlphaOffset + N;
	const int64 ScaleOffset = ColorOffset + N * 3;
	const int64 RotOffset = ScaleOffset + N * 3;
	const int64 SHOffset = RotOffset + N * RotationBytes;
	const int64 Expected = SHOffset + N * PerChannel * 3;
	if (Raw.Num() < Expected)
	{
		OutErrorMessage = FString::Printf(TEXT(".spz data truncated: expected %lld bytes, got %d"), Expected, Raw.Num());
		return false;
	}

	OutHeader.NumPoints = int32(N);
	OutHeader.SHDegree = SHDegree;
	OutHeader.FractionalBits = FractionalBits;
	DbvSplatPack::Reset(Out, int32(N), SHDegree);
	const float PosScale = 1.0f / float(1 << FractionalBits);
	for (int32 i = 0; i < int32(N); ++i)
	{
		const uint8* P = Bytes + PosOffset + i * 9;
		const FVector3f Rub(UnpackInt24(P) * PosScale, UnpackInt24(P + 3) * PosScale, UnpackInt24(P + 6) * PosScale);
		const uint8* C = Bytes + ColorOffset + i * 3;
		const FVector3f DC((C[0] / 255.0f - 0.5f) / SpzColorScale, (C[1] / 255.0f - 0.5f) / SpzColorScale, (C[2] / 255.0f - 0.5f) / SpzColorScale);
		const uint8* S = Bytes + ScaleOffset + i * 3;
		const FVector3f Scale(FMath::Exp(S[0] / 16.0f - 10.0f), FMath::Exp(S[1] / 16.0f - 10.0f), FMath::Exp(S[2] / 16.0f - 10.0f));
		const uint8* R = Bytes + RotOffset + i * RotationBytes;
		const FVector4f Q = Version == 3 ? UnpackRotationV3(R) : UnpackRotationV2(R);
		const float Opacity = Bytes[AlphaOffset + i] / 255.0f;
		const float Flip = bConvertRubToRdf ? -1.0f : 1.0f;
		DbvSplatPack::Set(Out, i, FVector3f(Rub.X, Rub.Y * Flip, Rub.Z * Flip), Scale, FVector4f(Q.X, Q.Y, Q.Z * Flip, Q.W * Flip), Opacity, DbvSplatPack::RgbFromDC(DC));
		for (int32 k = 0; k < PerChannel; ++k)
		{
			const uint8* H = Bytes + SHOffset + (int64(i) * PerChannel + k) * 3;
			const float Sign = bConvertRubToRdf ? RubToRdfSHSign[k] : 1.0f;
			DbvSplatPack::SetSH(Out, i, k, FVector3f((H[0] - 128.0f) / 128.0f, (H[1] - 128.0f) / 128.0f, (H[2] - 128.0f) / 128.0f) * Sign);
		}
	}

	OutHeader.BoundingBox = Out.Bounds;
	UE_LOG(LogDbvSpzReader, Log, TEXT("Parsed SPZ v%u file: %d splats, SH degree %d, bounds %s"), Version, OutHeader.NumPoints, SHDegree, *OutHeader.BoundingBox.ToString());
	return true;
}
