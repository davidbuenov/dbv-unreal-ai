// =============================================================================
// DBV Unreal AI — Cliente de Fab con la sesión EOS del propio editor
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-101, REQ-103, REQ-104
//
// Ver DbvFabClient.h. La plataforma EOS de Fab se reconoce así (P-29, R-48): el plugin `Fab` la crea con
// `IEOSSDKManager::CreatePlatform(EOS_Platform_Options&)`, sin nombre de configuración
// (FabAuthentication.cpp de UE 5.8.4), y tiene una cuenta Epic con sesión iniciada. Si no hay exactamente
// una plataforma así, no se envía ningún token. La operación `platforms` enseña lo que hay (sin el token)
// para fijar la regla en el spike.

#include "DbvFabClient.h"
#include "DbvFabDetail.h"

#include "Async/Async.h"
#include "FabDownloader.h"
#include "HAL/FileManager.h"
#include "Dom/JsonObject.h"
#include "GenericPlatform/GenericPlatformHttp.h"
#include "HAL/PlatformTime.h"
#include "HttpModule.h"
#include "IEOSSDKManager.h"
#include "Interfaces/IHttpRequest.h"
#include "Interfaces/IHttpResponse.h"
#include "Misc/EngineVersion.h"
#include "Misc/Guid.h"
#include "Misc/ScopeLock.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Policies/CondensedJsonPrintPolicy.h"
#include "Serialization/JsonWriter.h"

#include "eos_auth.h"
#include "eos_sdk.h"

DEFINE_LOG_CATEGORY_STATIC(LogDbvFab, Log, All);

namespace
{
	const FString FabHost = TEXT("https://www.fab.com");
	constexpr double JobLifetimeSeconds = 600.0;
	constexpr float RequestTimeoutSeconds = 30.0f;

	struct FFabJob
	{
		FString State = TEXT("running");
		FString ResultJson;
		FString Error;
		TSharedPtr<IHttpRequest, ESPMode::ThreadSafe> Request;
		TSharedPtr<FFabDownloadRequest> Download;
		FString StagingDir;
		float Progress = -1.0f;
		double StartedAt = 0.0;
	};

	/** Suelta una descarga fuera de la pila actual: su propio callback puede ser quien termina el trabajo. */
	void ReleaseLater(TSharedPtr<FFabDownloadRequest> Download)
	{
		if (Download.IsValid())
		{
			AsyncTask(ENamedThreads::GameThread, [Download]() {});
		}
	}

	/** Cancela la descarga de un trabajo y borra su carpeta temporal. Con `JobsLock` tomado. */
	void AbortJobLocked(FFabJob& Job)
	{
		if (Job.Request.IsValid())
		{
			Job.Request->OnProcessRequestComplete().Unbind();
			Job.Request->CancelRequest();
		}
		if (Job.Download.IsValid())
		{
			Job.Download->OnDownloadComplete().Clear();
			Job.Download->OnDownloadProgress().Clear();
			Job.Download->Cancel();
			ReleaseLater(Job.Download);
		}
		if (!Job.StagingDir.IsEmpty())
		{
			IFileManager::Get().DeleteDirectory(*Job.StagingDir, false, true);
		}
	}

	FCriticalSection JobsLock;
	TMap<FString, FFabJob> Jobs;

	FString ToJson(const TSharedRef<FJsonObject>& Object)
	{
		FString Out;
		const TSharedRef<TJsonWriter<TCHAR, TCondensedJsonPrintPolicy<TCHAR>>> Writer = TJsonWriterFactory<TCHAR, TCondensedJsonPrintPolicy<TCHAR>>::Create(&Out);
		FJsonSerializer::Serialize(Object, Writer);
		return Out;
	}

	FString ErrorJson(const FString& Code, const FString& Message)
	{
		const TSharedRef<FJsonObject> Object = MakeShared<FJsonObject>();
		Object->SetBoolField(TEXT("success"), false);
		Object->SetStringField(TEXT("error"), Code);
		Object->SetStringField(TEXT("message"), DbvFab::RedactSecrets(Message));
		return ToJson(Object);
	}

