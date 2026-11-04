// =============================================================================
// DBV Unreal AI — Tests de las imágenes adjuntas a la conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Lo que se protege: un modelo con visión recibe la imagen; uno sin visión, solo su nombre y su ruta (un modelo
// local sin visión falla si le llegan imágenes). En los dos casos la IA conoce la ruta para usarla con Tripo.

import { describe, expect, it, vi } from 'vitest';
import { canSeeImages, userMessage } from '../../src/ai/aiApp.js';
import { createChatPanel } from '../../src/ai/chatPanel.js';
import { pastedAttachments } from '../../src/app/app.js';
import tauriConfig from '../../src-tauri/tauri.conf.json';

const IMAGE = { kind: 'image', path: 'C:/refs/barco.png', name: 'barco.png', mime: 'image/png', base64: 'iVBORw==' };
const PROVIDERS = [{ provider: 'anthropic', cloud: true }, { provider: 'ollama', cloud: false }];

describe('imágenes adjuntas (REQ-32, REQ-68, REQ-73)', () => {
  /** @spec REQ-32 */
  it('decide quién ve imágenes: lo que diga la conexión y, si no dice nada, solo los modelos de nube', () => {
    expect(canSeeImages({ provider: 'ollama', supportsImages: true }, PROVIDERS)).toBe(true);
    expect(canSeeImages({ provider: 'ollama', supportsImages: null }, PROVIDERS)).toBe(false);
    expect(canSeeImages({ provider: 'anthropic', supportsImages: null }, PROVIDERS)).toBe(true);
    expect(canSeeImages({ provider: 'anthropic', supportsImages: false }, PROVIDERS)).toBe(false);
  });

  /** @spec REQ-32 */
  it('con visión la imagen va dentro; sin ella, solo su nota; la ruta va siempre', () => {
    const seen = userMessage('Haz un barco así', [IMAGE], true);
    expect(seen.images).toEqual([{ mime: 'image/png', base64: 'iVBORw==' }]);
    expect(seen.content).toContain('local file: C:/refs/barco.png');
    expect(seen.content).toContain('It is included in this message.');
    const blind = userMessage('Haz un barco así', [IMAGE], false);
    expect(blind.images).toBeUndefined();
    expect(blind.content).toContain('You cannot see it');
    expect(userMessage('Hola', [], true)).toEqual({ role: 'user', content: 'Hola' });
  });

  /** @spec REQ-68 */
  it('pegar una imagen en el cuadro de texto (Ctrl+V) la adjunta y pinta su miniatura', () => {
    const host = document.createElement('div');
    const onPasteFiles = vi.fn();
    const panel = createChatPanel({
      host,
      renderMarkdown: (el, text) => { el.textContent = text; },
      callbacks: { onPasteFiles, onSend: vi.fn(), onStop: vi.fn() },
    });
    const fakeFile = new File(['png-bytes'], 'captura.png', { type: 'image/png' });
    const pasteEvent = new Event('paste', { bubbles: true, cancelable: true });
    Object.defineProperty(pasteEvent, 'clipboardData', {
      value: { items: [{ kind: 'file', type: 'image/png', getAsFile: () => fakeFile }], files: [fakeFile] },
    });
    panel.input.dispatchEvent(pasteEvent);
    expect(pasteEvent.defaultPrevented).toBe(true);
    expect(onPasteFiles).toHaveBeenCalledWith([fakeFile]);

    panel.setAttachments([{ ...IMAGE, label: '🖼 barco.png' }], vi.fn());
    const thumb = host.querySelector('.ai-chip__thumb');
    expect(thumb).not.toBeNull();
    expect(thumb.getAttribute('src')).toBe('data:image/png;base64,iVBORw==');
  });

  /** @spec REQ-68 */
  it('soltar ficheros sobre el panel los adjunta igual que pegarlos', () => {
    // En Windows, el arrastre nativo de Tauri se queda los ficheros y el `drop` HTML5 del panel nunca llega.
    expect(tauriConfig.app.windows[0].dragDropEnabled).toBe(false);
    const host = document.createElement('div');
    const onPasteFiles = vi.fn();
    createChatPanel({ host, renderMarkdown: vi.fn(), callbacks: { onPasteFiles, onSend: vi.fn(), onStop: vi.fn() } });
    const dropped = new File(['# Nivel'], 'brief.md', { type: 'text/markdown' });
    const drop = new Event('drop', { bubbles: true, cancelable: true });
    Object.defineProperty(drop, 'dataTransfer', { value: { files: [dropped], types: ['Files'] } });
    host.dispatchEvent(drop);
    expect(drop.defaultPrevented).toBe(true);
    expect(onPasteFiles).toHaveBeenCalledWith([dropped]);
  });

  /** @spec REQ-68 */
  it('de lo pegado o soltado, la imagen se guarda en disco, el texto se lee y un binario se rechaza', async () => {
    const saveImage = vi.fn(async (mime, data, name) => ({ ok: true, value: { kind: 'image', path: `C:/tmp/${name}`, name, mime, base64: data } }));
    const onError = vi.fn();
    const files = [
      new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], 'captura.png', { type: 'image/png' }),
      new File(['Un puerto pirata'], 'brief.md', { type: 'text/markdown' }),
      new File([new Uint8Array([0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00])], 'anim.gif', { type: 'image/gif' }),
    ];
    const attached = await pastedAttachments(files, { saveImage, onError });
    expect(saveImage).toHaveBeenCalledWith('image/png', 'iVBORw==', 'captura.png');
    expect(attached.map((a) => [a.kind, a.name])).toEqual([['image', 'captura.png'], ['text', 'brief.md']]);
    expect(attached[1].content).toBe('Un puerto pirata');
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onError.mock.calls[0][0]).toContain('anim.gif');
  });

  /** @spec REQ-73 */
  it('el botón de capturar viewport de Unreal se muestra y llama a onCaptureViewport', () => {
    const host = document.createElement('div');
    const onCaptureViewport = vi.fn();
    createChatPanel({
      host,
      renderMarkdown: vi.fn(),
      callbacks: { onCaptureViewport, onSend: vi.fn(), onStop: vi.fn() },
    });
    const btn = host.querySelector('.ai-panel__capture-viewport');
    expect(btn).not.toBeNull();
    expect(btn.classList.contains('hidden')).toBe(false);
    btn.click();
    expect(onCaptureViewport).toHaveBeenCalledTimes(1);
  });
});
