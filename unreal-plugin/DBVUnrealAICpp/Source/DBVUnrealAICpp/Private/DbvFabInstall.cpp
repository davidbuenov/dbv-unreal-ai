// =============================================================================
// DBV Unreal AI — Instalar un listing de Fab en el proyecto abierto
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-104
//
// Operación `install` (args: listing_uid, asset_format y, si hay que elegir, artifact_id). Pasos, todos en el hilo
// del juego y encadenados por callbacks:
//   1. Ficha del listing sin sesión: título, si es gratis, formatos y versiones de Unreal.
//   2. Reglas del usuario (2026-11-15), comprobadas AQUÍ y no solo en el prompt: el formato tiene que valer para
//      la versión del editor (`unreal-engine` con `UE_<mayor>.<menor>`); lo de pago solo si está en la biblioteca
//      del usuario (`no_owned` con el enlace, sin descargar). Desde la app no se compra nada.
//   3. `download-info` con sesión, justo antes de descargar (las URLs caducan en minutos: un único reintento).
//   4. `binary` → descarga HTTP en Saved/DBV/Fab/<job>/ e importación con nuestro `import_model` en
//      /Game/Fab/<Título>/. `manifest` → BuildPatchServices en la carpeta temporal y, al terminar, se mueve
//      `Content/<Pack>/` al proyecto (como hace Epic, P-30): cancelar a medias no deja nada en Content/.
//      Un `code_plugin` no se instala: se da el enlace (P-31).

#include "DbvFabDetail.h"

#include "AssetRegistry/AssetRegistryModule.h"
#include "Containers/Ticker.h"
#include "DBVUnrealAICppToolset.h"
#include "FabDownloader.h"
#include "GenericPlatform/GenericPlatformHttp.h"
#include "HAL/FileManager.h"
#include "Interfaces/IPluginManager.h"
#include "Misc/EngineVersion.h"
#include "Misc/PackageName.h"
#include "Misc/Paths.h"
#include "Modules/ModuleManager.h"
#include "Utilities/AssetUtils.h"

using namespace DbvFab::Detail;

namespace
{
	const TArray<FString> InstallableFormats = { TEXT("unreal-engine"), TEXT("glb"), TEXT("gltf"), TEXT("fbx"), TEXT("converted-files") };

	struct FInstallContext
	{
		FString JobId;
		FString ListingUid;
		FString Format;
		FString ArtifactId;
		FString EngineTag;
		FString Title;
		FString FabUrl;
		FString Thumbnail;
		FString SellerName;
		bool bFree = false;
		bool bRetried = false;
	};
	using FContextRef = TSharedRef<FInstallContext>;

	/**
	 * Ejecuta en el siguiente tick del motor, fuera del TaskGraph. NO con AsyncTask(GameThread): la importación
	 * síncrona de Interchange espera al TaskGraph y, desde dentro de una tarea, salta la protección de recursión
	 * de TaskGraph.cpp y cierra el editor (visto en vivo el 2026-11-15).
	 */
	void RunNextTick(TFunction<void()> Work)
	{
		FTSTicker::GetCoreTicker().AddTicker(FTickerDelegate::CreateLambda([Work = MoveTemp(Work)](float)
		{
			Work();
			return false;
		}));
	}

	FString EditorEngineTag()
	{
		const FEngineVersion& Version = FEngineVersion::Current();
		return FString::Printf(TEXT("UE_%d.%d"), Version.GetMajor(), Version.GetMinor());
	}

	void Fail(const FContextRef& Ctx, const FString& Code, const FString& Message, TFunction<void(TSharedRef<FJsonObject>&)> Extra = nullptr)
	{
		TSharedRef<FJsonObject> Out = MakeShared<FJsonObject>();
		Out->SetBoolField(TEXT("success"), false);
		Out->SetStringField(TEXT("error"), Code);
		Out->SetStringField(TEXT("message"), Message);
		Out->SetStringField(TEXT("listing_uid"), Ctx->ListingUid);
		if (!Ctx->FabUrl.IsEmpty())
		{
			Out->SetStringField(TEXT("fab_url"), Ctx->FabUrl);
		}
		if (Extra)
		{
			Extra(Out);
		}
		Finish(Ctx->JobId, ToJson(Out), Code);
	}