	void Finish(const FString& JobId, const FString& ResultJson, const FString& Error)
	{
		FScopeLock Lock(&JobsLock);
		if (FFabJob* Job = Jobs.Find(JobId))
		{
			Job->State = Error.IsEmpty() ? TEXT("done") : TEXT("error");
			Job->ResultJson = ResultJson;
			Job->Error = Error;
			Job->Request.Reset();
			ReleaseLater(Job->Download);
			Job->Download.Reset();
		}
	}

	void PurgeOldJobs()
	{
		const double Now = FPlatformTime::Seconds();
		FScopeLock Lock(&JobsLock);
		for (auto It = Jobs.CreateIterator(); It; ++It)
		{
			if (Now - It->Value.StartedAt > JobLifetimeSeconds)
			{
				AbortJobLocked(It->Value);
				It.RemoveCurrent();
			}
		}
	}

	FString Utf8(const char* Text)
	{
		return Text ? FString(UTF8_TO_TCHAR(Text)) : FString();
	}

	/** Lo que se sabe de una plataforma EOS activa, sin el token. */
	struct FPlatformInfo
	{
		EOS_HPlatform Handle = nullptr;
		FString ConfigName;
		int32 LoggedIn = 0;
		FString App;
		bool bCandidate = false;
	};

	TArray<FPlatformInfo> InspectPlatforms()
	{
		TArray<FPlatformInfo> Infos;
		IEOSSDKManager* Manager = IEOSSDKManager::Get();
		if (Manager && Manager->IsInitialized())
		{
			for (const IEOSPlatformHandlePtr& Platform : Manager->GetActivePlatforms())
			{
				if (!Platform.IsValid())
				{
					continue;
				}
				FPlatformInfo Info;
				Info.Handle = *Platform;
				Info.ConfigName = Platform->GetConfigName();
				const EOS_HAuth Auth = EOS_Platform_GetAuthInterface(Info.Handle);
				Info.LoggedIn = Auth ? EOS_Auth_GetLoggedInAccountsCount(Auth) : 0;
				if (Info.LoggedIn > 0)
				{
					const EOS_EpicAccountId Account = EOS_Auth_GetLoggedInAccountByIndex(Auth, 0);
					EOS_Auth_CopyUserAuthTokenOptions Options = {};
					Options.ApiVersion = EOS_AUTH_COPYUSERAUTHTOKEN_API_LATEST;
					EOS_Auth_Token* Token = nullptr;
					if (EOS_Auth_CopyUserAuthToken(Auth, &Options, Account, &Token) == EOS_EResult::EOS_Success && Token)
					{
						Info.App = Utf8(Token->App);
						EOS_Auth_Token_Release(Token);
					}
				}
				Info.bCandidate = Info.ConfigName.IsEmpty() && Info.LoggedIn > 0;
				Infos.Add(Info);
			}
		}
		return Infos;
	}

	/** @spec REQ-103 Copia el token de la única plataforma candidata. `OutReason` explica por qué no hay. */
	bool CopyFabToken(FString& OutToken, FString& OutReason)
	{
		bool bOk = false;
		TArray<FPlatformInfo> Candidates = InspectPlatforms().FilterByPredicate([](const FPlatformInfo& Info) { return Info.bCandidate; });
		if (Candidates.Num() == 0)
		{
			OutReason = TEXT("no_session");
		}
		else if (Candidates.Num() > 1)
		{
			OutReason = TEXT("ambiguous_session");
		}
		else
		{
			const EOS_HAuth Auth = EOS_Platform_GetAuthInterface(Candidates[0].Handle);
			const EOS_EpicAccountId Account = EOS_Auth_GetLoggedInAccountByIndex(Auth, 0);
			EOS_Auth_CopyUserAuthTokenOptions Options = {};
			Options.ApiVersion = EOS_AUTH_COPYUSERAUTHTOKEN_API_LATEST;
			EOS_Auth_Token* Token = nullptr;
			if (EOS_Auth_CopyUserAuthToken(Auth, &Options, Account, &Token) == EOS_EResult::EOS_Success && Token && Token->AccessToken)
			{
				OutToken = Utf8(Token->AccessToken);
				bOk = !OutToken.IsEmpty();
			}
			if (Token)
			{
				EOS_Auth_Token_Release(Token);
			}
			OutReason = bOk ? FString() : TEXT("no_session");
		}
		return bOk;
	}

