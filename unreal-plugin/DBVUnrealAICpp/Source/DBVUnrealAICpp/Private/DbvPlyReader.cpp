// Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Developed using dbv-specs-ops methodology.
//
// @spec REQ-62, REQ-63
//
// El fichero se lee por bloques con tamaños de 64 bits: un .ply de 3DGS con armónicos esféricos pasa con facilidad
// de 2 GB (la iglesia de la prueba del 2026-10-31: 2,35 GB, 9.975.598 splats), y `LoadFileToArray` con un
// `TArray<uint8>` no admite más de 2 GB. Se aceptan propiedades de cualquier tipo escalar de PLY, no solo `float`.

#include "DbvPlyReader.h"
#include "DbvSplatPack.h"
#include "HAL/PlatformFileManager.h"
#include "Templates/UniquePtr.h"

DEFINE_LOG_CATEGORY_STATIC(LogDbvPlyReader, Log, All);

namespace
{
	/** Tipos escalares de PLY. */
	enum class EPlyType : uint8 { Unknown, Int8, UInt8, Int16, UInt16, Int32, UInt32, Float32, Float64 };

	EPlyType ParseType(const FString& Name)
	{
		static const TMap<FString, EPlyType> Types = {
			{TEXT("char"), EPlyType::Int8}, {TEXT("int8"), EPlyType::Int8},
			{TEXT("uchar"), EPlyType::UInt8}, {TEXT("uint8"), EPlyType::UInt8},
			{TEXT("short"), EPlyType::Int16}, {TEXT("int16"), EPlyType::Int16},
			{TEXT("ushort"), EPlyType::UInt16}, {TEXT("uint16"), EPlyType::UInt16},
			{TEXT("int"), EPlyType::Int32}, {TEXT("int32"), EPlyType::Int32},
			{TEXT("uint"), EPlyType::UInt32}, {TEXT("uint32"), EPlyType::UInt32},
			{TEXT("float"), EPlyType::Float32}, {TEXT("float32"), EPlyType::Float32},
			{TEXT("double"), EPlyType::Float64}, {TEXT("float64"), EPlyType::Float64},
		};
		const EPlyType* Found = Types.Find(Name.ToLower());
		return Found ? *Found : EPlyType::Unknown;
	}

	int32 TypeSize(EPlyType Type)
	{
		switch (Type)
		{
		case EPlyType::Int8:
		case EPlyType::UInt8: return 1;
		case EPlyType::Int16:
		case EPlyType::UInt16: return 2;
		case EPlyType::Int32:
		case EPlyType::UInt32:
		case EPlyType::Float32: return 4;
		case EPlyType::Float64: return 8;
		default: return 0;
		}
	}

	/** Una propiedad del vértice que el lector usa: dónde está dentro del vértice y de qué tipo es. */
	struct FPlyField
	{
		int32 Offset = -1;
		EPlyType Type = EPlyType::Unknown;

		bool IsSet() const { return Offset >= 0; }

		float Read(const uint8* Vertex, float Default) const
		{
			if (!IsSet())
			{
				return Default;
			}
			const uint8* P = Vertex + Offset;
			switch (Type)
			{
			case EPlyType::Int8: return static_cast<float>(*reinterpret_cast<const int8*>(P));
			case EPlyType::UInt8: return static_cast<float>(*P);
			case EPlyType::Int16: { int16 V; FMemory::Memcpy(&V, P, 2); return static_cast<float>(V); }
			case EPlyType::UInt16: { uint16 V; FMemory::Memcpy(&V, P, 2); return static_cast<float>(V); }
			case EPlyType::Int32: { int32 V; FMemory::Memcpy(&V, P, 4); return static_cast<float>(V); }
			case EPlyType::UInt32: { uint32 V; FMemory::Memcpy(&V, P, 4); return static_cast<float>(V); }
			case EPlyType::Float32: { float V; FMemory::Memcpy(&V, P, 4); return V; }
			case EPlyType::Float64: { double V; FMemory::Memcpy(&V, P, 8); return static_cast<float>(V); }
			default: return Default;
			}
		}
	};


