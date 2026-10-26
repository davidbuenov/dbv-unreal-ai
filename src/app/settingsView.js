// =============================================================================
// DBV Unreal AI — Vista Ajustes
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec RNF-IA.4, RNF-04, REQ-20, RNF-08, REQ-30, REQ-18
//
// Las claves de Tripo y World Labs solo viajan de ida: el campo nunca se rellena con la guardada, solo se dice
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
  /**
   * Campo de la clave de un servicio de generación y sus botones (la clave solo viaja de ida).
   * @param {'tripo'|'worldlabs'} service
   */
  function keyControls(service, hasKey) {
    const key = el('input', { attrs: { type: 'password', autocomplete: 'off', 'aria-label': t(`settings.${service}`) } });
    return [
      el('p', { className: 'hint', text: hasKey ? t('settings.keyPresent') : t('settings.keyMissing') }),
      el('label', {}, t(`settings.${service}`), key),
      el(
        'div',
        { className: 'row' },
        button(t('settings.save'), async () => {
          // El campo se vacía antes de esperar: la clave no se queda a la vista mientras se guarda.
          const value = key.value;
          key.value = '';
          const saved = await api.generationSetKey(service, value);
          if (!saved.ok) return deps.toast(saved.error.message, 'error');
          deps.toast(saved.value === 'system' ? t('settings.keySaved') : t('settings.keySession'));
          return render();
        }, 'btn-primary'),
        button(t('settings.delete'), async () => {
          await api.generationDeleteKey(service);
          await render();
        }, 'btn-ghost btn-danger'),
      ),
    ];
  }

  async function render() {
    const status = await api.generationStatus();
    const budget = el('input', { attrs: { type: 'number', min: '0', step: '10', value: String(status.ok ? status.value.budget : 300), 'aria-label': t('settings.budget') } });
    const port = deps.state.connection?.endpoint?.port ?? 8000;
    const [command, location] = await Promise.all([api.appMcpCommand(port), api.libraryLocation()]);
    const hasKey = status.ok && status.value.hasKey?.tripo;
    const balance = el('p', { className: 'hint' });
    if (hasKey) api.generationBalance().then((result) => (balance.textContent = result.ok && result.value ? t('gen.balance', { balance: String(result.value.balance), frozen: String(result.value.frozen) }) : (result.error?.message ?? '')));
    const copyExisting = el('input', { attrs: { type: 'checkbox', checked: '' } });
    async function moveLibrary(path) {
      const moved = await api.librarySetLocation(path, copyExisting.checked);
      if (!moved.ok) return deps.toast(moved.error.message, 'error');
      deps.toast(t('settings.libraryMoved'));
      return render();
    }
    const libraryCard = location.ok && location.value
      ? el(
          'div',
          { className: 'card stack' },
          el('h3', { text: t('settings.library') }),
          el('p', { className: 'hint', text: t('settings.libraryHint') }),
          el('p', { className: 'mono', text: location.value.current }),
          el('label', { className: 'row' }, copyExisting, t('settings.libraryCopy')),
          el(
            'div',
            { className: 'row' },
            button(t('settings.libraryChange'), async () => {
              const folder = await window.__TAURI__.dialog.open({ directory: true, defaultPath: location.value.current });
              return folder ? moveLibrary(String(folder)) : null;
            }),
            location.value.isDefault ? null : button(t('settings.libraryDefault'), () => moveLibrary(null), 'btn-ghost'),
            button(t('lib.reveal'), () => window.__TAURI__.opener.revealItemInDir(location.value.current), 'btn-ghost'),
          ),
        )
      : null;
    host.replaceChildren(
      el('div', { className: 'view-header' }, el('h2', { text: t('settings.title') })),
      el('div', { className: 'card stack' }, el('h3', { text: t('settings.ai') }), el('div', { className: 'row' }, button(t('settings.aiConnect'), deps.openConnect, 'btn-primary'))),
      el(
        'div',
        { className: 'card stack' },
        el('h3', { text: 'Tripo' }),
        balance,
        ...keyControls('tripo', hasKey),
        el('label', {}, t('settings.budget'), budget),
        el('div', { className: 'row' }, button(t('settings.save'), async () => {
          const saved = await api.generationSetBudget(Number(budget.value) || 0);
          deps.toast(saved.ok ? t('settings.budgetSaved') : saved.error.message, saved.ok ? 'info' : 'error');
        })),
      ),
      // @spec REQ-18, RNF-IA.4 — la clave de World Labs, en el mismo llavero que la de Tripo.
      el('div', { className: 'card stack' }, el('h3', { text: 'World Labs' }), el('p', { className: 'hint', text: t('settings.worldlabsHint') }), ...keyControls('worldlabs', status.ok && status.value.hasKey?.worldlabs)),
      libraryCard,
      el('div', { className: 'card stack' }, el('h3', { text: t('settings.mcp') }), el('p', { className: 'hint', text: t('settings.mcpHint') }), command.ok ? el('pre', { className: 'diff', text: mcpClientConfig(command.value) }) : null),
      el(
        'div',
        { className: 'card stack' },
        el('p', { className: 'hint', text: t('settings.diagnosticsHint') }),
        el('div', { className: 'row' }, button(t('settings.diagnostics'), async () => {
          const report = await api.appDiagnostics();
          if (!report.ok) return deps.toast(report.error.message, 'error');
          await navigator.clipboard.writeText(JSON.stringify(report.value, null, 2));
          return deps.toast(t('settings.diagnosticsCopied'));
        })),
      ),
    );
  }
  return { render };
}
