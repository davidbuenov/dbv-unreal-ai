# =============================================================================
# DBV Unreal AI — Genera el GLB mínimo de los tests del visor 3D (un cubo)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Escribe ``cubo.glb`` junto a este fichero: 24 vértices, 12 triángulos, un material naranja. @spec REQ-31

Sin dependencias: glTF 2.0 binario a mano (cabecera, trozo JSON, trozo BIN). Se versiona el resultado; este
script solo documenta cómo se hizo y permite rehacerlo.
"""

import json
import struct
from pathlib import Path

FACES = [
    ((1, 0, 0), [(1, -1, -1), (1, 1, -1), (1, 1, 1), (1, -1, 1)]),
    ((-1, 0, 0), [(-1, -1, 1), (-1, 1, 1), (-1, 1, -1), (-1, -1, -1)]),
    ((0, 1, 0), [(-1, 1, -1), (-1, 1, 1), (1, 1, 1), (1, 1, -1)]),
    ((0, -1, 0), [(-1, -1, 1), (-1, -1, -1), (1, -1, -1), (1, -1, 1)]),
    ((0, 0, 1), [(-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1)]),
    ((0, 0, -1), [(1, -1, -1), (-1, -1, -1), (-1, 1, -1), (1, 1, -1)]),
]


def build() -> bytes:
    """El cubo como GLB."""
    positions, normals, indices = [], [], []
    for normal, corners in FACES:
        base = len(positions)
        for corner in corners:
            positions.append([c * 0.5 for c in corner])
            normals.append(list(normal))
        indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    pos_bytes = b"".join(struct.pack("<3f", *p) for p in positions)
    nor_bytes = b"".join(struct.pack("<3f", *n) for n in normals)
    idx_bytes = b"".join(struct.pack("<H", i) for i in indices)
    idx_bytes += b"\x00" * (-len(idx_bytes) % 4)
    binary = pos_bytes + nor_bytes + idx_bytes
    gltf = {
        "asset": {"version": "2.0", "generator": "dbv-unreal-ai tests"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": "Cubo"}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2, "material": 0}]}],
        "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [0.95, 0.45, 0.1, 1], "metallicFactor": 0, "roughnessFactor": 0.6}}],
        "buffers": [{"byteLength": len(binary)}],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": len(pos_bytes), "target": 34962},
            {"buffer": 0, "byteOffset": len(pos_bytes), "byteLength": len(nor_bytes), "target": 34962},
            {"buffer": 0, "byteOffset": len(pos_bytes) + len(nor_bytes), "byteLength": len(indices) * 2, "target": 34963},
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 24, "type": "VEC3", "min": [-0.5, -0.5, -0.5], "max": [0.5, 0.5, 0.5]},
            {"bufferView": 1, "componentType": 5126, "count": 24, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5123, "count": len(indices), "type": "SCALAR"},
        ],
    }
    text = json.dumps(gltf, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    chunks = struct.pack("<I4s", len(text), b"JSON") + text + struct.pack("<I4s", len(binary), b"BIN\x00") + binary
    return struct.pack("<4sII", b"glTF", 2, 12 + len(chunks)) + chunks


if __name__ == "__main__":
    target = Path(__file__).with_name("cubo.glb")
    target.write_bytes(build())
    print(target, target.stat().st_size, "bytes")
