// =============================================================================
// DBV Unreal AI — Tests: contador de lo que devuelven las herramientas
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { createChatPanel } from '../../src/ai/chatPanel.js';
import { approxTokens, createUsageCounter, formatUsage, measureResult, WARN_TEXT_CHARS, withUsage } from '../../src/unreal/toolUsage.js';

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

describe('Contador del turno (REQ-51)', () => {
  it('cuenta texto, imágenes y recortes de cada llamada @spec REQ-51', async () => {
    const counter = createUsageCounter();
    const tools = withUsage(
      [
        { name: 'a', run: async () => 'x'.repeat(1000) },
        { name: 'b', run: async () => ({ text: 'captura', images: [{ mime: 'image/jpeg', base64: 'AAAA' }] }) },
        { name: 'c', run: async () => 'mucho\n… (truncated: 20000 characters; ask for less — a filter, a limit or one specific tool)' },
      ],
      counter,
    );
    for (const tool of tools) await tool.run({});
    const summary = counter.summary();
    expect(summary).toMatchObject({ calls: 3, images: 1, clipped: 1 });
    expect(summary.text).toBe(1000 + 'captura'.length + measureResult(await tools[2].run({})).text);
    expect(approxTokens({ text: 4000, images: 2 })).toBe(3000);
  });

  it('se muestra como aproximado y avisa por encima del umbral @spec REQ-51', () => {
    const small = formatUsage({ calls: 7, text: 48 * 1024, images: 1, clipped: 0 });
    expect(small.text).toBe('Herramientas: 7 llamadas · 48 KB · 1 imagen (~13 k tokens, aprox.)');
    expect(formatUsage({ calls: 1, text: 100, images: 0 }).text).toBe('Herramientas: 1 llamada · 1 KB (~25 tokens, aprox.)');
    expect(small.warning).toBe(false);
    expect(formatUsage({ calls: 2, text: WARN_TEXT_CHARS + 1, images: 0 }).warning).toBe(true);
  });

  it('el pie de la respuesta enseña el contador aunque no haya nada que deshacer @spec REQ-51', () => {
    const host = document.createElement('div');
    document.body.append(host);
    const panel = createChatPanel({ host, callbacks: { formatUsage, onUndoTurn: () => {} }, renderMarkdown: (target, markdown) => (target.textContent = markdown) });
    const bubble = panel.addAssistant('hecho');
    bubble.finish?.();
    panel.setLastAssistantUndo({ count: 0, usage: { calls: 3, text: 2048, images: 0, clipped: 0 } });
    const footer = host.querySelector('.ai-msg__footer');
    expect(footer?.textContent).toContain('Herramientas: 3 llamadas · 2 KB');
    expect(footer?.querySelector('.ai-msg__undo-btn')).toBeNull();
  });

  it('al reabrir una conversación, cada respuesta conserva su pie (contador y Deshacer) @spec REQ-51, REQ-40', () => {
    const host = document.createElement('div');
    const panel = createChatPanel({ host, callbacks: { formatUsage, onUndoTurn: () => {} }, renderMarkdown: (target, markdown) => (target.textContent = markdown) });
    panel.addAssistant('Hecho.', { undoInfo: { count: 2, expectedCurrent: 9, usage: { calls: 4, text: 3072, images: 0, clipped: 0 } } });
    const footer = host.querySelector('.ai-msg__footer');
    expect(footer?.textContent).toContain('Herramientas: 4 llamadas · 3 KB');
    expect(footer?.querySelector('.ai-msg__undo-btn')).not.toBeNull();
  });
});