	using FResponseHandler = TFunction<void(int32 /*Status*/, const FString& /*Body*/)>;

	/** GET a www.fab.com, con o sin sesión. Con sesión, un único reintento tras un 401, con el token copiado otra vez. */
	void SendGet(const FString& JobId, const FString& Url, bool bAuth, FResponseHandler OnDone, bool bRetried = false)
	{
		FString Token;
		FString Reason;
		if (!Url.StartsWith(FabHost + TEXT("/")))
		{
			Finish(JobId, ErrorJson(TEXT("insecure_host"), TEXT("The Fab token is only sent to https://www.fab.com.")), TEXT("insecure_host"));
		}
		else if (bAuth && !CopyFabToken(Token, Reason))
		{
			Finish(JobId, ErrorJson(Reason, TEXT("No Fab session in this editor. Click the Fab button in the Content Browser (it opens the Fab tab and signs you in), then try again.")), Reason);
		}
		else
		{
			TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
			Request->SetVerb(TEXT("GET"));
			Request->SetURL(Url);
			Request->SetTimeout(RequestTimeoutSeconds);
			Request->SetHeader(TEXT("accept"), TEXT("application/json"));
			if (bAuth)
			{
				Request->SetHeader(TEXT("Authorization"), FString::Printf(TEXT("Bearer %s"), *Token));
			}
			Token.Reset();
			Request->OnProcessRequestComplete().BindLambda(
				[JobId, Url, bAuth, OnDone, bRetried](FHttpRequestPtr, FHttpResponsePtr Response, bool bConnected)
				{
					const int32 Code = (bConnected && Response.IsValid()) ? Response->GetResponseCode() : 0;
					if (!DbvFab::Detail::IsRunning(JobId))
					{
						return;
					}
					if (Code == 401 && bAuth && !bRetried)
					{
						SendGet(JobId, Url, bAuth, OnDone, true);
					}
					else
					{
						OnDone(Code, Response.IsValid() ? Response->GetContentAsString() : FString());
					}
				});
			{
				FScopeLock Lock(&JobsLock);
				if (FFabJob* Job = Jobs.Find(JobId))
				{
					Job->Request = Request;
				}
			}
			Request->ProcessRequest();
		}
	}

