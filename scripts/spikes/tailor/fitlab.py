"""Prototipo del ajuste: conserva la malla de Tripo y la deforma con campos suaves.

Ejes: X lateral, +Y frente, Z arriba (cm), como el cuerpo de MetaHuman en su asset.
"""
import numpy as np

NT = 72  # direcciones por franja
ARM_KEYS = ("upperarm", "lowerarm", "hand", "thumb", "index_", "middle_", "ring_", "pinky", "wrist", "palm")
LEG_KEYS = ("thigh", "calf", "foot", "toe")
HEAD_KEYS = ("head", "neck", "facial")


# ----------------------------------------------------------------------------- cuerpo
class Body:
    def __init__(self, d):
        self.pos = d["pos"].astype(np.float64)
        self.tris = d["tris"].astype(np.int64)
        self.names = [str(n) for n in d["names"]]
        self.wi, self.ww = d["wi"].astype(np.int64), d["ww"].astype(np.float64)
        self.bpos = d["bpos"].astype(np.float64)
        a, b, c = (self.pos[self.tris[:, k]] for k in range(3))
        fn = np.cross(b - a, c - a)
        vn = np.zeros_like(self.pos)
        for k in range(3):
            np.add.at(vn, self.tris[:, k], fn)
        self.normal = vn / (np.linalg.norm(vn, axis=1, keepdims=True) + 1e-12)
        # El sentido de los triángulos depende del motor: las normales deben salir del cuerpo.
        radial = self.pos - self.pos.mean(0)
        radial[:, 2] = 0
        if (self.normal * radial).sum() < 0:
            self.normal = -self.normal
        self.arm_w = self.group_weight(ARM_KEYS)
        self.leg_w = self.group_weight(LEG_KEYS)
        self.head_w = self.group_weight(HEAD_KEYS)

    def group_weight(self, keys):
        flag = np.array([any(k in n.lower() for k in keys) for n in self.names])
        return (self.ww * flag[self.wi]).sum(1)

    def bone(self, name):
        return self.bpos[self.names.index(name)]

    def surface(self, mask):
        """Puntos y normales de la piel (vértices y centros de triángulo) donde mask es verdad."""
        tm = mask[self.tris].all(1)
        tc = self.pos[self.tris[tm]].mean(1)
        tn = self.normal[self.tris[tm]].mean(1)
        tn /= np.linalg.norm(tn, axis=1, keepdims=True) + 1e-12
        owner = self.tris[tm][:, 0]
        return (np.concatenate([self.pos[mask], tc]), np.concatenate([self.normal[mask], tn]), np.concatenate([np.nonzero(mask)[0], owner]))


# ----------------------------------------------------------------------------- utilidades
def convex_hull(pts):
    pts = np.unique(np.round(pts, 3), axis=0)
    if len(pts) < 3:
        return pts
    pts = pts[np.lexsort((pts[:, 1], pts[:, 0]))]

    def half(seq):
        ch = []
        for p in seq:
            while len(ch) >= 2 and np.cross(ch[-1] - ch[-2], p - ch[-2]) <= 0:
                ch.pop()
            ch.append(p)
        return ch[:-1]

    return np.array(half(pts) + half(pts[::-1]))


def hull_radii(hull, centre, nt=NT):
    """Radio del polígono convexo desde centre en nt direcciones."""
    th = np.arange(nt) * 2 * np.pi / nt
    d = np.stack([np.cos(th), np.sin(th)], 1)
    a = hull - centre
    b = np.roll(hull, -1, axis=0) - centre
    e = b - a
    out = np.zeros(nt)
    for i in range(nt):
        den = d[i, 0] * e[:, 1] - d[i, 1] * e[:, 0]
        ok = np.abs(den) > 1e-9
        t = np.where(ok, (a[:, 0] * e[:, 1] - a[:, 1] * e[:, 0]) / np.where(ok, den, 1), -1)
        s = np.where(ok, (a[:, 0] * d[i, 1] - a[:, 1] * d[i, 0]) / np.where(ok, den, 1), -1)
        good = (t > 0) & (s >= -1e-6) & (s <= 1 + 1e-6)
        out[i] = t[good].max() if good.any() else 0.0
    return out


def smooth_grid(g, passes=3, wrap=True):
    g = g.copy()
    for _ in range(passes):
        up = np.vstack([g[:1], g[:-1]])
        dn = np.vstack([g[1:], g[-1:]])
        lf = np.roll(g, 1, 1) if wrap else g
        rt = np.roll(g, -1, 1) if wrap else g
        g = (2 * g + up + dn + lf + rt) / 6.0
    return g


def fill_nan_grid(g):
    """Rellena huecos (NaN) por vecinos: primero a lo largo del ángulo, luego en altura."""
    g = g.copy()
    for _ in range(g.shape[0] + g.shape[1]):
        nan = np.isnan(g)
        if not nan.any():
            break
        stack = np.stack([np.roll(g, 1, 1), np.roll(g, -1, 1), np.vstack([g[:1], g[:-1]]), np.vstack([g[1:], g[-1:]])])
        cnt = (~np.isnan(stack)).sum(0)
        mean = np.nansum(stack, 0) / np.maximum(cnt, 1)
        g[nan & (cnt > 0)] = mean[nan & (cnt > 0)]
    return g


def sample_grid(g, zf, th):
    """Interpolación bilineal de g[nivel, dirección] con ángulo periódico."""
    nz, nt = g.shape
    zf = np.clip(zf, 0, nz - 1)
    z0 = np.floor(zf).astype(int)
    z1 = np.minimum(z0 + 1, nz - 1)
    wz = zf - z0
    tf = (th % (2 * np.pi)) / (2 * np.pi) * nt
    t0 = np.floor(tf).astype(int) % nt
    t1 = (t0 + 1) % nt
    wt = tf - np.floor(tf)
    return (g[z0, t0] * (1 - wt) + g[z0, t1] * wt) * (1 - wz) + (g[z1, t0] * (1 - wt) + g[z1, t1] * wt) * wz


