// =============================================================================
// DBV Unreal AI — Aviso cuando la IA espera una aprobación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-115
//
// Como el Unreal Assistant de UE 5.9: si la IA se queda esperando una aprobación y la ventana no tiene el foco (el
// usuario está en el editor de Unreal), la app hace parpadear su icono en la barra de tareas y suena un aviso corto.
// Con la ventana delante no hace nada: la tarjeta ya se ve.

/** Tauri v2 `UserAttentionType.Informational`: parpadea sin robar el foco. */
const INFORMATIONAL = 2;

/** Un pitido corto y suave con WebAudio (sin ficheros de sonido). Si no se puede, no pasa nada. */
function beep(audioContext) {
  const ctx = audioContext ?? new (globalThis.AudioContext ?? globalThis.webkitAudioContext)();
  const oscillator = ctx.createOscillator();
  const gain = ctx.createGain();
  oscillator.frequency.value = 880;
  gain.gain.value = 0.05;
  oscillator.connect(gain);
  gain.connect(ctx.destination);
  oscillator.start();
  oscillator.stop(ctx.currentTime + 0.15);
}

/**
 * Avisa de que hace falta el usuario, solo si la ventana no tiene el foco.
 * @param {{doc?: Document, tauri?: object, sound?: () => void}} [deps] para los tests.
 * @returns {boolean} si se avisó.
 */
export function needsAttention(deps = {}) {
  const doc = deps.doc ?? globalThis.document;
  const tauri = deps.tauri ?? globalThis.window?.__TAURI__;
  const away = Boolean(doc) && (doc.hidden || !doc.hasFocus());
  if (away) {
    try {
      tauri?.window?.getCurrentWindow?.().requestUserAttention?.(INFORMATIONAL)?.catch?.(() => {});
    } catch {
      // Sin Tauri (navegador) no hay barra de tareas que avisar.
    }
    try {
      (deps.sound ?? beep)();
    } catch {
      // Sin audio disponible, el parpadeo basta.
    }
  }
  return away;
}
