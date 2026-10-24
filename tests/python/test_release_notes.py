# =============================================================================
# DBV Unreal AI — Tests de las notas de versión en dos idiomas
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Lo que se protege: cada versión tiene sus notas en DOS ficheros (español e inglés), sacadas de su CHANGELOG,
y la Release de GitHub lleva las dos; si falta un idioma, no se generan."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts" / "release"))

import notes  # noqa: E402

ES = """# Changelog

## [Sin publicar] / [Unreleased]

_Nada todavía._

## [0.2.0] - 2026-11-02

### Added
- Visor de mundos.

## [0.1.0] - 2026-10-24

Primera versión.

### Added
- Conexión con Unreal.
"""

EN = ES.replace("Sin publicar] / [Unreleased", "Unreleased").replace("Nada todavía", "Nothing yet").replace("Visor de mundos", "World viewer").replace("Primera versión", "First release").replace("Conexión con Unreal", "Unreal connection")


class ReleaseNotesTests(unittest.TestCase):
    """Notas por idioma."""

    def test_saca_solo_la_seccion_de_la_version(self) -> None:
        """La sección de 0.1.0, sin la de 0.2.0 ni la de lo no publicado."""
        body = notes.section(ES, "0.1.0")
        self.assertIn("Conexión con Unreal", body)
        self.assertNotIn("Visor de mundos", body)
        self.assertIsNone(notes.section(ES, "9.9.9"))
        self.assertEqual(notes.release_date(ES, "0.2.0"), "2026-11-02")

    def test_dos_ficheros_que_se_enlazan_y_una_release_con_los_dos_idiomas(self) -> None:
        """Español e inglés en ficheros distintos; el cuerpo de GitHub, inglés y debajo español."""
        spanish = notes.notes("v0.1.0", "es", ES)
        english = notes.notes("v0.1.0", "en", EN)
        self.assertTrue(spanish.startswith("# DBV Unreal AI v0.1.0 — Notas de la versión"))
        self.assertIn("[🇬🇧 English](./v0.1.0.en.md)", spanish)
        self.assertIn("[🇪🇸 Español](./v0.1.0.md)", english)
        self.assertNotIn("Conexión", english)
        body = notes.github_body(english, spanish)
        self.assertLess(body.index("Unreal connection"), body.index("## Versión en español"))
        self.assertLess(body.index("## Versión en español"), body.index("Conexión con Unreal"))
        self.assertNotIn("# DBV Unreal AI", body, "sin los títulos de los ficheros")

    def test_el_proyecto_real_tiene_la_v0_1_0_en_los_dos_changelog(self) -> None:
        """Lo que se publicó tiene notas en los dos idiomas (y los ficheros se escriben)."""
        with tempfile.TemporaryDirectory() as out:
            self.assertEqual(notes.main(["v0.1.0", "--out", out, "--body", str(Path(out) / "body.md")]), 0)
            self.assertTrue((Path(out) / "v0.1.0.md").is_file() and (Path(out) / "v0.1.0.en.md").is_file())
            self.assertIn("## Versión en español", (Path(out) / "body.md").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
