// =============================================================================
// DBV Unreal AI — Toolset C++ (misma interfaz que la variante Python)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-06
//
// Para proyectos con C++ cuyo arranque de Python no registre el toolset. Cada herramienta
// delega en el paquete Python `dbv_unreal_ai` (plugin DBVUnrealAI), así que las dos variantes
// comparten el mismo contrato y la misma batería de tests (R-02b). Los argumentos viajan a
// Python en base64 para que el código del usuario no rompa el literal.
//
// PLANTILLA SIN COMPILAR en v0.1 (ADR-V010-001, D8): verificar con UBT antes de ofrecerla.

#pragma once

#include "CoreMinimal.h"
#include "ToolsetRegistry/ToolsetDefinition.h"
#include "DBVUnrealAICppToolset.generated.h"

UCLASS(BlueprintType)
class UDBVUnrealAICppToolset : public UToolsetDefinition
{
	GENERATED_BODY()

public:
	/** Run Python inside the Unreal Editor in one undoable transaction. Returns JSON (success, stdout, stderr, warnings, error, result). */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString RunPython(const FString& Code);

	/** Check Python code WITHOUT running it against the running engine API. Returns JSON (ok, syntax_error, findings). */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString ValidatePython(const FString& Code);

	/** Reference of an unreal Python API name from the running engine. Returns JSON (exists, kind, deprecated, doc, members). */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString DescribePythonApi(const FString& Name);

	/** Engine version, project, open level and toolset version. Returns JSON. */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString GetProjectInfo();

	/** Compact list of the actors of the open level. Returns JSON (total, counts, actors). */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString GetSceneSummary(const FString& NameFilter, const FString& ClassFilter, int32 Limit);
};
