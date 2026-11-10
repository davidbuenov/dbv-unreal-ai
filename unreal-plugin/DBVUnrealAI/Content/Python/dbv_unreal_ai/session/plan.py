# =============================================================================
# DBV Unreal AI — Plan de grabación: señales, presets y límites
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Qué se graba y hasta cuándo. Python puro, sin ``unreal``: lo valida y lo prueba la suite.

Un plan es un diccionario JSON (lo propone la IA con ``session_plan`` y lo acepta
el usuario)::

    {"presets": ["player", "performance"],
     "signals": [{"kind": "transform", "target": {"actor_label": "Grua"}, "socket": "cesta",
                  "channels": ["x", "y", "z"]},
                 {"kind": "variable", "target": {"actor_class": "BP_Cesta_C"}, "name": "Carga util"}],
     "limits": {"max_seconds": 120}}

Cada señal ocupa una o varias columnas de ``frames.bin`` (``Plan.columns``, en el
mismo orden). Un plan con errores no se descarta entero: lo malo se devuelve en
``Plan.errors`` y lo bueno se graba, para no perder la sesión por una señal mal escrita.

@spec REQ-91
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

TRANSFORM_CHANNELS = ("x", "y", "z", "yaw", "pitch", "roll")
FRAME_CHANNELS = ("dt_ms", "fps")
KINDS = ("transform", "variable", "anim_property", "frame")
RECOMMENDED_MAX_SIGNALS = 30  # medido en P-22: 30 columnas cuestan 0,19 ms por fotograma

DEFAULT_MAX_SECONDS = 600.0  # 10 minutos
DEFAULT_MAX_BYTES = 500 * 1024 * 1024  # 500 MB
DEFAULT_KEEP_SESSIONS = 20

# Presets: listas de señales ya resueltas. ``gameplay_events`` e ``input`` solo
# generan eventos (no columnas), así que no añaden señales por fotograma.
PRESETS: dict[str, dict[str, Any]] = {
    "player": {
        "signals": [
            {"kind": "transform", "target": {"player": 0}, "channels": ["x", "y", "z", "yaw"], "name": "player"},
            {"kind": "variable", "target": {"player": 0}, "name": "speed", "source": "velocity_xy"},
        ],
        "events": [],
    },
    "player_vr": {
        "signals": [
            {"kind": "transform", "target": {"player_camera": 0}, "channels": list(TRANSFORM_CHANNELS), "name": "hmd"},
        ],
        "events": ["grab"],
    },
    "performance": {
        "signals": [{"kind": "frame", "name": "frame", "channels": ["dt_ms", "fps"]}],
        "events": [],
    },
    "input": {"signals": [], "events": ["input"]},
    "gameplay_events": {"signals": [], "events": ["movement_mode", "movement_base", "pause", "log_error"]},
}


@dataclass(frozen=True)
class Limits:
    """Límites de una sesión. Al llegar a uno se deja de grabar y se avisa; el juego no se toca."""

    max_seconds: float = DEFAULT_MAX_SECONDS
    max_bytes: int = DEFAULT_MAX_BYTES
    keep_sessions: int = DEFAULT_KEEP_SESSIONS


@dataclass
class Signal:
    """Una señal con nombre estable: ``name`` es el prefijo de sus columnas."""

    kind: str
    name: str
    target: dict[str, Any] = field(default_factory=dict)
    channels: list[str] = field(default_factory=list)
    extra: dict[str, Any] = field(default_factory=dict)  # socket, source, name de la variable…

    @property
    def columns(self) -> list[str]:
        if self.channels:
            return [f"{self.name}.{c}" for c in self.channels]
        return [self.name]


@dataclass
class Plan:
    signals: list[Signal] = field(default_factory=list)
    events: list[str] = field(default_factory=list)
    limits: Limits = field(default_factory=Limits)
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)

    @property
    def columns(self) -> list[str]:
        return [col for s in self.signals for col in s.columns]


