# =============================================================================
# DBV Unreal AI — Tests del formato frames.bin de la caja negra
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Tests de ``dbv_unreal_ai.session.frames``: ida y vuelta, bloque cortado y bloque corrupto.

@spec REQ-91
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "unreal-plugin" / "DBVUnrealAI" / "Content" / "Python"))

from dbv_unreal_ai.session import frames


def _write(path: Path, n_rows: int, per_block: int) -> int:
    w = frames.FrameWriter(path, n_cols=3, rows_per_block=per_block)
    for i in range(n_rows):
        w.append(i / 72.0, [float(i), float(i) * 2, frames.NAN if i % 2 else 1.5])
    w.close()
    return w.rows_written


class FramesTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.path = Path(self._tmp.name) / "frames.bin"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_round_trip_in_several_blocks(self) -> None:
        self.assertEqual(_write(self.path, 10, 4), 10)  # 4 + 4 + 2
        blocks = list(frames.read_blocks(self.path))
        self.assertEqual([len(t) for t, _ in blocks], [4, 4, 2])
        times = [t for ts, _ in blocks for t in ts]
        col1 = [v for _, cols in blocks for v in cols[1]]
        self.assertAlmostEqual(times[9], 9 / 72.0)
        self.assertEqual(col1, [float(i) * 2 for i in range(10)])

    def test_nan_means_missing_signal(self) -> None:
        _write(self.path, 4, 4)
        ((_, cols),) = frames.read_blocks(self.path)
        self.assertEqual([frames.is_missing(v) for v in cols[2]], [False, True, False, True])

    def test_cut_last_block_is_discarded_but_earlier_ones_survive(self) -> None:
        _write(self.path, 10, 4)
        data = self.path.read_bytes()
        self.path.write_bytes(data[:-7])  # el editor se cerró a mitad del último bloque
        self.assertEqual([len(t) for t, _ in frames.read_blocks(self.path)], [4, 4])

    def test_corrupt_block_stops_reading(self) -> None:
        _write(self.path, 8, 4)
        data = bytearray(self.path.read_bytes())
        data[frames.HEADER.size + 3] ^= 0xFF  # un byte roto en la carga del primer bloque
        self.path.write_bytes(bytes(data))
        self.assertEqual(list(frames.read_blocks(self.path)), [])

    def test_wrong_column_count_is_rejected(self) -> None:
        w = frames.FrameWriter(self.path, n_cols=2)
        with self.assertRaises(ValueError):
            w.append(0.0, [1.0])
        w.close()

    def test_close_is_idempotent_and_writes_pending_rows(self) -> None:
        w = frames.FrameWriter(self.path, n_cols=1, rows_per_block=100)
        w.append(0.0, [3.0])
        w.close()
        w.close()
        ((times, cols),) = frames.read_blocks(self.path)
        self.assertEqual((len(times), cols[0][0]), (1, 3.0))


if __name__ == "__main__":
    unittest.main()
