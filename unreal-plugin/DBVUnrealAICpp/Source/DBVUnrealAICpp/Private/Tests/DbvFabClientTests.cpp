// =============================================================================
// DBV Unreal AI — Tests de Automation del cliente de Fab
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-101, REQ-103
//
// Se lanzan con «Automation RunTests DBV.Fab». No tocan la red ni la sesión de Fab.

#include "Misc/AutomationTest.h"
#include "../DbvFabClient.h"

#if WITH_DEV_AUTOMATION_TESTS

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FDbvFabRedactTest, "DBV.Fab.RedactSecrets", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FDbvFabRedactTest::RunTest(const FString& Parameters)
{
	const FString Header = DbvFab::RedactSecrets(TEXT("Authorization: Bearer eyJhbGciOi.abc-123_x and more"));
	TestFalse(TEXT("el token del Bearer desaparece"), Header.Contains(TEXT("eyJhbGciOi")));
	TestTrue(TEXT("el resto del texto se conserva"), Header.EndsWith(TEXT(" and more")));

	const FString Json = DbvFab::RedactSecrets(TEXT("{\"access_token\": \"secret1\", \"accessToken\":\"secret2\", \"ok\": \"keep\"}"));
	TestFalse(TEXT("access_token se borra"), Json.Contains(TEXT("secret1")));
	TestFalse(TEXT("accessToken se borra"), Json.Contains(TEXT("secret2")));
	TestTrue(TEXT("los demás campos quedan"), Json.Contains(TEXT("keep")));

	const FString Quoted = DbvFab::RedactSecrets(TEXT("\"Bearer tok\" y 'bearer tok2' x"));
	TestFalse(TEXT("dos Bearer seguidos"), Quoted.Contains(TEXT("tok\"")) || Quoted.Contains(TEXT("tok2")));

	TestEqual(TEXT("sin secretos no cambia"), DbvFab::RedactSecrets(TEXT("HTTP 404 not found")), FString(TEXT("HTTP 404 not found")));
	return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FDbvFabJobsTest, "DBV.Fab.Jobs", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FDbvFabJobsTest::RunTest(const FString& Parameters)
{
	const FString Started = DbvFab::Start(TEXT("no_such_operation"), TEXT("{}"));
	TestTrue(TEXT("Start devuelve un job_id"), Started.Contains(TEXT("\"job_id\"")));
	const int32 IdStart = Started.Find(TEXT("\"job_id\":\"")) + 10;
	const FString JobId = Started.Mid(IdStart, 32);
	const FString First = DbvFab::Status(JobId);
	TestTrue(TEXT("una operación desconocida termina en error"), First.Contains(TEXT("\"state\":\"error\"")) && First.Contains(TEXT("unknown_operation")));
	TestTrue(TEXT("un trabajo leído terminado se borra"), DbvFab::Status(JobId).Contains(TEXT("unknown_job")));
	return true;
}

#endif
