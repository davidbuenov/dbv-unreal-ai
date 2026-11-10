"""Banco de pruebas fuera del editor: carga de GLB y cuerpo, y un visor ortográfico con numpy."""
import json
import struct

import numpy as np
from PIL import Image

SP = __import__("pathlib").Path(__file__).resolve().parent.as_posix()
LIB = "C:/Users/bueno/DBV Unreal AI/Library"
TOP = LIB + "/camiseta_de_tirantas-1794237554702/model.glb"
SHORTS = LIB + "/pantalones_cortos_de_correr-1794237617957/model.glb"


def load_glb(path):
    """Vértices en el espacio de Unreal (cm, Z arriba): glTF (x, y, z) -> (x, z, y) * 100."""
    b = open(path, "rb").read()
    jl = struct.unpack("<I", b[12:16])[0]
    j = json.loads(b[20 : 20 + jl])
    bin0 = 20 + jl + 8

    def acc(i, comps, dtype):
        a = j["accessors"][i]
        bv = j["bufferViews"][a["bufferView"]]
        off = bin0 + bv.get("byteOffset", 0) + a.get("byteOffset", 0)
        return np.frombuffer(b, dtype=dtype, count=a["count"] * comps, offset=off).reshape(-1, comps)

    pr = j["meshes"][0]["primitives"][0]
    v = acc(pr["attributes"]["POSITION"], 3, "<f4").astype(np.float64)
    ia = j["accessors"][pr["indices"]]
    dt = {5123: "<u2", 5125: "<u4"}[ia["componentType"]]
    t = acc(pr["indices"], 1, dt).reshape(-1, 3).astype(np.int64)
    pos = np.stack([v[:, 0], v[:, 2], v[:, 1]], 1) * 100.0
    return pos, t


def load_body():
    d = np.load(SP + "/pablo_body.npz")
    return {k: d[k] for k in d.files}


def render(meshes, view="front", size=700, box=None, path=None):
    """meshes: lista de (pos, tris, color). Vistas: front (mira desde +Y), back, side, top."""
    allp = np.concatenate([m[0] for m in meshes])
    if box is None:
        lo, hi = allp.min(0), allp.max(0)
    else:
        lo, hi = np.array(box[0], float), np.array(box[1], float)
    img = np.full((size, size, 3), 235, np.uint8)
    zbuf = np.full((size, size), -1e9)
    light = np.array([0.3, 0.5, 0.8])
    light /= np.linalg.norm(light)
    for pos, tris, color in meshes:
        if view == "front":
            u, v, d = -pos[:, 0], pos[:, 2], pos[:, 1]
            ulo, uhi, vlo, vhi = -hi[0], -lo[0], lo[2], hi[2]
        elif view == "back":
            u, v, d = pos[:, 0], pos[:, 2], -pos[:, 1]
            ulo, uhi, vlo, vhi = lo[0], hi[0], lo[2], hi[2]
        elif view == "side":
            u, v, d = pos[:, 1], pos[:, 2], pos[:, 0]
            ulo, uhi, vlo, vhi = lo[1], hi[1], lo[2], hi[2]
        else:
            u, v, d = pos[:, 0], pos[:, 1], pos[:, 2]
            ulo, uhi, vlo, vhi = lo[0], hi[0], lo[1], hi[1]
        span = max(uhi - ulo, vhi - vlo) * 1.05
        cu, cv = (ulo + uhi) / 2, (vlo + vhi) / 2
        P = np.stack([(u - cu) / span + 0.5, (v - cv) / span + 0.5, d], 1)
        a, b, c = P[tris[:, 0]], P[tris[:, 1]], P[tris[:, 2]]
        n = np.cross(b - a, c - a) * np.array([span, span, 1.0])
        n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-12
        shade = 0.35 + 0.65 * np.abs(n @ light)
        # muestras baricéntricas según el tamaño en píxeles
        area = np.abs(np.cross((b - a)[:, :2], (c - a)[:, :2])) * size * size
        k = int(np.clip(np.sqrt(np.percentile(area, 95)) * 1.6 + 2, 3, 14))
        ws = [(i / k, jj / k, 1 - i / k - jj / k) for i in range(k + 1) for jj in range(k + 1 - i)]
        for w0, w1, w2 in ws:
            p = a * w0 + b * w1 + c * w2
            px = np.clip((p[:, 0] * size).astype(int), 0, size - 1)
            py = np.clip(((1 - p[:, 1]) * size).astype(int), 0, size - 1)
            order = np.argsort(p[:, 2])
            px, py, dz, sh = px[order], py[order], p[order, 2], shade[order]
            better = dz > zbuf[py, px]
            px, py, dz, sh = px[better], py[better], dz[better], sh[better]
            zbuf[py, px] = dz
            img[py, px] = (np.array(color)[None, :] * sh[:, None]).astype(np.uint8)
    im = Image.fromarray(img)
    if path:
        im.save(path)
    return im


def sheet(meshes, path, box=None, views=("front", "side", "back"), size=520):
    ims = [render(meshes, v, size, box) for v in views]
    out = Image.new("RGB", (size * len(ims), size), (255, 255, 255))
    for i, im in enumerate(ims):
        out.paste(im, (i * size, 0))
    out.save(path)
    return path
