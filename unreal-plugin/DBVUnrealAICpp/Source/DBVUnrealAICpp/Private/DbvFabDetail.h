// =============================================================================
// DBV Unreal AI — Piezas internas del cliente de Fab (trabajos y peticiones)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-103, REQ-104
//
// Lo comparten DbvFabClient.cpp (sesión, biblioteca, download-info) y DbvFabInstall.cpp (instalar). Todo se
// llama en el hilo del juego.

#pragma once

#include "CoreMinimal.h"
#include "Dom/JsonObject.h"

class FFabDownloadRequest;

namespace DbvFab::Detail
{
	using FResponseHandler = TFunction<void(int32 /*Status*/, const FString& /*Body*/)>;

	/** Termina un trabajo: `Error` vacío → `done`; si no, `error`. */
	void Finish(const FString& JobId, const FString& ResultJson, const FString& Error);

	/** ¿Sigue vivo (no cancelado ni terminado)? */
	bool IsRunning(const FString& JobId);

	/** Progreso de una descarga, de 0 a 1. */
	void SetProgress(const FString& JobId, float Progress);

	/** La descarga en curso del trabajo (para cancelarla) y su carpeta temporal (para borrarla al cancelar). */
	void SetDownload(const FString& JobId, const TSharedPtr<FFabDownloadRequest>& Download, const FString& StagingDir);

	/** GET a www.fab.com; con `bAuth`, con el token de la sesión de Fab (un único reintento tras un 401). */
	void Get(const FString& JobId, const FString& Url, bool bAuth, FResponseHandler OnDone);

	FString FabHost();
	TSharedPtr<FJsonObject> ParseJson(const FString& Text);
	FString ToJson(const TSharedRef<FJsonObject>& Object);
	FString ErrorJson(const FString& Code, const FString& Message);
	FString HttpErrorCode(int32 Code);
	FString ServerMessage(const FString& Body);
	void AddQuery(FString& Query, const TCHAR* Name, const FString& Value);
}

namespace DbvFab
{
	/** @spec REQ-104 Operación `install` (DbvFabInstall.cpp). */
	void StartInstall(const FString& JobId, const TSharedPtr<FJsonObject>& Args);
}
