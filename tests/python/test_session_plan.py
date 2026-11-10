# =============================================================================
# DBV Unreal AI — Tests del plan de grabación de la caja negra
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de ``dbv_unreal_ai.session.plan``: presets, señales a mano, límites y errores.

@spec REQ-91
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.session import plan as plan_mod


class PlanTest(unittest.TestCase):
    def test_empty_plan_has_default_limits(self) -> None:
        p = plan_mod.parse_plan({})
        self.assertEqual(p.columns, [])
        self.assertEqual(p.limits, plan_mod.Limits(600.0, 500 * 1024 * 1024, 20))
        self.assertEqual(p.errors, [])

    def test_presets_expand_to_columns_and_events(self) -> None:
        p = plan_mod.parse_plan({"presets": ["player", "performance", "gameplay_events"]})
        self.assertEqual(
            p.columns,
            ["player.x", "player.y", "player.z", "player.yaw", "speed", "frame.dt_ms", "frame.fps"],
        )
        self.assertEqual(p.events, ["movement_mode", "movement_base", "pause", "log_error", "mark_key"])  # mark_key va siempre

    def test_event_only_presets_add_no_columns(self) -> None:
        p = plan_mod.parse_plan({"presets": ["input", "gameplay_events"]})
        self.assertEqual(p.columns, [])
        self.assertIn("input", p.events)

    def test_manual_signals_and_default_channels(self) -> None:
        p = plan_mod.parse_plan(
            {
                "signals": [
                    {"kind": "transform", "target": {"actor_label": "Grua"}, "socket": "cesta", "channels": ["x", "z"]},
                    {"kind": "transform", "target": {"actor_label": "Puerta"}},
                    {"kind": "variable", "target": {"actor_class": "BP_Cesta_C"}, "name": "Carga util"},
                ]
            }
        )
        self.assertEqual(
            p.columns,
            ["Grua.x", "Grua.z", *[f"Puerta.{c}" for c in plan_mod.TRANSFORM_CHANNELS], "Carga util"],
        )
        self.assertEqual(p.signals[0].extra["socket"], "cesta")

    def test_bad_signal_does_not_discard_the_rest(self) -> None:
        p = plan_mod.parse_plan(
            {
                "presets": ["player", "no_existe"],
                "signals": [
                    {"kind": "inventado", "target": {"actor_label": "A"}},
                    {"kind": "transform"},
                    {"kind": "variable", "target": {"actor_label": "A"}},
                    {"kind": "transform", "target": {"actor_label": "A"}, "channels": ["x", "w"]},
                    {"kind": "transform", "target": {"actor_label": "Bueno"}, "channels": ["x"]},
                ],
            }
        )
        self.assertEqual(len(p.errors), 5)
        self.assertIn("Bueno.x", p.columns)
        self.assertIn("player.x", p.columns)

    def test_duplicate_names_get_a_suffix(self) -> None:
        p = plan_mod.parse_plan(
            {
                "signals": [
                    {"kind": "transform", "target": {"actor_label": "A"}, "channels": ["x"]},
                    {"kind": "transform", "target": {"actor_label": "A"}, "channels": ["x"], "socket": "s"},
                ]
            }
        )
        self.assertEqual(p.columns, ["A.x", "A_2.x"])

    def test_limits_are_validated(self) -> None:
        p = plan_mod.parse_plan({"limits": {"max_seconds": "120", "max_bytes": "mucho", "keep_sessions": 0}})
        self.assertEqual(p.limits.max_seconds, 120.0)
        self.assertEqual(p.limits.keep_sessions, 1)
        self.assertEqual(p.limits.max_bytes, plan_mod.DEFAULT_MAX_BYTES)
        self.assertEqual(len(p.errors), 1)
        self.assertEqual(len(p.warnings), 1)

    def test_more_than_recommended_columns_warns(self) -> None:
        sigs = [{"kind": "transform", "target": {"actor_label": f"A{i}"}} for i in range(6)]  # 36 columnas
        p = plan_mod.parse_plan({"signals": sigs})
        self.assertEqual(len(p.columns), 36)
        self.assertEqual(len(p.warnings), 1)

    def test_the_mark_key_is_always_listened_to_and_can_be_changed(self) -> None:
        default = plan_mod.parse_plan({})
        self.assertEqual((default.mark_key, default.events), ("F9", ["mark_key"]))
        custom = plan_mod.parse_plan({"presets": ["gameplay_events"], "mark_key": " F10 "})
        self.assertEqual(custom.mark_key, "F10")
        self.assertEqual(custom.events.count("mark_key"), 1)
        bad = plan_mod.parse_plan({"mark_key": 7})
        self.assertEqual(bad.mark_key, "F9")
        self.assertEqual(len(bad.errors), 1)

    def test_non_object_plan_is_an_error_not_an_exception(self) -> None:
        self.assertEqual(plan_mod.parse_plan("hola").errors, ["el plan debe ser un objeto"])
        self.assertEqual(plan_mod.parse_plan(None).columns, [])


if __name__ == "__main__":
    unittest.main()
