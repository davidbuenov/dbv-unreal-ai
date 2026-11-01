// =============================================================================
// DBV Unreal AI — «Mejorar con IA» el prompt de Tripo y de World Labs
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-66, REQ-27
//
// Convierte una idea corta en una descripción adecuada a cada servicio. El resultado solo sustituye al texto del
// campo: no se genera nada, porque generar gasta créditos y la persona tiene que revisarlo (REQ-27). Con un modelo
// conectado es una consulta suelta; con un agente no la hay, así que la petición va a la conversación y su respuesta
// vuelve al campo (pedido por el usuario el 2026-11-01: copiarla a mano eran demasiados pasos).

/** Longitud máxima del prompt que se deja en el campo, por servicio. */
export const IMPROVE_LIMITS = { tripo: 1000, worldlabs: 2000 };

const INSTRUCTIONS = {
  tripo:
    'You write prompts for Tripo, a text-to-3D generator that makes ONE object (a prop, a character, a creature or a vehicle), never a scene. Turn the idea into one English description of 40 to 80 words: what the object is, its shape and proportions, materials and colors, and its style (realistic, stylized, cartoon, low-poly…). For a character or creature that will be animated, say it stands in a neutral A-pose. No background, environment, lighting or camera.',
  worldlabs:
    'You write prompts for World Labs Marble, which generates an explorable 3D place you can walk through. Turn the idea into one English description of 60 to 120 words of a single coherent place at human scale: what kind of place it is, its layout and landmarks around the viewer, architecture or vegetation, materials, time of day, lighting, weather and mood. No characters doing actions, no text or signs to read, no camera instructions.',
};

const ANSWER_ONLY = 'Answer with the description only: no title, no quotes, no lists, no explanations.';

/** Mensajes para una consulta suelta al modelo conectado. */
export function improveMessages(service, idea) {
  return [
    { role: 'system', content: `${INSTRUCTIONS[service] ?? INSTRUCTIONS.tripo} ${ANSWER_ONLY}` },
    { role: 'user', content: String(idea ?? '').trim() },
  ];
}

/** Lo que se le pide a un agente en la conversación (no hay consulta suelta). */
export function improveRequestForAgent(service, idea) {
  return `${INSTRUCTIONS[service] ?? INSTRUCTIONS.tripo} ${ANSWER_ONLY} Do not call any tool and do not generate anything: only write the description.\n\nIdea: ${String(idea ?? '').trim()}`;
}

/**
 * La descripción limpia (sin bloques de código, comillas, «Prompt:» delante ni saltos de línea) y recortada al
 * tope del servicio, o `null` si el modelo no devolvió nada útil.
 */
export function cleanImproved(reply, service) {
  const limit = IMPROVE_LIMITS[service] ?? IMPROVE_LIMITS.tripo;
  // Un agente suele añadir una frase («Aquí tienes…») antes de la descripción: se queda el párrafo más largo.
  const paragraphs = String(reply ?? '')
    .replace(/<think>[\s\S]*?<\/think>/gi, '')
    .replace(/```[a-z]*\n?/gi, '')
    .split(/\n\s*\n/);
  const longest = paragraphs.reduce((best, p) => (p.trim().length > best.trim().length ? p : best), '');
  const text = longest
    .replace(/^\s*(?:\*\*)?(?:prompt|description|descripción)(?:\*\*)?\s*:\s*/i, '')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/^["'«“]+|["'»”]+$/g, '')
    .trim();
  const clipped = text.length > limit ? `${text.slice(0, limit).replace(/\s+\S*$/, '')}` : text;
  return clipped || null;
}

/**
 * Pide la mejora a la IA conectada.
 * @param {{activeConnection: () => ({provider: string}|null), complete: (messages: object[]) => Promise<string|null>, ask: (text: string, options?: object) => Promise<string|void>} | null} ai
 * @param {'tripo'|'worldlabs'} service
 * @param {string} idea
 * @param {string} label lo que se ve en la conversación si va a un agente.
 * @returns {Promise<{kind: 'text', text: string} | {kind: 'sent'} | {kind: 'none'} | {kind: 'empty'}>}
 */
export async function improvePrompt(ai, service, idea, label) {
  const connection = ai?.activeConnection?.() ?? null;
  let result = { kind: 'none' };
  if (connection?.provider === 'agent') {
    const text = cleanImproved(await ai.ask(improveRequestForAgent(service, idea), { label }), service);
    result = text ? { kind: 'text', text } : { kind: 'sent' };
  } else if (connection) {
    const text = cleanImproved(await ai.complete(improveMessages(service, idea)), service);
    result = text ? { kind: 'text', text } : { kind: 'empty' };
  }
  return result;
}
