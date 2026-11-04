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
  root.innerHTML = '<div class="help-card"><button data-help-tab="ue"></button><button data-help-tab="convai"></button><button data-help-tab="tripo"></button><button data-help-tab="worldlabs"></button><button data-help-tab="audio"></button><div id="content"></div></div>';
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

  it('la ayuda de audio explica los tres proveedores, bucles e importación @spec REQ-71', () => {
    window.DBV_I18N.setLang('es');
    const { root, help } = modal();
    help.open('audio');
    const text = root.textContent;
    for (const key of ['Sonidos y Efectos de Sonido', 'dbv-audio-service', 'Stable Audio Open 1.0', 'ElevenLabs SFX', 'Freesound', 'Bucle continuo', 'USoundWave', 'unreal_import_sound']) {
      expect(text).toContain(key);
    }
    expect(root.innerHTML).toContain('https://elevenlabs.io/app/developers/api-keys');
    expect(root.innerHTML).toContain('https://freesound.org/apiv2/apply');
    expect(root.querySelector('[data-help-tab="audio"]').classList.contains('active')).toBe(true);
    window.DBV_I18N.setLang('en');
    help.open('audio');
    expect(root.textContent).toContain('Sound Effects (SFX) and Audio');
    expect(root.textContent).toContain('Seamless loop');
    window.DBV_I18N.setLang('es');
  });

  it('las ayudas de Tripo y World Labs explican sus flujos de trabajo e importación', () => {
    window.DBV_I18N.setLang('es');
    const { root, help } = modal();
    help.open('tripo');
    expect(root.textContent).toContain('Modelos 3D con Tripo');
    expect(root.textContent).toContain('tripo3d.ai');
    expect(root.textContent).toContain('Rigging automático');
    expect(root.querySelector('[data-help-tab="tripo"]').classList.contains('active')).toBe(true);

    help.open('worldlabs');
    expect(root.textContent).toContain('Mundos y Escenarios con World Labs');
    expect(root.textContent).toContain('UDbvGaussianSplatComponent');
    expect(root.textContent).toContain('Gaussian Splatting');
    expect(root.querySelector('[data-help-tab="worldlabs"]').classList.contains('active')).toBe(true);
  });
});