	TSharedRef<FJsonObject> ListingSummary(const FContextRef& Ctx)
	{
		TSharedRef<FJsonObject> Listing = MakeShared<FJsonObject>();
		Listing->SetStringField(TEXT("uid"), Ctx->ListingUid);
		Listing->SetStringField(TEXT("title"), Ctx->Title);
		Listing->SetStringField(TEXT("fab_url"), Ctx->FabUrl);
		Listing->SetStringField(TEXT("thumbnail"), Ctx->Thumbnail);
		Listing->SetStringField(TEXT("seller_name"), Ctx->SellerName);
		Listing->SetBoolField(TEXT("is_free"), Ctx->bFree);
		return Listing;
	}

	void Succeed(const FContextRef& Ctx, const FString& Kind, const FString& Folder, const TArray<FString>& Imported, const FString& Primary, const TArray<FString>& MissingPlugins)
	{
		TSharedRef<FJsonObject> Out = MakeShared<FJsonObject>();
		Out->SetBoolField(TEXT("success"), true);
		Out->SetStringField(TEXT("kind"), Kind);
		Out->SetStringField(TEXT("format"), Ctx->Format);
		Out->SetStringField(TEXT("engine_tag"), Ctx->EngineTag);
		Out->SetStringField(TEXT("folder"), Folder);
		Out->SetStringField(TEXT("primary"), Primary);
		TArray<TSharedPtr<FJsonValue>> ImportedValues;
		// Acotado para la IA (REQ-51): los 20 primeros y el total.
		for (int32 Index = 0; Index < FMath::Min(Imported.Num(), 20); ++Index)
		{
			ImportedValues.Add(MakeShared<FJsonValueString>(Imported[Index]));
		}
		Out->SetArrayField(TEXT("imported"), ImportedValues);
		Out->SetNumberField(TEXT("imported_count"), Imported.Num());
		TArray<TSharedPtr<FJsonValue>> PluginValues;
		for (const FString& Plugin : MissingPlugins)
		{
			PluginValues.Add(MakeShared<FJsonValueString>(Plugin));
		}
		Out->SetArrayField(TEXT("missing_plugins"), PluginValues);
		Out->SetObjectField(TEXT("listing"), ListingSummary(Ctx));
		Finish(Ctx->JobId, ToJson(Out), FString());
	}

	FString StringField(const TSharedPtr<FJsonObject>& Object, const TCHAR* Field)
	{
		FString Value;
		if (Object.IsValid())
		{
			Object->TryGetStringField(Field, Value);
		}
		return Value;
	}

	FString StagingDirFor(const FContextRef& Ctx)
	{
		return FPaths::ConvertRelativePathToFull(FPaths::ProjectSavedDir() / TEXT("DBV") / TEXT("Fab") / Ctx->JobId);
	}

	void RequestDownloadInfo(const FContextRef& Ctx);

	// --- binary -------------------------------------------------------------------------------------------------

	/** Ficheros que se pueden importar, primero glTF/GLB (como Epic) y después FBX/OBJ. */
	TArray<FString> FindModelFiles(const FString& Dir)
	{
		TArray<FString> Files;
		for (const TCHAR* Extension : { TEXT("glb"), TEXT("gltf"), TEXT("fbx"), TEXT("obj") })
		{
			TArray<FString> Found;
			IFileManager::Get().FindFilesRecursive(Found, *Dir, *FString::Printf(TEXT("*.%s"), Extension), true, false);
			Files.Append(Found);
		}
		return Files;
	}