	TSharedPtr<FJsonObject> ParseJson(const FString& Text)
	{
		TSharedPtr<FJsonObject> Object;
		const TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Text);
		FJsonSerializer::Deserialize(Reader, Object);
		return Object;
	}

	/** Mensaje del servidor (`detail` o `message`) o el principio del cuerpo, sin secretos. */
	FString ServerMessage(const FString& Body)
	{
		FString Message;
		const TSharedPtr<FJsonObject> Object = ParseJson(Body);
		if (Object.IsValid() && !Object->TryGetStringField(TEXT("detail"), Message))
		{
			Object->TryGetStringField(TEXT("message"), Message);
		}
		if (Message.IsEmpty())
		{
			Message = Body.Left(300);
		}
		return DbvFab::RedactSecrets(Message);
	}

	FString HttpErrorCode(int32 Code)
	{
		FString Error = FString::Printf(TEXT("http_%d"), Code);
		if (Code == 0) { Error = TEXT("network"); }
		else if (Code == 401) { Error = TEXT("no_session"); }
		else if (Code == 403) { Error = TEXT("forbidden"); }
		else if (Code == 404) { Error = TEXT("not_found_or_no_agents_api"); }
		else if (Code == 429) { Error = TEXT("rate_limited"); }
		return Error;
	}

	/** Respuesta 2xx → `Build(Object)`; si no, error con el mensaje del servidor. */
	FResponseHandler JsonHandler(const FString& JobId, TFunction<TSharedRef<FJsonObject>(const TSharedPtr<FJsonObject>&)> Build)
	{
		return [JobId, Build](int32 Code, const FString& Body)
		{
			if (Code >= 200 && Code < 300)
			{
				const TSharedRef<FJsonObject> Result = Build(ParseJson(Body));
				Result->SetBoolField(TEXT("success"), true);
				Result->SetNumberField(TEXT("http_status"), Code);
				Finish(JobId, ToJson(Result), FString());
			}
			else
			{
				const FString Error = HttpErrorCode(Code);
				const TSharedRef<FJsonObject> Result = MakeShared<FJsonObject>();
				Result->SetBoolField(TEXT("success"), false);
				Result->SetStringField(TEXT("error"), Error);
				Result->SetNumberField(TEXT("http_status"), Code);
				Result->SetStringField(TEXT("message"), ServerMessage(Body));
				UE_LOG(LogDbvFab, Warning, TEXT("Fab request failed: HTTP %d (%s)"), Code, *Error);
				Finish(JobId, ToJson(Result), Error);
			}
		};
	}

	FString Arg(const TSharedPtr<FJsonObject>& Args, const TCHAR* Name)
	{
		FString Value;
		if (Args.IsValid())
		{
			Args->TryGetStringField(Name, Value);
		}
		return Value;
	}

	void AddQuery(FString& Query, const TCHAR* Name, const FString& Value)
	{
		if (!Value.IsEmpty())
		{
			Query += (Query.IsEmpty() ? TEXT("?") : TEXT("&")) + FString(Name) + TEXT("=") + FGenericPlatformHttp::UrlEncode(Value);
		}
	}

	/** @spec REQ-101 Biblioteca del usuario: `/p/agents/library/search`. */
	void StartLibrarySearch(const FString& JobId, const TSharedPtr<FJsonObject>& Args)
	{
		FString Query;
		AddQuery(Query, TEXT("q"), Arg(Args, TEXT("q")));
		AddQuery(Query, TEXT("asset_formats"), Arg(Args, TEXT("asset_formats")));
		AddQuery(Query, TEXT("listing_types"), Arg(Args, TEXT("listing_types")));
		AddQuery(Query, TEXT("cursor"), Arg(Args, TEXT("cursor")));
		AddQuery(Query, TEXT("count"), Arg(Args, TEXT("count")).IsEmpty() ? TEXT("12") : Arg(Args, TEXT("count")));
		SendGet(JobId, FabHost + TEXT("/p/agents/library/search") + Query, true, JsonHandler(JobId, [](const TSharedPtr<FJsonObject>& Body)
		{
			const TSharedRef<FJsonObject> Out = MakeShared<FJsonObject>();
			// Cada entrada de la biblioteca con su `listing` (el producto) y `canRequestDownloadUrl`: Rust la compacta
			// a la misma forma que el catálogo (miniatura, enlace, formatos compatibles). Sin `technicalDetails` (HTML largo).
			TArray<TSharedPtr<FJsonValue>> Entries;
			const TArray<TSharedPtr<FJsonValue>>* Results = nullptr;
			if (Body.IsValid() && Body->TryGetArrayField(TEXT("results"), Results))
			{
				for (const TSharedPtr<FJsonValue>& Result : *Results)
				{
					const TSharedPtr<FJsonObject>* Entry = nullptr;
					const TSharedPtr<FJsonObject>* Listing = nullptr;
					if (Result->TryGetObject(Entry) && (*Entry)->TryGetObjectField(TEXT("listing"), Listing))
					{
						const TArray<TSharedPtr<FJsonValue>>* Formats = nullptr;
						if ((*Listing)->TryGetArrayField(TEXT("assetFormats"), Formats))
						{
							for (const TSharedPtr<FJsonValue>& Format : *Formats)
							{
								const TSharedPtr<FJsonObject>* FormatObject = nullptr;
								const TSharedPtr<FJsonObject>* Specs = nullptr;
								if (Format->TryGetObject(FormatObject) && (*FormatObject)->TryGetObjectField(TEXT("technicalSpecs"), Specs))
								{
									(*Specs)->RemoveField(TEXT("technicalDetails"));
								}
							}
						}
						bool bCanDownload = false;
						(*Entry)->TryGetBoolField(TEXT("canRequestDownloadUrl"), bCanDownload);
						const TSharedRef<FJsonObject> Item = MakeShared<FJsonObject>();
						Item->SetObjectField(TEXT("listing"), *Listing);
						Item->SetBoolField(TEXT("canRequestDownloadUrl"), bCanDownload);
						Entries.Add(MakeShared<FJsonValueObject>(Item));
					}
				}
			}
			const TSharedPtr<FJsonObject>* Cursors = nullptr;
			FString Next;
			if (Body.IsValid() && Body->TryGetObjectField(TEXT("cursors"), Cursors))
			{
				(*Cursors)->TryGetStringField(TEXT("next"), Next);
			}
			Out->SetArrayField(TEXT("entries"), Entries);
			Out->SetStringField(TEXT("next_cursor"), Next);
			return Out;
		}));
	}

	/** @spec REQ-101 `download-info` de un listing. No devuelve las URLs firmadas: solo qué hay y cuándo caduca. */
	void StartDownloadInfo(const FString& JobId, const TSharedPtr<FJsonObject>& Args)
	{
		const FString Uid = Arg(Args, TEXT("listing_uid"));
		FString Query;
		AddQuery(Query, TEXT("assetFormat"), Arg(Args, TEXT("asset_format")));
		AddQuery(Query, TEXT("engineVersion"), Arg(Args, TEXT("engine_version")));
		AddQuery(Query, TEXT("platform"), TEXT("Windows"));
		if (Uid.IsEmpty())
		{
			Finish(JobId, ErrorJson(TEXT("bad_arguments"), TEXT("listing_uid is required.")), TEXT("bad_arguments"));
		}
		else
		{
			SendGet(JobId, FabHost + TEXT("/p/agents/listings/") + FGenericPlatformHttp::UrlEncode(Uid) + TEXT("/download-info") + Query, true, JsonHandler(JobId, [](const TSharedPtr<FJsonObject>& Body)
			{
				const TSharedRef<FJsonObject> Out = MakeShared<FJsonObject>();
				TArray<TSharedPtr<FJsonValue>> Entries;
				const TArray<TSharedPtr<FJsonValue>>* Infos = nullptr;
				if (Body.IsValid() && Body->TryGetArrayField(TEXT("downloadInfo"), Infos))
				{
					for (const TSharedPtr<FJsonValue>& Info : *Infos)
					{
						const TSharedPtr<FJsonObject>* Entry = nullptr;
						if (Info->TryGetObject(Entry))
						{
							const TSharedRef<FJsonObject> Item = MakeShared<FJsonObject>();
							for (const TCHAR* Field : { TEXT("type"), TEXT("assetFormat"), TEXT("artifactId"), TEXT("expires"), TEXT("fileName"), TEXT("fileId") })
							{
								FString Value;
								if ((*Entry)->TryGetStringField(Field, Value))
								{
									Item->SetStringField(Field, Value);
								}
							}
							Item->SetBoolField(TEXT("has_download_url"), (*Entry)->HasField(TEXT("downloadUrl")));
							TArray<TSharedPtr<FJsonValue>> Keys;
							for (const auto& Pair : (*Entry)->Values)
							{
								Keys.Add(MakeShared<FJsonValueString>(FString(Pair.Key)));
							}
							Item->SetArrayField(TEXT("fields"), Keys);
							Entries.Add(MakeShared<FJsonValueObject>(Item));
						}
					}
				}
				Out->SetArrayField(TEXT("entries"), Entries);
				return Out;
			}));
		}
	}

	/** @spec REQ-103 Estado de la sesión, sin datos personales. */
	FString SessionJson()
	{
		const TArray<FPlatformInfo> Infos = InspectPlatforms();
		const int32 Candidates = Infos.FilterByPredicate([](const FPlatformInfo& Info) { return Info.bCandidate; }).Num();
		const TSharedRef<FJsonObject> Out = MakeShared<FJsonObject>();
		Out->SetBoolField(TEXT("success"), true);
		Out->SetBoolField(TEXT("signed_in"), Candidates == 1);
		const FEngineVersion& Version = FEngineVersion::Current();
		Out->SetStringField(TEXT("engine_tag"), FString::Printf(TEXT("UE_%d.%d"), Version.GetMajor(), Version.GetMinor()));
		Out->SetStringField(TEXT("reason"), Candidates == 1 ? TEXT("") : (Candidates == 0 ? TEXT("no_session") : TEXT("ambiguous_session")));
		return ToJson(Out);
	}

	/** @spec REQ-101 Diagnóstico de P-29: plataformas EOS activas, sin token ni ids de cuenta. */
	FString PlatformsJson()
	{
		TArray<TSharedPtr<FJsonValue>> List;
		for (const FPlatformInfo& Info : InspectPlatforms())
		{
			const TSharedRef<FJsonObject> Item = MakeShared<FJsonObject>();
			Item->SetStringField(TEXT("config_name"), Info.ConfigName);
			Item->SetNumberField(TEXT("logged_in_accounts"), Info.LoggedIn);
			Item->SetStringField(TEXT("app"), Info.App);
			Item->SetBoolField(TEXT("candidate"), Info.bCandidate);
			List.Add(MakeShared<FJsonValueObject>(Item));
		}
		const TSharedRef<FJsonObject> Out = MakeShared<FJsonObject>();
		Out->SetBoolField(TEXT("success"), true);
		Out->SetBoolField(TEXT("eos_initialized"), IEOSSDKManager::Get() && IEOSSDKManager::Get()->IsInitialized());
		Out->SetArrayField(TEXT("platforms"), List);
		return ToJson(Out);
	}
}

