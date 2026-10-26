// =============================================================================
// DBV Unreal AI — Tests: la IA habla de Unreal, no de documentos
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// La base del asistente viene de un editor de documentos Markdown. Estos tests impiden que sus textos
// vuelvan a colarse en lo que ve el usuario o en lo que se le dice a un modelo o a un agente.

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { agentPreamble, systemPrompt } from '../../src/ai/context.js';
import { memoryPrompt } from '../../src/memory/memory.js';
import { createTools } from '../../src/ai/tools.js';
import { AI_TRANSLATIONS } from '../../src/ai/translations.js';
import { unrealPrompt } from '../../src/unreal/unrealTools.js';

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..');

describe('el prompt de un agente (REQ-11)', () => {
  /**
   * Claude Code recibe las herramientas de Unreal por el servidor MCP `dbv-unreal-ai`, pero hay que DECIRLE
   * que existen: antes solo oía «estás ayudando con documentos Markdown».
   * @spec REQ-11, REQ-20
   */
  it('presenta el editor de Unreal y todas las herramientas del servidor MCP @spec REQ-11, REQ-20', () => {
    // Las herramientas de Tripo se definen en `generation/tools.rs` (D10) y el servidor las publica: las dos fuentes.
    const rust = ['mcp_server.rs', path.join('generation', 'tools.rs')].map((file) => readFileSync(path.join(root, 'src-tauri', 'src', file), 'utf8')).join('\n');
    const published = [...new Set([...rust.matchAll(/tool\(\s*"([a-z_0-9]+)"/g)].map((m) => m[1]))];
    expect(published).toEqual(expect.arrayContaining(['rig_model', 'animate_model', 'tripo_run', 'tripo_balance', 'generation_jobs']));
    // Las de la memoria del proyecto (`project_memory/tools.rs`) las presenta su propio prompt cuando hay memoria (REQ-38).
    const memoryRust = readFileSync(path.join(root, 'src-tauri', 'src', 'project_memory', 'tools.rs'), 'utf8');
    const memoryTools = [...new Set([...memoryRust.matchAll(/tool\(\s*"([a-z_0-9]+)"/g)].map((m) => m[1]))];
    expect(memoryTools).toEqual(expect.arrayContaining(['project_memory_read', 'project_memory_propose', 'project_tasks_list', 'project_tasks_propose', 'project_task_update']));
    const prompt = agentPreamble({ lang: 'es', domain: `${unrealPrompt({ projectName: 'DBVMCPFirst', engineVersion: '5.8.2', agent: true })}\n\n${memoryPrompt({ active: true, agent: true })}` });
    for (const name of [...published, ...memoryTools]) expect(prompt, name).toContain(name);
    expect(prompt).toContain('dbv-unreal-ai');
    expect(prompt).toContain('/Game/DBV');
    expect(prompt).toContain('confirm_cost=true');
    expect(prompt).toContain('Responde en español.');
    expect(prompt).not.toMatch(/Markdown|documents|syntax intact/i);
  });

  it('un agente no recibe la instrucción de `purpose` (su interfaz de permisos es la del agente) @spec REQ-11', () => {
    expect(unrealPrompt({ agent: true })).not.toContain('`purpose`');
    expect(unrealPrompt({ agent: false })).toContain('`purpose`');
    expect(unrealPrompt({ agent: true })).not.toContain('unreal_scene_summary');
  });

  it('sin dominio sigue siendo el asistente de documentos de la base @spec REQ-11', () => {
    expect(agentPreamble({ lang: 'en' })).toMatch(/Answer in English\./);
    expect(agentPreamble({ lang: 'en' })).toContain('folder of the open document');
  });
});

describe('el prompt de sistema de los modelos directos (REQ-10)', () => {
  it('con el dominio de Unreal no habla de Markdown ni del «autor» del texto @spec REQ-10', () => {
    for (const tools of [true, false]) {
      const prompt = systemPrompt({ lang: 'en', tools, domain: unrealPrompt({ projectName: 'X' }) });
      expect(prompt, `tools=${tools}`).toContain('Unreal Engine assistant');
      expect(prompt).toContain('project folder');
      expect(prompt).not.toMatch(/Markdown|author|CommonMark|KaTeX|mermaid/i);
    }
    expect(systemPrompt({ lang: 'en' })).toContain('Markdown dialect');
  });

  it('get_outline solo existe si la app tiene un documento abierto @spec REQ-10', () => {
    const base = { listFiles: async () => [], readText: async () => null, search: async () => [], checkProposal: async () => null, getProposal: () => ({ files: new Map() }) };
    expect(createTools(base).map((t) => t.name)).not.toContain('get_outline');
    expect(createTools({ ...base, outline: () => [] }).map((t) => t.name)).toContain('get_outline');
  });

  it('las herramientas de ficheros hablan del proyecto, no de documentos Markdown @spec REQ-13, REQ-22', () => {
    const base = { listFiles: async () => [], readText: async () => null, search: async () => [], checkProposal: async () => null, getProposal: () => ({ files: new Map() }) };
    for (const tool of createTools(base)) expect(tool.description, tool.name).not.toMatch(/Markdown documents/);
    expect(createTools(base).find((t) => t.name === 'list_files').description).toMatch(/Python\/C\+\+/);
  });
});

describe('lo que ve el usuario (REQ-24)', () => {
  it('las sugerencias de la conversación son de Unreal en español y en inglés @spec REQ-24', () => {
    for (const lang of ['es', 'en']) {
      const dictionary = AI_TRANSLATIONS[lang];
      const suggestions = ['ai.suggestLevel', 'ai.suggestPlace', 'ai.suggestGenerate'].map((key) => dictionary[key]);
      for (const text of suggestions) expect(text, `${lang}: falta una sugerencia`).toBeTruthy();
      expect(suggestions.join(' ')).toMatch(/nivel|level/i);
      expect(suggestions.join(' ')).not.toMatch(/documento|document|ortograf|spelling|tabla|table/i);
      for (const old of ['ai.suggestSummary', 'ai.suggestSpelling', 'ai.suggestTables']) expect(dictionary[old]).toBeUndefined();
    }
  });

  it('los avisos de nube y de herramientas hablan del proyecto de Unreal @spec RNF-IA.2, REQ-24', () => {
    for (const lang of ['es', 'en']) {
      expect(AI_TRANSLATIONS[lang]['ai.cloudText']).toMatch(/Unreal/);
      expect(AI_TRANSLATIONS[lang]['ai.toolsHint']).toMatch(/Unreal/);
      expect(AI_TRANSLATIONS[lang]['ai.cloudText']).not.toMatch(/documento abierto|open document/);
    }
  });

  it('español e inglés de la IA tienen las mismas claves @spec REQ-24', () => {
    expect(Object.keys(AI_TRANSLATIONS.en).sort()).toEqual(Object.keys(AI_TRANSLATIONS.es).sort());
  });
});