	void OnBinaryDownloaded(const FContextRef& Ctx, const FString& Staging, bool bSuccess, const TArray<FString>& Downloaded)
	{
		if (!IsRunning(Ctx->JobId))
		{
			return;
		}
		if (!bSuccess || Downloaded.IsEmpty())
		{
			if (!Ctx->bRetried)
			{
				// La URL firmada puede haber caducado: se pide otra una sola vez (R-51).
				Ctx->bRetried = true;
				IFileManager::Get().DeleteDirectory(*Staging, false, true);
				RequestDownloadInfo(Ctx);
			}
			else
			{
				IFileManager::Get().DeleteDirectory(*Staging, false, true);
				Fail(Ctx, TEXT("download_failed"), TEXT("The download from Fab failed twice."));
			}
			return;
		}
		for (const FString& File : Downloaded)
		{
			if (FPaths::GetExtension(File).Equals(TEXT("zip"), ESearchCase::IgnoreCase))
			{
				FAssetUtils::Unzip(File, FPaths::GetPath(File) / (FPaths::GetBaseFilename(File) + TEXT("_extracted")));
			}
		}
		const TArray<FString> Models = FindModelFiles(Staging);
		if (Models.IsEmpty())
		{
			IFileManager::Get().DeleteDirectory(*Staging, false, true);
			Fail(Ctx, TEXT("nothing_to_import"), TEXT("The downloaded files contain no glTF, GLB, FBX or OBJ model."));
			return;
		}
		FString FolderName = Ctx->Title.IsEmpty() ? Ctx->ListingUid : Ctx->Title;
		FAssetUtils::SanitizeFolderName(FolderName);
		const FString Folder = TEXT("/Game/Fab/") + FolderName;
		const FString ImportJson = UDBVUnrealAICppToolset::import_model(Models[0], Folder, FolderName);
		const TSharedPtr<FJsonObject> Import = ParseJson(ImportJson);
		bool bImported = false;
		if (Import.IsValid())
		{
			Import->TryGetBoolField(TEXT("success"), bImported);
		}
		IFileManager::Get().DeleteDirectory(*Staging, false, true);
		if (!bImported)
		{
			Fail(Ctx, TEXT("import_failed"), StringField(Import, TEXT("error")).IsEmpty() ? ImportJson.Left(400) : StringField(Import, TEXT("error")));
			return;
		}
		TArray<FString> Imported;
		const TArray<TSharedPtr<FJsonValue>>* ImportedValues = nullptr;
		if (Import->TryGetArrayField(TEXT("imported"), ImportedValues))
		{
			for (const TSharedPtr<FJsonValue>& Value : *ImportedValues)
			{
				Imported.Add(Value->AsString());
			}
		}
		FAssetUtils::SyncContentBrowserToFolder(Folder, false);
		Succeed(Ctx, TEXT("binary"), Folder, Imported, StringField(Import, TEXT("primary")), {});
	}

	void DownloadBinary(const FContextRef& Ctx, const TSharedPtr<FJsonObject>& Entry)
	{
		const FString Staging = StagingDirFor(Ctx);
		IFileManager::Get().MakeDirectory(*Staging, true);
		TSharedPtr<FFabDownloadRequest> Download = MakeShared<FFabDownloadRequest>(Ctx->ListingUid, StringField(Entry, TEXT("downloadUrl")), Staging, EFabDownloadType::HTTP);
		const FString JobId = Ctx->JobId;
		Download->OnDownloadProgress().AddLambda([JobId](const FFabDownloadRequest*, const FFabDownloadStats& Stats)
		{
			SetProgress(JobId, Stats.PercentComplete > 1.0f ? Stats.PercentComplete / 100.0f : Stats.PercentComplete);
		});
		Download->OnDownloadComplete().AddLambda([Ctx, Staging](const FFabDownloadRequest*, const FFabDownloadStats& Stats)
		{
			const bool bSuccess = Stats.bIsSuccess;
			const TArray<FString> Files = Stats.DownloadedFiles;
			RunNextTick([Ctx, Staging, bSuccess, Files]() { OnBinaryDownloaded(Ctx, Staging, bSuccess, Files); });
		});
		SetDownload(Ctx->JobId, Download, Staging);
		Download->ExecuteRequest();
	}

	// --- manifest (pack de Unreal) -----------------------------------------------------------------------------

	/** Plugins desactivados que necesitan los assets del pack (como `CheckForDependencies` de Epic, sin reiniciar). */
	TArray<FString> MissingPluginsFor(const TArray<FString>& PackageNames)
	{
		TArray<FString> Missing;
		const IAssetRegistry& Registry = FModuleManager::LoadModuleChecked<FAssetRegistryModule>(TEXT("AssetRegistry")).Get();
		for (const FString& PackageName : PackageNames)
		{
			TArray<FAssetData> Assets;
			Registry.GetAssetsByPackageName(FName(*PackageName), Assets);
			for (const FAssetData& Asset : Assets)
			{
				FString ModuleName;
				const FString ClassPath = Asset.AssetClassPath.ToString();
				if (!ClassPath.StartsWith(TEXT("/Script/")) || !ClassPath.RightChop(8).Split(TEXT("."), &ModuleName, nullptr) || FModuleManager::Get().IsModuleLoaded(*ModuleName))
				{
					continue;
				}
				for (const TSharedRef<IPlugin>& Plugin : IPluginManager::Get().GetDiscoveredPlugins())
				{
					const bool bOwns = Plugin->GetDescriptor().Modules.ContainsByPredicate([&ModuleName](const FModuleDescriptor& Module) { return Module.Name.ToString().Equals(ModuleName, ESearchCase::IgnoreCase); });
					if (bOwns && !Plugin->IsEnabled())
					{
						Missing.AddUnique(Plugin->GetName());
					}
				}
			}
		}
		return Missing;
	}