namespace DbvFab::Detail
{
	void Finish(const FString& JobId, const FString& ResultJson, const FString& Error) { ::Finish(JobId, ResultJson, Error); }

	bool IsRunning(const FString& JobId)
	{
		FScopeLock Lock(&JobsLock);
		const FFabJob* Job = Jobs.Find(JobId);
		return Job && Job->State == TEXT("running");
	}

	void SetProgress(const FString& JobId, float Progress)
	{
		FScopeLock Lock(&JobsLock);
		if (FFabJob* Job = Jobs.Find(JobId))
		{
			Job->Progress = FMath::Clamp(Progress, 0.0f, 1.0f);
		}
	}

	void SetDownload(const FString& JobId, const TSharedPtr<FFabDownloadRequest>& Download, const FString& StagingDir)
	{
		FScopeLock Lock(&JobsLock);
		if (FFabJob* Job = Jobs.Find(JobId))
		{
			Job->Download = Download;
			Job->StagingDir = StagingDir;
		}
	}

	void Get(const FString& JobId, const FString& Url, bool bAuth, FResponseHandler OnDone) { SendGet(JobId, Url, bAuth, MoveTemp(OnDone)); }
	FString FabHost() { return ::FabHost; }
	TSharedPtr<FJsonObject> ParseJson(const FString& Text) { return ::ParseJson(Text); }
	FString ToJson(const TSharedRef<FJsonObject>& Object) { return ::ToJson(Object); }
	FString ErrorJson(const FString& Code, const FString& Message) { return ::ErrorJson(Code, Message); }
	FString HttpErrorCode(int32 Code) { return ::HttpErrorCode(Code); }
	FString ServerMessage(const FString& Body) { return ::ServerMessage(Body); }
	void AddQuery(FString& Query, const TCHAR* Name, const FString& Value) { ::AddQuery(Query, Name, Value); }
}

