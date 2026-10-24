// =============================================================================
// DBV Unreal AI — Ayudas mínimas de DOM
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Todo texto entra por `textContent`: nada de lo que llega del editor, de un modelo o de un
// fichero se interpreta como HTML.

/** Elemento con clase, texto e hijos. */
export function el(tag, options = {}, ...children) {
  const node = document.createElement(tag);
  if (options.className) node.className = options.className;
  if (options.text !== undefined) node.textContent = options.text;
  if (options.title) node.title = options.title;
  for (const [name, value] of Object.entries(options.attrs ?? {})) node.setAttribute(name, value);
  for (const [name, handler] of Object.entries(options.on ?? {})) node.addEventListener(name, handler);
  for (const child of children.flat()) if (child !== null && child !== undefined && child !== false) node.append(child);
  return node;
}

/** Botón con texto y acción. */
export function button(text, onClick, className = '') {
  return el('button', { className, text, attrs: { type: 'button' }, on: { click: onClick } });
}

/** Último tramo de una ruta (`D:/x/Demo.uproject` → `Demo.uproject`). */
export const baseName = (path) => String(path ?? '').split(/[\\/]/).filter(Boolean).pop() ?? '';

/** Carpeta de una ruta. */
export const dirName = (path) => String(path ?? '').replace(/[\\/][^\\/]*$/, '');