	void OnPackDownloaded(const FContextRef& Ctx, const FString& Staging, bool bSuccess, const TArray<FString>& Files)
	{
		if (!IsRunning(Ctx->JobId))
		{
			return;
		}
		if (!bSuccess || Files.IsEmpty())
		{
			IFileManager::Get().DeleteDirectory(*Staging, false, true);
			if (!Ctx->bRetried)
			{
				Ctx->bRetried = true;
				RequestDownloadInfo(Ctx);
			}
			else
			{
				Fail(Ctx, TEXT("download_failed"), TEXT("The pack download from Fab failed twice."));
			}
			return;
		}
		// El manifiesto trae rutas `Content/<Pack>/...` relativas a la carpeta del proyecto.
		const FString ProjectDir = FPaths::ConvertRelativePathToFull(FPaths::ProjectDir());
		FString PackFolder;
		TArray<FString> Packages;
		bool bMoved = true;
		for (const FString& Relative : Files)
		{
			TArray<FString> Parts;
			Relative.ParseIntoArray(Parts, TEXT("/"));
			if (Parts.Num() < 3 || !Parts[0].Equals(TEXT("Content"), ESearchCase::IgnoreCase) || Relative.Contains(TEXT("..")))
			{
				continue;
			}
			if (PackFolder.IsEmpty())
			{
				PackFolder = TEXT("/Game/") + Parts[1];
			}
			const FString Source = Staging / Relative;
			const FString Target = ProjectDir / Relative;
			bMoved &= IFileManager::Get().Move(*Target, *Source, true, true);
			if (Relative.EndsWith(TEXT(".uasset")) || Relative.EndsWith(TEXT(".umap")))
			{
				FString PackageName;
				if (FPackageName::TryConvertFilenameToLongPackageName(Target, PackageName))
				{
					Packages.Add(PackageName);
				}
			}
		}
		IFileManager::Get().DeleteDirectory(*Staging, false, true);
		if (PackFolder.IsEmpty() || !bMoved)
		{
			Fail(Ctx, TEXT("install_failed"), PackFolder.IsEmpty() ? TEXT("The pack has no Content/ folder.") : TEXT("Some pack files could not be moved into the project's Content folder."));
			return;
		}
		FAssetUtils::ScanForAssets(PackFolder);
		FAssetUtils::SyncContentBrowserToFolder(PackFolder, false);
		Succeed(Ctx, TEXT("asset_pack"), PackFolder, Packages, FString(), MissingPluginsFor(Packages));
	}

	void DownloadPack(const FContextRef& Ctx, const TSharedPtr<FJsonObject>& Entry)
	{
		TArray<FString> BaseUrls;
		const TArray<TSharedPtr<FJsonValue>>* Bases = nullptr;
		if (Entry->TryGetArrayField(TEXT("distributionPointBaseUrls"), Bases))
		{
			for (const TSharedPtr<FJsonValue>& Base : *Bases)
			{
				BaseUrls.Add(Base->AsString());
			}
		}
		const FString Staging = StagingDirFor(Ctx);
		IFileManager::Get().MakeDirectory(*Staging, true);
		const FString Url = StringField(Entry, TEXT("downloadUrl")) + TEXT(",") + FString::Join(BaseUrls, TEXT(","));
		TSharedPtr<FFabDownloadRequest> Download = MakeShared<FFabDownloadRequest>(Ctx->ListingUid, Url, Staging, EFabDownloadType::BuildPatchRequest);
		const FString JobId = Ctx->JobId;
		Download->OnDownloadProgress().AddLambda([JobId](const FFabDownloadRequest*, const FFabDownloadStats& Stats)
		{
			const float Progress = Stats.PercentComplete > 1.0f ? Stats.PercentComplete / 100.0f : Stats.PercentComplete;
			SetProgress(JobId, Progress);
		});
		Download->OnDownloadComplete().AddLambda([Ctx, Staging](const FFabDownloadRequest*, const FFabDownloadStats& Stats)
		{
			const bool bSuccess = Stats.bIsSuccess;
			const TArray<FString> Files = Stats.DownloadedFiles;
			RunNextTick([Ctx, Staging, bSuccess, Files]() { OnPackDownloaded(Ctx, Staging, bSuccess, Files); });
		});
		SetDownload(Ctx->JobId, Download, Staging);
		Download->ExecuteRequest();
	}

