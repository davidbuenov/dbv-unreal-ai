// =============================================================================
// DBV Unreal AI — Tests del orden de la conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Con un agente un turno entero usa una sola burbuja. La respuesta salía ARRIBA del todo y los pasos y la
// tarjeta de permiso debajo, aunque llegaran antes: el usuario esperaba la respuesta tras sus aprobaciones.

import { beforeAll, describe, expect, it } from 'vitest';
import { createChatPanel } from '../../src/ai/chatPanel.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});


function panelWithMessages() {
  const host = document.createElement('div');
  document.body.append(host);
  const panel = createChatPanel({ host, callbacks: {}, renderMarkdown: (target, markdown) => (target.textContent = markdown) });
  const messages = host.querySelector('[aria-live]') ?? host;
  return { panel, host, messages };
}

/** Textos visibles de la conversación, en el orden en que aparecen. */
const visible = (host) => [...host.querySelectorAll('.ai-msg__body, .ai-step, .ai-note')].filter((node) => !node.closest('.hidden') && node.textContent.trim()).map((node) => node.textContent.trim());

describe('orden de la conversación', () => {
  /** @spec REQ-12 */
  it('la respuesta va DESPUÉS de los pasos y de la tarjeta de permiso que la precedieron', () => {
    const { panel, host } = panelWithMessages();
    const bubble = panel.addAssistant();
    panel.addStep('Conectando con el agente');
    const card = document.createElement('div');
    card.className = 'ai-note';
    card.textContent = 'El agente pide permiso';
    panel.addNode(card);
    bubble.append('El nivel tiene ');
    bubble.append('37 actores.');
    bubble.finish();

    expect(visible(host)).toEqual(['⋯ Conectando con el agente', 'El agente pide permiso', 'El nivel tiene 37 actores.']);
    expect(bubble.text()).toBe('El nivel tiene 37 actores.');
  });

  it('el texto anterior a un paso se queda antes y el posterior va en otra burbuja', () => {
    const { panel, host } = panelWithMessages();
    const bubble = panel.addAssistant();
    bubble.append('Voy a mirar el nivel.');
    panel.addStep('unreal_list_toolsets');
    bubble.append('Hay 37 actores.');
    bubble.finish();

    expect(visible(host)).toEqual(['Voy a mirar el nivel.', '⋯ unreal_list_toolsets', 'Hay 37 actores.']);
    expect(bubble.text()).toBe('Voy a mirar el nivel.Hay 37 actores.');
  });

  it('sin pasos de por medio sigue siendo una sola burbuja, y una respuesta vacía no deja burbuja', () => {
    const { panel, host } = panelWithMessages();
    const bubble = panel.addAssistant();
    bubble.append('Hola');
    bubble.append(' mundo');
    bubble.finish('Hola mundo');
    expect(host.querySelectorAll('.ai-msg--assistant')).toHaveLength(1);

    const empty = panel.addAssistant();
    empty.finish('');
    expect(host.querySelectorAll('.ai-msg--assistant')).toHaveLength(1);
  });

  it('el pie del turno enseña «Deshacer este turno» y gestiona la llamada y avisos @spec REQ-40', async () => {
    const host = document.createElement('div');
    document.body.append(host);
    let undoCalled = null;
    const panel = createChatPanel({
      host,
      callbacks: {
        onUndoTurn: async (info, handles) => {
          undoCalled = info;
          if (info.count === 1) handles.setDone();
          else handles.setWarning('El historial ha cambiado');
        },
      },
      renderMarkdown: (target, markdown) => (target.textContent = markdown),
    });

    const bubble = panel.addAssistant('He colocado 3 luces.');
    bubble.setUndo({ count: 1, expectedCurrent: 5 });
    const btn = host.querySelector('.ai-msg__undo-btn');
    expect(btn).not.toBeNull();
    expect(btn.textContent).toContain('Deshacer');

    btn.click();
    await Promise.resolve();
    expect(undoCalled).toEqual({ count: 1, expectedCurrent: 5, undone: true });
    expect(btn.textContent).toContain('deshecho');
    expect(btn.disabled).toBe(true);

    const bubble2 = panel.addAssistant('He cambiado el sol.');
    bubble2.setUndo({ count: 2, expectedCurrent: 8 });
    const btn2 = host.querySelectorAll('.ai-msg__undo-btn')[1];
    btn2.click();
    await Promise.resolve();
    expect(host.textContent).toContain('El historial ha cambiado');
  });
});

