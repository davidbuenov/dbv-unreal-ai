// =============================================================================
// DBV Unreal AI — Tests del visor 3D (partes sin WebGL)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// jsdom no tiene WebGL: el pintado real lo comprueba el banco E2E con un GLB de prueba. Aquí, el encuadre, los
// nombres de las animaciones y qué ficheros se pueden ver.

import { beforeAll, describe, expect, it } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { clipLabel, frameDistance } from '../../src/generation/modelViewer.js';
import { generateThumbnailForEntry, openPreview, viewableFile } from '../../src/generation/preview.js';

beforeAll(() => {
  window.DBV_I18N?.register(APP_TRANSLATIONS);
  window.DBV_I18N?.setLang('es');
});

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

  /** @spec REQ-31 */
  it('generateThumbnailForEntry reutiliza la miniatura si existe o retorna null sin fichero 3D', async () => {
    const existing = await generateThumbnailForEntry({ thumbnail: 'thumb.png' });
    expect(existing).toBe('thumb.png');

    const noFile = await generateThumbnailForEntry({ files: ['test.txt'] });
    expect(noFile).toBeNull();
  });

  /** @spec REQ-31, REQ-16 */
  it('openPreview no pasa elementos nulos ni texto literal null a openModal para modelos y animaciones', async () => {
    let capturedChildren = null;
    const deps = {
      openModal: (children) => { capturedChildren = children; },
      closeModal: () => {},
    };

    // Modelo normal (sin aviso de colisión ni animación)
    await openPreview({ name: 'Pirata', kind: 'model' }, deps);
    expect(capturedChildren).toBeTruthy();
    expect(capturedChildren.every((c) => c !== null && c !== undefined)).toBe(true);
    expect(capturedChildren.some((c) => c.textContent === 'null')).toBe(false);

    // Animación (con aviso de animación esquelética)
    await openPreview({ name: 'Pumuki dance', kind: 'animation' }, deps);
    expect(capturedChildren).toBeTruthy();
    expect(capturedChildren.every((c) => c !== null && c !== undefined)).toBe(true);
    expect(capturedChildren.some((c) => c.textContent === 'null')).toBe(false);
    expect(capturedChildren.some((c) => c.textContent?.includes('animación esquelética'))).toBe(true);

    // Mundo (con aviso de malla de colisión)
    await openPreview({ name: 'Playa', kind: 'world' }, deps);
    expect(capturedChildren).toBeTruthy();
    expect(capturedChildren.every((c) => c !== null && c !== undefined)).toBe(true);
    expect(capturedChildren.some((c) => c.textContent === 'null')).toBe(false);
    expect(capturedChildren.some((c) => c.textContent?.includes('colisión'))).toBe(true);
  });
});

