// =============================================================================
// DBV Unreal AI — Paridad de idiomas: el chino tiene las mismas claves que el español
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { AI_TRANSLATIONS } from '../../src/ai/translations.js';
import { helpTab, HELP_TABS } from '../../src/app/helpContent.js';
import { planRequest, taskRequest } from '../../src/app/planView.js';
import { rememberRequest } from '../../src/memory/memory.js';

const placeholders = (text) => [...String(text).matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

/** Claves de cada bloque de idioma del diccionario de la ventana (`i18n.js`, IIFE sin exportar). */
function windowKeys(lang) {
  const source = readFileSync('src/i18n.js', 'utf8');
  const start = source.indexOf(`${lang === 'zh-CN' ? '"zh-CN"' : lang}: {`);
  const block = source.slice(start, source.indexOf('\n    },', start));
  return [...block.matchAll(/^\s+"([\w.]+)":/gm)].map((m) => m[1]).sort();
}

describe('chino (zh-CN) @spec REQ-64', () => {
  for (const [name, dictionary] of [['interfaz', APP_TRANSLATIONS], ['IA', AI_TRANSLATIONS]]) {
    it(`${name}: no falta ninguna clave de es y los marcadores son los mismos`, () => {
      expect(Object.keys(dictionary['zh-CN']).sort()).toEqual(Object.keys(dictionary.es).sort());
      for (const [key, text] of Object.entries(dictionary.es)) {
        expect(placeholders(dictionary['zh-CN'][key]), key).toEqual(placeholders(text));
        expect(dictionary['zh-CN'][key].trim(), key).not.toBe('');
      }
    });
  }

  it('la ventana (i18n.js) tiene las mismas claves en los tres idiomas', () => {
    expect(windowKeys('zh-CN')).toEqual(windowKeys('es'));
    expect(windowKeys('en')).toEqual(windowKeys('es'));
  });

  it('la ayuda tiene todas sus pestañas en chino, y un idioma sin traducir cae al inglés', () => {
    for (const tab of HELP_TABS) {
      expect(helpTab(tab, 'zh-CN'), tab).toMatch(/[\u4e00-\u9fff]/);
      expect(helpTab(tab, 'fr'), tab).toBe(helpTab(tab, 'en'));
    }
    expect(helpTab('convai', 'zh-CN')).toContain('Character ID');
  });

  it('las peticiones a la IA salen en chino y con el inglés como respaldo', () => {
    expect(planRequest('一个森林关卡', 'zh-CN')).toContain('project_tasks_propose');
    expect(planRequest('x', 'zh-CN')).toMatch(/[\u4e00-\u9fff]/);
    expect(taskRequest({ title: '放置灯光', criterion: 'actor exists DirectionalLight' }, 'zh-CN')).toBe('我们来做计划中的这项任务：「放置灯光」。完成的标准：actor exists DirectionalLight。');
    expect(taskRequest({ title: 'x', criterion: '' }, 'fr')).toBe('Let\'s do this task of the plan: "x".');
    expect(rememberRequest('zh-CN')).toContain('project_memory_propose');
  });
});
