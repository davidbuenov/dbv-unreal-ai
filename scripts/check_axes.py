import struct
import json

glb_path = r"C:\Users\bueno\DBV Unreal AI\Library\chaqueta_pablo_v2-1794225333306\model.glb"
with open(glb_path, "rb") as f:
    f.seek(12)
    chunk_len, _ = struct.unpack("<I4s", f.read(8))
    json_data = json.loads(f.read(chunk_len).decode("utf-8"))
    f.seek(12 + 8 + chunk_len)
    bin_len, _ = struct.unpack("<I4s", f.read(8))
    bin_data = f.read(bin_len)

primitive = json_data["meshes"][0]["primitives"][0]
pos_accessor = json_data["accessors"][primitive["attributes"]["POSITION"]]
buffer_view = json_data["bufferViews"][pos_accessor["bufferView"]]
offset = buffer_view.get("byteOffset", 0) + pos_accessor.get("byteOffset", 0)
count = pos_accessor["count"]

# Sample vertices at extreme values of X, Y, Z
sample_step = max(count // 1000, 1)
samples = []
for i in range(0, count, sample_step):
    idx = offset + i * 12
    x, y, z = struct.unpack_from("<fff", bin_data, idx)
    samples.append((x, y, z))

print("Min/Max samples:")
for axis_idx, axis_name in enumerate(["X", "Y", "Z"]):
    min_pt = min(samples, key=lambda p: p[axis_idx])
    max_pt = max(samples, key=lambda p: p[axis_idx])
    print(f"  {axis_name}: min at {min_pt}, max at {max_pt}")

