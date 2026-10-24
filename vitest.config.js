// =============================================================================
// DBV Unreal AI — Configuración de los tests del frontend (Vitest)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Tests del frontend (`tests/**/*.test.js`) con `jsdom`, porque parte de los módulos pintan DOM (el panel, las
// vistas, las tarjetas). Viven FUERA de `src/` a propósito: `frontendDist` es `../src` y Tauri embebe en el
// ejecutable todo lo que haya dentro. `three` se resuelve como en la app (mapa de importación de `index.html`).

import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  resolve: { alias: { three: fileURLToPath(new URL('./src/vendor/three/build/three.module.min.js', import.meta.url)) } },
  test: {
    include: ['tests/**/*.test.js'],
    environment: 'jsdom',
    setupFiles: ['./vitest.setup.js'],
    restoreMocks: true,
    reporters: ['default'],
  },
});