namespace DbvFab
{
	FString Start(const FString& Operation, const FString& ArgsJson)
	{
		PurgeOldJobs();
		const FString JobId = FGuid::NewGuid().ToString(EGuidFormats::Digits).ToLower();
		{
			FScopeLock Lock(&JobsLock);
			FFabJob& Job = Jobs.Add(JobId);
			Job.StartedAt = FPlatformTime::Seconds();
		}
		const TSharedPtr<FJsonObject> Args = ParseJson(ArgsJson.IsEmpty() ? TEXT("{}") : ArgsJson);
		if (Operation == TEXT("session"))
		{
			Finish(JobId, SessionJson(), FString());
		}
		else if (Operation == TEXT("platforms"))
		{
			Finish(JobId, PlatformsJson(), FString());
		}
		else if (Operation == TEXT("library_search"))
		{
			StartLibrarySearch(JobId, Args);
		}
		else if (Operation == TEXT("download_info"))
		{
			StartDownloadInfo(JobId, Args);
		}
		else if (Operation == TEXT("install"))
		{
			StartInstall(JobId, Args);
		}
		else
		{
			Finish(JobId, ErrorJson(TEXT("unknown_operation"), FString::Printf(TEXT("Unknown Fab operation \"%s\"."), *Operation)), TEXT("unknown_operation"));
		}
		const TSharedRef<FJsonObject> Out = MakeShared<FJsonObject>();
		Out->SetBoolField(TEXT("success"), true);
		Out->SetStringField(TEXT("job_id"), JobId);
		return ToJson(Out);
	}

