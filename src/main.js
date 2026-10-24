// =============================================================================
// DBV Unreal AI — Arranque de la ventana
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Script clásico en IIFE (NATIVE_DESKTOP_APPS.md §3): idioma, tema, «Acerca de» y cierre. La
// aplicación son módulos ES que se cargan con `import()` (`app/app.js`), la misma excepción que el
// starter usaba para `ai/`: sin bundler, los módulos se piden solo cuando hacen falta.
(function () {
  const { getCurrentWindow } = window.__TAURI__.window;
  const { getVersion } = window.__TAURI__.app;
  const { t, applyTranslations, setLanguage, getLanguage } = window.dbvI18n;
  const THEME_KEY = 'dbv-unreal-ai:theme';
  let app = null;

  function readTheme() {
    let stored = null;
    try {
      stored = localStorage.getItem(THEME_KEY);
    } catch {
      stored = null;
    }
    const prefersLight = window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches;
    return stored === 'light' || stored === 'dark' ? stored : prefersLight ? 'light' : 'dark';
  }

  function applyTheme(theme) {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem(THEME_KEY, theme);
    } catch {
      // Sin almacenamiento local el tema vale para esta sesión.
    }
  }

  window.addEventListener('DOMContentLoaded', async () => {
    applyTheme(readTheme());
    const langSelect = document.querySelector('#lang-select');
    langSelect.value = getLanguage();
    langSelect.addEventListener('change', () => setLanguage(langSelect.value));
    document.querySelector('#btn-theme').addEventListener('click', () => applyTheme(document.documentElement.dataset.theme === 'light' ? 'dark' : 'light'));
    document.querySelector('#btn-about').addEventListener('click', async () => {
      document.querySelector('#about-version').textContent = t('about.version', { version: await getVersion() });
      document.querySelector('#about-modal').classList.remove('hidden');
    });
    document.querySelector('#about-close').addEventListener('click', () => document.querySelector('#about-modal').classList.add('hidden'));
    applyTranslations();
    try {
      const module = await import('./app/app.js');
      app = module.start();
    } catch (error) {
      console.error('[app]', error);
    }
    // Al cerrar se detienen la respuesta en curso y el agente, si los hay.
    await getCurrentWindow().onCloseRequested(() => app?.dispose());
  });
})();
