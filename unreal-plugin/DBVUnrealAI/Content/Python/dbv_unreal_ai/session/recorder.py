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
RETRY_SECONDS = 0.25  # cada cuánto se reintenta resolver un objetivo ausente (buscar actores no es gratis)
FLUSH_SECONDS = 1.0  # a disco al menos cada segundo, aunque el bloque no esté lleno (FPS bajos)
SLOW_CHECK_TICKS = 30  # fotogramas de la ventana con la que se juzga si el editor va lento
SLOW_WARMUP_S = 2.0  # al empezar PIE hay tirones de carga: no cuentan para el aviso
SLOW_FRACTION = 0.5
ARM_SECONDS = 60.0  # armada sin que empiece PIE en este tiempo: se cierra sola
MAX_EVENTS = 20000
LOW_FRAME_DT = 0.1  # un fotograma de más de 100 ms cuenta como hueco en la medición


class Adapter(Protocol):
    """Lo que la grabadora necesita del motor. Lo implementa ``session.live`` (S106d)."""

    def game_world(self) -> Any | None: ...

    def world_alive(self, world: Any) -> bool: ...

    def level_name(self) -> str: ...

    def make_reader(self, world: Any, signal: Signal) -> Callable[[], list[float]] | None:
        """Devuelve un lector de la señal, o ``None`` si todavía no existe el objetivo."""

    def begin(self) -> list[str]:
        """Prepara el editor para grabar (por ejemplo, que no se frene al perder el foco). Devuelve notas para ``meta.json``."""

    def end(self) -> None:
        """Deja el editor como estaba. Se llama siempre al cerrar la sesión."""

    supported_events: frozenset[str]
    """Tipos de evento que sabe emitir (los demás del plan se avisan una vez y se ignoran)."""

    def poll_event(self, world: Any, kind: str) -> dict[str, Any] | None:
        """Mira si ``kind`` cambió desde la última vez y devuelve los datos del evento, o ``None``."""

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
        self._t0 = 0.0  # el cero de los tiempos: el momento en que aparece el mundo de juego, no el de armar
        self._armed_at = 0.0
        self._world_t0 = 0.0
        self._costs: list[float] = []
        self._gaps = 0
        self._ticks = 0
        self._recent: list[float] = []
        self._last_flush = 0.0
        self._next_try: dict[int, float] = {}
        self._dead_events: set[str] = set()
        self._notes: list[str] = []
        self._n_marks = 0
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
        self._t0 = self._armed_at = self._clock()
        self._started_at = time.strftime("%Y-%m-%dT%H:%M:%S")
        self.status = "recording"
        Recorder.active = self
        self._write_meta()
        for err in self.plan.errors:
            self._warn_once(f"plan: {err}")
        try:
            self._notes = list(self.adapter.begin() or [])
        except Exception as exc:  # noqa: BLE001 - sin esto se graba igual, solo que el editor puede frenarse al perder el foco
            self._warn_once(f"no se pudo preparar el editor: {type(exc).__name__}: {exc}")
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
            self._restore_editor()
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

    def _restore_editor(self) -> None:
        try:
            self.adapter.end()
        except Exception as exc:  # noqa: BLE001 - cerrar la sesión no puede fallar por esto
            self._warn_once(f"no se pudo restaurar el editor: {type(exc).__name__}: {exc}")

    def mark(self, label: str, **data: Any) -> None:
        self._n_marks += 1
        self.event("mark", label=label, **data)

    def event(self, kind: str, **data: Any) -> None:
        if self._events is None:
            return
        if self._n_events >= MAX_EVENTS:
            self._warn_once(f"límite de {MAX_EVENTS} eventos alcanzado; se ignoran los siguientes")
            return
        self._n_events += 1
        t = round(self._clock() - self._t0, 4) if self._world is not None else 0.0  # antes de Play, t = 0
        self._events.write(json.dumps({"t": t, "type": kind, **data}, ensure_ascii=False) + "\n")
        self._events.flush()

    # ------------------------------------------------------------------ tick
    def _on_tick(self, dt: float) -> None:
        if self.status == "recording":
            t0 = self._clock()
            try:
                self._tick(dt)
                self._costs.append(self._clock() - t0)
            except Exception as exc:  # un callback de Slate que falla se traga en silencio: se cuenta y se cierra
                self._warn_once(f"error interno de la grabadora: {type(exc).__name__}: {exc}")
                self.finish("internal_error")

    def _tick(self, dt: float) -> None:
        world = self.adapter.game_world()
        if world is None and self._world is None:  # armada, esperando a que empiece PIE
            if self._clock() - self._armed_at > ARM_SECONDS:
                self.finish("no_pie")
            return
        if world is None or (self._world is not None and not self.adapter.world_alive(self._world)):
            self.finish("pie_ended")  # antes de tocar ningún objeto del juego
            return
        if self._world is None:
            self._world = world
            self._world_t0 = self._t0 = self._clock()  # t = 0 es el principio de la partida
        self._ticks += 1
        if dt > LOW_FRAME_DT:
            self._gaps += 1
        now = self._clock() - self._t0
        self._poll_events(world)
        values = self._read_all(world, dt)
        assert self._writer is not None
        self._writer.append(now, values)
        if now - self._last_flush >= FLUSH_SECONDS:
            self._writer.flush()
            self._last_flush = now
        self._check_slow_editor(dt)
        if now >= self.plan.limits.max_seconds:
            self.finish("limit_seconds")
        elif self._writer.bytes_written >= self.plan.limits.max_bytes:
            self.finish("limit_bytes")

    def _poll_events(self, world: Any) -> None:
        """Emite los eventos de cambio de estado del plan. Un tipo que falla se apaga solo, con un aviso (R2)."""
        for kind in self.plan.events:
            if kind in self._dead_events:
                continue
            if kind not in self.adapter.supported_events:
                self._dead_events.add(kind)
                self._warn_once(f"evento «{kind}» todavía no disponible en esta variante: se ignora")
                continue
            try:
                data = self.adapter.poll_event(world, kind)
            except Exception as exc:  # noqa: BLE001 - objetivo destruido, API distinta…
                self._dead_events.add(kind)
                self._warn_once(f"evento «{kind}» apagado: {type(exc).__name__}: {exc}")
                continue
            if kind == "mark_key" and data is not None:  # la tecla del juego es una marca más, numerada
                self.mark(f"Momento {self._n_marks + 1}", source="key", key=self.plan.mark_key)
            elif data is not None:
                self.event(kind, **data)

    def _read_all(self, world: Any, dt: float) -> list[float]:
        out: list[float] = []
        for i, sig in enumerate(self.plan.signals):
            n = len(sig.columns)
            vals = self._read_one(world, i, sig, dt, n)
            out.extend(vals)
        return out

    def _read_one(self, world: Any, i: int, sig: Signal, dt: float, n: int) -> list[float]:
        out = [NAN] * n
        if sig.name in self.disabled:
            return out
        if sig.kind == "frame":
            return self._frame_values(sig, dt)
        reader = self._reader_for(world, i, sig)
        if reader is not None:
            out = self._call_reader(reader, sig, n)
        return out

    def _reader_for(self, world: Any, i: int, sig: Signal) -> Callable[[], list[float]] | None:
        """El lector de la señal; si el objetivo todavía no existe, se reintenta cada ``RETRY_SECONDS``."""
        reader = self._readers.get(i)
        now = self._clock()
        if reader is None and now >= self._next_try.get(i, 0.0):
            reader = self.adapter.make_reader(world, sig)
            if reader is None:
                self._next_try[i] = now + RETRY_SECONDS
                if now - self._world_t0 > RESOLVE_SECONDS:
                    self._disable(sig, "no se encontró el objetivo")
            else:
                self._readers[i] = reader
        return reader

    def _call_reader(self, reader: Callable[[], list[float]], sig: Signal, n: int) -> list[float]:
        out = [NAN] * n
        try:
            vals = list(reader())
            if len(vals) == n:
                out = [float(v) if v is not None else NAN for v in vals]
            else:
                self._disable(sig, f"devolvió {len(vals)} valores y se esperaban {n}")
        except Exception as exc:  # objetivo destruido, propiedad inexistente o valor no numérico (R2)
            self._disable(sig, f"{type(exc).__name__}: {exc}")
        return out

    def _check_slow_editor(self, dt: float) -> None:
        """R5: si la mitad de los últimos fotogramas es lenta, la medición no es fiable: se avisa una vez.

        Se mira una ventana móvil y se saltan los primeros segundos, donde el arranque de PIE da tirones normales.
        """
        if self._clock() - self._world_t0 >= SLOW_WARMUP_S:
            self._recent.append(dt)
            del self._recent[:-SLOW_CHECK_TICKS]
            full = len(self._recent) >= SLOW_CHECK_TICKS
            if full and sum(1 for d in self._recent if d > LOW_FRAME_DT) / len(self._recent) >= SLOW_FRACTION:
                self._warn_once(
                    "el editor va muy lento durante la grabación (fotogramas de más de 100 ms): la medición no es fiable. "
                    "Trae la ventana del editor al frente, desactiva «Use Less CPU when in Background» "
                    "y revisa la memoria de vídeo (puede haber una GPU saturada)"
                )

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
            "mark_key": self.plan.mark_key,
            "notes": self._notes,
        }
        (self.folder / "meta.json").write_text(json.dumps(meta, ensure_ascii=False, indent=2), encoding="utf-8")