	FString Status(const FString& JobId)
	{
		FString Result = ErrorJson(TEXT("unknown_job"), TEXT("No Fab job with that id (it may have finished and been read already)."));
		FScopeLock Lock(&JobsLock);
		if (const FFabJob* Job = Jobs.Find(JobId))
		{
			const TSharedRef<FJsonObject> Out = MakeShared<FJsonObject>();
			Out->SetStringField(TEXT("state"), Job->State);
			if (Job->Progress >= 0.0f)
			{
				Out->SetNumberField(TEXT("progress"), Job->Progress);
			}
			if (Job->State != TEXT("running"))
			{
				const TSharedPtr<FJsonObject> Inner = ParseJson(Job->ResultJson);
				if (Inner.IsValid())
				{
					Out->SetObjectField(TEXT("result"), Inner);
				}
				Jobs.Remove(JobId);
			}
			Result = ToJson(Out);
		}
		return Result;
	}

	FString Cancel(const FString& JobId)
	{
		bool bFound = false;
		{
			FScopeLock Lock(&JobsLock);
			if (FFabJob* Job = Jobs.Find(JobId))
			{
				AbortJobLocked(*Job);
				Jobs.Remove(JobId);
				bFound = true;
			}
		}
		const TSharedRef<FJsonObject> Out = MakeShared<FJsonObject>();
		Out->SetBoolField(TEXT("success"), bFound);
		return ToJson(Out);
	}

	FString RedactSecrets(const FString& Text)
	{
		FString Out = Text;
		// `Bearer <token>`: se corta hasta el siguiente espacio, comilla o fin.
		int32 Index = Out.Find(TEXT("Bearer "), ESearchCase::IgnoreCase);
		while (Index != INDEX_NONE)
		{
			const int32 Start = Index + 7;
			int32 End = Start;
			while (End < Out.Len() && !FChar::IsWhitespace(Out[End]) && Out[End] != TEXT('"') && Out[End] != TEXT('\''))
			{
				++End;
			}
			Out = Out.Left(Start) + TEXT("[redacted]") + Out.Mid(End);
			Index = Out.Find(TEXT("Bearer "), ESearchCase::IgnoreCase, ESearchDir::FromStart, Start + 10);
		}
		// `"access_token": "..."` / `"accessToken":"..."`
		for (const TCHAR* Key : { TEXT("\"access_token\""), TEXT("\"accessToken\"") })
		{
			int32 KeyIndex = Out.Find(Key, ESearchCase::IgnoreCase);
			while (KeyIndex != INDEX_NONE)
			{
				const int32 Open = Out.Find(TEXT("\""), ESearchCase::CaseSensitive, ESearchDir::FromStart, KeyIndex + FCString::Strlen(Key));
				const int32 Close = Open == INDEX_NONE ? INDEX_NONE : Out.Find(TEXT("\""), ESearchCase::CaseSensitive, ESearchDir::FromStart, Open + 1);
				if (Close == INDEX_NONE)
				{
					break;
				}
				Out = Out.Left(Open + 1) + TEXT("[redacted]") + Out.Mid(Close);
				KeyIndex = Out.Find(Key, ESearchCase::IgnoreCase, ESearchDir::FromStart, Open + 12);
			}
		}
		return Out;
	}
}
