// =============================================================================
// DBV Unreal AI — Tests: tutorial de Convai en la ayuda (F1)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it } from 'vitest';
import { createHelpModal } from '../../src/app/helpModal.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

beforeAll(() => window.DBV_I18N.register(APP_TRANSLATIONS));

function modal() {
  const root = document.createElement('div');
  root.innerHTML = '<div class="help-card"><button data-help-tab="ue"></button><button data-help-tab="convai"></button><div id="content"></div></div>';
  return { root, help: createHelpModal(root, root.querySelector('#content')) };
}

describe('Tutorial de Convai en F1 (REQ-59)', () => {
  it('dice los cuatro requisitos, los pasos y los problemas conocidos, en el idioma de la interfaz @spec REQ-59', () => {
    window.DBV_I18N.setLang('es');
    const { root, help } = modal();
    help.open('convai');
    const text = root.textContent;
    for (const needed of ['Unreal Engine 5.8 o posterior', 'plugin Convai', 'sesión iniciada', 'Playground', 'Character ID', 'MetaHuman ensamblado', 'Ajustes → Convai', 'Problemas conocidos', 'metahuman_base_skel', 'Escort', '.mcp.json']) {
      expect(text).toContain(needed);
    }
    expect(root.querySelector('[data-help-tab="convai"]').classList.contains('active')).toBe(true);
    window.DBV_I18N.setLang('en');
    help.open('convai');
    expect(root.textContent).toContain('What you need');
    window.DBV_I18N.setLang('es');
  });
});
