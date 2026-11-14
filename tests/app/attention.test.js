// =============================================================================
// DBV Unreal AI — Tests del aviso cuando la IA espera una aprobación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { describe, expect, it, vi } from 'vitest';
import { needsAttention } from '../../src/app/attention.js';

function fakes({ focused }) {
  const requestUserAttention = vi.fn(() => Promise.resolve());
  const tauri = { window: { getCurrentWindow: () => ({ requestUserAttention }) } };
  const doc = { hidden: false, hasFocus: () => focused };
  const sound = vi.fn();
  return { requestUserAttention, deps: { doc, tauri, sound }, sound };
}

describe('aviso de aprobación pendiente (REQ-115)', () => {
  /** @spec REQ-115 */
  it('con la ventana en segundo plano parpadea en la barra de tareas y suena; delante, no hace nada', () => {
    const away = fakes({ focused: false });
    expect(needsAttention(away.deps)).toBe(true);
    expect(away.requestUserAttention).toHaveBeenCalledWith(2);
    expect(away.sound).toHaveBeenCalledTimes(1);

    const here = fakes({ focused: true });
    expect(needsAttention(here.deps)).toBe(false);
    expect(here.requestUserAttention).not.toHaveBeenCalled();
    expect(here.sound).not.toHaveBeenCalled();
  });

  /** @spec REQ-115 */
  it('sin Tauri ni audio no falla', () => {
    const doc = { hidden: true, hasFocus: () => false };
    expect(needsAttention({ doc, tauri: null, sound: () => { throw new Error('sin audio'); } })).toBe(true);
  });
});
