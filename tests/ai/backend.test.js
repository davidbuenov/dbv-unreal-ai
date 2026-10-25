// =============================================================================
// DBV Unreal AI — Tests del puente IPC de la IA
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//

import { afterEach, describe, expect, it, vi } from 'vitest';
import { openExternalUrl } from '../../src/ai/backend.js';

describe('openExternalUrl', () => {
  afterEach(() => {
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  it('abre la dirección con el plugin opener de Tauri', async () => {
    const openUrl = vi.fn().mockResolvedValue(undefined);
    window.__TAURI__ = { opener: { openUrl } };
    const popup = vi.spyOn(window, 'open').mockImplementation(() => null);
    const result = await openExternalUrl('https://ollama.com/download');
    expect(openUrl).toHaveBeenCalledWith('https://ollama.com/download');
    expect(popup).not.toHaveBeenCalled();
    expect(result).toEqual({ ok: true, value: undefined });
  });

  it('devuelve un error tipado si el sistema no puede abrirla', async () => {
    window.__TAURI__ = { opener: { openUrl: vi.fn().mockRejectedValue('not allowed') } };
    const result = await openExternalUrl('https://ollama.com/download');
    expect(result).toEqual({ ok: false, error: { kind: 'bridge', message: 'not allowed' } });
  });

  it('fuera de Tauri recurre a window.open', async () => {
    const popup = vi.spyOn(window, 'open').mockImplementation(() => null);
    await openExternalUrl('https://lmstudio.ai');
    expect(popup).toHaveBeenCalledWith('https://lmstudio.ai', '_blank', 'noopener,noreferrer');
  });
});
