// =============================================================================
// DBV Unreal AI — Tests del menú de recetas y sus herramientas
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it, vi } from 'vitest';
import { createRecipeTools, localized, recipesCard } from '../../src/app/recipes.js';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';

// La misma definición que compila Rust y que lee el plugin en el editor.
const RECIPES = JSON.parse(readFileSync('unreal-plugin/DBVUnrealAI/Content/Python/dbv_unreal_ai/recipes.json', 'utf8')).recipes;

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

describe('Recetas', () => {
  it('el menú enseña las cuatro recetas en el idioma de la app, con lo que necesita cada una @spec REQ-23', () => {
    const onPick = vi.fn();
    const card = recipesCard(RECIPES, onPick);
    const titles = [...card.querySelectorAll('.recipe strong')].map((n) => n.textContent);
    expect(titles).toEqual(['Nivel de prueba', 'Personaje de Tripo con animaciones', 'Caminar por un mundo de World Labs', 'MetaHuman que habla con Convai']);
    expect(card.querySelector('[data-recipe="tripo_character"]').parentElement.textContent).toContain('tu clave de Tripo');
    expect(card.querySelector('[data-recipe="test_level"]').parentElement.querySelector('.recipe__needs')).toBeNull();

    card.querySelector('[data-recipe="world_walk"]').click();
    expect(onPick).toHaveBeenCalledWith(RECIPES[2]);
    expect([...card.querySelectorAll('button')].every((b) => b.disabled)).toBe(true);
  });

  it('las herramientas usan las definiciones y la ejecución de Rust @spec REQ-23', async () => {
    const run = vi.fn(async (name) => (name === 'recipes_list' ? { ok: true, value: '- test_level — Test level' } : { ok: false, error: { message: 'no editor is connected' } }));
    const specs = [
      { name: 'recipes_list', description: 'Guided recipes', inputSchema: { type: 'object', properties: {}, required: [] } },
      { name: 'recipe_check', description: 'Check', inputSchema: { type: 'object', properties: { id: { type: 'string' } }, required: ['id'] } },
    ];
    const [list, check] = createRecipeTools({ specs, run });
    expect(list.parameters.additionalProperties).toBe(false);
    expect(await list.run({})).toBe('- test_level — Test level');
    expect(await check.run({ id: 'test_level' })).toBe('error: no editor is connected');
    expect(run).toHaveBeenLastCalledWith('recipe_check', { id: 'test_level' });
    expect(check.label({ id: 'test_level' })).toBe('Comprobando la receta test_level');
  });

  it('un texto sin traducir cae al inglés @spec REQ-23, REQ-64', () => {
    expect(localized({ es: 'Hola', en: 'Hello' }, 'zh-CN')).toBe('Hello');
    expect(localized({ es: 'Hola', en: 'Hello' }, 'es')).toBe('Hola');
  });
});
