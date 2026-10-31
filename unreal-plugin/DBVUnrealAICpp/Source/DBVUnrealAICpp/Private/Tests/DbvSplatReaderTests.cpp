// =============================================================================
// DBV Unreal AI — Tests de Automation de los lectores de splats y del empaquetado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-62, REQ-63
//
// Se lanzan con «Automation RunTests DBV.Splats» (o AutomationTestToolset.RunTestsByFilter por MCP). No necesitan
// RHI: el proyecto anfitrión los ejecuta con -nullrhi (D39).

#include "Misc/AutomationTest.h"
#include "Misc/Compression.h"
#include "Misc/FileHelper.h"
#include "Misc/Paths.h"
#include "HAL/FileManager.h"
#include "DbvPlyReader.h"
#include "DbvSpzReader.h"
#include "DbvSplatPack.h"

#if WITH_DEV_AUTOMATION_TESTS

namespace
{
	constexpr EAutomationTestFlags DbvTestFlags = EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter;

	void Append(TArray<uint8>& Out, const void* Data, int32 Size)
	{
		Out.Append(static_cast<const uint8*>(Data), Size);
	}

	template <typename T>
	void AppendValue(TArray<uint8>& Out, T Value)
	{
		Append(Out, &Value, sizeof(T));
	}

	void AppendText(TArray<uint8>& Out, const FString& Text)
	{
		const FTCHARToUTF8 Utf8(*Text);
		Append(Out, Utf8.Get(), Utf8.Length());
	}

	/**
	 * Un .ply de 3DGS con tipos mezclados (una propiedad double y otra uchar que el lector ignora), SH de grado 3 y
	 * `Count` vértices. El vértice i tiene x = i, f_dc_0 = 0, f_rest_k = canal * 100 + k, opacidad 0 (sigmoide 0,5),
	 * escala log 0 (1) y un cuaternión sin normalizar (2, 0, 0, 0).
	 */
	TArray<uint8> MakePly(int32 Count, const FString& Format = TEXT("binary_little_endian"))
	{
		FString Header = FString::Printf(TEXT("ply\nformat %s 1.0\nelement vertex %d\nproperty float x\nproperty float y\nproperty float z\nproperty double nx\nproperty uchar red\n"), *Format, Count);
		Header += TEXT("property float f_dc_0\nproperty float f_dc_1\nproperty float f_dc_2\n");
		for (int32 k = 0; k < 45; ++k)
		{
			Header += FString::Printf(TEXT("property float f_rest_%d\n"), k);
		}
		Header += TEXT("property float opacity\nproperty float scale_0\nproperty float scale_1\nproperty float scale_2\nproperty float rot_0\nproperty float rot_1\nproperty float rot_2\nproperty float rot_3\nend_header\n");
		TArray<uint8> Bytes;
		AppendText(Bytes, Header);
		for (int32 i = 0; i < Count; ++i)
		{
			AppendValue<float>(Bytes, float(i));
			AppendValue<float>(Bytes, 2.0f);
			AppendValue<float>(Bytes, 3.0f);
			AppendValue<double>(Bytes, 9.0);
			AppendValue<uint8>(Bytes, 255);
			AppendValue<float>(Bytes, 0.0f);
			AppendValue<float>(Bytes, 0.0f);
			AppendValue<float>(Bytes, 0.0f);
			for (int32 k = 0; k < 45; ++k)
			{
				AppendValue<float>(Bytes, float((k / 15) * 100 + k % 15) / 1000.0f);
			}
			AppendValue<float>(Bytes, 0.0f);
			AppendValue<float>(Bytes, 0.0f);
			AppendValue<float>(Bytes, 0.0f);
			AppendValue<float>(Bytes, 0.0f);
			AppendValue<float>(Bytes, 2.0f);
			AppendValue<float>(Bytes, 0.0f);
			AppendValue<float>(Bytes, 0.0f);
			AppendValue<float>(Bytes, 0.0f);
		}
		return Bytes;
	}