	/** Lo que dice la cabecera: cuántos vértices, dónde empiezan, cuánto ocupa cada uno y dónde está cada campo. */
	struct FPlyLayout
	{
		int64 VertexCount = 0;
		int64 DataOffset = 0;
		int32 Stride = 0;
		FPlyField X, Y, Z, DC0, DC1, DC2, Opacity, Scale0, Scale1, Scale2, Rot0, Rot1, Rot2, Rot3;
		/** f_rest_* en el orden del fichero: primero todos los del rojo, luego el verde y el azul. @spec REQ-62 */
		TArray<FPlyField> Rest;
		int32 SHDegree = 0;
	};

	/** Interpreta la cabecera de `Data` (los primeros bytes del fichero). */
	bool ParseHeader(const uint8* Data, int64 Size, FPlyLayout& Out, FString& OutError)
	{
		bool bValid = Size >= 64 && FMemory::Memcmp(Data, "ply", 3) == 0;
		int64 EndHeader = -1;
		for (int64 i = 0; bValid && EndHeader < 0 && i + 10 <= Size; ++i)
		{
			if (FMemory::Memcmp(Data + i, "end_header", 10) == 0)
			{
				EndHeader = i + 10;
				// Un solo salto de línea (\n o \r\n): más bytes ya son datos binarios.
				EndHeader += (EndHeader < Size && Data[EndHeader] == '\r') ? 1 : 0;
				EndHeader += (EndHeader < Size && Data[EndHeader] == '\n') ? 1 : 0;
			}
		}
		if (!bValid || EndHeader < 0)
		{
			OutError = TEXT("Not a PLY file, or its header is missing end_header");
			return false;
		}

		const FString HeaderText(static_cast<int32>(EndHeader), reinterpret_cast<const ANSICHAR*>(Data));
		TArray<FString> Lines;
		HeaderText.ParseIntoArrayLines(Lines);

		TMap<FString, FPlyField> Fields;
		bool bInVertex = false;
		bool bVertexSeen = false;
		FString Format;
		for (const FString& RawLine : Lines)
		{
			TArray<FString> Words;
			RawLine.TrimStartAndEnd().ParseIntoArrayWS(Words);
			if (Words.Num() >= 2 && Words[0] == TEXT("format"))
			{
				Format = Words[1];
			}
			else if (Words.Num() >= 3 && Words[0] == TEXT("element"))
			{
				// Solo se leen los vértices, y tienen que ser el primer elemento (así lo escriben los entrenadores de 3DGS).
				bInVertex = Words[1] == TEXT("vertex") && !bVertexSeen;
				bVertexSeen = bVertexSeen || bInVertex;
				if (bInVertex)
				{
					Out.VertexCount = FCString::Atoi64(*Words[2]);
				}
				else if (!bVertexSeen)
				{
					OutError = FString::Printf(TEXT("The PLY element '%s' comes before the vertices: not supported"), *Words[1]);
					return false;
				}
			}
			else if (bInVertex && Words.Num() >= 3 && Words[0] == TEXT("property"))
			{
				const EPlyType Type = ParseType(Words[1]);
				if (Words[1] == TEXT("list") || Type == EPlyType::Unknown)
				{
					OutError = FString::Printf(TEXT("Unsupported PLY vertex property: %s"), *RawLine.TrimStartAndEnd());
					return false;
				}
				FPlyField Field;
				Field.Offset = Out.Stride;
				Field.Type = Type;
				Fields.Add(Words[2], Field);
				Out.Stride += TypeSize(Type);
			}
		}

		if (Format != TEXT("binary_little_endian"))
		{
			OutError = FString::Printf(TEXT("PLY format '%s' is not supported (only binary_little_endian)"), *Format);
			return false;
		}
		if (Out.VertexCount <= 0 || Out.VertexCount > MAX_int32 || Out.Stride <= 0)
		{
			OutError = TEXT("The PLY header does not give a valid vertex count and properties");
			return false;
		}

		auto Get = [&Fields](const TCHAR* Name) { const FPlyField* F = Fields.Find(Name); return F ? *F : FPlyField(); };
		Out.X = Get(TEXT("x")); Out.Y = Get(TEXT("y")); Out.Z = Get(TEXT("z"));
		Out.DC0 = Get(TEXT("f_dc_0")); Out.DC1 = Get(TEXT("f_dc_1")); Out.DC2 = Get(TEXT("f_dc_2"));
		Out.Opacity = Get(TEXT("opacity"));
		Out.Scale0 = Get(TEXT("scale_0")); Out.Scale1 = Get(TEXT("scale_1")); Out.Scale2 = Get(TEXT("scale_2"));
		Out.Rot0 = Get(TEXT("rot_0")); Out.Rot1 = Get(TEXT("rot_1")); Out.Rot2 = Get(TEXT("rot_2")); Out.Rot3 = Get(TEXT("rot_3"));

		// Los SH completos de un grado: 9, 24 o 45 f_rest (3, 8 o 15 por canal). Otra cantidad se ignora.
		int32 RestCount = 0;
		while (Fields.Contains(FString::Printf(TEXT("f_rest_%d"), RestCount)))
		{
			++RestCount;
		}
		Out.SHDegree = RestCount == 45 ? 3 : RestCount == 24 ? 2 : RestCount == 9 ? 1 : 0;
		for (int32 i = 0; Out.SHDegree > 0 && i < RestCount; ++i)
		{
			Out.Rest.Add(Fields[FString::Printf(TEXT("f_rest_%d"), i)]);
		}
		Out.DataOffset = EndHeader;
		return true;
	}

