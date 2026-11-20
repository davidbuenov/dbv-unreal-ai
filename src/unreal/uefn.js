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