	FString WriteTemp(const TArray<uint8>& Bytes, const TCHAR* Name)
	{
		const FString Path = FPaths::Combine(FPaths::AutomationTransientDir(), Name);
		FFileHelper::SaveArrayToFile(Bytes, *Path);
		return Path;
	}
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FDbvPlyReaderChunksTest, "DBV.Splats.PlyReader.ChunksTypesAndSH", DbvTestFlags)
bool FDbvPlyReaderChunksTest::RunTest(const FString& Parameters)
{
	// Lectura por bloques de 2 vértices: el último bloque va incompleto (5 = 2 + 2 + 1).
	const FString Path = WriteTemp(MakePly(5), TEXT("dbv_chunks.ply"));
	FDbvSplatHeader Header;
	FDbvSplatGpuData Data;
	FString Error;
	TestTrue(TEXT("reads"), FDbvPlyReader::LoadFromFile(Path, Header, Data, Error, 2));
	TestEqual(TEXT("splats"), Data.Num(), 5);
	TestEqual(TEXT("SH degree from 45 f_rest"), Data.SHDegree, 3);
	TestEqual(TEXT("x of the last vertex (other types skipped by size)"), Data.PosOpacity[4].X, 4.0f);
	TestEqual(TEXT("y"), Data.PosOpacity[4].Y, 2.0f);
	TestEqual(TEXT("opacity is the sigmoid"), Data.PosOpacity[4].W, 0.5f);
	// Cuaternión (2, 0, 0, 0) normalizado → identidad; escala 1 → covarianza identidad.
	TestEqual(TEXT("cov xx"), Data.Cov[0].X, 1.0f, 1e-5f);
	TestEqual(TEXT("cov xy"), Data.Cov[0].Y, 0.0f, 1e-5f);
	TestEqual(TEXT("cov zz"), Data.Cov[1].Y, 1.0f, 1e-5f);
	// f_rest: rojo 0..14, verde 100..114, azul 200..214 (÷1000) → coeficiente k = (r, g, b).
	const FVector3f K1 = DbvSplatPack::GetSH(Data, 3, 1);
	TestEqual(TEXT("SH k1 red"), K1.X, 0.001f, 1e-4f);
	TestEqual(TEXT("SH k1 green"), K1.Y, 0.101f, 1e-4f);
	TestEqual(TEXT("SH k1 blue"), K1.Z, 0.201f, 1e-4f);
	TestEqual(TEXT("SH k14 blue"), DbvSplatPack::GetSH(Data, 0, 14).Z, 0.214f, 1e-4f);
	IFileManager::Get().Delete(*Path);
	return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FDbvPlyReaderRejectsTest, "DBV.Splats.PlyReader.RejectsWithReason", DbvTestFlags)
bool FDbvPlyReaderRejectsTest::RunTest(const FString& Parameters)
{
	FDbvSplatHeader Header;
	FDbvSplatGpuData Data;
	FString Error;
	TestFalse(TEXT("ascii"), FDbvPlyReader::LoadFromMemory(MakePly(2, TEXT("ascii")), Header, Data, Error));
	TestTrue(TEXT("ascii reason"), Error.Contains(TEXT("binary_little_endian")));

	TArray<uint8> Truncated = MakePly(3);
	Truncated.SetNum(Truncated.Num() - 10);
	Error.Empty();
	TestFalse(TEXT("truncated"), FDbvPlyReader::LoadFromMemory(Truncated, Header, Data, Error));
	TestTrue(TEXT("truncated reason"), Error.Contains(TEXT("truncated")));

	TArray<uint8> FaceFirst;
	AppendText(FaceFirst, TEXT("ply\nformat binary_little_endian 1.0\nelement face 1\nproperty list uchar int vertex_indices\nelement vertex 1\nproperty float x\nend_header\n"));
	FaceFirst.AddZeroed(64);
	Error.Empty();
	TestFalse(TEXT("element before vertex"), FDbvPlyReader::LoadFromMemory(FaceFirst, Header, Data, Error));
	TestTrue(TEXT("element reason"), Error.Contains(TEXT("before the vertices")));

	Error.Empty();
	TestFalse(TEXT("missing file"), FDbvPlyReader::LoadFromFile(TEXT("Z:/no/existe.ply"), Header, Data, Error));
	TestFalse(TEXT("missing file reason"), Error.IsEmpty());
	return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FDbvSpzReaderTest, "DBV.Splats.SpzReader.V2AndAxes", DbvTestFlags)
