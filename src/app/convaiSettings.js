// =============================================================================
// DBV Unreal AI — Personajes de Convai en Ajustes
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-58, REQ-59
//
// Como las tarjetas de Tripo y World Labs, pero sin clave: el plugin de Convai se autentica solo en el editor. Aquí
// se registran los personajes del Playground (nombre, Character ID, nota) para que la IA use el ID correcto por su
// nombre. Rust valida y guarda la lista entera (`convai_characters_save`), así que las reglas viven en un solo sitio.

import { t } from '../ai/i18nBridge.js';
import * as api from './backend.js';
import { button, el } from './dom.js';
import { brandHeader } from './brandHeader.js';

/** El Playground de Convai, donde se crean los personajes y se copia su Character ID. */
export const CONVAI_PLAYGROUND = 'https://convai.com';

/** Abre la pestaña Convai de la ayuda (F1), con el tutorial (REQ-59). */
export const openConvaiHelp = () => document.dispatchEvent(new CustomEvent('dbv-open-help', { detail: { tab: 'convai' } }));

/**
 * Tarjeta de Convai: explicación, enlace al tutorial y la lista editable de personajes.
 * @param {{toast: (text: string, kind?: string) => void}} deps
 * @returns {Promise<HTMLElement>}
 */
export async function convaiSettingsCard(deps) {
  const loaded = await api.convaiCharactersList();
  const list = el('div', { className: 'stack convai-characters' });
  const rows = [];

  function addRow(character = { name: '', characterId: '', notes: '' }) {
    const name = el('input', { attrs: { type: 'text', placeholder: t('convai.name'), 'aria-label': t('convai.name') } });
    name.value = character.name;
    const id = el('input', { className: 'mono', attrs: { type: 'text', placeholder: t('convai.idPlaceholder'), 'aria-label': t('convai.id') } });
    id.value = character.characterId;
    const notes = el('input', { attrs: { type: 'text', placeholder: t('convai.notes'), 'aria-label': t('convai.notes') } });
    notes.value = character.notes ?? '';
    const row = el('div', { className: 'convai-character' }, name, id, notes);
    const entry = { row, name, id, notes };
    const remove = button('×', () => {
      row.remove();
      rows.splice(rows.indexOf(entry), 1);
    }, 'btn-ghost button--compact');
    remove.setAttribute('aria-label', t('convai.remove'));
    row.append(remove);
    rows.push(entry);
    list.append(row);
  }

  for (const character of loaded.ok && Array.isArray(loaded.value) ? loaded.value : []) addRow(character);
  if (!rows.length) addRow();

  const save = button(t('settings.save'), async () => {
    const characters = rows.map((r) => ({ name: r.name.value, characterId: r.id.value, notes: r.notes.value })).filter((c) => c.name.trim() || c.characterId.trim());
    const saved = await api.convaiCharactersSave(characters);
    deps.toast(saved.ok ? t('convai.saved', { n: String(saved.value.length) }) : saved.error.message ?? String(saved.error), saved.ok ? 'info' : 'error');
  }, 'btn-primary');

  return el(
    'div',
    { className: 'card stack' },
    brandHeader('Convai', 'convai'),
    el('p', { className: 'hint', text: t('convai.intro') }),
    el('div', { className: 'row' }, button(t('convai.tutorial'), openConvaiHelp), button(t('convai.playground'), () => window.__TAURI__?.opener?.openUrl?.(CONVAI_PLAYGROUND), 'btn-ghost')),
    el('div', { className: 'subheading', text: t('convai.characters') }),
    list,
    el('div', { className: 'row' }, button(t('convai.add'), () => addRow(), 'btn-ghost'), save),
  );
}
