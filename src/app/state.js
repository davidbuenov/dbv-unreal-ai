// =============================================================================
// DBV Unreal AI — Estado compartido de la ventana
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Un único sitio sabe a qué proyectos y a qué editores está conectada la app; las vistas se
// suscriben. Soporta múltiples instancias simultáneas (REQ-07, D17).

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
  /** @type {Map<string, Connection>} */
  const connections = new Map();
  /** @type {string|null} */
  let activeProject = null;
  const listeners = new Set();

  function notify() {
    const current = activeProject ? connections.get(activeProject) ?? null : null;
    for (const listener of listeners) listener(current);
  }

  return {
    get connection() {
      return activeProject ? connections.get(activeProject) ?? null : null;
    },
    get connections() {
      return Array.from(connections.values());
    },
    get activeProject() {
      return activeProject;
    },
    setActiveProject(key) {
      if (key && connections.has(key)) {
        activeProject = key;
        notify();
      } else if (!key) {
        activeProject = null;
        notify();
      }
    },
    /** Sustituye (o fusiona) la conexión y avisa. */
    setConnection(next, merge = false) {
      if (!next) {
        if (activeProject) {
          connections.delete(activeProject);
          activeProject = connections.keys().next().value ?? null;
        } else {
          connections.clear();
        }
      } else {
        const key = next.projectPath || next.projectName || 'default';
        const prev = connections.get(key) || (activeProject ? connections.get(activeProject) : null);
        const merged = merge && prev ? { ...prev, ...next } : next;
        const effectiveKey = merged.projectPath || merged.projectName || key;
        connections.set(effectiveKey, merged);
        if (!activeProject || !connections.has(activeProject) || activeProject === key) {
          activeProject = effectiveKey;
        }
      }
      notify();
    },
    removeConnection(key) {
      if (key && connections.has(key)) {
        connections.delete(key);
        if (activeProject === key) {
          activeProject = connections.keys().next().value ?? null;
        }
        notify();
      }
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}
