// =============================================================================
// DBV Unreal AI — Idiomas de la interfaz (ES / EN)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// i18n mínima sin librería — mismo patrón que dbv-md-reader (ver dbv-specs-ops/docs/NATIVE_DESKTOP_APPS.md
// §4, lección 7): dos objetos planos, sustitución simple de {placeholder}, sin dependencia nueva.
//
// IIFE obligatoria (NATIVE_DESKTOP_APPS.md §3): los scripts clásicos comparten el ámbito global —
// sin este cierre, cualquier nombre declarado aquí (t, currentLang...) colisionaría con main.js
// y el SyntaxError resultante mataría el script entero en silencio.
//
// Expone `window.dbvI18n` (la API de `main.js`) y `window.DBV_I18N` (la que esperan los módulos de la
// IA de `src/ai/`: `t`, `getLang`, `setLang`, `applyTranslations`, `register`). Los textos de la IA
// no viven aquí: se registran con `register()` al cargarla (carga perezosa, `src/ai/translations.js`).
(function () {
  // Solo los textos de la ventana; los de cada vista llegan con su módulo (`src/app/translations.js`).
  const translations = {
    es: {
      "app.title": "DBV Unreal AI",
      "app.theme": "Tema",
      "theme.light": "Claro",
      "theme.dark": "Oscuro",
      "theme.sepia": "Sepia",
      "action.alwaysOnTop": "Mantener la ventana encima de las demás",
      "action.alwaysOnTopActive": "La ventana está encima de las demás; pulsa para soltarla",
      "action.alwaysOnTopFailed": "No se pudo fijar la ventana",
      "nav.project": "Proyecto",
      "nav.chat": "Conversación",
      "nav.generate": "Generar",
      "nav.library": "Biblioteca",
      "nav.settings": "Ajustes",
      "unreal.pill.offline": "Sin editor",
      "log.toggle": "Registro de acciones",
      "gen.spentTitle": "Créditos gastados en esta sesión",
      "lang.label": "Idioma",
      "toolbar.about": "Acerca de",
      "about.title": "Acerca de DBV Unreal AI",
      "about.version": "Versión {version}",
      "about.authors": "David Bueno Vallejo y David Bueno Carmona · MIT",
      "about.builtWith": "Construido con dbv-specs-ops.",
      "about.close": "Cerrar",
      "action.close": "Cerrar",
      "action.copy": "Copiar",
      "action.copyDenied": "No se pudo copiar al portapapeles",
      "ai.connectTitle": "Conectar una IA",
      "ai.loadFailed": "No se pudo cargar el asistente de IA",
    },
    en: {
      "app.title": "DBV Unreal AI",
      "app.theme": "Theme",
      "theme.light": "Light",
      "theme.dark": "Dark",
      "theme.sepia": "Sepia",
      "action.alwaysOnTop": "Keep the window on top of the others",
      "action.alwaysOnTopActive": "The window is on top of the others; click to release it",
      "action.alwaysOnTopFailed": "Could not pin the window",
      "nav.project": "Project",
      "nav.chat": "Conversation",
      "nav.generate": "Generate",
      "nav.library": "Library",
      "nav.settings": "Settings",
      "unreal.pill.offline": "No editor",
      "log.toggle": "Action log",
      "gen.spentTitle": "Credits spent in this session",
      "lang.label": "Language",
      "toolbar.about": "About",
      "about.title": "About DBV Unreal AI",
      "about.version": "Version {version}",
      "about.authors": "David Bueno Vallejo and David Bueno Carmona · MIT",
      "about.builtWith": "Built with dbv-specs-ops.",
      "about.close": "Close",
      "action.close": "Close",
      "action.copy": "Copy",
      "action.copyDenied": "Could not copy to the clipboard",
      "ai.connectTitle": "Connect an AI",
      "ai.loadFailed": "Could not load the AI assistant",
    },
  };

  const STORAGE_KEY = "dbv-unreal-ai:lang";

  function detectLanguage() {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "es" || stored === "en") return stored;
    const browserLang = (navigator.language || "es").slice(0, 2);
    return browserLang === "en" ? "en" : "es";
  }

  let currentLang = detectLanguage();

  function t(key, vars) {
    const dict = translations[currentLang] || translations.es;
    let text = dict[key] || translations.es[key] || key;
    if (vars) {
      for (const [placeholder, value] of Object.entries(vars)) {
        text = text.replace(`{${placeholder}}`, value);
      }
    }
    return text;
  }

  function applyTranslations() {
    document.documentElement.lang = currentLang;
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      el.textContent = t(el.getAttribute("data-i18n"));
    });
    document.querySelectorAll("[data-i18n-title]").forEach((el) => {
      el.title = t(el.getAttribute("data-i18n-title"));
    });
    document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
      el.placeholder = t(el.getAttribute("data-i18n-placeholder"));
    });
  }

  function setLanguage(lang) {
    if (lang !== "es" && lang !== "en") return;
    currentLang = lang;
    localStorage.setItem(STORAGE_KEY, lang);
    applyTranslations();
    document.dispatchEvent(new CustomEvent("dbv-lang-changed", { detail: { lang } }));
  }

  function getLanguage() {
    return currentLang;
  }

  // Añade textos al diccionario: los módulos de la IA se cargan de forma perezosa y traen los suyos.
  function registerTranslations(more) {
    if (!more) return;
    for (const [lang, strings] of Object.entries(more)) {
      translations[lang] = Object.assign(translations[lang] || {}, strings);
    }
    applyTranslations();
  }

  window.dbvI18n = { t, applyTranslations, setLanguage, getLanguage, registerTranslations };
  // Nombres que esperan los módulos de `src/ai/` (mismos que dbv-md-reader).
  window.DBV_I18N = { t, applyTranslations, setLang: setLanguage, getLang: getLanguage, register: registerTranslations };
})();
