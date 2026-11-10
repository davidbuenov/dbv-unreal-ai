import os
import struct
import json

glb_path = r"C:\Users\bueno\DBV Unreal AI\Library\chaqueta_pablo_v2-1794225333306\model.glb"
with open(glb_path, "rb") as f:
    f.seek(12)
    chunk_len, chunk_type = struct.unpack("<I4s", f.read(8))
    json_bytes = f.read(chunk_len)
    json_data = json.loads(json_bytes.decode("utf-8"))
    f.seek(12 + 8 + chunk_len)
    bin_len, bin_type = struct.unpack("<I4s", f.read(8))
    bin_data = f.read(bin_len)

# Find positions accessor
mesh = json_data["meshes"][0]
primitive = mesh["primitives"][0]
pos_accessor_idx = primitive["attributes"]["POSITION"]
pos_accessor = json_data["accessors"][pos_accessor_idx]
buffer_view = json_data["bufferViews"][pos_accessor["bufferView"]]
offset = buffer_view.get("byteOffset", 0) + pos_accessor.get("byteOffset", 0)
count = pos_accessor["count"]

positions = []
for i in range(count):
    idx = offset + i * 12
    x, y, z = struct.unpack_from("<fff", bin_data, idx)
    positions.append((x, y, z))

xs = [p[0] for p in positions]
ys = [p[1] for p in positions]
zs = [p[2] for p in positions]

print(f"Total vertices: {count}")
print(f"X range: min={min(xs):.3f}, max={max(xs):.3f}, extent={max(xs)-min(xs):.3f}")
print(f"Y range: min={min(ys):.3f}, max={max(ys):.3f}, extent={max(ys)-min(ys):.3f}")
print(f"Z range: min={min(zs):.3f}, max={max(zs):.3f}, extent={max(zs)-min(zs):.3f}")

# Histogram of Z to see where the waist band is
z_min, z_max = min(zs), max(zs)
print(f"Relative height percentages:")
for pct in range(0, 101, 10):
    val = z_min + (z_max - z_min) * (pct / 100.0)
    print(f"  {pct}% height = {val:.3f}")

