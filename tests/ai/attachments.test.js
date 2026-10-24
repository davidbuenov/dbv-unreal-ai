// =============================================================================
// DBV Unreal AI — Tests de las imágenes adjuntas a la conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Lo que se protege: un modelo con visión recibe la imagen; uno sin visión, solo su nombre y su ruta (un modelo
// local sin visión falla si le llegan imágenes). En los dos casos la IA conoce la ruta para usarla con Tripo.

import { describe, expect, it } from 'vitest';
import { canSeeImages, userMessage } from '../../src/ai/aiApp.js';

const IMAGE = { kind: 'image', path: 'C:/refs/barco.png', name: 'barco.png', mime: 'image/png', base64: 'iVBORw==' };
const PROVIDERS = [{ provider: 'anthropic', cloud: true }, { provider: 'ollama', cloud: false }];

describe('imágenes adjuntas (REQ-32)', () => {
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
});
