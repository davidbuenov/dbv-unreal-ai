// =============================================================================
// DBV Unreal AI — Tests: Convai y accesos directos en la vista Proyecto
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it, vi } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { convaiCard, convaiLive, convaiSkillCount, directAccessBanner, hasConvaiToolsets } from '../../src/app/convaiCard.js';
import { createApprovalCard } from '../../src/unreal/approvalCard.js';
import { unrealPrompt } from '../../src/unreal/unrealTools.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const recorded = (file) => JSON.parse(readFileSync(path.join(root, 'tests/fixtures/unreal-mcp/ue-5.8.4', file), 'utf8'));

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

describe('Accesos directos al editor (REQ-52)', () => {
  it('avisa de lo que escribió Convai y explica cómo evitarlo; sin ficheros no pinta nada @spec REQ-52', () => {
    expect(directAccessBanner([])).toBeNull();
    const banner = directAccessBanner([{ file: '.mcp.json', kind: 'mcp' }, { file: 'CLAUDE.md', kind: 'primer' }]);
    expect(banner.textContent).toContain('Un agente vería el editor dos veces');
    expect(banner.textContent).toContain('.mcp.json');
    expect(banner.textContent).toContain('instrucciones de Convai');
    expect(banner.textContent).toContain('La app no modifica estos ficheros');
  });
});

describe('Convai en la vista Proyecto (REQ-55)', () => {
  const signed = { cited: true, enabled: true, version: '4.0.0-beta.27', engineOk: true, signedIn: true, keyInGit: false };

  it('reconoce los toolsets y las 17 guías de las respuestas grabadas del editor real @spec REQ-55', () => {
    const listing = recorded('list_toolsets.response.json').result.content[0].text;
    const names = listing.split('\n').filter((l) => l.startsWith('- ')).map((l) => ({ name: l.slice(2).split(':')[0] }));
    expect(hasConvaiToolsets(names)).toBe(true);
    expect(hasConvaiToolsets([{ name: 'EditorToolset.EditorAppToolset' }])).toBe(false);
    const skills = JSON.parse(recorded('call_tool.ListSkills.response.json').result.content[0].text);
    expect(convaiSkillCount(skills.returnValue)).toBe(17);
  });

  it('pinta todo en verde con el editor conectado y nunca enseña la clave @spec REQ-55', () => {
    const card = convaiCard(signed, { toolsets: true, skills: 17 });
    expect(card.textContent).toContain('Plugin ConvAI 4.0.0-beta.27 activo');
    expect(card.textContent).toContain('Guías de Convai (17)');
    expect(card.querySelectorAll('.check-fail')).toHaveLength(0);
    expect(card.textContent).not.toMatch(/API_Key|AuthToken/);
  });

  it('sin editor deja pendientes las comprobaciones en vivo; avisa si la clave está en git @spec REQ-55', () => {
    const card = convaiCard({ ...signed, keyInGit: true }, null);
    expect(card.textContent).toContain('Conecta este proyecto');
    expect(card.textContent).toContain('La clave de Convai se subiría a git');
  });

  it('un proyecto sin Convai lo ofrece plegado como opcional @spec REQ-55', () => {
    const card = convaiCard({ cited: false, enabled: false, version: null, engineOk: true, signedIn: false, keyInGit: false }, null);
    expect(card.tagName).toBe('DETAILS');
    expect(card.textContent).toContain('Convai (opcional)');
  });

  it('las comprobaciones en vivo preguntan al editor por los toolsets y las guías @spec REQ-55', async () => {
    const api = {
      unrealListToolsets: vi.fn(async () => ({ ok: true, value: [{ name: 'ConvaiToolset.ConvaiSetupToolset' }] })),
      unrealCallTool: vi.fn(async () => ({ ok: true, value: { text: '', isError: false, value: { '/ConvAI/Skills/ConvaiQuickStart.ConvaiQuickStart_C': 'x', '/PCGToolset/Skills/X': 'y' } } })),
    };
    expect(await convaiLive(api, null)).toBeNull();
    expect(await convaiLive(api, { host: '127.0.0.1', port: 8000 })).toEqual({ toolsets: true, skills: 1 });
    expect(api.unrealCallTool).toHaveBeenCalledWith({ host: '127.0.0.1', port: 8000 }, 'ToolsetRegistry.AgentSkillToolset', 'ListSkills', {});
  });
});

describe('Convai en la conversación (REQ-56)', () => {
  it('el prompt real del asistente y el del agente llevan las reglas de Convai solo si está @spec REQ-56', () => {
    for (const agent of [false, true]) {
      const withConvai = unrealPrompt({ projectName: 'Demo', agent, convai: true });
      expect(withConvai).toContain('ListSkills');
      expect(withConvai).toContain('ConvaiQuickStart');
      expect(withConvai).toContain('Character ID');
      expect(withConvai).toContain('never invent one');
      expect(withConvai).toContain('convai_characters');
      expect(withConvai).toContain('Play inside Unreal');
      expect(unrealPrompt({ projectName: 'Demo', agent })).not.toContain('Convai');
    }
  });

  it('la tarjeta de aprobación de una acción de Convai avisa de que guarda el Blueprint @spec REQ-56', () => {
    const convai = createApprovalCard({ kind: 'tool', toolset: 'ConvaiToolset.ConvaiSetupToolset', tool: 'SetupConvaiCharacter', args: { characterBlueprintPath: '/Game/BP_Guard', characterId: 'abc' } });
    expect(convai.node.textContent).toContain('Convai compila y guarda el Blueprint');
    const other = createApprovalCard({ kind: 'tool', toolset: 'EditorToolset.SceneTools', tool: 'add_to_scene_from_asset', args: {} });
    expect(other.node.textContent).not.toContain('Convai');
  });
});