	/** Convierte `Count` vértices seguidos de `Data` en splats empaquetados, a partir del splat `First`. */
	void ConvertVertices(const uint8* Data, int64 Count, int64 First, const FPlyLayout& Layout, FDbvSplatGpuData& Out)
	{
		const int32 PerChannel = DbvSplatPack::RestCoefficients(Layout.SHDegree);
		for (int64 i = 0; i < Count; ++i)
		{
			const uint8* Vertex = Data + i * Layout.Stride;
			const int32 Index = static_cast<int32>(First + i);
			const FVector3f Position(Layout.X.Read(Vertex, 0.0f), Layout.Y.Read(Vertex, 0.0f), Layout.Z.Read(Vertex, 0.0f));
			// Escala guardada como logaritmo y opacidad antes de la sigmoide.
			const FVector3f Scale(
				Layout.Scale0.IsSet() ? FMath::Exp(Layout.Scale0.Read(Vertex, 0.0f)) : 1.0f,
				Layout.Scale1.IsSet() ? FMath::Exp(Layout.Scale1.Read(Vertex, 0.0f)) : 1.0f,
				Layout.Scale2.IsSet() ? FMath::Exp(Layout.Scale2.Read(Vertex, 0.0f)) : 1.0f);
			const FVector4f Rotation(Layout.Rot0.Read(Vertex, 1.0f), Layout.Rot1.Read(Vertex, 0.0f), Layout.Rot2.Read(Vertex, 0.0f), Layout.Rot3.Read(Vertex, 0.0f));
			const float Opacity = 1.0f / (1.0f + FMath::Exp(-Layout.Opacity.Read(Vertex, 0.0f)));
			const FVector3f DC(Layout.DC0.Read(Vertex, 0.0f), Layout.DC1.Read(Vertex, 0.0f), Layout.DC2.Read(Vertex, 0.0f));
			DbvSplatPack::Set(Out, Index, Position, Scale, Rotation, Opacity, DbvSplatPack::RgbFromDC(DC));
			for (int32 k = 0; k < PerChannel; ++k)
			{
				const FVector3f Coefficient(Layout.Rest[k].Read(Vertex, 0.0f), Layout.Rest[PerChannel + k].Read(Vertex, 0.0f), Layout.Rest[2 * PerChannel + k].Read(Vertex, 0.0f));
				DbvSplatPack::SetSH(Out, Index, k, Coefficient);
			}
		}
	}

