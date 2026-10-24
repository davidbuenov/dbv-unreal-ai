// =============================================================================
// DBV Unreal AI — Registro de acciones de la sesión
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-14
//
// Todo lo que la IA hizo en el editor (y lo que se denegó), en orden, exportable a Markdown:
// auditoría y material didáctico.

import { t } from '../ai/i18nBridge.js';
import { button, el } from './dom.js';

const time = (at) => new Date(at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });

/** Argumentos resumidos para una línea (el código Python no cabe: se cuenta). */
export function summarizeArgs(args) {
  if (!args || typeof args !== 'object') return '';
  const parts = Object.entries(args).map(([key, value]) => {
    const text = typeof value === 'string' ? value : JSON.stringify(value);
    return key === 'code' ? `code: ${String(value).split('\n').length} lines` : `${key}: ${text.length > 60 ? `${text.slice(0, 57)}…` : text}`;
  });
  return parts.join(', ');
}

/** El registro en Markdown. */
export function toMarkdown(entries, { project = '' } = {}) {
  const lines = [`# DBV Unreal AI — ${project || 'session'}`, '', '| Time | Toolset | Tool | Kind | Outcome | ms | Arguments |', '| --- | --- | --- | --- | --- | --- | --- |'];
  for (const e of entries) {
    const cells = [time(e.at), e.toolset, e.tool, e.kind, e.outcome, String(e.ms ?? 0), summarizeArgs(e.args)].map((c) => String(c ?? '').replace(/\|/g, '\\|'));
    lines.push(`| ${cells.join(' | ')} |`);
  }
  return `${lines.join('\n')}\n`;
}

/**
 * @param {HTMLElement} host
 * @param {{toast: (m: string) => void, projectName: () => string}} deps
 */
export function createActionLog(host, deps) {
  const entries = [];
  const list = el('ol');
  const empty = el('p', { className: 'hint', text: t('log.empty') });
  host.replaceChildren(
    el('header', {}, el('strong', { text: t('log.title') }), button(t('log.export'), async () => {
      await navigator.clipboard.writeText(toMarkdown(entries, { project: deps.projectName() }));
      deps.toast(t('log.copied'));
    }, 'btn-link')),
    empty,
    list,
  );
  return {
    entries,
    add(entry) {
      entries.push(entry);
      empty.classList.add('hidden');
      const item = el(
        'li',
        { className: `action-entry action-entry--${entry.outcome === 'ok' ? entry.kind : entry.outcome}` },
        el('span', { text: `${time(entry.at)} · ${entry.tool}` }),
        el('span', { className: 'mono', text: `${String(entry.toolset).split('.').pop()} ${summarizeArgs(entry.args)}` }),
      );
      list.append(item);
      item.scrollIntoView({ block: 'nearest' });
    },
  };
}
