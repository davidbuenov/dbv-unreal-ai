# =============================================================================
# DBV Unreal AI — Grabadora de sesión (núcleo, sin dependencia de Unreal)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Núcleo de la grabadora: ciclo de vida, límites, avisos únicos y coste medido.

No toca ``unreal``: todo lo del motor entra por un *adaptador* (ver ``Adapter``), así
que la suite lo prueba con un mundo simulado. El adaptador real llega en S106d.

Reglas que cumple (REQ-91, R1..R9):

* **Termina siempre (R1):** en cada tick lo primero es comprobar que el mundo
  de juego sigue vivo; si ya no lo está, ``finish`` va antes de leer ningún objeto.
  También cierra por tiempo o por tamaño, nunca por número de fotogramas.
* **Un fallo apaga solo esa señal (R2):** un aviso, una sola vez; el resto sigue.
* **Mide su propio coste (R3)** y lo guarda en ``meta.json``.
* **No modifica nada del proyecto (R4):** solo escribe en su carpeta de sesión.
* **Bloques en disco (R6)** y **una sola sesión activa (R9)**.

@spec REQ-91
"""

from __future__ import annotations

import json
import math
import time
from pathlib import Path
from typing import Any, Callable, Protocol

from .frames import NAN, FrameWriter
from .plan import Plan, Signal

META_VERSION = 1
RESOLVE_SECONDS = 5.0  # los actores que aparecen tarde se reintentan unos segundos
MAX_EVENTS = 20000
LOW_FRAME_DT = 0.1  # un fotograma de más de 100 ms cuenta como hueco en la medición


class Adapter(Protocol):
    """Lo que la grabadora necesita del motor. Lo implementa ``session.live`` (S106d)."""

    def game_world(self) -> Any | None: ...

    def world_alive(self, world: Any) -> bool: ...

    def level_name(self) -> str: ...

    def make_reader(self, world: Any, signal: Signal) -> Callable[[], list[float]] | None:
        """Devuelve un lector de la señal, o ``None`` si todavía no existe el objetivo."""

    def register_tick(self, callback: Callable[[float], None]) -> Any: ...

    def unregister_tick(self, handle: Any) -> None: ...


class SessionBusyError(RuntimeError):
    """Ya hay una grabación activa o un playtest en marcha (R9)."""


class Recorder:
    active: "Recorder | None" = None  # una sola sesión activa por editor (R9)

    def __init__(
        self,
        plan: Plan,
        folder: Path | str,
        adapter: Adapter,
        *,
        clock: Callable[[], float] = time.perf_counter,
        busy: Callable[[], bool] = lambda: False,
        warn: Callable[[str], None] = print,
    ) -> None:
        self.plan = plan
        self.folder = Path(folder)
        self.adapter = adapter
        self._clock = clock
        self._busy = busy
        self._warn_out = warn
        self.status = "idle"
        self.reason = ""
        self.warnings: list[str] = []
        self.disabled: dict[str, str] = {}
        self._writer: FrameWriter | None = None
        self._events = None
        self._n_events = 0
        self._handle: Any = None
        self._world: Any = None
        self._readers: dict[int, Callable[[], list[float]]] = {}
        self._t0 = 0.0
        self._world_t0 = 0.0
        self._costs: list[float] = []
        self._gaps = 0
        self._ticks = 0
        self._started_at = ""

    # ------------------------------------------------------------------ ciclo de vida
    def start(self) -> None:
        if Recorder.active is not None:
            raise SessionBusyError("ya hay una grabación activa")
        if self._busy():
            raise SessionBusyError("hay un playtest en marcha; espera a que termine")
        self.folder.mkdir(parents=True, exist_ok=True)
        self._writer = FrameWriter(self.folder / "frames.bin", len(self.plan.columns))
        self._events = open(self.folder / "events.jsonl", "a", encoding="utf-8")  # noqa: SIM115
        self._t0 = self._clock()
        self._started_at = time.strftime("%Y-%m-%dT%H:%M:%S")
        self.status = "recording"
        Recorder.active = self
        self._write_meta()
        for err in self.plan.errors:
            self._warn_once(f"plan: {err}")
        self._handle = self.adapter.register_tick(self._on_tick)

    def finish(self, reason: str = "stopped") -> None:
        """Cierra la sesión. Idempotente y lo primero que se hace al detectar el fin (R1)."""
        if self.status != "recording":
            return
        self.status, self.reason = "finished", reason
        handle, self._handle = self._handle, None
        try:
            if handle is not None:
                self.adapter.unregister_tick(handle)
        finally:
            self._world = None
            self._readers.clear()
            if self._writer is not None:
                self._writer.close()
            if self._events is not None:
                self._events.close()
                self._events = None
            if Recorder.active is self:
                Recorder.active = None
            self._write_meta()

    def mark(self, label: str, **data: Any) -> None:
        self.event("mark", label=label, **data)

    def event(self, kind: str, **data: Any) -> None:
        if self._events is None:
            return
        if self._n_events >= MAX_EVENTS:
            self._warn_once(f"límite de {MAX_EVENTS} eventos alcanzado; se ignoran los siguientes")
            return
        self._n_events += 1
        self._events.write(json.dumps({"t": round(self._clock() - self._t0, 4), "type": kind, **data}, ensure_ascii=False) + "\n")
        self._events.flush()

    # ------------------------------------------------------------------ tick
    def _on_tick(self, dt: float) -> None:
        if self.status != "recording":
            return
        t0 = self._clock()
        try:
            self._tick(dt)
        except Exception as exc:  # un callback de Slate que falla se traga en silencio: se cuenta y se cierra
            self._warn_once(f"error interno de la grabadora: {type(exc).__name__}: {exc}")
            self.finish("internal_error")
            return
        self._costs.append(self._clock() - t0)

    def _tick(self, dt: float) -> None:
        world = self.adapter.game_world()
        if world is None or (self._world is not None and not self.adapter.world_alive(self._world)):
            self.finish("pie_ended")  # antes de tocar ningún objeto del juego
            return
        if self._world is None:
            self._world = world
            self._world_t0 = self._clock()
        self._ticks += 1
        if dt > LOW_FRAME_DT:
            self._gaps += 1
        now = self._clock() - self._t0
        values = self._read_all(world, dt)
        assert self._writer is not None
        self._writer.append(now, values)
        if now >= self.plan.limits.max_seconds:
            self.finish("limit_seconds")
        elif self._writer.bytes_written >= self.plan.limits.max_bytes:
            self.finish("limit_bytes")

    def _read_all(self, world: Any, dt: float) -> list[float]:
        out: list[float] = []
        for i, sig in enumerate(self.plan.signals):
            n = len(sig.columns)
            vals = self._read_one(world, i, sig, dt, n)
            out.extend(vals)
        return out

    def _read_one(self, world: Any, i: int, sig: Signal, dt: float, n: int) -> list[float]:
        missing = [NAN] * n
        if sig.name in self.disabled:
            return missing
        if sig.kind == "frame":
            return self._frame_values(sig, dt)
        reader = self._readers.get(i)
        if reader is None:
            reader = self.adapter.make_reader(world, sig)
            if reader is None:
                if self._clock() - self._world_t0 > RESOLVE_SECONDS:
                    self._disable(sig, "no se encontró el objetivo")
                return missing
            self._readers[i] = reader
        try:
            vals = list(reader())
        except Exception as exc:  # objetivo destruido o propiedad inexistente (R2)
            self._disable(sig, f"{type(exc).__name__}: {exc}")
            return missing
        if len(vals) != n:
            self._disable(sig, f"devolvió {len(vals)} valores y se esperaban {n}")
            return missing
        return [float(v) if v is not None else NAN for v in vals]

    @staticmethod
    def _frame_values(sig: Signal, dt: float) -> list[float]:
        table = {"dt_ms": dt * 1000.0, "fps": (1.0 / dt) if dt > 0 else NAN}
        return [table[c] for c in sig.channels]

    def _disable(self, sig: Signal, why: str) -> None:
        self.disabled[sig.name] = why
        self._warn_once(f"señal «{sig.name}» apagada: {why}")

    def _warn_once(self, msg: str) -> None:
        if msg in self.warnings:
            return
        self.warnings.append(msg)
        self._warn_out(f"[DBV session] {msg}")

    # ------------------------------------------------------------------ meta.json
    def cost_ms(self) -> dict[str, float]:
        if not self._costs:
            return {"mean": 0.0, "p95": 0.0, "max": 0.0, "ticks": 0}
        s = sorted(self._costs)
        return {
            "mean": round(1000 * sum(s) / len(s), 4),
            "p95": round(1000 * s[max(0, math.ceil(len(s) * 0.95) - 1)], 4),
            "max": round(1000 * s[-1], 4),
            "ticks": len(s),
        }

    def _write_meta(self) -> None:
        meta = {
            "version": META_VERSION,
            "started_at": self._started_at,
            "level": self.adapter.level_name(),
            "mode": "pie",
            "status": self.status,
            "reason": self.reason,
            "columns": self.plan.columns,
            "events": self.plan.events,
            "limits": {
                "max_seconds": self.plan.limits.max_seconds,
                "max_bytes": self.plan.limits.max_bytes,
            },
            "rows": self._writer.rows_written if self._writer else 0,
            "duration_s": round(self._clock() - self._t0, 3),
            "cost_ms": self.cost_ms(),
            "gaps": {"slow_frames": self._gaps, "threshold_s": LOW_FRAME_DT},
            "disabled": self.disabled,
            "warnings": self.warnings,
        }
        (self.folder / "meta.json").write_text(json.dumps(meta, ensure_ascii=False, indent=2), encoding="utf-8")
