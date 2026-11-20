// =============================================================================
// DBV Unreal AI — UEFN en la interfaz: proyectos, distintivo y avisos
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-122
//
// Lo que se calcula sale de Rust (`uefn_overview`); aquí solo se decide qué enseñar, con funciones puras que se
// prueban sin la app. UEFN solo existe en Windows: fuera de él, `install` llega vacío y no se muestra nada.

import { t } from '../ai/i18nBridge.js';
import { button, el } from '../app/dom.js';

/**
 * Filas de la sección UEFN de la vista Proyecto, con el proyecto abierto primero.
 * @param {{install: {version: {version: string}} | null, running: boolean, projects: Array<object>} | null} overview
 * @returns {{version: string, rows: Array<{title: string, path: string, open: boolean, newer: boolean, linked: boolean, compatibilityVersion: string}>}}
 */
export function uefnRows(overview) {
  const version = overview?.install?.version?.version ?? '';
  const projects = version ? (overview?.projects ?? []) : [];
  const rows = projects
    .map((p) => ({ title: p.title, path: p.path, open: Boolean(p.open), newer: Boolean(p.newerThanInstalled), linked: Boolean(p.linked), compatibilityVersion: p.compatibilityVersion ?? '' }))
    .sort((a, b) => Number(b.open) - Number(a.open));
  return { version, rows };
}

/**
 * Sección UEFN de la vista Proyecto (DESIGN.md v0.7.0): `[UEFN]` y la versión junto al nombre, y un aviso si el
 * proyecto pide una UEFN más nueva que la instalada.
 * @param {object | null} overview Respuesta de `uefn_overview`.
 * @param {{select: (path: string) => void}} actions
 * @returns {HTMLElement[]}
 */
export function uefnSection(overview, actions) {
  const { version, rows } = uefnRows(overview);
  const nodes = version ? [el('div', { className: 'subheading', text: t('uefn.projects', { version }) })] : [];
  if (version && !rows.length) nodes.push(el('p', { className: 'empty', text: t('uefn.noProjects') }));
  for (const row of rows) {
    const status = row.open ? el('span', { className: 'pill pill--online', text: t('uefn.open') }) : null;
    const warning = row.newer ? el('p', { className: 'warning', text: t('uefn.newerProject', { project: row.compatibilityVersion, installed: version }) }) : null;
    nodes.push(
      el(
        'div',
        { className: 'card editor-card' },
        el(
          'div',
          { className: 'row' },
          el('div', {}, el('strong', { text: row.title }), ' ', el('span', { className: 'badge badge--accent', text: `[UEFN] ${version}` }), el('div', { className: 'hint mono', text: row.path }), warning),
          el('div', { className: 'row' }, status, button(t('project.select'), () => actions.select(row.path))),
        ),
      ),
    );
  }
  return nodes;
}

/**
 * ¿Hay que ofrecer «Activar Python»? Solo con el servidor de UEFN respondiendo (fila 5) y Python apagado (fila 4).
 * @param {Array<{step: number, ok: boolean}>} rows
 */
export const canEnablePython = (rows) => rows.some((r) => r.step === 5 && r.ok) && rows.some((r) => r.step === 4 && !r.ok);

/**
 * Tarjeta del diagnóstico de UEFN (REQ-123, DESIGN.md v0.7.0): la lista numerada de REQ-120 con solo las filas de
 * UEFN, cada una con su estado y su solución en una frase. Python es opcional: si falta, se marca con «–», no con «✗».
 * @param {{title: string}} project
 * @param {Array<{step: number, key: string, ok: boolean, optional: boolean, detail: string, fix: string | null}>} rows
 * @param {{connect: () => void, enablePython: () => void, connected: boolean}} actions
 * @returns {HTMLElement}
 */
export function uefnDiagnosticCard(project, rows, actions) {
  const items = rows.map((row) => {
    const mark = row.ok ? '✓' : row.optional ? '–' : '✗';
    const label = t(row.key, { detail: row.detail });
    const fix = row.ok || !row.fix ? null : el('p', { className: 'check-fix', text: t(row.fix, { detail: row.detail }) });
    return el('li', { className: 'check' }, el('span', { className: row.ok ? 'check-ok' : 'check-fail', text: mark, attrs: { 'aria-label': row.ok ? 'ok' : 'falta' } }), el('div', {}, el('span', { text: `${row.step}. ${label}` }), fix));
  });
  const ready = rows.filter((r) => !r.optional).every((r) => r.ok);
  const buttons = [
    actions.connected ? el('span', { className: 'pill pill--online', text: t('uefn.connected') }) : button(t('uefn.connect'), actions.connect, ready ? 'btn-primary' : ''),
    canEnablePython(rows) ? button(t('uefn.enablePython'), actions.enablePython) : null,
  ];
  return el(
    'div',
    { className: 'card' },
    el('h3', {}, el('span', { text: project.title }), ' ', el('span', { className: 'badge badge--accent', text: '[UEFN]' })),
    el('p', { className: 'hint', text: t('uefn.diagIntro') }),
    el('ul', { className: 'checklist' }, items),
    el('div', { className: 'row' }, ...buttons.filter(Boolean)),
  );
}
