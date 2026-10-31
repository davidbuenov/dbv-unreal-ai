// =============================================================================
// DBV Unreal AI — Convai y accesos directos en la vista Proyecto
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-52, REQ-55, REQ-59
//
// Lo que ya se sabe por los ficheros llega con el diagnóstico del proyecto (`unreal_readiness`: `directAccess` y
// `convai`). Lo que necesita el editor (los toolsets y las guías de Convai) se pregunta aquí si está conectado (D27).
// La clave de Convai NUNCA se muestra: solo «sesión iniciada en el plugin».

import { t } from '../ai/i18nBridge.js';
import { button, el } from './dom.js';
import { openConvaiHelp } from './convaiSettings.js';

/** Documentación del plugin y de sus herramientas para agentes. */
export const CONVAI_DOCS = 'https://docs.convai.com/api-docs/plugins-and-integrations/convai-unreal-engine-plugin/ai-coding-assistant';

/** ¿Hay toolsets de Convai entre los del editor? */
export const hasConvaiToolsets = (toolsets) => (toolsets ?? []).some((ts) => /ConvaiToolset\./i.test(String(ts?.name ?? ts)));

/** Cuántas AgentSkills de Convai devuelve `ListSkills` (un mapa ruta → descripción). */
export function convaiSkillCount(listed) {
  const value = listed?.value ?? listed;
  const paths = value && typeof value === 'object' && !Array.isArray(value) ? Object.keys(value) : [];
  return paths.filter((path) => path.includes('/ConvAI/Skills/')).length;
}

/** Comprobaciones con el editor conectado: toolsets y guías. `null` si no hay editor. */
export async function convaiLive(api, endpoint) {
  let live = null;
  if (endpoint) {
    const listed = await api.unrealListToolsets(endpoint);
    const toolsets = listed.ok && hasConvaiToolsets(listed.value);
    const skills = toolsets ? await api.unrealCallTool(endpoint, 'ToolsetRegistry.AgentSkillToolset', 'ListSkills', {}) : null;
    live = { toolsets, skills: skills?.ok ? convaiSkillCount(skills.value) : 0 };
  }
  return live;
}

/** Franja de aviso: el agente vería el editor dos veces (REQ-52). La app no toca esos ficheros. */
export function directAccessBanner(list) {
  return list?.length
    ? el(
        'div',
        { className: 'card banner banner--warning', attrs: { role: 'status' } },
        el('strong', { text: t('project.directAccess.title') }),
        el('ul', {}, list.map((item) => el('li', {}, el('span', { className: 'mono', text: item.file }), ` — ${t(`project.directAccess.${item.kind === 'primer' ? 'primer' : 'mcp'}`)}`))),
        el('p', { className: 'hint', text: t('project.directAccess.why') }),
        el('p', { className: 'hint', text: t('project.directAccess.how') }),
      )
    : null;
}

function check(ok, label, why, pending = false) {
  const mark = pending ? '…' : ok ? '✓' : '✗';
  return el('li', { className: 'check' }, el('span', { className: pending ? 'hint' : ok ? 'check-ok' : 'check-fail', text: mark }), el('div', {}, el('span', { text: label }), ok || !why ? null : el('details', {}, el('summary', { text: '?' }), why)));
}

/**
 * Sección Convai (REQ-55): plegada como «opcional» si el proyecto no lo usa.
 * @param {object} status `readiness.convai` (Rust).
 * @param {{toolsets: boolean, skills: number}|null} live Comprobaciones con el editor, o `null` sin editor.
 */
export function convaiCard(status, live) {
  const used = Boolean(status?.cited || status?.version);
  const docs = el('div', { className: 'row' }, button(t('convai.tutorial'), openConvaiHelp), button(t('project.convai.docs'), () => window.__TAURI__?.opener?.openUrl?.(CONVAI_DOCS), 'btn-ghost'));
  let card;
  if (!used) {
    card = el('details', { className: 'card' }, el('summary', { text: t('project.convai.optional') }), el('p', { className: 'hint', text: t('project.convai.intro') }), docs);
  } else {
    const items = [
      check(Boolean(status.enabled && status.version), t('project.convai.plugin', { version: status.version ?? '?' }), t('project.convai.pluginWhy')),
      check(Boolean(status.engineOk), t('project.convai.engine'), t('project.convai.engineWhy')),
      check(Boolean(live?.toolsets), t('project.convai.toolsets'), t('project.convai.toolsetsWhy'), live === null),
      check(Boolean(live?.skills), t('project.convai.skills', { n: String(live?.skills ?? 0) }), t('project.convai.skillsWhy'), live === null),
      check(Boolean(status.signedIn), t('project.convai.signedIn'), t('project.convai.signedInWhy')),
    ];
    if (status.keyInGit) items.push(check(false, t('project.convai.keyInGit'), t('project.convai.keyInGitWhy')));
    card = el(
      'div',
      { className: 'card stack' },
      el('h3', { text: 'Convai' }),
      el('ul', { className: 'checklist' }, items),
      live === null ? el('p', { className: 'hint', text: t('project.convai.connectToCheck') }) : null,
      docs,
    );
  }
  return card;
}
