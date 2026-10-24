// =============================================================================
// DBV Unreal AI — Tests del visor 3D (partes sin WebGL)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// jsdom no tiene WebGL: el pintado real lo comprueba el banco E2E con un GLB de prueba. Aquí, el encuadre, los
// nombres de las animaciones y qué ficheros se pueden ver.

import { describe, expect, it } from 'vitest';
import { clipLabel, frameDistance } from '../../src/generation/modelViewer.js';
import { viewableFile } from '../../src/generation/preview.js';

describe('visor 3D (REQ-31)', () => {
  /** @spec REQ-31 */
  it('encuadra el modelo entero: más lejos cuanto más grande o más cerrado el campo de visión', () => {
    const near = frameDistance(1, 40);
    expect(near).toBeGreaterThan(1 / Math.sin((20 * Math.PI) / 180));
    expect(frameDistance(2, 40)).toBeCloseTo(near * 2);
    expect(frameDistance(1, 20)).toBeGreaterThan(near);
    expect(frameDistance(0, 40)).toBeGreaterThan(0);
  });

  /** @spec REQ-31, REQ-16 */
  it('nombra los clips de Tripo y solo intenta ver GLB o glTF', () => {
    expect(clipLabel('preset:biped:dance_01')).toBe('dance 01');
    expect(clipLabel('')).toBe('clip');
    expect(viewableFile({ files: ['model.fbx', 'model.GLB'] })).toBe('model.GLB');
    expect(viewableFile({ files: ['barco.png'] })).toBeNull();
    expect(viewableFile({})).toBeNull();
  });
});
