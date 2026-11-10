// =============================================================================
// DBV Unreal AI — Tests del botón «Caja negra» del panel de conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createChatPanel } from '../../src/ai/chatPanel.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

/** El panel con los callbacks que se le den (el resto no hace falta para pintarse). */
function panelWith(callbacks) {
  const host = document.createElement('div');
  document.body.append(host);
  createChatPanel({ host, callbacks, renderMarkdown: (target, markdown) => (target.textContent = markdown) });
  return { host };
}

describe('botón «Caja negra» (REQ-94)', () => {
  it('sin el callback del host el botón no se ve @spec REQ-94', () => {
    const { host } = panelWith({});
    expect(host.querySelector('.ai-panel__blackbox').classList.contains('hidden')).toBe(true);
  });

  it('el botón abre el desplegable con tres modos y elegir uno lo cierra y avisa @spec REQ-94', () => {
    const onBlackbox = vi.fn();
    const { host } = panelWith({ onBlackbox });
    const menu = host.querySelector('.ai-panel__blackbox-menu');
    const button = host.querySelector('.ai-panel__blackbox-button');
    expect(menu.classList.contains('hidden')).toBe(true);
    button.click();
    expect(menu.classList.contains('hidden')).toBe(false);
    expect(button.getAttribute('aria-expanded')).toBe('true');
    expect([...menu.querySelectorAll('button')].map((b) => b.dataset.mode)).toEqual(['pie', 'attach', 'choose']);
    menu.querySelector('[data-mode="attach"]').click();
    expect(onBlackbox).toHaveBeenCalledWith('attach');
    expect(menu.classList.contains('hidden')).toBe(true);
    expect(button.getAttribute('aria-expanded')).toBe('false');
  });

  it('un clic fuera o Esc cierran el desplegable @spec REQ-94', () => {
    const { host } = panelWith({ onBlackbox: vi.fn() });
    const menu = host.querySelector('.ai-panel__blackbox-menu');
    host.querySelector('.ai-panel__blackbox-button').click();
    document.body.click();
    expect(menu.classList.contains('hidden')).toBe(true);
    host.querySelector('.ai-panel__blackbox-button').click();
    host.querySelector('.ai-panel__blackbox').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(menu.classList.contains('hidden')).toBe(true);
  });

  it('el desplegable usa los colores del tema (claro, oscuro y sepia) y no colores fijos @spec REQ-94', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/styles.css'), 'utf8');
    const block = css.slice(css.indexOf('/* Caja negra (REQ-94)'));
    // Un `var(--surface, #1e2230)` con respaldo oscuro dejó el texto ilegible en el tema sepia (visto por el usuario).
    expect(block).not.toMatch(/var\(--surface|#1e2230|rgba\(255, 255, 255/);
    for (const token of ['--bg-secondary', '--text-primary', '--border', '--bg-tertiary', '--shadow']) expect(block).toContain(`var(${token})`);
  });

  it('los textos existen en es, en y zh-CN para el botón, los modos y las tres peticiones @spec REQ-94', async () => {
    const read = (file) => readFileSync(resolve(process.cwd(), file), 'utf8');
    const keys = ['ai.blackbox', 'ai.blackboxHint', ...['pie', 'attach', 'choose'].flatMap((m) => [`ai.blackbox.${m}`, `ai.blackbox.${m}Hint`])];
    const aiEs = read('src/ai/translations.js');
    const aiZh = read('src/ai/translations.zh.js');
    for (const key of keys) {
      expect(aiEs.split(`'${key}':`).length - 1, `${key} en es y en`).toBe(2);
      expect(aiZh, `${key} en zh`).toContain(`'${key}':`);
    }
    const appEs = read('src/app/translations.js');
    const appZh = read('src/app/translations.zh.js');
    for (const mode of ['pie', 'attach', 'choose']) {
      const key = `'unreal.blackboxPrompt.${mode}':`;
      expect(appEs.split(key).length - 1, `${key} en es y en`).toBe(2);
      expect(appZh, `${key} en zh`).toContain(key);
    }
    // Las peticiones apuntan al flujo con las herramientas reales.
    for (const tool of ['session_plan', 'session_start', 'session_stop', 'session_analyze']) expect(appEs).toContain(tool);
  });
});
