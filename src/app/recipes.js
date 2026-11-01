// =============================================================================
// DBV Unreal AI — Recetas guiadas en la conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-23
//
// El menú «Recetas» de la conversación y sus herramientas para el asistente integrado. Los datos, el prompt de cada
// receta y la ejecución de las herramientas son de Rust (`recipes.rs`), los mismos que usa el `--mcp` (D10).

import { getLanguage, t } from '../ai/i18nBridge.js';
import { button, el } from './dom.js';

/** Texto de un idioma con el inglés como respaldo. */
export const localized = (texts, lang) => texts?.[lang] ?? texts?.en ?? '';

/**
 * Herramientas `recipes_list` y `recipe_check` a partir de las definiciones de Rust.
 * @param {{specs: Array<{name: string, description: string, inputSchema: object}>, run: (name: string, args: object) => Promise<{ok: boolean, value?: string, error?: {message: string}}>}} deps
 */
export function createRecipeTools({ specs, run }) {
  return specs.map((spec) => ({
    name: spec.name,
    description: spec.description,
    parameters: { ...spec.inputSchema, additionalProperties: false },
    label: (args) => (spec.name === 'recipe_check' ? t('recipes.checking', { id: args?.id ?? '' }) : t('recipes.title')),
    run: async (args) => {
      const result = await run(spec.name, args ?? {});
      return result.ok ? result.value : `error: ${result.error.message}`;
    },
  }));
}

/**
 * Tarjeta con las recetas; al elegir una, `onPick(recipe)`.
 * @param {Array<{id: string, title: object, summary: object, needs: string[]}>} recipes
 * @param {(recipe: object) => void} onPick
 */
export function recipesCard(recipes, onPick) {
  const lang = getLanguage();
  const items = recipes.map((recipe) => {
    const needs = recipe.needs.filter((need) => need !== 'editor').map((need) => t(`recipes.need.${need}`));
    const choose = button(t('recipes.start'), () => {
      for (const other of card.querySelectorAll('button')) other.disabled = true;
      onPick(recipe);
    }, 'btn-primary button--compact');
    choose.dataset.recipe = recipe.id;
    return el(
      'div',
      { className: 'recipe' },
      el('strong', { text: localized(recipe.title, lang) }),
      el('p', { className: 'hint', text: localized(recipe.summary, lang) }),
      needs.length ? el('p', { className: 'hint recipe__needs', text: t('recipes.needs', { list: needs.join(', ') }) }) : null,
      choose,
    );
  });
  const card = el('div', { className: 'ai-review recipes' }, el('strong', { text: t('recipes.title') }), el('p', { className: 'hint', text: t('recipes.intro') }), ...items);
  return card;
}
