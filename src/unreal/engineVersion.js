// =============================================================================
// DBV Unreal AI — Versiones de Unreal: Preview y editor distinto del proyecto
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-109
//
// La instalación de Unreal no dice si es una Preview (`Build.version` de la 5.9 MegaJam Preview es igual que el de una
// versión final), así que se lleva aquí la lista. Y la app abre siempre un proyecto con el motor de su versión, pero
// el usuario puede abrirlo a mano con otro: al conectar se compara la versión del editor (`serverInfo.version`) con
// la del proyecto, porque abrir un proyecto 5.8 con 5.9 lo convierte sin vuelta atrás.

/** `major.minor` de las versiones que hoy son Preview. Quitar la 5.9 cuando salga la final. */
export const PREVIEW_VERSIONS = ['5.9'];

/** `5.9.0-60351404` → `5.9`; `null` si no es una versión. */
export function majorMinor(version) {
  const match = /^(\d+)\.(\d+)/.exec(String(version ?? '').trim());
  return match ? `${match[1]}.${match[2]}` : null;
}

/** `5.9.0-60351404` → `5.9.0`: lo que se enseña y se da a la IA. */
export function shortVersion(version) {
  const match = /^(\d+\.\d+(?:\.\d+)?)/.exec(String(version ?? '').trim());
  return match ? match[1] : String(version ?? '');
}

/** ¿Es una Preview? */
export const isPreview = (version) => PREVIEW_VERSIONS.includes(majorMinor(version));

/**
 * El editor conectado es de otra versión que el proyecto (lo abrió el usuario a mano con otro motor), o `null`.
 * Sin versión del editor no se puede saber y no se avisa.
 */
export function versionMismatch(projectVersion, editorVersion) {
  const project = majorMinor(projectVersion);
  const editor = majorMinor(editorVersion);
  return project && editor && project !== editor ? { project, editor } : null;
}
