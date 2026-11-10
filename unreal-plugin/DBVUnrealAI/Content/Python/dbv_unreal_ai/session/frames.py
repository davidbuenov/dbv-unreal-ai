# =============================================================================
# DBV Unreal AI — Formato binario de frames.bin (bloques autocontenidos)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Escritor y lector de ``frames.bin``: señales por fotograma, por bloques.

Solo biblioteca estándar (sin ``numpy``, D68). El lector de Rust de S107 sigue
este mismo contrato.

Formato (little endian), sin cabecera de fichero: una sucesión de bloques.

    bloque   = cabecera (20 bytes) + carga
    cabecera = magic ``b"DBVB"`` (4) · filas u32 · columnas u32 · bytes de carga u32 · crc32 de la carga u32
    carga    = tiempos f64 × filas (segundos desde el inicio) ·
               columna 0 f32 × filas · … · columna N-1 f32 × filas   (columnar)

El nombre de cada columna vive en ``meta.json`` (mismo orden). Una señal
apagada o sin dato vale ``NaN``. Cada bloque se valida con su CRC, así que un
bloque a medias (el editor se cerró de golpe, R6) se descarta y los anteriores
se leen igual.

@spec REQ-91
"""

from __future__ import annotations

import math
import os
import struct
import zlib
from pathlib import Path
from typing import BinaryIO, Iterator, Sequence

MAGIC = b"DBVB"
HEADER = struct.Struct("<4sIIII")
NAN = float("nan")


def encode_block(times: Sequence[float], rows: Sequence[Sequence[float]], n_cols: int) -> bytes:
    """Codifica un bloque. ``rows[i]`` son los ``n_cols`` valores del fotograma ``i``."""
    n = len(times)
    if len(rows) != n:
        raise ValueError("times y rows deben tener la misma longitud")
    payload = bytearray(struct.pack(f"<{n}d", *times))
    for c in range(n_cols):
        payload += struct.pack(f"<{n}f", *(row[c] for row in rows))
    body = bytes(payload)
    return HEADER.pack(MAGIC, n, n_cols, len(body), zlib.crc32(body)) + body


class FrameWriter:
    """Acumula fotogramas y escribe un bloque cada ``rows_per_block`` (por defecto ~1 s a 72 FPS)."""

    def __init__(self, path: Path | str, n_cols: int, rows_per_block: int = 72) -> None:
        self.path = Path(path)
        self.n_cols = n_cols
        self.rows_per_block = max(1, rows_per_block)
        self.bytes_written = 0
        self.rows_written = 0
        self._times: list[float] = []
        self._rows: list[Sequence[float]] = []
        self._fh: BinaryIO | None = open(self.path, "ab")  # noqa: SIM115 - se cierra en close()

    def append(self, t: float, values: Sequence[float]) -> None:
        if self._fh is None:
            raise ValueError("el escritor está cerrado")
        if len(values) != self.n_cols:
            raise ValueError(f"se esperaban {self.n_cols} valores y llegaron {len(values)}")
        self._times.append(float(t))
        self._rows.append(values)
        if len(self._times) >= self.rows_per_block:
            self.flush()

    def flush(self) -> None:
        """Escribe el bloque pendiente y lo fuerza a disco."""
        if self._fh is None or not self._times:
            return
        data = encode_block(self._times, self._rows, self.n_cols)
        self._fh.write(data)
        self._fh.flush()
        os.fsync(self._fh.fileno())
        self.bytes_written += len(data)
        self.rows_written += len(self._times)
        self._times.clear()
        self._rows.clear()

    def close(self) -> None:
        """Escribe lo pendiente y cierra. Se puede llamar más de una vez."""
        if self._fh is None:
            return
        try:
            self.flush()
        finally:
            self._fh.close()
            self._fh = None


def read_blocks(path: Path | str) -> Iterator[tuple[list[float], list[list[float]]]]:
    """Lee los bloques válidos: ``(tiempos, columnas)``. Se detiene en el primero cortado o corrupto."""
    with open(path, "rb") as fh:
        while True:
            head = fh.read(HEADER.size)
            if len(head) < HEADER.size:
                return
            magic, n, n_cols, size, crc = HEADER.unpack(head)
            if magic != MAGIC or size != n * 8 + n_cols * n * 4:
                return
            body = fh.read(size)
            if len(body) < size or zlib.crc32(body) != crc:
                return
            times = list(struct.unpack_from(f"<{n}d", body, 0))
            cols = [list(struct.unpack_from(f"<{n}f", body, n * 8 + c * n * 4)) for c in range(n_cols)]
            yield times, cols


def is_missing(value: float) -> bool:
    """``True`` si la señal no tuvo dato en ese fotograma."""
    return math.isnan(value)