def _check_signal(raw: Any, index: int) -> tuple[str | None, list[str]]:
    """Valida una señal escrita a mano. Devuelve ``(error, canales)``; sin error, los canales ya están elegidos."""
    error: str | None = None
    channels: list[str] = []
    kind = raw.get("kind") if isinstance(raw, dict) else None
    target = (raw.get("target") or {}) if isinstance(raw, dict) else {}
    if not isinstance(raw, dict):
        error = f"señal {index}: debe ser un objeto"
    elif kind not in KINDS:
        error = f"señal {index}: tipo «{kind}» desconocido (válidos: {', '.join(KINDS)})"
    elif kind != "frame" and not isinstance(target, dict):
        error = f"señal {index}: «target» debe ser un objeto"
    elif kind != "frame" and not target:
        error = f"señal {index}: falta «target» (actor_label, actor_class, player o player_camera)"
    elif kind in ("variable", "anim_property") and not raw.get("name"):
        error = f"señal {index}: falta «name» (nombre de la variable o propiedad)"
    elif kind in ("transform", "frame"):
        allowed = TRANSFORM_CHANNELS if kind == "transform" else FRAME_CHANNELS
        channels = list(raw.get("channels") or allowed)
        bad = [c for c in channels if c not in allowed]
        if bad:
            error = f"señal {index}: canales {'de fotograma ' if kind == 'frame' else ''}desconocidos {bad}"
    return error, channels


def _signal_from_dict(raw: Any, index: int) -> tuple[Signal | None, str | None]:
    """Convierte una señal escrita a mano. Devuelve ``(señal, error)``."""
    error, channels = _check_signal(raw, index)
    signal: Signal | None = None
    if error is None:
        kind, target = raw["kind"], raw.get("target") or {}
        default_name = raw.get("alias") or raw.get("name") or _target_label(target) or kind
        extra = {k: v for k, v in raw.items() if k in ("socket", "source", "component", "name") and v}
        signal = Signal(kind, str(default_name), dict(target), channels, extra)
    return signal, error


def _target_label(target: dict[str, Any]) -> str:
    return next((str(target[key]) for key in ("actor_label", "actor_class", "player", "player_camera") if key in target), "")


def parse_plan(raw: Any) -> Plan:
    """Construye un ``Plan`` desde un diccionario. Nunca lanza: lo malo va a ``errors``."""
    plan = Plan()
    if not isinstance(raw, dict):
        plan.errors.append("el plan debe ser un objeto")
        return plan

    for name in raw.get("presets") or []:
        preset = PRESETS.get(name)
        if preset is None:
            plan.errors.append(f"preset «{name}» desconocido (válidos: {', '.join(PRESETS)})")
            continue
        for i, sig in enumerate(preset["signals"]):
            s, _ = _signal_from_dict(sig, i)
            if s is not None:
                plan.signals.append(s)
        for ev in preset["events"]:
            if ev not in plan.events:
                plan.events.append(ev)

    for i, sig in enumerate(raw.get("signals") or []):
        s, err = _signal_from_dict(sig, i)
        if err:
            plan.errors.append(err)
        elif s is not None:
            plan.signals.append(s)

    _dedupe_names(plan)
    plan.limits = _limits_from(raw.get("limits"), plan)
    if len(plan.columns) > RECOMMENDED_MAX_SIGNALS:
        plan.warnings.append(
            f"{len(plan.columns)} columnas: por encima de las {RECOMMENDED_MAX_SIGNALS} recomendadas "
            "(coste medido 0,19 ms por fotograma con 30)"
        )
    return plan


def _dedupe_names(plan: Plan) -> None:
    """Dos señales con el mismo nombre chocarían en las columnas: la segunda recibe un sufijo."""
    seen: dict[str, int] = {}
    for s in plan.signals:
        n = seen.get(s.name, 0)
        seen[s.name] = n + 1
        if n:
            s.name = f"{s.name}_{n + 1}"


def _limits_from(raw: Any, plan: Plan) -> Limits:
    if not isinstance(raw, dict):
        return Limits()
    out: dict[str, Any] = {}
    for key, cast, lo in (("max_seconds", float, 1.0), ("max_bytes", int, 1024), ("keep_sessions", int, 1)):
        if key not in raw:
            continue
        try:
            value = cast(raw[key])
        except (TypeError, ValueError):
            plan.errors.append(f"límite «{key}» no es un número")
            continue
        if value < lo:
            plan.warnings.append(f"límite «{key}» demasiado bajo; se usa {lo}")
            value = cast(lo)
        out[key] = value
    return Limits(**out)
