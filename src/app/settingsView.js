// =============================================================================
// DBV Unreal AI — Vista Ajustes
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec RNF-IA.4, RNF-04, REQ-20, RNF-08, REQ-30, REQ-18, REQ-58, REQ-60, REQ-25
//
// Las claves de Tripo y World Labs solo viajan de ida: el campo nunca se rellena con la guardada, solo se dice
// si hay una (RNF-IA.4). Cada apartado va en su pestaña (REQ-60): todos se pintan y solo se ve el activo, así cambiar
// de pestaña no vuelve a preguntar al backend y la pestaña elegida se conserva al refrescar.

import { t } from '../ai/i18nBridge.js';
import * as api from './backend.js';
import { button, el } from './dom.js';
import { convaiSettingsCard } from './convaiSettings.js';
import { brandHeader } from './brandHeader.js';
import { openHelp } from './helpModal.js';

/** Repositorio con las herramientas que descargan y preparan las guías de Unreal (REQ-25). */
export const KNOWLEDGE_REPO = 'https://github.com/davidbuenov/dbv-mcp-server';

/** Pestañas de Ajustes, en orden (REQ-60, REQ-74). */
export const SETTINGS_TABS = ['ai', 'tripo', 'worldlabs', 'convai', 'library', 'knowledge', 'mcp', 'project', 'audio', 'diagnostics'];

/** Configuración MCP para pegar en un cliente (Claude Code `.mcp.json`, Cursor…). */
export function mcpClientConfig(command) {
  return JSON.stringify({ mcpServers: { 'dbv-unreal-ai': { command: command.command, args: command.args } } }, null, 2);
}

/**
 * @param {HTMLElement} host
 * @param {object} deps `{toast, openConnect, state}`.
 */
