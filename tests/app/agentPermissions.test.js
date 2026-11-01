// =============================================================================
// DBV Unreal AI — Tests de los permisos de los agentes: las consultas no preguntan
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { describe, expect, it } from 'vitest';
import { autoAllow, ownTool } from '../../src/unreal/agentPermissions.js';

const READ_ONLY = ['unreal_list_toolsets', 'unreal_describe_toolset', 'unreal_scene_summary', 'recipes_list', 'recipe_check'];
const OPTIONS = [{ optionId: 'always', kind: 'allow_always' }, { optionId: 'once', kind: 'allow_once' }, { optionId: 'no', kind: 'reject_once' }];
const request = (title, rawInput = {}) => ({ options: OPTIONS, toolCall: { title, rawInput } });

describe('permisos de los agentes @spec REQ-12', () => {
  it('reconoce las herramientas del servidor de la app con el nombre que pone cada agente', () => {
    expect(ownTool(request('dbv-unreal-ai_recipe_check'))).toBe('recipe_check'); // Antigravity (lo de la captura)
    expect(ownTool(request('mcp__dbv-unreal-ai__unreal_scene_summary'))).toBe('unreal_scene_summary'); // Claude Code
    expect(ownTool(request('dbv-unreal-ai.recipes_list'))).toBe('recipes_list');
    expect(ownTool(request('Run shell command: rm -rf dbv-unreal-ai_recipe_check'))).toBe('recipe_check');
    expect(ownTool(request('other-server_recipe_check'))).toBeNull();
    expect(ownTool(request('Edit Source/Demo/DemoActor.cpp'))).toBeNull();
  });

  it('las consultas se conceden solas una vez, sin «siempre»', () => {
    expect(autoAllow(request('dbv-unreal-ai_recipe_check'), READ_ONLY)).toEqual({ optionId: 'once', tool: 'recipe_check' });
    expect(autoAllow(request('mcp__dbv-unreal-ai__unreal_list_toolsets'), READ_ONLY)).toEqual({ optionId: 'once', tool: 'unreal_list_toolsets' });
  });

  it('unreal_call_tool solo va solo si la herramienta de Unreal es de lectura', () => {
    expect(autoAllow(request('dbv-unreal-ai_unreal_call_tool', { toolset: 'editor_toolset.toolsets.scene.SceneTools', tool: 'get_actors' }), READ_ONLY)).toEqual({ optionId: 'once', tool: 'get_actors' });
    expect(autoAllow(request('dbv-unreal-ai_unreal_call_tool', { toolset: 'editor_toolset.toolsets.scene.SceneTools', tool: 'spawn_actor' }), READ_ONLY)).toBeNull();
    expect(autoAllow(request('dbv-unreal-ai_unreal_call_tool', { tool: 'run_python' }), READ_ONLY)).toBeNull();
    expect(autoAllow(request('dbv-unreal-ai_unreal_call_tool', {}), READ_ONLY)).toBeNull();
  });

  it('lo que modifica, gasta o no es del servidor de la app sigue preguntando', () => {
    for (const title of ['dbv-unreal-ai_unreal_run_python', 'dbv-unreal-ai_library_send_to_unreal', 'dbv-unreal-ai_generate_3d_model', 'Write file', 'Run terminal command']) {
      expect(autoAllow(request(title), READ_ONLY), title).toBeNull();
    }
    // Sin opción de permitir no hay nada que conceder.
    expect(autoAllow({ options: [{ optionId: 'no', kind: 'reject_once' }], toolCall: { title: 'dbv-unreal-ai_recipe_check' } }, READ_ONLY)).toBeNull();
  });
});