	// --- download-info ------------------------------------------------------------------------------------------

	void OnDownloadInfo(const FContextRef& Ctx, int32 Code, const FString& Body)
	{
		if (Code < 200 || Code >= 300)
		{
			const FString Message = ServerMessage(Body);
			const bool bEula = Message.Contains(TEXT("eula"), ESearchCase::IgnoreCase) || Message.Contains(TEXT("license agreement"), ESearchCase::IgnoreCase);
			Fail(Ctx, bEula ? TEXT("eula_required") : HttpErrorCode(Code), bEula ? FString::Printf(TEXT("Accept the license for this listing on fab.com first (it is never accepted for you): %s"), *Message) : Message);
			return;
		}
		const TSharedPtr<FJsonObject> Info = ParseJson(Body);
		TArray<TSharedPtr<FJsonObject>> Entries;
		const TArray<TSharedPtr<FJsonValue>>* Values = nullptr;
		if (Info.IsValid() && Info->TryGetArrayField(TEXT("downloadInfo"), Values))
		{
			for (const TSharedPtr<FJsonValue>& Value : *Values)
			{
				const TSharedPtr<FJsonObject>* Entry = nullptr;
				if (Value->TryGetObject(Entry) && (Ctx->ArtifactId.IsEmpty() || StringField(*Entry, TEXT("artifactId")) == Ctx->ArtifactId))
				{
					Entries.Add(*Entry);
				}
			}
		}
		if (Entries.IsEmpty())
		{
			Fail(Ctx, TEXT("nothing_to_download"), Ctx->ArtifactId.IsEmpty() ? TEXT("Fab returned no files for this format.") : TEXT("No file with that artifact_id."));
			return;
		}
		if (Entries.Num() > 1)
		{
			// Varios ficheros: se devuelve la lista para elegir, sin descargar nada (REQ-104).
			Fail(Ctx, TEXT("choose_file"), TEXT("This listing has several files for this format. Call install again with artifact_id."), [&Entries](TSharedRef<FJsonObject>& Out)
			{
				TArray<TSharedPtr<FJsonValue>> Files;
				for (const TSharedPtr<FJsonObject>& Entry : Entries)
				{
					TSharedRef<FJsonObject> File = MakeShared<FJsonObject>();
					File->SetStringField(TEXT("artifact_id"), StringField(Entry, TEXT("artifactId")));
					File->SetStringField(TEXT("name"), StringField(Entry, TEXT("name")));
					double Size = 0.0;
					Entry->TryGetNumberField(TEXT("fileSize"), Size);
					File->SetNumberField(TEXT("file_size"), Size);
					bool bMain = false;
					Entry->TryGetBoolField(TEXT("isMainFile"), bMain);
					File->SetBoolField(TEXT("is_main_file"), bMain);
					Files.Add(MakeShared<FJsonValueObject>(File));
				}
				Out->SetArrayField(TEXT("files"), Files);
			});
			return;
		}
		const TSharedPtr<FJsonObject>& Entry = Entries[0];
		const FString Type = StringField(Entry, TEXT("type"));
		const FString AssetType = StringField(Entry, TEXT("assetType"));
		if (AssetType.Contains(TEXT("plugin"), ESearchCase::IgnoreCase))
		{
			Fail(Ctx, TEXT("code_plugin"), TEXT("Code plugins are installed into the engine: install it from the Fab tab of the editor (P-31)."));
		}
		else if (Type == TEXT("binary"))
		{
			DownloadBinary(Ctx, Entry);
		}
		else if (Type == TEXT("manifest"))
		{
			DownloadPack(Ctx, Entry);
		}
		else
		{
			Fail(Ctx, TEXT("unsupported_download"), FString::Printf(TEXT("Unknown download type \"%s\"."), *Type));
		}
	}

