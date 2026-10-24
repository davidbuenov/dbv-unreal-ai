// =============================================================================
// DBV Unreal AI — Estado compartido de la ventana
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Un único sitio sabe a qué proyecto y a qué editor está conectada la app; las vistas se
// suscriben. Sin dependencias: un objeto y una lista de oyentes.

/**
 * @typedef {object} Connection
 * @property {string} projectPath Ruta del `.uproject`.
 * @property {string} projectName
 * @property {string} engineVersion
 * @property {{port: number, path: string}} endpoint
 * @property {'offline'|'connecting'|'online'|'prep'} status
 * @property {number} toolsets Toolsets disponibles al conectar.
 */

export function createState() {
  /** @type {Connection|null} */
  let connection = null;
  const listeners = new Set();
  return {
    get connection() {
      return connection;
    },
    /** Sustituye (o fusiona) la conexión y avisa. */
    setConnection(next, merge = false) {
      connection = next && merge && connection ? { ...connection, ...next } : next;
      for (const listener of listeners) listener(connection);
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}
