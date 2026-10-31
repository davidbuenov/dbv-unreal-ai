// =============================================================================
// DBV Unreal AI — Contador de lo que las herramientas devuelven en un turno
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-51
//
// Al pie de cada respuesta: «Herramientas: 7 llamadas · 48 KB (~12 k tokens, aprox.)». Con el asistente integrado
// la app cuenta sus propias llamadas; con un agente, las cuenta el `--mcp` en `tool-usage.jsonl` y la ventana las
// pide al acabar el turno (`tool_usage_since`). La estimación de tokens es aproximada y se dice que lo es.

import { t } from '../ai/i18nBridge.js';

/** Tokens que se cuentan por imagen (una captura de 1024 px ronda los 800–1.000). */
const TOKENS_PER_IMAGE = 1000;
/** Por encima de esto en un turno, el contador avisa. */
export const WARN_TEXT_CHARS = 200 * 1024;

/** Tamaño de un resultado de herramienta: texto, imágenes y si se recortó. */
export function measureResult(out) {
  const text = typeof out === 'string' ? out : String(out?.text ?? '');
  const images = Array.isArray(out?.images) ? out.images.length : 0;
  return { text: Array.from(text).length, images, clipped: text.includes('… (truncated: ') };
}

/** Contador de un turno del asistente integrado. */
export function createUsageCounter() {
  const total = { calls: 0, text: 0, images: 0, clipped: 0 };
  return {
    add(out) {
      const size = measureResult(out);
      total.calls += 1;
      total.text += size.text;
      total.images += size.images;
      total.clipped += size.clipped ? 1 : 0;
    },
    summary: () => ({ ...total }),
  };
}

/** Las mismas herramientas, contando lo que devuelve cada una. */
export function withUsage(tools, counter) {
  return counter
    ? tools.map((tool) => ({
        ...tool,
        run: async (args) => {
          const out = await tool.run(args);
          counter.add(out);
          return out;
        },
      }))
    : tools;
}

/** Tokens aproximados de un resumen (caracteres / 4 más un tanto por imagen). */
export const approxTokens = (summary) => Math.round((summary?.text ?? 0) / 4 + (summary?.images ?? 0) * TOKENS_PER_IMAGE);

/** Texto, aviso y explicación del contador. */
export function formatUsage(summary) {
  const kb = Math.max(1, Math.round((summary.text ?? 0) / 1024));
  const tokens = approxTokens(summary);
  const shown = tokens >= 1000 ? `${Math.round(tokens / 1000)} k` : String(tokens);
  const images = summary.images ? t(summary.images === 1 ? 'unreal.usageImage' : 'unreal.usageImages', { images: String(summary.images) }) : '';
  return {
    text: t(summary.calls === 1 ? 'unreal.usageOne' : 'unreal.usage', { calls: String(summary.calls), kb: String(kb), images, tokens: shown }),
    warning: (summary.text ?? 0) > WARN_TEXT_CHARS,
    title: t('unreal.usageHint'),
  };
}