	void RequestDownloadInfo(const FContextRef& Ctx)
	{
		FString Query;
		AddQuery(Query, TEXT("assetFormat"), Ctx->Format);
		if (Ctx->Format == TEXT("unreal-engine"))
		{
			AddQuery(Query, TEXT("engineVersion"), Ctx->EngineTag);
		}
		AddQuery(Query, TEXT("platform"), TEXT("Windows"));
		const FString Url = FabHost() + TEXT("/p/agents/listings/") + FGenericPlatformHttp::UrlEncode(Ctx->ListingUid) + TEXT("/download-info") + Query;
		Get(Ctx->JobId, Url, true, [Ctx](int32 Code, const FString& Body) { OnDownloadInfo(Ctx, Code, Body); });
	}

	// --- propiedad (lo de pago) --------------------------------------------------------------------------------

	void OnLibrary(const FContextRef& Ctx, int32 Code, const FString& Body)
	{
		bool bOwned = false;
		const TSharedPtr<FJsonObject> Page = ParseJson(Body);
		const TArray<TSharedPtr<FJsonValue>>* Results = nullptr;
		if (Code >= 200 && Code < 300 && Page.IsValid() && Page->TryGetArrayField(TEXT("results"), Results))
		{
			for (const TSharedPtr<FJsonValue>& Result : *Results)
			{
				const TSharedPtr<FJsonObject>* Entry = nullptr;
				const TSharedPtr<FJsonObject>* Listing = nullptr;
				bool bCanDownload = false;
				if (Result->TryGetObject(Entry) && (*Entry)->TryGetObjectField(TEXT("listing"), Listing) && StringField(*Listing, TEXT("uid")) == Ctx->ListingUid)
				{
					(*Entry)->TryGetBoolField(TEXT("canRequestDownloadUrl"), bCanDownload);
					bOwned |= bCanDownload;
				}
			}
		}
		if (Code == 401 || Code == 0)
		{
			Fail(Ctx, HttpErrorCode(Code), ServerMessage(Body));
		}
		else if (!bOwned)
		{
			Fail(Ctx, TEXT("no_owned"), TEXT("This listing is not free and is not in your Fab library. Nothing is bought from DBV Unreal AI: open it on fab.com."));
		}
		else
		{
			RequestDownloadInfo(Ctx);
		}
	}

	// --- ficha --------------------------------------------------------------------------------------------------

