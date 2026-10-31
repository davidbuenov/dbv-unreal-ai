// =============================================================================
// DBV Unreal AI — Tests: zonas del nivel en la conversación
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

import { beforeAll, describe, expect, it, vi } from 'vitest';
import { APP_TRANSLATIONS } from '../../src/app/translations.js';
import { createZonesCard, createZoneTools } from '../../src/unreal/zones.js';

const PROPOSE = { name: 'level_zones_propose', description: 'Propose zones…', inputSchema: { type: 'object', properties: { zones: { type: 'array' } }, required: ['zones'] } };
const LIST = { name: 'level_zones_list', description: 'List zones…', inputSchema: { type: 'object', properties: {}, required: [] } };
const cocina = { name: 'Cocina', description: 'El horno', label: 'DBV_Zone_Cocina', x: 300, y: 100, z: 0, radius: 2 };
const puerta = { name: 'Puerta', description: 'Salida', label: 'DBV_Zone_Puerta', x: 0, y: 800, z: 0, radius: 3 };

beforeAll(() => {
  window.DBV_I18N.register(APP_TRANSLATIONS);
  window.DBV_I18N.setLang('es');
});

function deps(askZones) {
  return {
    specs: [PROPOSE, LIST],
    preview: vi.fn(async (args) => ({ ok: true, value: args.zones.map((z) => ({ ...z, label: `DBV_Zone_${z.name}`, x: z.x * 100, y: z.y * 100, z: (z.z ?? 0) * 100, radius: z.radius ?? 2 })) })),
    apply: vi.fn(async (zones) => ({ ok: true, value: { success: true, result: { zones: zones.map((z) => ({ name: z.name, created: true, convai: true })), convai: true } } })),
    list: vi.fn(async () => ({ ok: true, value: { success: true, result: { zones: [{ label: 'DBV_Zone_Cocina', name: 'Cocina' }] } } })),
    askZones,
  };
}

describe('Zonas del nivel (REQ-57)', () => {
  it('la tarjeta se edita, quita filas, marca las que ya existen y el foco va a Descartar @spec REQ-57', async () => {
    const card = createZonesCard([cocina, puerta], ['DBV_Zone_Cocina']);
    document.body.append(card.node);
    await Promise.resolve();
    expect(document.activeElement.textContent).toBe('Descartar');
    expect(card.node.textContent).toContain('actualizar');
    expect(card.node.textContent).toContain('(3, 1, 0) m');
    const accept = [...card.node.querySelectorAll('button')].find((b) => b.textContent.startsWith('Crear zonas'));
    expect(accept.textContent).toBe('Crear zonas (2)');
    card.node.querySelector('input').value = 'Cocina grande';
    card.node.querySelector('button[aria-label="Quitar esta zona"]:last-of-type');
    [...card.node.querySelectorAll('button')].filter((b) => b.getAttribute('aria-label') === 'Quitar esta zona')[1].click();
    expect(accept.textContent).toBe('Crear zonas (1)');
    accept.click();
    const choice = await card.choice;
    expect(choice.action).toBe('accept');
    expect(choice.zones.map((z) => z.name)).toEqual(['Cocina grande']);
    card.node.remove();
  });

  it('nada cambia en el nivel sin aceptar la tarjeta @spec REQ-57', async () => {
    const d = deps(async () => ({ action: 'discard', zones: [] }));
    const [propose] = createZoneTools(d);
    const text = await propose.run({ zones: [{ name: 'Cocina', description: 'x', x: 3, y: 1 }] });
    expect(d.apply).not.toHaveBeenCalled();
    expect(text).toContain('DISCARDED');
  });

  it('aceptar crea lo que dejó la persona, revalidado en Rust, y avisa de las que ya existían @spec REQ-57', async () => {
    const askZones = vi.fn(async (zones) => ({ action: 'accept', zones: [{ ...zones[0], name: 'Cocina grande' }] }));
    const d = deps(askZones);
    const [propose, list] = createZoneTools(d);
    const text = await propose.run({ zones: [{ name: 'Cocina', description: 'El horno', x: 3, y: 1 }, { name: 'Puerta', description: 'Salida', x: 0, y: 8 }] });
    expect(askZones.mock.calls[0][1]).toEqual(['DBV_Zone_Cocina']);
    expect(d.preview).toHaveBeenCalledTimes(2);
    expect(d.preview.mock.calls[1][0].zones[0]).toMatchObject({ name: 'Cocina grande', x: 3, y: 1 });
    expect(d.apply.mock.calls[0][0][0].label).toBe('DBV_Zone_Cocina grande');
    expect(text).toContain('Zones saved in the level');
    expect(await list.run({})).toContain('Cocina');
  });
});
