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
// Nombres de funciones y parámetros en snake_case A PROPÓSITO (excepción al estilo de Unreal): el
// ToolsetRegistry deriva de ellos los nombres de herramienta y de argumento, y así coinciden con la
// variante Python (`run_python(code)`, `import_model(source_file, …)`): un único contrato (R-02b).
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
	static FString run_python(const FString& code);

	/** Check Python code WITHOUT running it against the running engine API. Returns JSON (ok, syntax_error, findings). */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString validate_python(const FString& code);

	/** Reference of an unreal Python API name from the running engine. Returns JSON (exists, kind, deprecated, doc, members). */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString describe_python_api(const FString& name);

	/** Engine version, project, open level and toolset version. Returns JSON. */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString get_project_info();

	/** Import a 3D model file (glTF/GLB, FBX, OBJ) letting the engine choose the importer. Returns JSON (success, imported, primary). */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString import_model(const FString& source_file, const FString& folder_path, const FString& asset_name);

	/** Spawn an asset in front of the editor camera inside a World Outliner folder. Returns JSON (success, actor, location). */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString place_asset(const FString& asset_path, float distance, const FString& folder);

	/** Compact list of the actors of the open level. Returns JSON (total, counts, actors). */
	UFUNCTION(meta = (AICallable), Category = "DBV Unreal AI")
	static FString get_scene_summary(const FString& name_filter, const FString& class_filter, int32 limit);
};
