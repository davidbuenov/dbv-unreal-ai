// =============================================================================
// DBV Unreal AI — Toolset C++: delegación en el paquete Python
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-06, REQ-25, REQ-26

#include "DBVUnrealAICppToolset.h"

#include "IPythonScriptPlugin.h"
#include "Misc/Base64.h"

namespace
{
	/** Llama a `dbv_unreal_ai.cpp_bridge.<Function>(<args en base64>)` y devuelve el JSON que imprime. */
	FString CallPython(const FString& Function, const TArray<FString>& Arguments)
	{
		FString Result = TEXT("{\"success\": false, \"error\": \"Python Script Plugin is not available\"}");
		IPythonScriptPlugin* Python = IPythonScriptPlugin::Get();
		if (Python && Python->IsPythonAvailable())
		{
			TArray<FString> Encoded;
			for (const FString& Argument : Arguments)
			{
				Encoded.Add(FString::Printf(TEXT("'%s'"), *FBase64::Encode(Argument)));
			}
			FPythonCommandEx Command;
			Command.ExecutionMode = EPythonCommandExecutionMode::ExecuteFile;
			Command.FileExecutionScope = EPythonFileExecutionScope::Private;
			Command.Command = FString::Printf(TEXT("from dbv_unreal_ai import cpp_bridge\nprint(cpp_bridge.%s(%s))"), *Function, *FString::Join(Encoded, TEXT(", ")));
			const bool bOk = Python->ExecPythonCommandEx(Command);
			FString Output;
			for (const FPythonLogOutputEntry& Entry : Command.LogOutput)
			{
				if (Entry.Type == EPythonLogOutputType::Info)
				{
					Output += Entry.Output;
				}
			}
			Result = bOk ? Output.TrimStartAndEnd() : FString::Printf(TEXT("{\"success\": false, \"error\": %s}"), *Command.CommandResult.ReplaceCharWithEscapedChar());
		}
		return Result;
	}
}

FString UDBVUnrealAICppToolset::run_python(const FString& code)
{
	return CallPython(TEXT("run_python"), { code });
}

FString UDBVUnrealAICppToolset::validate_python(const FString& code)
{
	return CallPython(TEXT("validate_python"), { code });
}

FString UDBVUnrealAICppToolset::describe_python_api(const FString& name)
{
	return CallPython(TEXT("describe_python_api"), { name });
}

FString UDBVUnrealAICppToolset::get_project_info()
{
	return CallPython(TEXT("get_project_info"), {});
}

FString UDBVUnrealAICppToolset::import_model(const FString& source_file, const FString& folder_path, const FString& asset_name)
{
	return CallPython(TEXT("import_model"), { source_file, folder_path, asset_name });
}

FString UDBVUnrealAICppToolset::place_asset(const FString& asset_path, float distance, const FString& folder)
{
	return CallPython(TEXT("place_asset"), { asset_path, FString::SanitizeFloat(distance), folder });
}

FString UDBVUnrealAICppToolset::play_animation(const FString& actor_label, const FString& animation_path, bool looping)
{
	return CallPython(TEXT("play_animation"), { actor_label, animation_path, looping ? TEXT("true") : TEXT("false") });
}

FString UDBVUnrealAICppToolset::get_scene_summary(const FString& name_filter, const FString& class_filter, int32 limit)
{
	return CallPython(TEXT("get_scene_summary"), { name_filter, class_filter, FString::FromInt(limit) });
}