def nearest(points, ref, chunk=400):
    """Índice y distancia del punto de ref más cercano a cada punto (fuerza bruta por bloques)."""
    idx = np.zeros(len(points), np.int64)
    dist = np.zeros(len(points))
    r2 = (ref**2).sum(1)
    for s in range(0, len(points), chunk):
        p = points[s : s + chunk]
        d2 = (p**2).sum(1)[:, None] - 2 * p @ ref.T + r2[None, :]
        i = d2.argmin(1)
        idx[s : s + chunk] = i
        dist[s : s + chunk] = np.sqrt(np.maximum(d2[np.arange(len(p)), i], 0))
    return idx, dist


def weld_map(pos, tol=0.02):
    """Agrupa vértices coincidentes (las prendas de Tripo llegan en islas sin soldar)."""
    key = np.round(pos / tol).astype(np.int64)
    _, inv = np.unique(key, axis=0, return_inverse=True)
    return inv.reshape(-1)


def adjacency(tris, inv):
    """Pares de vecinos entre grupos soldados."""
    t = inv[tris]
    e = np.concatenate([t[:, [0, 1]], t[:, [1, 2]], t[:, [2, 0]]])
    e = e[e[:, 0] != e[:, 1]]
    e = np.unique(np.sort(e, 1), axis=0)
    return e


def laplacian_smooth(values, edges, n, passes, keep=0.5):
    """Suaviza un valor por grupo soldado con sus vecinos."""
    v = values.copy()
    deg = np.zeros(n)
    np.add.at(deg, edges[:, 0], 1)
    np.add.at(deg, edges[:, 1], 1)
    deg = np.maximum(deg, 1)
    for _ in range(passes):
        acc = np.zeros_like(v)
        np.add.at(acc, edges[:, 0], v[edges[:, 1]])
        np.add.at(acc, edges[:, 1], v[edges[:, 0]])
        v = keep * v + (1 - keep) * acc / (deg[:, None] if v.ndim > 1 else deg)
    return v


# ----------------------------------------------------------------------------- prenda
def normalise(pos, kind):
    """Centra, pone el ancho en X y el frente en +Y. Devuelve (pos, informe)."""
    p = pos - (pos.min(0) + pos.max(0)) / 2
    ext = p.max(0) - p.min(0)
    info = {"rot90": bool(ext[1] > ext[0])}
    if info["rot90"]:
        p = np.stack([-p[:, 1], p[:, 0], p[:, 2]], 1)
    h = p[:, 2].max() - p[:, 2].min()
    w = p[:, 0].max() - p[:, 0].min()
    if kind == "top":
        # el escote baja más por delante
        c = np.abs(p[:, 0]) < 0.06 * w
        front_top = p[c & (p[:, 1] > 0), 2].max()
        back_top = p[c & (p[:, 1] < 0), 2].max()
        flip = front_top > back_top
        info["neck_front_back"] = (round(float(front_top), 1), round(float(back_top), 1))
    else:
        # el trasero abulta más que el frente respecto a la cinturilla
        zt = p[:, 2].max()
        waist = p[p[:, 2] > zt - 0.06 * h]
        seat = p[(p[:, 2] < zt - 0.2 * h) & (p[:, 2] > zt - 0.45 * h)]
        bulge_plus = seat[:, 1].max() - waist[:, 1].max()
        bulge_minus = waist[:, 1].min() - seat[:, 1].min()
        flip = bulge_plus > bulge_minus
        info["bulge_plus_minus"] = (round(float(bulge_plus), 1), round(float(bulge_minus), 1))
    info["flip180"] = bool(flip)
    if flip:
        p = p * np.array([-1.0, -1.0, 1.0])
    return p, info


def slice_stats(p, z, half):
    m = np.abs(p[:, 2] - z) < half
    q = p[m]
    if len(q) < 8:
        return None
    return {"x0": q[:, 0].min(), "x1": q[:, 0].max(), "y0": q[:, 1].min(), "y1": q[:, 1].max(), "n": len(q), "pts": q}


def polar_inner(p, centre_fn, z_lo, z_hi, step=1.0, nt=NT, pct=10):
    """Radio interior de la prenda por franja y dirección (NaN donde no hay tela)."""
    nz = int(round((z_hi - z_lo) / step)) + 1
    g = np.full((nz, nt), np.nan)
    zi = np.round((p[:, 2] - z_lo) / step).astype(int)
    ok = (zi >= 0) & (zi < nz)
    c = centre_fn(p[:, 2])
    r = np.hypot(p[:, 0] - c[:, 0], p[:, 1] - c[:, 1])
    ti = np.round(np.arctan2(p[:, 1] - c[:, 1], p[:, 0] - c[:, 0]) / (2 * np.pi) * nt).astype(int) % nt
    order = np.lexsort((r, ti, zi))
    zi, ti, r, ok = zi[order], ti[order], r[order], ok[order]
    zi, ti, r = zi[ok], ti[ok], r[ok]
    key = zi * nt + ti
    starts = np.r_[0, np.nonzero(np.diff(key))[0] + 1]
    ends = np.r_[starts[1:], len(key)]
    for s, e in zip(starts, ends):
        if e - s >= 2:
            g[zi[s], ti[s]] = r[s + int((e - s - 1) * pct / 100.0)]
    return g