bool FDbvSpzReaderTest::RunTest(const FString& Parameters)
{
	// Un .spz v2 de 1 punto con SH de grado 1, comprimido con gzip como los de World Labs.
	TArray<uint8> Raw;
	AppendValue<uint32>(Raw, 0x5053474E);
	AppendValue<uint32>(Raw, 2);
	AppendValue<uint32>(Raw, 1);
	AppendValue<uint8>(Raw, 1);  // grado de SH
	AppendValue<uint8>(Raw, 12); // bits fraccionarios
	AppendValue<uint8>(Raw, 0);
	AppendValue<uint8>(Raw, 0);
	const int32 One = 1 << 12;
	for (const int32 Coordinate : { One, 2 * One, 3 * One })
	{
		AppendValue<uint8>(Raw, uint8(Coordinate & 0xFF));
		AppendValue<uint8>(Raw, uint8((Coordinate >> 8) & 0xFF));
		AppendValue<uint8>(Raw, uint8((Coordinate >> 16) & 0xFF));
	}
	AppendValue<uint8>(Raw, 255);                    // alfa
	for (int32 c = 0; c < 3; ++c) { AppendValue<uint8>(Raw, 128); } // color: DC ≈ 0 → gris 0,5
	for (int32 c = 0; c < 3; ++c) { AppendValue<uint8>(Raw, 160); } // escala: exp(160/16 − 10) = 1
	for (int32 c = 0; c < 3; ++c) { AppendValue<uint8>(Raw, 128); } // rotación ≈ identidad
	for (int32 k = 0; k < 3; ++k) { for (int32 c = 0; c < 3; ++c) { AppendValue<uint8>(Raw, 192); } } // SH = 0,5

	TArray<uint8> Gzip;
	int32 CompressedSize = FCompression::CompressMemoryBound(NAME_Gzip, Raw.Num());
	Gzip.SetNumUninitialized(CompressedSize);
	TestTrue(TEXT("gzip"), FCompression::CompressMemory(NAME_Gzip, Gzip.GetData(), CompressedSize, Raw.GetData(), Raw.Num()));
	Gzip.SetNum(CompressedSize);

	FDbvSplatHeader Header;
	FDbvSplatGpuData Data;
	FString Error;
	TestTrue(TEXT("reads"), FDbvSpzReader::LoadFromMemory(Gzip, Header, Data, Error));
	TestEqual(TEXT("splats"), Data.Num(), 1);
	TestEqual(TEXT("SH degree"), Data.SHDegree, 1);
	// RUB → RDF: y y z cambian de signo.
	TestEqual(TEXT("x"), Data.PosOpacity[0].X, 1.0f, 1e-4f);
	TestEqual(TEXT("y flipped"), Data.PosOpacity[0].Y, -2.0f, 1e-4f);
	TestEqual(TEXT("z flipped"), Data.PosOpacity[0].Z, -3.0f, 1e-4f);
	TestEqual(TEXT("opacity"), Data.PosOpacity[0].W, 1.0f, 1e-4f);
	TestEqual(TEXT("cov xx (scale 1)"), Data.Cov[0].X, 1.0f, 1e-2f);
	// Grado 1: (y, z, x) → y y z cambian de signo al girar 180° alrededor de X.
	TestEqual(TEXT("SH k0 flipped"), DbvSplatPack::GetSH(Data, 0, 0).X, -0.5f, 1e-3f);
	TestEqual(TEXT("SH k1 flipped"), DbvSplatPack::GetSH(Data, 0, 1).X, -0.5f, 1e-3f);
	TestEqual(TEXT("SH k2 kept"), DbvSplatPack::GetSH(Data, 0, 2).X, 0.5f, 1e-3f);

	TArray<uint8> OldMagic = Raw;
	OldMagic[0] = 'S';
	Error.Empty();
	TestFalse(TEXT("wrong magic"), FDbvSpzReader::LoadFromMemory(OldMagic, Header, Data, Error));
	TestTrue(TEXT("magic reason"), Error.Contains(TEXT("NGSP")));
	return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FDbvSplatPackTest, "DBV.Splats.Pack.MostVisibleAndScale", DbvTestFlags)
bool FDbvSplatPackTest::RunTest(const FString& Parameters)
{
	// Diez splats; el i tiene opacidad i/10: se quedan los 3 más visibles, en el orden del fichero.
	FDbvSplatGpuData Data;
	DbvSplatPack::Reset(Data, 10, 1);
	for (int32 i = 0; i < 10; ++i)
	{
		DbvSplatPack::Set(Data, i, FVector3f(float(i), 0.0f, 0.0f), FVector3f(1.0f), FVector4f(1.0f, 0.0f, 0.0f, 0.0f), i / 10.0f, FVector3f(0.5f));
		DbvSplatPack::SetSH(Data, i, 2, FVector3f(float(i)));
	}
	TestEqual(TEXT("kept"), DbvSplatPack::KeepMostVisible(Data, 3), 3);
	TestEqual(TEXT("first kept"), Data.PosOpacity[0].X, 7.0f);
	TestEqual(TEXT("last kept"), Data.PosOpacity[2].X, 9.0f);
	TestEqual(TEXT("SH travels with its splat"), DbvSplatPack::GetSH(Data, 1, 2).X, 8.0f);
	DbvSplatPack::ScaleSize(Data, 2.0f);
	TestEqual(TEXT("cov scales with the square"), Data.Cov[0].X, 4.0f, 1e-4f);
	// Un cuaternión sin normalizar da la misma covarianza que normalizado.
	FDbvSplatGpuData A, B;
	DbvSplatPack::Reset(A, 1, 0);
	DbvSplatPack::Reset(B, 1, 0);
	DbvSplatPack::Set(A, 0, FVector3f::ZeroVector, FVector3f(1.0f, 2.0f, 3.0f), FVector4f(0.9f, 0.1f, 0.3f, 0.2f), 1.0f, FVector3f(0.5f));
	DbvSplatPack::Set(B, 0, FVector3f::ZeroVector, FVector3f(1.0f, 2.0f, 3.0f), FVector4f(0.9f, 0.1f, 0.3f, 0.2f) * 7.0f, 1.0f, FVector3f(0.5f));
	TestEqual(TEXT("normalised quaternion"), A.Cov[0].Y, B.Cov[0].Y, 1e-4f);
	return true;
}

#endif
