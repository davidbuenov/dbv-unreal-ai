// i18n mínima sin librería — mismo patrón que dbv-md-reader (ver dbv-specs-ops/docs/NATIVE_DESKTOP_APPS.md
// §4, lección 7): dos objetos planos, sustitución simple de {placeholder}, sin dependencia nueva.
//
// IIFE obligatoria (NATIVE_DESKTOP_APPS.md §3): los scripts clásicos comparten el ámbito global —
// sin este cierre, cualquier nombre declarado aquí (t, currentLang...) colisionaría con main.js
// y el SyntaxError resultante mataría el script entero en silencio.
//
// Expone `window.dbvI18n` (la API de la demo) y `window.DBV_I18N` (la que esperan los módulos de la
// IA de `src/ai/`: `t`, `getLang`, `setLang`, `applyTranslations`, `register`). Los textos de la IA
// no viven aquí: se registran con `register()` al cargarla (carga perezosa, `src/ai/translations.js`).
(function () {
  const translations = {
    es: {
      "app.title": "DBV Unreal AI",
      "demo.banner": "Esto es una demo de DBV Unreal AI — ver README.md para los próximos pasos.",
      "demo.heading": "Prueba el comando Rust",
      "demo.inputPlaceholder": "Escribe tu nombre...",
      "demo.greetingEmpty": "Escribe tu nombre para ver un saludo generado desde Rust.",
      "demo.greetingHello": "¡Hola, {name}! Este saludo viene de un comando #[tauri::command].",
      "demo.dirtyHint": "Tienes cambios sin guardar en memoria — cierra la ventana para probar la confirmación.",
      "demo.docHeading": "Prueba la IA con un documento",
      "demo.docHint": "Documento de ejemplo ({name}). Pulsa ✨ para conectar una IA y pedirle cambios; los revisas antes de aplicarlos.",
      "demo.docSave": "Guardar",
      "demo.docSaved": "Guardado.",
      "demo.docFailed": "No se pudo abrir o guardar el documento de la demo: {error}",
      "modal.title": "¿Cerrar sin guardar?",
      "modal.body": "Hay texto sin guardar en el campo de la demo o en el documento. Si cierras ahora, se perderá.",
      "modal.confirm": "Cerrar de todas formas",
      "modal.cancel": "Cancelar",
      "lang.label": "Idioma",
      "ai.button": "Asistente de IA (Ctrl+Mayús+I)",
      "ai.connectTitle": "Conectar una IA",
      "ai.loadFailed": "No se pudo cargar la IA. Reinicia la aplicación e inténtalo de nuevo.",
      "action.close": "Cerrar",
      "action.copy": "Copiar",
      "action.copyDenied": "El sistema no permitió acceder al portapapeles.",
      "toolbar.alwaysOnTop": "Mantener siempre visible",
      "toolbar.alwaysOnTopActive": "Siempre visible (activado)",
      "toolbar.about": "Acerca de",
      "about.title": "Acerca de DBV Unreal AI",
      "about.version": "Versión {version}",
      "about.builtWith": "Construido con dbv-specs-ops.",
      "about.updatesTitle": "Actualizaciones",
      "about.updatesHint": "Este starter no incluye tauri-plugin-updater de fábrica — añádelo cuando tu app real lo necesite (ver NATIVE_DESKTOP_APPS.md §4).",
      "about.checkUpdates": "Buscar actualizaciones",
      "about.close": "Cerrar",
    },
    en: {
      "app.title": "DBV Unreal AI",
      "demo.banner": "This is a DBV Unreal AI demo — see README.md for the next steps.",
      "demo.heading": "Try the Rust command",
      "demo.inputPlaceholder": "Type your name...",
      "demo.greetingEmpty": "Type your name to see a greeting generated from Rust.",
      "demo.greetingHello": "Hello, {name}! This greeting comes from a #[tauri::command].",
      "demo.dirtyHint": "You have unsaved in-memory changes — close the window to test the confirmation.",
      "demo.docHeading": "Try the AI with a document",
      "demo.docHint": "Sample document ({name}). Press ✨ to connect an AI and ask it for changes; you review them before they are applied.",
      "demo.docSave": "Save",
      "demo.docSaved": "Saved.",
      "demo.docFailed": "Could not open or save the demo document: {error}",
      "modal.title": "Close without saving?",
      "modal.body": "There's unsaved text in the demo field or the document. Closing now will discard it.",
      "modal.confirm": "Close anyway",
      "modal.cancel": "Cancel",
      "lang.label": "Language",
      "ai.button": "AI assistant (Ctrl+Shift+I)",
      "ai.connectTitle": "Connect an AI",
      "ai.loadFailed": "Could not load the AI. Restart the app and try again.",
      "action.close": "Close",
      "action.copy": "Copy",
      "action.copyDenied": "The system did not allow access to the clipboard.",
      "toolbar.alwaysOnTop": "Keep always on top",
      "toolbar.alwaysOnTopActive": "Always on top (active)",
      "toolbar.about": "About",
      "about.title": "About DBV Unreal AI",
      "about.version": "Version {version}",
      "about.builtWith": "Built with dbv-specs-ops.",
      "about.updatesTitle": "Updates",
      "about.updatesHint": "This starter doesn't ship tauri-plugin-updater by default — add it when your real app needs it (see NATIVE_DESKTOP_APPS.md §4).",
      "about.checkUpdates": "Check for updates",
      "about.close": "Close",
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
