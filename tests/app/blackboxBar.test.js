// =============================================================================
// DBV Unreal AI — Tests del panel de grabación de la caja negra
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it, vi } from 'vitest';
import { clock, createBlackboxBar } from '../../src/app/blackboxBar.js';
import { t } from '../../src/ai/i18nBridge.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

const STARTED = '2026-11-10T20:00:00';
const at = (seconds) => Date.parse(STARTED) + seconds * 1000;

/** Panel con una grabación «en marcha» que se puede cambiar desde el test. */
function barWith({ info = { id: 's1', startedAt: STARTED, marks: 0 }, run, now = at(23), enabled = () => true } = {}) {
  const state = { info, now };
  const runTool = run ?? vi.fn(async () => ({ ok: true, value: 'ok' }));
  const bar = createBlackboxBar({ active: async () => ({ ok: true, value: state.info }), run: runTool, t, enabled, nowMs: () => state.now });
  return { bar, state, runTool };
}

describe('panel de grabación (REQ-94)', () => {
  it('el reloj da mm:ss @spec REQ-94', () => {
    expect([clock(0), clock(59.9), clock(61), clock(3725), clock(-3)]).toEqual(['00:00', '00:59', '01:01', '62:05', '00:00']);
  });

  it('está oculto sin grabación y se enciende con el tiempo y las marcas @spec REQ-94', async () => {
    const { bar, state } = barWith({ info: null });
    await bar.refresh();
    expect(bar.node.classList.contains('hidden')).toBe(true);
    state.info = { id: 's1', startedAt: STARTED, marks: 2 };
    await bar.refresh();
    expect(bar.node.classList.contains('hidden')).toBe(false);
    expect(bar.node.querySelector('.blackbox-bar__status').textContent).toBe('Grabando · 00:23 · 2 marcas');
  });

  it('«Marcar momento» llama a session_mark con un nombre numerado y suma la marca @spec REQ-94', async () => {
    const { bar, state, runTool } = barWith({ info: { id: 's1', startedAt: STARTED, marks: 1 } });
    await bar.refresh();
    runTool.mockImplementation(async () => {
      state.info = { ...state.info, marks: 2 }; // lo que leería el siguiente `active`
      return { ok: true, value: 'Marked' };
    });
    bar.node.querySelectorAll('button')[0].click();
    await vi.waitFor(() => expect(bar.node.querySelector('.blackbox-bar__status').textContent).toContain('2 marcas'));
    expect(runTool).toHaveBeenCalledWith('session_mark', { label: 'Momento 2' });
    expect(bar.node.querySelector('.blackbox-bar__note').textContent).toBe('Marca 2 guardada');
  });

  it('el botón de marcar recuerda la tecla del juego, que no le quita el foco @spec REQ-94', async () => {
    const { bar } = barWith({ info: { id: 's1', startedAt: STARTED, marks: 0, markKey: 'F11' } });
    await bar.refresh();
    expect(bar.node.querySelectorAll('button')[0].title).toBe('F11 en el juego marca sin perder el foco');
  });

  it('«Terminar» llama a session_stop y el panel se apaga cuando la grabación ya no está activa @spec REQ-94', async () => {
    const { bar, state, runTool } = barWith();
    await bar.refresh();
    runTool.mockImplementation(async () => {
      state.info = null;
      return { ok: true, value: '{}' };
    });
    bar.node.querySelectorAll('button')[1].click();
    await vi.waitFor(() => expect(bar.node.classList.contains('hidden')).toBe(true));
    expect(runTool).toHaveBeenCalledWith('session_stop', {});
  });

  it('un error de la herramienta se enseña y los botones vuelven a estar disponibles @spec REQ-94', async () => {
    const { bar } = barWith({ run: vi.fn(async () => ({ ok: false, error: { message: 'no editor' } })) });
    await bar.refresh();
    bar.node.querySelectorAll('button')[0].click();
    await vi.waitFor(() => expect(bar.node.querySelector('.blackbox-bar__note').textContent).toBe('no editor'));
    expect([...bar.node.querySelectorAll('button')].every((b) => !b.disabled)).toBe(true);
  });

  it('sin editor conectado no pregunta ni enseña nada @spec REQ-94', async () => {
    const active = vi.fn(async () => ({ ok: true, value: { id: 's', startedAt: STARTED, marks: 0 } }));
    const bar = createBlackboxBar({ active, run: vi.fn(), t, enabled: () => false });
    await bar.refresh();
    expect(active).not.toHaveBeenCalled();
    expect(bar.node.classList.contains('hidden')).toBe(true);
  });

  it('los textos existen en es, en y zh-CN @spec REQ-94', async () => {
    const { readFileSync } = await import('node:fs');
    const zh = readFileSync('src/app/translations.zh.js', 'utf8');
    const es = readFileSync('src/app/translations.js', 'utf8');
    for (const key of ['recording', 'marks', 'mark', 'finish', 'moment', 'marked', 'finishing', 'finished', 'key']) {
      expect(zh).toContain(`'blackbox.${key}':`);
      expect(es.split(`'blackbox.${key}':`).length - 1, key).toBe(2);
    }
  });
});
