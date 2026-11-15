// =============================================================================
// DBV Unreal AI — Cliente de Fab con la sesión EOS del propio editor
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-101, REQ-103
//
// Las peticiones con sesión a https://www.fab.com/p/agents/* se hacen AQUÍ, dentro del proceso del editor,
// con el token de usuario de Epic Online Services de la sesión que ya abrió el plugin `Fab` (ADR-V060-001).
// El token se copia en cada petición, no se guarda, no se escribe en logs y no sale en ninguna respuesta.
//
// Las herramientas AICallable corren en el hilo del juego, y el HTTP de UE completa sus callbacks en ese
// mismo hilo: nunca se espera una respuesta dentro de la llamada (se bloquearía el editor). Cada operación
// es un trabajo: `Start` devuelve un id al momento y `Status` dice cómo va (§19, Architect Review).

#pragma once

#include "CoreMinimal.h"

namespace DbvFab
{
	/** Inicia una operación (`session`, `platforms`, `library_search`, `download_info`, `install`). Devuelve JSON con `job_id` o el error. */
	FString Start(const FString& Operation, const FString& ArgsJson);

	/** Estado de un trabajo: `running`, `done` (con `result`) o `error`. Un trabajo terminado se borra al leerlo. */
	FString Status(const FString& JobId);

	/** Cancela un trabajo en curso. */
	FString Cancel(const FString& JobId);

	/** Quita de un texto cualquier `Bearer <token>` y el valor de los campos `access_token`/`accessToken`. Público para los tests. */
	FString RedactSecrets(const FString& Text);
}
