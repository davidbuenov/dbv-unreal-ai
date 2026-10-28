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
  const THEMES = ['light', 'dark', 'sepia'];
  let app = null;

  function readTheme() {
    let stored = null;
    try {
      stored = localStorage.getItem(THEME_KEY);
    } catch {
      stored = null;
    }
    const prefersLight = window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches;
    return THEMES.includes(stored) ? stored : prefersLight ? 'light' : 'dark';
  }

  function applyTheme(theme) {
    document.documentElement.dataset.theme = theme;
    for (const button of document.querySelectorAll('[data-theme-choice]')) button.setAttribute('aria-pressed', String(button.dataset.themeChoice === theme));
    try {
      localStorage.setItem(THEME_KEY, theme);
    } catch {
      // Sin almacenamiento local el tema vale para esta sesión.
    }
  }

  // «Mantener encima»: por ventana y sin persistencia (una decisión del momento, no una preferencia). Solo se
  // da por bueno lo que la ventana confirma: si falla, el botón no se queda diciendo que está fijada.
  let pinned = false;

  function showPin() {
    const button = document.querySelector('#btn-pin');
    const label = t(pinned ? 'action.alwaysOnTopActive' : 'action.alwaysOnTop');
    button.setAttribute('aria-pressed', String(pinned));
    button.setAttribute('data-i18n-title', pinned ? 'action.alwaysOnTopActive' : 'action.alwaysOnTop');
    button.title = label;
    button.setAttribute('aria-label', label);
  }

  async function togglePin() {
    try {
      await getCurrentWindow().setAlwaysOnTop(!pinned);
      pinned = !pinned;
    } catch (error) {
      console.error('[pin]', error);
      document.querySelector('#btn-pin').title = t('action.alwaysOnTopFailed');
    }
    showPin();
  }

  window.addEventListener('DOMContentLoaded', async () => {
    applyTheme(readTheme());
    for (const button of document.querySelectorAll('[data-theme-choice]')) button.addEventListener('click', () => applyTheme(button.dataset.themeChoice));
    const markLanguage = () => {
      for (const button of document.querySelectorAll('[data-lang-choice]')) button.setAttribute('aria-pressed', String(button.dataset.langChoice === getLanguage()));
    };
    for (const button of document.querySelectorAll('[data-lang-choice]')) {
      button.addEventListener('click', () => {
        setLanguage(button.dataset.langChoice);
        markLanguage();
        showPin();
      });
    }
    markLanguage();
    document.querySelector('#btn-pin').addEventListener('click', togglePin);

    // Telemetría para el modal Acerca de (Typst style)
    const platformEl = document.querySelector('#about-platform-val');
    if (platformEl) {
      const p = navigator.userAgentData?.platform || (navigator.platform?.toLowerCase().includes('win') ? 'windows' : navigator.platform?.toLowerCase().includes('mac') ? 'macos' : 'linux');
      platformEl.textContent = p.toLowerCase();
    }

    document.querySelector('#btn-about').addEventListener('click', async () => {
      document.querySelector('#about-version').textContent = t('about.version', { version: await getVersion() });
      document.querySelector('#about-modal').classList.remove('hidden');
    });
    document.querySelector('#about-close').addEventListener('click', () => document.querySelector('#about-modal').classList.add('hidden'));
    document.querySelector('#about-modal').addEventListener('click', (e) => {
      if (e.target === document.querySelector('#about-modal')) document.querySelector('#about-modal').classList.add('hidden');
    });

    // Enlaces de Acerca de con opener externo
    document.querySelector('#about-link-web')?.addEventListener('click', (e) => {
      e.preventDefault();
      window.__TAURI__?.opener?.openUrl('https://davidbuenov.com');
    });
    document.querySelector('#about-link-github')?.addEventListener('click', (e) => {
      e.preventDefault();
      window.__TAURI__?.opener?.openUrl('https://github.com/davidbuenov/dbv-unreal-ai');
    });

    // Centro de Ayuda (F1 y botón)
    let help = null;
    const initHelp = async () => {
      if (!help) {
        const { createHelpModal } = await import('./app/helpModal.js');
        help = createHelpModal(document.querySelector('#help-modal'), document.querySelector('#help-content'));
      }
      return help;
    };

    document.querySelector('#btn-help')?.addEventListener('click', async () => {
      const h = await initHelp();
      h.open();
    });

    document.addEventListener('dbv-open-help', async (e) => {
      const h = await initHelp();
      h.open(e.detail?.tab || 'ai');
    });

    // Atajos globales: F1 para ayuda, Escape para modales
    window.addEventListener('keydown', async (e) => {
      if (e.key === 'F1') {
        e.preventDefault();
        const h = await initHelp();
        h.open();
      } else if (e.key === 'Escape') {
        document.querySelector('#about-modal')?.classList.add('hidden');
        document.querySelector('#help-modal')?.classList.add('hidden');
        document.querySelector('#ai-connect-modal')?.classList.add('hidden');
      }
    });

    applyTranslations();
    showPin();
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