	void OnListing(const FContextRef& Ctx, int32 Code, const FString& Body)
	{
		const TSharedPtr<FJsonObject> Listing = ParseJson(Body);
		if (Code < 200 || Code >= 300 || !Listing.IsValid())
		{
			Fail(Ctx, HttpErrorCode(Code), ServerMessage(Body));
			return;
		}
		Ctx->Title = StringField(Listing, TEXT("title"));
		const FString FabUrl = StringField(Listing, TEXT("fabUrl"));
		Ctx->FabUrl = FabUrl.StartsWith(TEXT("https://www.fab.com/listings/")) ? FabUrl : TEXT("https://www.fab.com/listings/") + Ctx->ListingUid;
		Listing->TryGetBoolField(TEXT("isFree"), Ctx->bFree);
		const TSharedPtr<FJsonObject>* User = nullptr;
		if (Listing->TryGetObjectField(TEXT("user"), User))
		{
			Ctx->SellerName = StringField(*User, TEXT("sellerName"));
		}
		const TArray<TSharedPtr<FJsonValue>>* Thumbnails = nullptr;
		const TArray<TSharedPtr<FJsonValue>>* Images = nullptr;
		const TSharedPtr<FJsonObject>* FirstThumbnail = nullptr;
		if (Listing->TryGetArrayField(TEXT("thumbnails"), Thumbnails) && Thumbnails->Num() > 0 && (*Thumbnails)[0]->TryGetObject(FirstThumbnail) && (*FirstThumbnail)->TryGetArrayField(TEXT("images"), Images))
		{
			for (const TSharedPtr<FJsonValue>& Image : *Images)
			{
				const TSharedPtr<FJsonObject>* ImageObject = nullptr;
				double Width = 0.0;
				if (Image->TryGetObject(ImageObject) && (*ImageObject)->TryGetNumberField(TEXT("width"), Width) && FMath::IsNearlyEqual(Width, 320.0) && StringField(*ImageObject, TEXT("url")).StartsWith(TEXT("https://media.fab.com/")))
				{
					Ctx->Thumbnail = StringField(*ImageObject, TEXT("url"));
				}
			}
		}
		// Formatos del listing y versiones de Unreal del formato `unreal-engine`.
		bool bHasFormat = false;
		TArray<FString> UeVersions;
		const TArray<TSharedPtr<FJsonValue>>* Formats = nullptr;
		if (Listing->TryGetArrayField(TEXT("assetFormats"), Formats))
		{
			for (const TSharedPtr<FJsonValue>& Format : *Formats)
			{
				const TSharedPtr<FJsonObject>* FormatObject = nullptr;
				const TSharedPtr<FJsonObject>* Type = nullptr;
				if (!Format->TryGetObject(FormatObject) || !(*FormatObject)->TryGetObjectField(TEXT("assetFormatType"), Type))
				{
					continue;
				}
				const FString FormatCode = StringField(*Type, TEXT("code"));
				bHasFormat |= FormatCode == Ctx->Format;
				const TSharedPtr<FJsonObject>* Specs = nullptr;
				const TArray<TSharedPtr<FJsonValue>>* Versions = nullptr;
				if (FormatCode == TEXT("unreal-engine") && (*FormatObject)->TryGetObjectField(TEXT("technicalSpecs"), Specs) && (*Specs)->TryGetArrayField(TEXT("unrealEngineEngineVersions"), Versions))
				{
					for (const TSharedPtr<FJsonValue>& Version : *Versions)
					{
						UeVersions.Add(Version->AsString());
					}
				}
			}
		}
		if (!bHasFormat)
		{
			Fail(Ctx, TEXT("format_not_available"), FString::Printf(TEXT("This listing has no \"%s\" format."), *Ctx->Format));
		}
		else if (Ctx->Format == TEXT("unreal-engine") && !UeVersions.Contains(Ctx->EngineTag))
		{
			// Solo lo compatible con la versión del editor (decisión del usuario, 2026-11-15).
			Fail(Ctx, TEXT("incompatible_engine"), FString::Printf(TEXT("This Unreal pack is not available for %s (it has %s). Only content compatible with this editor is installed."), *Ctx->EngineTag, *FString::Join(UeVersions, TEXT(", "))), [&UeVersions](TSharedRef<FJsonObject>& Out)
			{
				TArray<TSharedPtr<FJsonValue>> Values;
				for (const FString& Version : UeVersions)
				{
					Values.Add(MakeShared<FJsonValueString>(Version));
				}
				Out->SetArrayField(TEXT("ue_versions"), Values);
			});
		}
		else if (Ctx->bFree)
		{
			RequestDownloadInfo(Ctx);
		}
		else
		{
			FString Query;
			AddQuery(Query, TEXT("q"), Ctx->Title);
			AddQuery(Query, TEXT("count"), TEXT("24"));
			Get(Ctx->JobId, FabHost() + TEXT("/p/agents/library/search") + Query, true, [Ctx](int32 LibraryCode, const FString& LibraryBody) { OnLibrary(Ctx, LibraryCode, LibraryBody); });
		}
	}
}

namespace DbvFab
{
	void StartInstall(const FString& JobId, const TSharedPtr<FJsonObject>& Args)
	{
		FContextRef Ctx = MakeShared<FInstallContext>();
		Ctx->JobId = JobId;
		Ctx->ListingUid = StringField(Args, TEXT("listing_uid"));
		Ctx->Format = StringField(Args, TEXT("asset_format"));
		Ctx->ArtifactId = StringField(Args, TEXT("artifact_id"));
		Ctx->EngineTag = EditorEngineTag();
		const bool bValidUid = !Ctx->ListingUid.IsEmpty() && Ctx->ListingUid.Len() <= 64 && !Ctx->ListingUid.Contains(TEXT("/")) && !Ctx->ListingUid.Contains(TEXT("."));
		if (!bValidUid || !InstallableFormats.Contains(Ctx->Format))
		{
			Fail(Ctx, TEXT("bad_arguments"), FString::Printf(TEXT("listing_uid and asset_format (%s) are required."), *FString::Join(InstallableFormats, TEXT(", "))));
		}
		else
		{
			Get(JobId, FabHost() + TEXT("/p/agents/listings/") + FGenericPlatformHttp::UrlEncode(Ctx->ListingUid), false, [Ctx](int32 Code, const FString& Body) { OnListing(Ctx, Code, Body); });
		}
	}
}