export function createSettingsView(host, deps) {
  let activeTab = 'ai';

  /** Barra de pestañas y paneles: solo se ve el activo. */
  function tabbed(panels) {
    const buttons = SETTINGS_TABS.filter((id) => panels[id]).map((id) => {
      const tab = button(t(`settings.tab.${id}`), () => select(id), 'help-tab-btn');
      tab.dataset.settingsTab = id;
      tab.setAttribute('role', 'tab');
      return tab;
    });
    const sections = SETTINGS_TABS.filter((id) => panels[id]).map((id) => {
      const panel = el('div', { className: 'stack settings-panel', attrs: { role: 'tabpanel', 'data-settings-panel': id } }, ...[panels[id]].flat().filter(Boolean));
      return panel;
    });
    function select(id) {
      activeTab = id;
      for (const b of buttons) b.classList.toggle('active', b.dataset.settingsTab === id);
      for (const p of sections) p.classList.toggle('hidden', p.dataset.settingsPanel !== id);
    }
    select(panels[activeTab] ? activeTab : 'ai');
    return [el('div', { className: 'help-tabs settings-tabs', attrs: { role: 'tablist' } }, ...buttons), ...sections];
  }

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
    const worldlabsBudget = el('input', { attrs: { type: 'number', min: '0', step: '10', value: String(status.ok ? status.value.budgetByService?.worldlabs ?? 2000 : 2000), 'aria-label': t('settings.worldlabsBudget') } });
    const port = deps.state.connection?.endpoint?.port ?? 8000;
    const [command, location, projectSettingsRes] = await Promise.all([
      api.appMcpCommand(port, deps.state.connection?.projectPath ?? null),
      api.libraryLocation(),
      api.unrealGetProjectSettings ? api.unrealGetProjectSettings() : Promise.resolve({ ok: true, value: {} }),
    ]);
    const projectSettings = projectSettingsRes?.ok && projectSettingsRes.value ? projectSettingsRes.value : {};
    const copyright = el('input', { attrs: { type: 'text', value: projectSettings.copyrightHolder ?? '', placeholder: t('settings.projectCopyrightPlaceholder'), 'aria-label': t('settings.projectCopyright') } });
    const localAudioStatus = el('p', { className: 'hint', text: '' });
    const checkAudioBtn = button(t('gen.soundLocalCheck'), async () => {
      checkAudioBtn.disabled = true;
      localAudioStatus.textContent = t('gen.soundGenerating');
      const st = await api.generationCheckLocalAudio();
      checkAudioBtn.disabled = false;
      if (st?.ready) {
        localAudioStatus.textContent = t('gen.soundLocalReady', { device: st.device || 'GPU' });
        localAudioStatus.className = 'hint check-ok';
      } else {
        localAudioStatus.textContent = t('gen.soundLocalOffline');
        localAudioStatus.className = 'hint check-fail';
      }
    }, 'btn-ghost');
    const hasKey = status.ok && status.value.hasKey?.tripo;
    const balance = el('p', { className: 'hint' });
    if (hasKey) api.generationBalance().then((result) => (balance.textContent = result.ok && result.value ? t('gen.balance', { balance: String(result.value.balance), frozen: String(result.value.frozen) }) : (result.error?.message ?? '')));
    // Los créditos de World Labs, como los de Tripo: es otra cuenta (REQ-18).
    const worldlabsBalance = el('p', { className: 'hint' });
    if (status.ok && status.value.hasKey?.worldlabs) api.generationWorldLabsBalance().then((result) => (worldlabsBalance.textContent = result.ok && result.value ? t('gen.worldLabsBalance', { balance: String(result.value.balance) }) : (result.error?.message ?? '')));
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
        // REQ-53: la biblioteca pasó a Documentos; lo que había en la ubicación antigua se copió y se conserva.
        location.value.migrated ? el('p', { className: 'hint', text: t('settings.libraryMigrated', { n: String(location.value.migrated) }) }) : null,
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
    const convai = await convaiSettingsCard({ toast: deps.toast });
    // REQ-25: la copia local de las guías de Unreal (formato de MCPCLiente); la app no descarga nada de Epic.
    const listed = await api.guidesStatus();
    const guides = listed.ok && listed.value ? listed.value : null;
    async function chooseGuides(folder) {
      const chosen = await api.guidesSetFolder(folder);
      if (!chosen.ok) return deps.toast(chosen.error.message ?? String(chosen.error), 'error');
      deps.toast(folder ? t('settings.knowledgeSaved', { n: String(chosen.value.chunks) }) : t('settings.knowledgeRemoved'));
      return render();
    }
    const knowledge = el(
      'div',
      { className: 'card stack' },
      el('h3', { text: t('settings.knowledge') }),
      el('p', { className: 'hint', text: t('settings.knowledgeHint') }),
      el('p', { className: 'hint', text: t('settings.knowledgeHow') }),
      el('div', { className: 'row' }, button(t('settings.knowledgeRepo'), () => window.__TAURI__?.opener?.openUrl?.(KNOWLEDGE_REPO), 'btn-ghost')),
      guides?.folder ? el('p', { className: 'mono', text: guides.folder }) : null,
      el('p', {
        className: guides && !guides.problem ? 'hint' : 'hint check-fail',
        text: guides && !guides.problem
          ? t('settings.knowledgeStatus', { guides: String(guides.guides), chunks: String(guides.chunks), updated: guides.updated || '—' })
          : guides?.folder ? guides.problem : t('settings.knowledgeNone'),
      }),
      el(
        'div',
        { className: 'row' },
        button(t('settings.knowledgeChoose'), async () => {
          const folder = await window.__TAURI__.dialog.open({ directory: true });
          return folder ? chooseGuides(String(folder)) : null;
        }),
        guides?.folder ? button(t('settings.knowledgeRemove'), () => chooseGuides(null), 'btn-ghost') : null,
      ),
    );
    const panels = {
      ai: el('div', { className: 'card stack' }, el('h3', { text: t('settings.ai') }), el('div', { className: 'row' }, button(t('settings.aiConnect'), deps.openConnect, 'btn-primary'), button(`📖 ${t('settings.helpAi')}`, () => openHelp('ai'), 'btn-ghost'))),
      project: el(
        'div',
        { className: 'card stack' },
        el('h3', { text: t('settings.project') }),
        el('p', { className: 'hint', text: t('settings.projectCopyrightHint') }),
        el('label', {}, t('settings.projectCopyright'), copyright),
        el('div', { className: 'row' }, button(t('settings.save'), async () => {
          const holder = copyright.value.trim() || null;
          const saved = await api.unrealSetProjectSettings({ copyrightHolder: holder });
          if (!saved.ok) return deps.toast(saved.error.message, 'error');
          deps.toast(t('settings.projectSaved'));
        }, 'btn-primary')),
      ),
      audio: el(
        'div',
        { className: 'card stack' },
        el('h3', { text: t('settings.audio') }),
        el(
          'div',
          { className: 'row' },
          button(`📖 ${t('settings.helpAudio')}`, () => openHelp('audio'), 'btn-ghost'),
          button('ElevenLabs API Keys ↗', () => window.__TAURI__?.opener?.openUrl?.('https://elevenlabs.io/app/developers/api-keys'), 'btn-ghost button--compact'),
          button('Freesound API ↗', () => window.__TAURI__?.opener?.openUrl?.('https://freesound.org/apiv2/apply'), 'btn-ghost button--compact'),
        ),
        el('p', { className: 'hint', text: t('settings.audioHint') }),
        ...keyControls('elevenlabs', status.ok && status.value.hasKey?.elevenlabs),
        ...keyControls('freesound', status.ok && status.value.hasKey?.freesound),
        el('div', { className: 'row', style: 'margin-top: 12px;' }, checkAudioBtn),
        localAudioStatus,
      ),
      tripo: el(
        'div',
        { className: 'card stack' },
        brandHeader('Tripo', 'tripo'),
        balance,
        el('div', { className: 'row' }, button(`📖 ${t('settings.helpTripo')}`, () => openHelp('tripo'), 'btn-ghost')),
        ...keyControls('tripo', hasKey),
        el('label', {}, t('settings.budget'), budget),
        el('div', { className: 'row' }, button(t('settings.save'), async () => {
          const saved = await api.generationSetBudget(Number(budget.value) || 0);
          deps.toast(saved.ok ? t('settings.budgetSaved') : saved.error.message, saved.ok ? 'info' : 'error');
        })),
      ),
      // @spec REQ-18, RNF-IA.4 — la clave de World Labs, en el mismo llavero que la de Tripo.
      worldlabs: el(
        'div',
        { className: 'card stack' },
        brandHeader('World Labs', 'worldlabs'),
        worldlabsBalance,
        el('div', { className: 'row' }, button(`📖 ${t('settings.helpWorldLabs')}`, () => openHelp('worldlabs'), 'btn-ghost')),
        el('p', { className: 'hint', text: t('settings.worldlabsHint') }),
        ...keyControls('worldlabs', status.ok && status.value.hasKey?.worldlabs),
        // Su propio tope: los créditos de World Labs no son los de Tripo (REQ-18, RNF-04).
        el('label', {}, t('settings.worldlabsBudget'), worldlabsBudget),
        el('div', { className: 'row' }, button(t('settings.save'), async () => {
          const saved = await api.generationSetBudget(Number(worldlabsBudget.value) || 0, 'worldlabs');
          deps.toast(saved.ok ? t('settings.budgetSaved') : saved.error.message, saved.ok ? 'info' : 'error');
        })),
      ),
      convai,
      library: libraryCard,
      knowledge,
      mcp: el('div', { className: 'card stack' }, el('h3', { text: t('settings.mcp') }), el('p', { className: 'hint', text: t('settings.mcpHint') }), command.ok ? el('pre', { className: 'diff', text: mcpClientConfig(command.value) }) : null),
      diagnostics: el(
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
    };
    host.replaceChildren(el('div', { className: 'view-header' }, el('h2', { text: t('settings.title') })), ...tabbed(panels));
  }
  return { render };
}
