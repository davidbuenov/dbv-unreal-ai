# =============================================================================
# DBV Unreal AI — Notas de cada versión en español y en inglés (dos ficheros)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Escribe las notas de una versión en DOS ficheros a partir de los dos CHANGELOG.

Regla del proyecto (``project.config.md``): README, CHANGELOG y notas de release van siempre en dos ficheros,
español e inglés. Este script saca la sección ``## [X.Y.Z]`` de ``dbv-specs-ops/CHANGELOG.md`` y de
``CHANGELOG.en.md`` y escribe:

* ``dbv-specs-ops/releases/vX.Y.Z.md`` (español) y ``vX.Y.Z.en.md`` (inglés), que se versionan en ``/ship``;
* con ``--body FICHERO``, el cuerpo de la Release de GitHub (que es un solo texto): inglés y, debajo, español.

Uso, desde la raíz del repositorio::

    python scripts/release/notes.py v0.1.0
    python scripts/release/notes.py v0.1.0 --out release-notes --body release-notes/body.md

Solo biblioteca estándar. Código de salida 1 si falta la sección en alguno de los dos CHANGELOG.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CHANGELOGS = {"es": ROOT / "dbv-specs-ops" / "CHANGELOG.md", "en": ROOT / "dbv-specs-ops" / "CHANGELOG.en.md"}
TITLES = {"es": "DBV Unreal AI {tag} — Notas de la versión", "en": "DBV Unreal AI {tag} — Release notes"}
MIRRORS = {"es": "🇪🇸 Español · [🇬🇧 English](./{tag}.en.md)", "en": "[🇪🇸 Español](./{tag}.md) · 🇬🇧 English"}


def section(text: str, version: str) -> str | None:
    """Cuerpo de ``## [version]`` (sin su título) hasta la siguiente sección de versión, o ``None``."""
    pattern = re.compile(rf"^## \[{re.escape(version)}\][^\n]*\n(.*?)(?=^## \[|\Z)", re.MULTILINE | re.DOTALL)
    match = pattern.search(text)
    return match.group(1).strip() if match else None


def release_date(text: str, version: str) -> str:
    """Fecha de la sección (``## [0.1.0] - 2026-10-24``), o cadena vacía."""
    match = re.search(rf"^## \[{re.escape(version)}\]\s*-\s*(\S+)", text, re.MULTILINE)
    return match.group(1) if match else ""


def notes(tag: str, language: str, text: str) -> str | None:
    """Notas completas de un idioma: título, fecha, enlace al otro idioma y la sección del CHANGELOG."""
    version = tag.lstrip("v")
    body = section(text, version)
    if body is None:
        return None
    date = release_date(text, version)
    title = TITLES[language].format(tag=f"v{version}")
    return f"# {title}\n\n**{MIRRORS[language].format(tag=f'v{version}')}**{f' · {date}' if date else ''}\n\n{body}\n"


def github_body(english: str, spanish: str) -> str:
    """Cuerpo único de la Release de GitHub: inglés y, debajo, español (sin los títulos de fichero)."""
    strip = lambda text: text.split("\n", 4)[-1].strip()  # noqa: E731 — quita título y enlace de idioma
    return f"{strip(english)}\n\n---\n\n## Versión en español\n\n{strip(spanish)}\n"


def main(argv: list[str] | None = None) -> int:
    """Punto de entrada."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("tag", help="Versión: v0.1.0 o 0.1.0")
    parser.add_argument("--out", default=str(ROOT / "dbv-specs-ops" / "releases"), help="Carpeta de los dos ficheros")
    parser.add_argument("--body", help="Escribe también el cuerpo de la Release de GitHub en este fichero")
    args = parser.parse_args(argv)
    tag = f"v{args.tag.lstrip('v')}"
    written = {lang: notes(tag, lang, path.read_text(encoding="utf-8")) for lang, path in CHANGELOGS.items()}
    missing = [CHANGELOGS[lang].name for lang, text in written.items() if text is None]
    code = 0
    if missing:
        print(f"falta la sección [{tag.lstrip('v')}] en {', '.join(missing)}: las notas van siempre en los dos idiomas", file=sys.stderr)
        code = 1
    else:
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        (out / f"{tag}.md").write_text(written["es"], encoding="utf-8")
        (out / f"{tag}.en.md").write_text(written["en"], encoding="utf-8")
        if args.body:
            Path(args.body).write_text(github_body(written["en"], written["es"]), encoding="utf-8")
        print(f"{out / f'{tag}.md'}\n{out / f'{tag}.en.md'}")
    return code


if __name__ == "__main__":
    sys.exit(main())
