// =============================================================================
// DBV Unreal AI — Vista Ajustes
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec RNF-IA.4, RNF-04, REQ-20
//
// La clave de Tripo solo viaja de ida: el campo nunca se rellena con la guardada, solo se dice
// si hay una (RNF-IA.4).

import { t } from '../ai/i18nBridge.js';
import * as api from './backend.js';
import { button, el } from './dom.js';

/** Configuración MCP para pegar en un cliente (Claude Code `.mcp.json`, Cursor…). */
export function mcpClientConfig(command) {
  return JSON.stringify({ mcpServers: { 'dbv-unreal-ai': { command: command.command, args: command.args } } }, null, 2);
}

/**
 * @param {HTMLElement} host
 * @param {object} deps `{toast, openConnect, state}`.
 */
export function createSettingsView(host, deps) {
  async function render() {
    const status = await api.generationStatus();
    const key = el('input', { attrs: { type: 'password', autocomplete: 'off', 'aria-label': t('settings.tripo') } });
    const budget = el('input', { attrs: { type: 'number', min: '0', step: '10', value: String(status.ok ? status.value.budget : 300), 'aria-label': t('settings.budget') } });
    const port = deps.state.connection?.endpoint?.port ?? 8000;
    const command = await api.appMcpCommand(port);
    host.replaceChildren(
      el('div', { className: 'view-header' }, el('h2', { text: t('settings.title') })),
      el('div', { className: 'card stack' }, el('h3', { text: t('settings.ai') }), el('div', { className: 'row' }, button(t('settings.aiConnect'), deps.openConnect, 'btn-primary'))),
      el(
        'div',
        { className: 'card stack' },
        el('h3', { text: 'Tripo' }),
        el('p', { className: 'hint', text: status.ok && status.value.hasKey?.tripo ? t('settings.keyPresent') : t('settings.keyMissing') }),
        el('label', {}, t('settings.tripo'), key),
        el(
          'div',
          { className: 'row' },
          button(t('settings.save'), async () => {
            const saved = await api.generationSetKey('tripo', key.value);
            key.value = '';
            if (!saved.ok) return deps.toast(saved.error.message, 'error');
            deps.toast(saved.value === 'system' ? t('settings.keySaved') : t('settings.keySession'));
            return render();
          }, 'btn-primary'),
          button(t('settings.delete'), async () => {
            await api.generationDeleteKey('tripo');
            await render();
          }, 'btn-ghost btn-danger'),
        ),
        el('label', {}, t('settings.budget'), budget),
        el('div', { className: 'row' }, button(t('settings.save'), async () => {
          const saved = await api.generationSetBudget(Number(budget.value) || 0);
          deps.toast(saved.ok ? t('settings.budgetSaved') : saved.error.message, saved.ok ? 'info' : 'error');
        })),
      ),
      el('div', { className: 'card stack' }, el('h3', { text: t('settings.mcp') }), el('p', { className: 'hint', text: t('settings.mcpHint') }), command.ok ? el('pre', { className: 'diff', text: mcpClientConfig(command.value) }) : null),
    );
  }
  return { render };
}