	void BeginPoints(const FPlyLayout& Layout, FDbvSplatHeader& OutHeader, FDbvSplatGpuData& Out)
	{
		OutHeader.NumPoints = static_cast<int32>(Layout.VertexCount);
		OutHeader.SHDegree = Layout.SHDegree;
		OutHeader.FractionalBits = 0;
		DbvSplatPack::Reset(Out, OutHeader.NumPoints, Layout.SHDegree);
	}
}

bool FDbvPlyReader::LoadFromFile(const FString& FilePath, FDbvSplatHeader& OutHeader, FDbvSplatGpuData& Out, FString& OutErrorMessage, int64 VerticesPerChunk)
{
	TUniquePtr<IFileHandle> File(FPlatformFileManager::Get().GetPlatformFile().OpenRead(*FilePath));
	if (!File)
	{
		OutErrorMessage = FString::Printf(TEXT("File does not exist or cannot be opened: %s"), *FilePath);
		return false;
	}

	const int64 FileSize = File->Size();
	TArray64<uint8> Head;
	Head.SetNumUninitialized(FMath::Min<int64>(FileSize, 65536));
	FPlyLayout Layout;
	if (!File->Read(Head.GetData(), Head.Num()) || !ParseHeader(Head.GetData(), Head.Num(), Layout, OutErrorMessage))
	{
		OutErrorMessage = OutErrorMessage.IsEmpty() ? FString::Printf(TEXT("Failed to read file: %s"), *FilePath) : OutErrorMessage;
		return false;
	}

	const int64 Expected = Layout.DataOffset + Layout.VertexCount * Layout.Stride;
	if (FileSize < Expected)
	{
		OutErrorMessage = FString::Printf(TEXT("PLY file truncated: expected %lld bytes, got %lld"), Expected, FileSize);
		return false;
	}

	BeginPoints(Layout, OutHeader, Out);
	const int64 ChunkVertices = FMath::Max<int64>(1, VerticesPerChunk);
	TArray64<uint8> Chunk;
	Chunk.SetNumUninitialized(ChunkVertices * Layout.Stride);
	bool bRead = File->Seek(Layout.DataOffset);
	for (int64 First = 0; bRead && First < Layout.VertexCount; First += ChunkVertices)
	{
		const int64 Count = FMath::Min(ChunkVertices, Layout.VertexCount - First);
		bRead = File->Read(Chunk.GetData(), Count * Layout.Stride);
		if (bRead)
		{
			ConvertVertices(Chunk.GetData(), Count, First, Layout, Out);
		}
	}
	if (!bRead)
	{
		Out = FDbvSplatGpuData();
		OutErrorMessage = FString::Printf(TEXT("Failed to read the PLY vertex data: %s"), *FilePath);
		return false;
	}

	OutHeader.BoundingBox = Out.Bounds;
	UE_LOG(LogDbvPlyReader, Log, TEXT("Parsed PLY file: %d splats, SH degree %d (%lld bytes), bounds %s"), OutHeader.NumPoints, OutHeader.SHDegree, FileSize, *OutHeader.BoundingBox.ToString());
	return true;
}

bool FDbvPlyReader::LoadFromMemory(const TArray<uint8>& FileData, FDbvSplatHeader& OutHeader, FDbvSplatGpuData& Out, FString& OutErrorMessage)
{
	FPlyLayout Layout;
	if (!ParseHeader(FileData.GetData(), FileData.Num(), Layout, OutErrorMessage))
	{
		return false;
	}
	const int64 Expected = Layout.DataOffset + Layout.VertexCount * Layout.Stride;
	if (FileData.Num() < Expected)
	{
		OutErrorMessage = FString::Printf(TEXT("PLY file truncated: expected %lld bytes, got %d"), Expected, FileData.Num());
		return false;
	}
	BeginPoints(Layout, OutHeader, Out);
	ConvertVertices(FileData.GetData() + Layout.DataOffset, Layout.VertexCount, 0, Layout, Out);
	OutHeader.BoundingBox = Out.Bounds;
	return true;
}
