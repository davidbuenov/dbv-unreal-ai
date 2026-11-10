# =============================================================================
# DBV Unreal AI — Grabadora de sesión: adaptador real de Unreal y arranque
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Une el núcleo de la grabadora con el motor. Todo lo de ``unreal`` vive aquí.

Solo lee: no abre transacciones del editor, no cambia propiedades y no crea
assets (R4). Lo medido en los spikes P-22 y P-23 (2026-11-10) manda:

* El mundo de juego es ``UnrealEditorSubsystem.get_game_world()``; al terminar PIE
  devuelve ``None`` y un mundo retenido lanza ``ObjectInstance is null`` al leerlo.
* Un error dentro de un callback de Slate se traga en silencio, así que el núcleo
  los captura y los cuenta.

@spec REQ-91
"""

from __future__ import annotations

import json
import math
import time
from pathlib import Path
from typing import Any, Callable

from . import recorder as rec_mod
from .plan import Signal, parse_plan

try:
    import unreal

    HAS_UNREAL = True
except ImportError:
    HAS_UNREAL = False
    unreal = None  # type: ignore[assignment]

ROT_CHANNELS = ("yaw", "pitch", "roll")
_UNSET = object()  # «todavía no se ha leído»: distinto de un valor vacío


def _to_float(value: Any) -> float:
    """Número, booleano o enum de Unreal a ``float``. Otra cosa lanza y apaga la señal."""
    if isinstance(value, bool):
        return 1.0 if value else 0.0
    if isinstance(value, (int, float)):
        return float(value)
    inner = getattr(value, "value", None)  # enums de Unreal
    if isinstance(inner, (int, float)):
        return float(inner)
    raise TypeError(f"valor no numérico ({type(value).__name__})")


def _vec_channel(vec: Any, channel: str) -> float:
    return float(getattr(vec, channel))


def _find_actor(world: Any, target: dict[str, Any]) -> Any | None:
    label, cls = target.get("actor_label"), target.get("actor_class")
    if "player" in target:
        found = unreal.GameplayStatics.get_player_pawn(world, int(target["player"]))
    elif "player_camera" in target:
        found = unreal.GameplayStatics.get_player_camera_manager(world, int(target["player_camera"]))
    else:
        found = next(
            (
                a
                for a in unreal.GameplayStatics.get_all_actors_of_class(world, unreal.Actor)
                if (label is not None and a.get_actor_label() == label)
                or (cls is not None and a.get_class().get_name() == cls)
            ),
            None,
        )
    return found


def _get_property(obj: Any, name: str) -> Any:
    """Lee una propiedad por reflexión; prueba el nombre tal cual (con espacios) y en snake_case."""
    try:
        value = obj.get_editor_property(name)
    except Exception:  # noqa: BLE001 - se reintenta con otro formato de nombre
        value = obj.get_editor_property(name.strip().replace(" ", "_").lower())
    return value


class UnrealAdapter:
    """Implementa ``recorder.Adapter`` sobre la API de Python de Unreal."""

    supported_events = frozenset({"pause", "movement_mode", "movement_base"})

    def __init__(self) -> None:
        self._ues = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem)
        self._last: dict[str, Any] = {}
        self._pawn: Any = None

    def game_world(self) -> Any | None:
        return self._ues.get_game_world()

    def world_alive(self, world: Any) -> bool:
        alive = True
        try:
            world.get_path_name()
        except Exception:  # noqa: BLE001 - «ObjectInstance is null» (P-23)
            alive = False
        return alive

    def level_name(self) -> str:
        world = self._ues.get_game_world() or self._ues.get_editor_world()
        name = ""
        try:
            if world is not None:
                name = str(world.get_path_name())
        except Exception:  # noqa: BLE001 - un mundo que ya no existe no tiene nombre
            name = ""
        return name

    def register_tick(self, callback: Callable[[float], None]) -> Any:
        return unreal.register_slate_post_tick_callback(callback)

    def unregister_tick(self, handle: Any) -> None:
        unreal.unregister_slate_post_tick_callback(handle)

    def make_reader(self, world: Any, signal: Signal) -> Callable[[], list[float]] | None:
        actor = _find_actor(world, signal.target)
        if actor is None:
            return None
        if signal.kind == "transform":
            return self._transform_reader(actor, signal)
        if signal.kind == "variable":
            return self._variable_reader(actor, signal)
        if signal.kind == "anim_property":
            return self._anim_reader(actor, signal)
        raise ValueError(f"tipo de señal sin lector: {signal.kind}")

    # ------------------------------------------------------------------ eventos
    def poll_event(self, world: Any, kind: str) -> dict[str, Any] | None:
        """Detecta el cambio de ``pause``, ``movement_mode`` o ``movement_base`` desde la última comprobación."""
        try:
            now = self._state_of(world, kind)
        except Exception:
            self._pawn = None  # el personaje pudo reaparecer: se vuelve a buscar la próxima vez
            raise
        before = self._last.get(kind, _UNSET)
        self._last[kind] = now
        event: dict[str, Any] | None = None
        if before is _UNSET:  # primera lectura: línea base; solo una pausa ya activa merece evento
            if kind == "pause" and now:
                event = {"paused": True}
        elif now != before:
            event = {"paused": bool(now)} if kind == "pause" else {"from": before, "to": now}
        return event

    def _movement(self, world: Any) -> Any:
        """El ``CharacterMovementComponent`` del personaje del jugador (se guarda el personaje entre llamadas)."""
        if self._pawn is None:
            self._pawn = unreal.GameplayStatics.get_player_pawn(world, 0)
        if self._pawn is None:
            raise ValueError("no hay personaje del jugador")
        mv = self._pawn.get_component_by_class(unreal.CharacterMovementComponent)
        if mv is None:
            raise ValueError("el personaje no tiene CharacterMovementComponent")
        return mv

    def _state_of(self, world: Any, kind: str) -> Any:
        if kind == "pause":
            state: Any = bool(unreal.GameplayStatics.is_game_paused(world))
        elif kind == "movement_mode":
            mode = self._movement(world).get_editor_property("movement_mode")
            state = str(getattr(mode, "name", mode))
        else:  # movement_base: componente sobre el que se apoya (una cesta, el suelo…)
            base = self._movement(world).get_movement_base()
            owner = None if base is None else base.get_owner()
            state = "" if base is None else (base.get_name() if owner is None else f"{owner.get_name()}.{base.get_name()}")
        return state

    # ------------------------------------------------------------------ lectores
    @staticmethod
    def _transform_reader(actor: Any, signal: Signal) -> Callable[[], list[float]]:
        socket = signal.extra.get("socket")
        comp = None
        if socket:
            comp = actor.get_component_by_class(unreal.SkeletalMeshComponent) or actor.get_component_by_class(
                unreal.SceneComponent
            )
            if comp is None:
                raise ValueError(f"«{signal.name}» no tiene componente con sockets")

        def read() -> list[float]:
            if comp is not None:
                loc, rot = comp.get_socket_location(socket), comp.get_socket_rotation(socket)
            elif hasattr(actor, "get_camera_location"):  # PlayerCameraManager
                loc, rot = actor.get_camera_location(), actor.get_camera_rotation()
            else:
                loc, rot = actor.get_actor_location(), actor.get_actor_rotation()
            return [_vec_channel(rot if c in ROT_CHANNELS else loc, c) for c in signal.channels]

        return read

    @staticmethod
    def _variable_reader(actor: Any, signal: Signal) -> Callable[[], list[float]]:
        if signal.extra.get("source") == "velocity_xy":
            return lambda: [math.hypot(actor.get_velocity().x, actor.get_velocity().y)]
        name = signal.extra["name"]
        return lambda: [_to_float(_get_property(actor, name))]

    @staticmethod
    def _anim_reader(actor: Any, signal: Signal) -> Callable[[], list[float]]:
        mesh = actor.get_component_by_class(unreal.SkeletalMeshComponent)
        if mesh is None:
            raise ValueError(f"«{signal.name}» no tiene malla esquelética")
        name = signal.extra["name"]
        return lambda: [_to_float(_get_property(mesh.get_anim_instance(), name))]


# ---------------------------------------------------------------------- arranque público
_SESSION: rec_mod.Recorder | None = None


def sessions_dir() -> Path:
    return Path(unreal.Paths.project_saved_dir()).resolve() / "DBVUnrealAI" / "Sessions"


def _playtest_busy() -> bool:
    busy = False
    try:
        from .. import editor  # noqa: PLC0415 - import tardío: evita ciclos al importar el paquete

        busy = (getattr(editor, "_PLAYTEST_STATE", {}) or {}).get("handle") is not None
    except Exception:  # noqa: BLE001 - sin editor.py no hay playtest que excluir
        busy = False
    return busy


def start_recording(plan_json: str, name: str = "") -> str:
    """Arranca una grabación con el plan dado. Devuelve JSON con la carpeta, las columnas y los avisos."""
    global _SESSION  # noqa: PLW0603
    result: dict[str, Any]
    try:
        raw = json.loads(plan_json) if plan_json else {}
    except json.JSONDecodeError as exc:
        raw = None
        result = {"success": False, "error": f"plan no es JSON válido: {exc}"}
    if raw is not None:
        plan = parse_plan(raw)
        sid = time.strftime("%Y%m%d-%H%M%S") + (f"-{name}" if name else "")
        rec = rec_mod.Recorder(plan, sessions_dir() / sid, UnrealAdapter(), busy=_playtest_busy)
        try:
            rec.start()
            _SESSION = rec
            result = {
                "success": True,
                "id": sid,
                "folder": str(rec.folder),
                "columns": plan.columns,
                "errors": plan.errors,
                "warnings": plan.warnings,
                "waiting_for_pie": rec.adapter.game_world() is None,
            }
        except rec_mod.SessionBusyError as exc:
            result = {"success": False, "error": str(exc)}
    return json.dumps(result, ensure_ascii=False)


def status() -> str:
    rec = _SESSION
    if rec is None:
        return json.dumps({"active": False})
    return json.dumps(
        {
            "active": rec.status == "recording",
            "status": rec.status,
            "reason": rec.reason,
            "rows": rec._writer.rows_written if rec._writer else 0,
            "disabled": rec.disabled,
            "warnings": rec.warnings,
            "cost_ms": rec.cost_ms(),
            "folder": str(rec.folder),
        },
        ensure_ascii=False,
    )


def mark(label: str) -> str:
    if _SESSION is None or _SESSION.status != "recording":
        return json.dumps({"success": False, "error": "no hay grabación activa"})
    _SESSION.mark(label)
    return json.dumps({"success": True})


def stop_recording() -> str:
    if _SESSION is None:
        return json.dumps({"success": False, "error": "no hay grabación"})
    _SESSION.finish("stopped")
    return status()
