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

FString UDBVUnrealAICppToolset::RunPython(const FString& Code)
{
	return CallPython(TEXT("run_python"), { Code });
}

FString UDBVUnrealAICppToolset::ValidatePython(const FString& Code)
{
	return CallPython(TEXT("validate_python"), { Code });
}

FString UDBVUnrealAICppToolset::DescribePythonApi(const FString& Name)
{
	return CallPython(TEXT("describe_python_api"), { Name });
}

FString UDBVUnrealAICppToolset::GetProjectInfo()
{
	return CallPython(TEXT("get_project_info"), {});
}

FString UDBVUnrealAICppToolset::GetSceneSummary(const FString& NameFilter, const FString& ClassFilter, int32 Limit)
{
	return CallPython(TEXT("get_scene_summary"), { NameFilter, ClassFilter, FString::FromInt(Limit) });
}
