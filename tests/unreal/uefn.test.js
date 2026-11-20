// =============================================================================
// DBV Unreal AI — Tests: UEFN en la vista Proyecto
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it, vi } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { uefnRows, uefnSection } from '../../src/unreal/uefn.js';

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

const project = (title, extra = {}) => ({ title, path: `D:/UEFN/${title}/${title}.uefnproject`, linked: false, compatibilityVersion: '43.10', open: false, newerThanInstalled: false, ...extra });
const overview = (projects) => ({ install: { root: 'C:/F', version: { version: '43.12', changelist: 1, source: 'build' } }, running: true, projects });

describe('UEFN en la vista Proyecto (REQ-122)', () => {
  it('pone primero el proyecto abierto y lleva la versión instalada', () => {
    // @spec REQ-122
    const { version, rows } = uefnRows(overview([project('Uno'), project('Abierto', { open: true })]));
    expect(version).toBe('43.12');
    expect(rows.map((r) => r.title)).toEqual(['Abierto', 'Uno']);
  });

  it('sin UEFN instalado (o fuera de Windows) no muestra nada', () => {
    // @spec REQ-122
    expect(uefnSection({ install: null, running: false, projects: [project('Uno')] }, { select: vi.fn() })).toEqual([]);
    expect(uefnSection(null, { select: vi.fn() })).toEqual([]);
  });

  it('muestra el distintivo [UEFN] con la versión y avisa si el proyecto pide una UEFN más nueva', () => {
    // @spec REQ-122
    const select = vi.fn();
    const nodes = uefnSection(overview([project('Nuevo', { compatibilityVersion: '43.20', newerThanInstalled: true })]), { select });
    const text = nodes.map((n) => n.textContent).join(' ');
    expect(text).toContain('[UEFN] 43.12');
    expect(text).toContain('UEFN 43.20');
    expect(nodes[1].querySelector('.warning')).not.toBeNull();
    nodes[1].querySelector('button').click();
    expect(select).toHaveBeenCalledWith('D:/UEFN/Nuevo/Nuevo.uefnproject');
  });

  it('con UEFN y sin proyectos lo dice', () => {
    // @spec REQ-122
    const nodes = uefnSection(overview([]), { select: vi.fn() });
    expect(nodes.map((n) => n.textContent).join(' ')).toContain('no se ha encontrado ningún proyecto');
  });
});
