// IIFE obligatoria (dbv-specs-ops/docs/NATIVE_DESKTOP_APPS.md §3): los scripts clásicos comparten
// el ámbito global — sin este cierre, `const t` colisionaría con cualquier declaración de i18n.js
// y el SyntaxError resultante mataría este fichero entero en silencio, sin listeners ni error visible.
(function () {
  const { invoke } = window.__TAURI__.core;
  const { getCurrentWindow } = window.__TAURI__.window;
  const { getVersion } = window.__TAURI__.app;
  const { t, applyTranslations, setLanguage, getLanguage } = window.dbvI18n;

  let demoInputEl;
  let greetMsgEl;
  let discardModalEl;
  let aboutVersionEl;
  let langSelectEl;
  let aboutModalEl;
  let btnAlwaysOnTop;
  let btnAi;
  let aiPanelEl;
  let connectModalEl;
  let docEl;
  let docSaveBtn;
  let docStatusEl;
  let currentWindow;

  let isDirty = false;
  let closeConfirmPending = false;

  async function updateGreeting() {
    const name = await invoke("get_greeting_name", { name: demoInputEl.value });
    greetMsgEl.textContent = name ? t("demo.greetingHello", { name }) : t("demo.greetingEmpty");
  }

  function setDirty(value) {
    isDirty = value;
    document.body.classList.toggle("is-dirty", isDirty);
  }

  // ─── Documento de demostración (el que la IA ve y propone cambiar) ─────────────────────────────
  // En tu app real esto es TU documento o carpeta: la IA solo necesita una carpeta de trabajo
  // (`workspaceRoot`) y un documento abierto con su texto. Ver docs/IA.md, «Qué adaptar».
  const DEMO_DOC = "notas.md";
  let workspaceRoot = null;
  let docDirty = false;

  function refreshDirty() {
    setDirty(demoInputEl.value.trim().length > 0 || docDirty);
  }

  async function loadDemoDocument() {
    try {
      workspaceRoot = await invoke("demo_workspace");
      docEl.value = await invoke("ai_read_file", { root: workspaceRoot, relative: DEMO_DOC });
      document.querySelector("#doc-hint").textContent = t("demo.docHint", { name: DEMO_DOC });
      docEl.disabled = false;
      docSaveBtn.disabled = false;
    } catch (error) {
      docStatusEl.textContent = t("demo.docFailed", { error: String(error?.message ?? error) });
    }
  }

  async function saveDemoDocument() {
    if (!workspaceRoot) return;
    try {
      await invoke("ai_write_file", { root: workspaceRoot, relative: DEMO_DOC, content: docEl.value });
      docDirty = false;
      refreshDirty();
      docStatusEl.textContent = t("demo.docSaved");
    } catch (error) {
      docStatusEl.textContent = t("demo.docFailed", { error: String(error?.message ?? error) });
    }
  }

  // ─── Avisos y confirmaciones ───────────────────────────────────────────────────────────────────
  let toastTimer = null;
  function showToast(message, tone) {
    let el = document.querySelector("#app-toast");
    if (!el) {
      el = document.createElement("div");
      el.id = "app-toast";
      el.setAttribute("role", "status");
      el.setAttribute("aria-live", "polite");
      document.body.appendChild(el);
    }
    el.textContent = message;
    el.className = "app-toast" + (tone === "error" ? " app-toast-error" : "");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      el.className = "app-toast hidden";
    }, 4200);
  }

  /** Confirmación modal con botones propios (aviso de nube, borrar conversación). Resuelve con la `key` elegida. */
  function aiConfirm(options) {
    return new Promise((resolve) => {
      const modal = document.querySelector("#ai-confirm-modal");
      const actions = document.querySelector("#ai-confirm-actions");
      document.querySelector("#ai-confirm-title").textContent = options.title || "";
      document.querySelector("#ai-confirm-text").textContent = options.text || "";
      actions.replaceChildren();
      const finish = (key) => {
        modal.classList.add("hidden");
        document.removeEventListener("keydown", onKey, true);
        resolve(key);
      };
      const onKey = (event) => {
        if (event.key !== "Escape") return;
        event.stopPropagation();
        finish("cancel");
      };
      for (const choice of options.choices) {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "button button--compact" + (choice.tone === "primary" ? " button--primary" : choice.tone === "danger" ? " button--danger" : "");
        button.textContent = choice.label;
        button.addEventListener("click", () => finish(choice.key));
        actions.append(button);
      }
      document.querySelector("#ai-confirm-backdrop").onclick = () => finish("cancel");
      document.addEventListener("keydown", onKey, true);
      modal.classList.remove("hidden");
      actions.lastElementChild?.focus();
    });
  }

  function confirmDiscardChanges() {
    return new Promise((resolve) => {
      discardModalEl.classList.remove("hidden");
      const confirmBtn = discardModalEl.querySelector("#discard-confirm");
      const cancelBtn = discardModalEl.querySelector("#discard-cancel");
      const cleanup = () => {
        discardModalEl.classList.add("hidden");
        confirmBtn.removeEventListener("click", onConfirm);
        cancelBtn.removeEventListener("click", onCancel);
      };
      const onConfirm = () => {
        cleanup();
        resolve(true);
      };
      const onCancel = () => {
        cleanup();
        resolve(false);
      };
      confirmBtn.addEventListener("click", onConfirm);
      cancelBtn.addEventListener("click", onCancel);
    });
  }

  // ─── IA opcional (docs/IA.md) ──────────────────────────────────────────────────────────────────
  // Zero-footprint: sin ninguna IA conectada, `src/ai/` no se descarga hasta que el usuario pulsa ✨ o
  // usa su atajo. `loadAi()` hace el `import()` dinámico (por eso `src/ai/` son módulos ES aunque el
  // resto de la app sean scripts clásicos: es la excepción documentada a NATIVE_DESKTOP_APPS.md §3).
  let aiApp = null;
  let aiLoading = null;

  function replaceDocText(text) {
    docEl.focus({ preventScroll: true });
    docEl.select();
    const done = document.execCommand("insertText", false, text);
    if (!done || docEl.value.replace(/\r\n/g, "\n") !== text.replace(/\r\n/g, "\n")) {
      // Sin execCommand el cambio sigue valiendo, solo que Ctrl+Z no lo deshace.
      docEl.value = text;
      docEl.dispatchEvent(new Event("input", { bubbles: true }));
    }
    return true;
  }

  function addCopyButton(pre) {
    const code = pre.querySelector("code")?.textContent ?? "";
    const actions = document.createElement("div");
    actions.className = "code-actions";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "code-action-btn";
    button.textContent = t("action.copy");
    button.addEventListener("click", () => {
      navigator.clipboard.writeText(code).catch(() => showToast(t("action.copyDenied"), "error"));
    });
    actions.append(button);
    pre.append(actions);
  }

  function outline(text) {
    const headings = [];
    for (const line of text.split("\n")) {
      const match = /^(#{1,6})\s+(.*)$/.exec(line);
      if (match) headings.push({ level: match[1].length, text: match[2].trim() });
    }
    return headings;
  }

  /** Lo que la IA sabe de la app: el documento, el editor y la interfaz (ver `src/ai/aiApp.js`). */
  function buildAiHost(renderMarkdownNodes) {
    const selection = () => ({ from: docEl.selectionStart, to: docEl.selectionEnd });
    return {
      elements: {
        panel: aiPanelEl,
        connectBody: document.querySelector("#ai-connect-body"),
        editorOverlay: document.querySelector("#editor-overlay"),
      },
      toast: showToast,
      filesChanged() {},
      confirm: aiConfirm,
      renderMarkdown(target, markdown) {
        // `markdown.js` construye nodos y todo texto entra por `textContent`: no hay `innerHTML`, así que ni
        // una imagen ni un atributo que venga de la salida de un modelo puede cargar nada (exfiltración).
        target.replaceChildren(renderMarkdownNodes(markdown));
        target.querySelectorAll("pre").forEach(addCopyButton);
      },
      getFolderName: () => (workspaceRoot ?? "").split(/[\\/]/).filter(Boolean).pop() ?? "",
      getActive() {
        if (!workspaceRoot) return null;
        const { from, to } = selection();
        return { relative: DEMO_DOC, content: docEl.value, cursor: from, selection: docEl.value.slice(from, to) };
      },
      isOpenDocument: (relative) => String(relative).replace(/\\/g, "/").toLowerCase() === DEMO_DOC,
      getOpenText: () => docEl.value,
      isDirty: () => docDirty,
      getOutline: () => outline(docEl.value),
      getVisibleSection: () => (docEl.value.trim() ? { title: DEMO_DOC, text: docEl.value.slice(0, 6000) } : null),
      openDocument: () => docEl.focus(),
      applyToEditor: replaceDocText,
      insertAtCursor(text) {
        docEl.focus({ preventScroll: true });
        return document.execCommand("insertText", false, text);
      },
      /** Selección del documento para la IA en línea. */
      getEditorSelection() {
        const { from, to } = selection();
        return { from, to, text: docEl.value.slice(from, to), value: docEl.value };
      },
      /** Sustituye [from, to) por `text` (deshacible) solo si el texto sigue siendo `expected`. */
      replaceRange(from, to, text, expected) {
        if (docEl.value.slice(from, to) !== expected) return false;
        docEl.focus({ preventScroll: true });
        docEl.setSelectionRange(from, to);
        return document.execCommand("insertText", false, text);
      },
      showPanel(open) {
        aiPanelEl.classList.toggle("hidden", !open);
      },
      openConnect() {
        connectModalEl.classList.remove("hidden");
      },
      closeConnect() {
        connectModalEl.classList.add("hidden");
      },
      onVisibilityChanged(info) {
        btnAi.classList.toggle("hidden", !info.visible);
      },
    };
  }

  function loadAi() {
    if (!aiLoading) {
      aiLoading = Promise.all([import("./ai/entry.js"), import("./ai/markdown.js")])
        .then(([entry, markdown]) => entry.createAi(buildAiHost(markdown.renderMarkdown)))
        .then((app) => {
          aiApp = app;
          app.onRootChanged(workspaceRoot);
          return app;
        })
        .catch((error) => {
          aiLoading = null;
          console.error("[ai]", error);
          showToast(t("ai.loadFailed"), "error");
          throw error;
        });
    }
    return aiLoading;
  }

  /** Abre el Asistente; sin ninguna IA conectada, abre antes el asistente de conexión. */
  function openAssistant() {
    loadAi()
      .then((app) => {
        if (!app.isReady()) {
          app.openConnect();
          return;
        }
        aiPanelEl.classList.remove("hidden");
        app.focusInput();
      })
      .catch(() => {});
  }

  function toggleAssistant() {
    if (!aiPanelEl.classList.contains("hidden")) aiPanelEl.classList.add("hidden");
    else openAssistant();
  }

  /** IA en línea sobre la selección del documento (Ctrl+Alt+I). */
  function openInlineAi() {
    loadAi()
      .then((app) => {
        if (!app.isReady()) app.openConnect();
        else app.inline.open();
      })
      .catch(() => {});
  }

  function closeConnectModal() {
    connectModalEl.classList.add("hidden");
  }

  function setAlwaysOnTopButtonState(active) {
    const key = active ? "toolbar.alwaysOnTopActive" : "toolbar.alwaysOnTop";
    btnAlwaysOnTop.classList.toggle("active", active);
    btnAlwaysOnTop.setAttribute("data-i18n-title", key);
    btnAlwaysOnTop.title = t(key);
  }

  async function toggleAlwaysOnTop() {
    const current = await currentWindow.isAlwaysOnTop();
    await currentWindow.setAlwaysOnTop(!current);
    setAlwaysOnTopButtonState(!current);
  }

  async function openAboutModal() {
    const version = await getVersion();
    aboutVersionEl.textContent = t("about.version", { version });
    aboutModalEl.classList.remove("hidden");
  }

  function closeAboutModal() {
    aboutModalEl.classList.add("hidden");
  }

  async function handleCloseRequested(event) {
    // Sin cambios sin guardar: no se llama a event.preventDefault(), así que Tauri completa el
    // cierre por su cuenta (internamente invoca destroy() — no hace falta llamarlo a mano).
    if (!isDirty) {
      aiApp?.dispose?.();
      return;
    }

    // Sin este guardián, un segundo cierre (doble clic en la X, Alt+F4 repetido) mientras el modal
    // ya está abierto reabriría un segundo modal encima del primero.
    if (closeConfirmPending) {
      event.preventDefault();
      return;
    }
    closeConfirmPending = true;

    const shouldClose = await confirmDiscardChanges();
    closeConfirmPending = false;

    // Solo se cancela el cierre si el usuario NO confirma. Si confirma, no se llama a
    // preventDefault() y Tauri procede a cerrar la ventana por su cuenta (patrón validado en
    // dbv-md-reader, ADR-030 de su memory.md: el wrapper de @tauri-apps/api/window hace destroy()
    // internamente cuando el handler no cancela el evento).
    if (!shouldClose) {
      event.preventDefault();
      return;
    }
    // Se detienen la respuesta en curso y el agente, si los hay.
    aiApp?.dispose?.();
  }

  window.addEventListener("DOMContentLoaded", async () => {
    demoInputEl = document.querySelector("#demo-input");
    greetMsgEl = document.querySelector("#greet-msg");
    discardModalEl = document.querySelector("#discard-modal");
    langSelectEl = document.querySelector("#lang-select");
    aboutModalEl = document.querySelector("#about-modal");
    aboutVersionEl = document.querySelector("#about-version");
    btnAlwaysOnTop = document.querySelector("#btn-always-on-top");
    btnAi = document.querySelector("#btn-ai");
    aiPanelEl = document.querySelector("#ai-panel-container");
    connectModalEl = document.querySelector("#ai-connect-modal");
    docEl = document.querySelector("#demo-doc");
    docSaveBtn = document.querySelector("#doc-save");
    docStatusEl = document.querySelector("#doc-status");
    currentWindow = getCurrentWindow();

    // Listeners primero: si alguna llamada async al backend fallara, la interfaz seguiría
    // respondiendo en vez de quedarse muerta sin ningún error visible.
    demoInputEl.addEventListener("input", () => {
      refreshDirty();
      updateGreeting();
    });

    docEl.addEventListener("input", () => {
      docDirty = true;
      docStatusEl.textContent = "";
      refreshDirty();
      aiApp?.refreshContext?.();
    });
    docSaveBtn.addEventListener("click", saveDemoDocument);

    langSelectEl.addEventListener("change", () => {
      setLanguage(langSelectEl.value);
      updateGreeting();
    });

    btnAlwaysOnTop.addEventListener("click", toggleAlwaysOnTop);
    btnAi.addEventListener("click", toggleAssistant);
    document.querySelector("#btn-about").addEventListener("click", openAboutModal);
    document.querySelector("#about-close").addEventListener("click", closeAboutModal);
    document.querySelector("#ai-connect-close").addEventListener("click", closeConnectModal);
    document.querySelector("#ai-connect-backdrop").addEventListener("click", closeConnectModal);

    document.addEventListener("keydown", (event) => {
      const mod = event.ctrlKey || event.metaKey;
      const key = event.key.toLowerCase();
      if (event.key === "Escape" && !connectModalEl.classList.contains("hidden")) {
        closeConnectModal();
      } else if (mod && event.shiftKey && key === "i") {
        event.preventDefault();
        toggleAssistant();
      } else if (mod && event.altKey && key === "i") {
        event.preventDefault();
        openInlineAi();
      } else if (mod && !event.shiftKey && !event.altKey && key === "s" && document.activeElement === docEl) {
        event.preventDefault();
        saveDemoDocument();
      }
    });

    langSelectEl.value = getLanguage();
    applyTranslations();
    docEl.disabled = true;
    docSaveBtn.disabled = true;
    await updateGreeting();
    setAlwaysOnTopButtonState(await currentWindow.isAlwaysOnTop());
    await loadDemoDocument();

    await currentWindow.onCloseRequested(handleCloseRequested);
  });
})();
