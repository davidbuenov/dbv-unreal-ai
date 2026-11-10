# =============================================================================
# DBV Unreal AI — Tests del núcleo de la grabadora de la caja negra
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de ``dbv_unreal_ai.session.recorder`` con un mundo simulado.

@spec REQ-91
"""

from __future__ import annotations

import json
import math
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.session import frames, plan as plan_mod, recorder as rec_mod

DT = 1 / 72.0


class FakeClock:
    def __init__(self) -> None:
        self.t = 0.0

    def __call__(self) -> float:
        return self.t


class FakeWorld:
    alive = True


class FakeAdapter:
    """Mundo simulado: un solo actor «A» con posición; el resto de objetivos se configura por test."""

    def __init__(self, clock: FakeClock) -> None:
        self.clock = clock
        self.world: FakeWorld | None = FakeWorld()
        self.callback = None
        self.unregistered = 0
        self.readers: dict[str, object] = {}
        self.reader_calls = 0
        self.supported_events = frozenset({"pause", "movement_mode"})
        self.event_values: dict[str, object] = {}
        self.event_errors: set[str] = set()

    def game_world(self):
        return self.world

    def world_alive(self, world) -> bool:
        return world.alive

    def level_name(self) -> str:
        return "Mapa"

    def make_reader(self, world, signal):
        return self.readers.get(signal.name)

    def poll_event(self, world, kind):
        if kind in self.event_errors:
            raise RuntimeError("sin personaje")
        return self.event_values.pop(kind, None)

    def register_tick(self, callback):
        self.callback = callback
        return "h"

    def unregister_tick(self, handle) -> None:
        self.unregistered += 1
        self.callback = None

    def tick(self, dt: float = DT) -> None:
        self.clock.t += dt
        if self.callback:
            self.callback(dt)


def _plan(signals, **limits):
    return plan_mod.parse_plan({"signals": signals, "limits": limits})


def _sig(name: str, ch=("x",)):
    return {"kind": "transform", "target": {"actor_label": name}, "channels": list(ch)}


class RecorderTest(unittest.TestCase):
    def setUp(self) -> None:
        rec_mod.Recorder.active = None
        self._tmp = tempfile.TemporaryDirectory()
        self.folder = Path(self._tmp.name) / "s1"
        self.clock = FakeClock()
        self.adapter = FakeAdapter(self.clock)
        self.warnings: list[str] = []

    def tearDown(self) -> None:
        if rec_mod.Recorder.active is not None:
            rec_mod.Recorder.active.finish("test")  # en Windows no se borra un fichero abierto
        rec_mod.Recorder.active = None
        self._tmp.cleanup()

    def _rec(self, plan, **kw):
        return rec_mod.Recorder(plan, self.folder, self.adapter, clock=self.clock, warn=self.warnings.append, **kw)

    def _rows(self):
        return [(t, [c[i] for c in cols]) for ts, cols in frames.read_blocks(self.folder / "frames.bin") for i, t in enumerate(ts)]

    def _meta(self):
        return json.loads((self.folder / "meta.json").read_text(encoding="utf-8"))

    def test_records_rows_and_finishes_when_pie_ends(self) -> None:
        self.adapter.readers["A"] = lambda: [self.clock.t * 10]
        r = self._rec(_plan([_sig("A")]))
        r.start()
        for _ in range(5):
            self.adapter.tick()
        self.adapter.world = None  # termina PIE
        self.adapter.tick()
        self.assertEqual((r.status, r.reason), ("finished", "pie_ended"))
        self.assertEqual(len(self._rows()), 5)
        self.assertEqual(self.adapter.unregistered, 1)
        self.assertIsNone(rec_mod.Recorder.active)
        self.assertEqual(self._meta()["rows"], 5)
        self.assertEqual(self._meta()["status"], "finished")

    def test_stale_world_finishes_before_reading_any_object(self) -> None:
        calls = []
        self.adapter.readers["A"] = lambda: calls.append(1) or [0.0]
        r = self._rec(_plan([_sig("A")]))
        r.start()
        self.adapter.tick()
        self.adapter.world.alive = False  # otro PIE con el mismo nombre: el mundo retenido ya no vale
        self.adapter.tick()
        self.assertEqual(r.reason, "pie_ended")
        self.assertEqual(len(calls), 1)

    def test_failing_signal_is_disabled_once_and_the_rest_continue(self) -> None:
        def boom():
            raise RuntimeError("actor destruido")

        self.adapter.readers["A"] = lambda: [1.0]
        self.adapter.readers["B"] = boom
        r = self._rec(_plan([_sig("A"), _sig("B")]))
        r.start()
        for _ in range(4):
            self.adapter.tick()
        r.finish()
        rows = self._rows()
        self.assertEqual([row[1][0] for row in rows], [1.0] * 4)
        self.assertTrue(all(math.isnan(row[1][1]) for row in rows))
        self.assertEqual(len([w for w in self.warnings if "«B»" in w]), 1)
        self.assertIn("B", self._meta()["disabled"])

    def test_late_target_is_retried_then_disabled_after_the_window(self) -> None:
        r = self._rec(_plan([_sig("A")]))
        r.start()
        self.adapter.tick()
        self.adapter.readers["A"] = lambda: [7.0]  # el actor aparece un poco después
        self.clock.t += rec_mod.RETRY_SECONDS
        self.adapter.tick()
        self.assertTrue(math.isnan(self._rows_after_flush(r)[0][1][0]))
        self.assertEqual(self._rows_after_flush(r)[1][1][0], 7.0)
        self.assertEqual(r.disabled, {})

    def _rows_after_flush(self, r):
        r._writer.flush()
        return self._rows()

    def test_missing_target_is_disabled_after_resolve_window(self) -> None:
        r = self._rec(_plan([_sig("Nada")]))
        r.start()
        self.adapter.tick()
        self.clock.t += rec_mod.RESOLVE_SECONDS + 1
        self.adapter.tick()
        self.assertIn("Nada", r.disabled)
        self.assertEqual(r.status, "recording")  # el juego y la sesión siguen

    def test_time_limit_closes_the_session(self) -> None:
        self.adapter.readers["A"] = lambda: [0.0]
        r = self._rec(_plan([_sig("A")], max_seconds=1.0))
        r.start()
        for _ in range(100):
            self.adapter.tick()
        self.assertEqual(r.reason, "limit_seconds")
        self.assertLess(len(self._rows()), 80)

    def test_size_limit_closes_the_session(self) -> None:
        self.adapter.readers["A"] = lambda: [0.0]
        p = _plan([_sig("A")])
        p.limits = plan_mod.Limits(max_seconds=600, max_bytes=2000)
        r = self._rec(p)
        r.start()
        for _ in range(2000):
            self.adapter.tick()
        self.assertEqual(r.reason, "limit_bytes")

    def test_only_one_active_session_and_exclusion_with_playtest(self) -> None:
        self.adapter.readers["A"] = lambda: [0.0]
        first = self._rec(_plan([_sig("A")]))
        first.start()
        second = rec_mod.Recorder(_plan([]), Path(self._tmp.name) / "s2", self.adapter, clock=self.clock)
        with self.assertRaises(rec_mod.SessionBusyError):
            second.start()
        first.finish()
        busy = rec_mod.Recorder(_plan([]), Path(self._tmp.name) / "s3", self.adapter, clock=self.clock, busy=lambda: True)
        with self.assertRaises(rec_mod.SessionBusyError):
            busy.start()
        self.assertFalse((Path(self._tmp.name) / "s3").exists())

    def test_frame_signal_and_marks(self) -> None:
        p = plan_mod.parse_plan({"presets": ["performance"]})
        r = self._rec(p)
        r.start()
        self.adapter.tick(DT)
        r.mark("se atasca", nota="ahora")
        self.adapter.tick(0.2)  # fotograma lento: cuenta como hueco
        r.finish()
        rows = self._rows()
        self.assertAlmostEqual(rows[0][1][0], DT * 1000, places=2)
        self.assertAlmostEqual(rows[1][1][1], 5.0, places=3)
        self.assertEqual(self._meta()["gaps"]["slow_frames"], 1)
        ev = [json.loads(line) for line in (self.folder / "events.jsonl").read_text(encoding="utf-8").splitlines()]
        self.assertEqual((ev[0]["type"], ev[0]["label"]), ("mark", "se atasca"))

    def test_wrong_value_count_disables_the_signal(self) -> None:
        self.adapter.readers["A"] = lambda: [1.0, 2.0]
        r = self._rec(_plan([_sig("A")]))
        r.start()
        self.adapter.tick()
        self.assertIn("A", r.disabled)

    def test_internal_error_finishes_instead_of_hanging(self) -> None:
        r = self._rec(_plan([_sig("A")]))
        r.start()
        self.adapter.world = object()  # sin world_alive válido: provoca un error interno
        self.adapter.world_alive = None  # type: ignore[assignment]
        r._world = object()
        self.adapter.tick()
        self.assertEqual(r.reason, "internal_error")
        self.assertIsNone(rec_mod.Recorder.active)

    def test_finish_is_idempotent_and_ticks_after_finish_are_ignored(self) -> None:
        self.adapter.readers["A"] = lambda: [0.0]
        r = self._rec(_plan([_sig("A")]))
        r.start()
        self.adapter.tick()
        r.finish("stopped")
        r.finish("otra")
        r._on_tick(DT)
        self.assertEqual((r.reason, self.adapter.unregistered), ("stopped", 1))
        self.assertEqual(len(self._rows()), 1)

    def test_armed_session_waits_for_pie_and_gives_up_after_the_window(self) -> None:
        self.adapter.world = None  # todavía no se ha dado a Play
        self.adapter.readers["A"] = lambda: [3.0]
        r = self._rec(_plan([_sig("A")]))
        r.start()
        self.adapter.tick()
        self.assertEqual(r.status, "recording")
        self.adapter.world = FakeWorld()  # empieza PIE
        self.adapter.tick()
        self.assertEqual(len(self._rows_after_flush(r)), 1)
        r.finish()

        rec_mod.Recorder.active = None
        self.adapter.world = None
        self.folder = Path(self._tmp.name) / "s2"
        r2 = self._rec(_plan([_sig("A")]))
        r2.start()
        self.clock.t += rec_mod.ARM_SECONDS + 1
        self.adapter.tick()
        self.assertEqual(r2.reason, "no_pie")

    def test_missing_target_is_looked_up_at_most_every_retry_interval(self) -> None:
        calls = []
        self.adapter.make_reader = lambda world, signal: calls.append(1)  # type: ignore[assignment]
        r = self._rec(_plan([_sig("Nada")]))
        r.start()
        for _ in range(20):  # 20 fotogramas de 1/72 s = 0,28 s
            self.adapter.tick()
        self.assertLessEqual(len(calls), 2)
        r.finish()

    def test_data_reaches_disk_every_second_even_with_a_partial_block(self) -> None:
        self.adapter.readers["A"] = lambda: [1.0]
        r = self._rec(_plan([_sig("A")]))
        r.start()
        for _ in range(4):
            self.adapter.tick(0.3)  # 4 fotogramas lentos: muy lejos de un bloque de 72
        self.assertGreaterEqual(len(self._rows()), 3)
        r.finish()

    def test_slow_editor_warns_once(self) -> None:
        self.adapter.readers["A"] = lambda: [1.0]
        r = self._rec(_plan([_sig("A")]))
        r.start()
        for _ in range(80):
            self.adapter.tick(0.125)  # 8 FPS, como el editor sin foco medido el 2026-11-10
        self.assertEqual(len([w for w in self.warnings if "va muy lento" in w]), 1)
        r.finish()

    def test_startup_hitches_do_not_trigger_the_slow_editor_warning(self) -> None:
        self.adapter.readers["A"] = lambda: [1.0]
        r = self._rec(_plan([_sig("A")]))
        r.start()
        for _ in range(12):
            self.adapter.tick(0.15)  # tirones de carga al empezar PIE (1,8 s)
        for _ in range(120):
            self.adapter.tick(DT)  # luego va fluido
        self.assertEqual([w for w in self.warnings if "va muy lento" in w], [])
        r.finish()

    def _events(self):
        path = self.folder / "events.jsonl"
        return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]

    def test_state_change_events_are_written_with_their_time(self) -> None:
        self.adapter.readers["A"] = lambda: [0.0]
        p = plan_mod.parse_plan({"presets": ["gameplay_events"], "signals": [_sig("A")]})
        r = self._rec(p)
        r.start()
        self.adapter.tick()
        self.adapter.event_values["movement_mode"] = {"from": "MOVE_WALKING", "to": "MOVE_FALLING"}
        self.adapter.tick()
        self.adapter.event_values["pause"] = {"paused": True}
        self.adapter.tick()
        r.finish()
        ev = self._events()
        self.assertEqual([e["type"] for e in ev], ["movement_mode", "pause"])
        self.assertEqual(ev[0]["to"], "MOVE_FALLING")
        self.assertTrue(ev[1]["paused"])
        self.assertGreater(ev[1]["t"], ev[0]["t"])

    def test_unsupported_event_kinds_warn_once_and_the_rest_keep_working(self) -> None:
        p = plan_mod.parse_plan({"presets": ["gameplay_events"]})  # incluye movement_base y log_error: no soportados
        r = self._rec(p)
        r.start()
        for _ in range(3):
            self.adapter.tick()
        self.assertEqual(len([w for w in self.warnings if "«movement_base»" in w]), 1)
        self.assertEqual(len([w for w in self.warnings if "«log_error»" in w]), 1)
        self.assertEqual(r.status, "recording")

    def test_a_failing_event_source_is_turned_off_once(self) -> None:
        self.adapter.event_errors.add("movement_mode")
        p = plan_mod.parse_plan({"presets": ["gameplay_events"]})
        r = self._rec(p)
        r.start()
        for _ in range(3):
            self.adapter.tick()
        self.assertEqual(len([w for w in self.warnings if "«movement_mode» apagado" in w]), 1)
        self.adapter.event_values["pause"] = {"paused": True}
        self.adapter.tick()
        r.finish()
        self.assertEqual([e["type"] for e in self._events()], ["pause"])  # la pausa sigue funcionando

    def test_cost_is_stored_in_meta(self) -> None:
        self.adapter.readers["A"] = lambda: [0.0]
        r = self._rec(_plan([_sig("A")]))
        r.start()
        for _ in range(3):
            self.adapter.tick()
        r.finish()
        self.assertEqual(self._meta()["cost_ms"]["ticks"], 3)


if __name__ == "__main__":
    unittest.main()
