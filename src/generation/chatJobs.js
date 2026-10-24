// =============================================================================
// DBV Unreal AI — Trabajos de generación dentro de la conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-27
//
// Mientras Tripo trabaja, la conversación enseña la tarjeta viva de cada trabajo, en el punto en que se lanzó:
//   · los del asistente integrado, porque su herramienta los pide (`track`);
//   · los de un agente (Claude Code…), que corren en el subproceso `--mcp`: se descubren en los ficheros de
//     trabajo mientras el turno del agente sigue abierto (`follow`). Antes la conversación parecía parada.

import { createJobCard, createJobWatcher } from './jobCard.js';

/** Cada cuánto se buscan trabajos nuevos mientras un agente está respondiendo (ms). */
export const FOLLOW_MS = 1500;

/**
 * @param {object} deps `{list(): Promise<{ok, value}>, renderResult?(record, slot)}`.
 */
export function createChatJobs(deps) {
  const cards = new Map();
  const wanted = new Set();
  let panel = null;
  let agentSince = Infinity;
  let followTimer = null;

  const watcher = createJobWatcher({
    list: deps.list,
    onRecord(record) {
      const known = cards.get(record.id);
      const mine = wanted.has(record.id) || (record.source === 'agent' && record.startedAt >= agentSince);
      if (known) known.update(record);
      else if (mine && panel) {
        const card = createJobCard(record, { renderResult: deps.renderResult });
        cards.set(record.id, card);
        panel.addNode(card.node);
      }
    },
  });

  return {
    /** Trabajos lanzados por una herramienta del asistente integrado en este `panel`. */
    track(target, ids) {
      panel = target;
      for (const id of ids) wanted.add(id);
      watcher.poke();
    },
    /** Un agente empieza un turno: lo que lance desde ahora aparece en `target` mientras `isBusy()`. */
    follow(target, isBusy) {
      panel = target;
      // Unos segundos de margen: el reloj del subproceso y el de la ventana son el mismo, pero el turno empieza antes.
      agentSince = Math.floor(Date.now() / 1000) - 2;
      clearInterval(followTimer);
      followTimer = setInterval(() => {
        watcher.poke();
        if (!isBusy()) {
          clearInterval(followTimer);
          followTimer = null;
        }
      }, FOLLOW_MS);
    },
    /** Tarjeta de un trabajo, si la hay (tests). */
    card: (id) => cards.get(id),
    stop() {
      clearInterval(followTimer);
      watcher.stop();
    },
  };
}
