# =============================================================================
# DBV Unreal AI — Ajuste «sastre» de prendas de Tripo sobre MetaHumans
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Ajusta tops y pantalones de Tripo al cuerpo del MetaHuman conservando su malla, UV y material.

Sustituye el ajuste por franjas (REJECTED-13) y la carcasa del cuerpo (REJECTED-15)
por el algoritmo «sastre» (ADR-V050-007), aprobado en ``BP_Pablo_fable``:
orientación automática, exclusión geométrica y por pesos de los brazos, escala
por medidas anatómicas, eliminación de tapas interiores de modelos macizos,
ajuste radial por la mediana de triángulos muestreados, empuje suave por
distancia con signo sobre la malla soldada y transferencia de pesos suavizados.

Ejes: X lateral, +Y frente, Z arriba (cm), como el cuerpo de MetaHuman en su asset.

@spec REQ-81, REQ-98
"""

from __future__ import annotations

import os
import site
import sys
import time
from typing import Any

try:
    import numpy as np

    HAS_NUMPY = True
except ImportError:
    _user_site = site.getusersitepackages()
    if os.path.isdir(_user_site) and _user_site not in sys.path:
        sys.path.append(_user_site)
    try:
        import numpy as np

        HAS_NUMPY = True
    except ImportError:
        np = None  # type: ignore[assignment]
        HAS_NUMPY = False

try:
    import unreal

    HAS_UNREAL = True
except ImportError:
    unreal = None  # type: ignore[assignment]
    HAS_UNREAL = False

from . import fit

NT = 72
STEP = 1.0
ARM_KEYS = ("upperarm", "lowerarm", "hand", "thumb", "index_", "middle_", "ring_", "pinky", "wrist", "palm")
LEG_KEYS = ("thigh", "calf", "foot", "toe")
HEAD_KEYS = ("head", "neck", "facial")

DEFAULT_CLEARANCE_CM = {"top": 1.0, "pants": 0.8}
MIN_CLEARANCE_CM = 0.3
MAX_CLEARANCE_CM = 3.0
TAILOR_KINDS = ("top", "pants")


def resolve_clearance(kind: str, clearance_cm: float | None = None) -> float:
    """Holgura objetivo en cm para ``kind``, acotada a ``[MIN_CLEARANCE_CM, MAX_CLEARANCE_CM]``. @spec REQ-98"""
    wanted = DEFAULT_CLEARANCE_CM.get(kind, 1.0) if clearance_cm is None else float(clearance_cm)
    result = min(MAX_CLEARANCE_CM, max(MIN_CLEARANCE_CM, wanted))
    return result


class Body:
    """Modelo de cuerpo en numpy con normales orientadas hacia fuera y pesos agrupados por región. @spec REQ-98"""

    def __init__(self, data: dict[str, Any]) -> None:
        self.pos = data["pos"].astype(np.float64)
        self.tris = data["tris"].astype(np.int64)
        self.names = [str(n) for n in data["names"]]
        self.wi = data["wi"].astype(np.int64)
        self.ww = data["ww"].astype(np.float64)
        self.bpos = data["bpos"].astype(np.float64)
        a, b, c = (self.pos[self.tris[:, k]] for k in range(3))
        fn = np.cross(b - a, c - a)
        vn = np.zeros_like(self.pos)
        for k in range(3):
            np.add.at(vn, self.tris[:, k], fn)
        self.normal = vn / (np.linalg.norm(vn, axis=1, keepdims=True) + 1e-12)
        # El sentido de los triángulos depende del motor: las normales deben salir del cuerpo.
        radial = self.pos - self.pos.mean(0)
        radial[:, 2] = 0.0
        if float((self.normal * radial).sum()) < 0.0:
            self.normal = -self.normal
        self.arm_w = self.group_weight(ARM_KEYS)
        self.leg_w = self.group_weight(LEG_KEYS)
        self.head_w = self.group_weight(HEAD_KEYS)

    def group_weight(self, keys: tuple[str, ...]) -> Any:
        """Peso acumulado por vértice de los huesos cuyo nombre contiene alguna clave de ``keys``."""
        flag = np.array([any(k in n.lower() for k in keys) for n in self.names], dtype=bool)
        weights = (self.ww * flag[self.wi]).sum(1)
        return weights

    def bone(self, name: str) -> Any:
        """Posición ``(x, y, z)`` del hueso ``name``."""
        pos = self.bpos[self.names.index(name)]
        return pos

    def surface(self, mask: Any) -> tuple[Any, Any, Any]:
        """Puntos, normales y vértice propietario de la piel (vértices y centros de triángulo) donde ``mask`` es verdad."""
        tm = mask[self.tris].all(1)
        tc = self.pos[self.tris[tm]].mean(1)
        tn = self.normal[self.tris[tm]].mean(1)
        tn = tn / (np.linalg.norm(tn, axis=1, keepdims=True) + 1e-12)
        owner = self.tris[tm][:, 0]
        surface_tuple = (
            np.concatenate([self.pos[mask], tc]),
            np.concatenate([self.normal[mask], tn]),
            np.concatenate([np.nonzero(mask)[0], owner]),
        )
        return surface_tuple


class Collider:
    """Superficie contra la que se ajusta la prenda: piel (sin brazos) y prendas interiores ya equipadas. @spec REQ-98"""

    def __init__(self, pts: Any, nrm: Any) -> None:
        self.pts = pts
        self.nrm = nrm

    def signed(self, p: Any) -> tuple[Any, Any, Any]:
        """Distancia con signo, normal y punto más cercano de la superficie a cada punto de ``p``."""
        box_lo, box_hi = p.min(0) - 12.0, p.max(0) + 12.0
        m = ((self.pts > box_lo) & (self.pts < box_hi)).all(1)
        if not bool(m.any()):
            m = np.ones(len(self.pts), dtype=bool)
        ref, nr = self.pts[m], self.nrm[m]
        idx, _ = nearest(p, ref)
        d = ((p - ref[idx]) * nr[idx]).sum(1)
        signed_tuple = (d, nr[idx], np.nonzero(m)[0][idx])
        return signed_tuple

    def hull_grid(self, z_lo: float, z_hi: float, half: float = 1.5) -> tuple[Any, Any]:
        """Envolvente convexa por franja de altura: centros ``(x, y)`` y radios por dirección."""
        nz = int(round((z_hi - z_lo) / STEP)) + 1
        centres = np.zeros((nz, 2))
        radii = np.full((nz, NT), np.nan)
        for k in range(nz):
            z = z_lo + k * STEP
            q = self.pts[np.abs(self.pts[:, 2] - z) < half][:, :2]
            if len(q) >= 6:
                hull = convex_hull(q)
                if len(hull) >= 3:
                    centres[k] = (0.0, float(hull[:, 1].min() + hull[:, 1].max()) / 2.0)
                    radii[k] = hull_radii(hull, centres[k])
        valid = ~np.isnan(radii[:, 0])
        if bool(valid.any()):
            cy = np.interp(np.arange(nz), np.nonzero(valid)[0], centres[valid, 1])
            for _ in range(8):
                cy = np.convolve(np.pad(cy, 1, mode="edge"), [0.25, 0.5, 0.25], mode="valid")
            centres[:, 1] = cy
        for k in range(nz):
            z = z_lo + k * STEP
            q = self.pts[np.abs(self.pts[:, 2] - z) < half][:, :2]
            if len(q) >= 6:
                hull = convex_hull(q)
                if len(hull) >= 3:
                    radii[k] = hull_radii(hull, centres[k])
        grid_tuple = (centres, fill_nan_grid(radii))
        return grid_tuple


def _monotone_half_hull(seq: Any) -> list[Any]:
    """Mitad de la cadena monótona 2D para ``convex_hull``."""
    ch: list[Any] = []
    for p in seq:
        while len(ch) >= 2:
            u = ch[-1] - ch[-2]
            v = p - ch[-2]
            cross_z = float(u[0] * v[1] - u[1] * v[0])
            if cross_z <= 0.0:
                ch.pop()
            else:
                break
        ch.append(p)
    result = ch[:-1]
    return result


def convex_hull(pts: Any) -> Any:
    """Envolvente convexa 2D (cadena monótona) de un conjunto de puntos ``(x, y)``. @spec REQ-98"""
    unique_pts = np.unique(np.round(pts, 3), axis=0)
    hull = unique_pts
    if len(unique_pts) >= 3:
        ordered = unique_pts[np.lexsort((unique_pts[:, 1], unique_pts[:, 0]))]
        hull = np.array(_monotone_half_hull(ordered) + _monotone_half_hull(ordered[::-1]))
    return hull


def hull_radii(hull: Any, centre: Any, nt: int = NT) -> Any:
    """Radio del polígono convexo ``hull`` desde ``centre`` en ``nt`` direcciones. @spec REQ-98"""
    th = np.arange(nt) * 2.0 * np.pi / nt
    d = np.stack([np.cos(th), np.sin(th)], 1)
    a = hull - centre
    b = np.roll(hull, -1, axis=0) - centre
    e = b - a
    out = np.zeros(nt)
    for i in range(nt):
        den = d[i, 0] * e[:, 1] - d[i, 1] * e[:, 0]
        ok = np.abs(den) > 1e-9
        safe_den = np.where(ok, den, 1.0)
        t = np.where(ok, (a[:, 0] * e[:, 1] - a[:, 1] * e[:, 0]) / safe_den, -1.0)
        s = np.where(ok, (a[:, 0] * d[i, 1] - a[:, 1] * d[i, 0]) / safe_den, -1.0)
        good = (t > 0.0) & (s >= -1e-6) & (s <= 1.0 + 1e-6)
        out[i] = float(t[good].max()) if bool(good.any()) else 0.0
    return out


def smooth_grid(g: Any, passes: int = 3, wrap: bool = True) -> Any:
    """Suaviza una rejilla 2D ``[altura, dirección]`` con condición periódica en ángulo."""
    out = g.copy()
    for _ in range(passes):
        up = np.vstack([out[:1], out[:-1]])
        dn = np.vstack([out[1:], out[-1:]])
        lf = np.roll(out, 1, 1) if wrap else out
        rt = np.roll(out, -1, 1) if wrap else out
        out = (2.0 * out + up + dn + lf + rt) / 6.0
    return out


def fill_nan_grid(g: Any) -> Any:
    """Rellena huecos (``NaN``) por difusión desde los vecinos en ángulo y altura."""
    out = g.copy()
    for _ in range(out.shape[0] + out.shape[1]):
        nan = np.isnan(out)
        if not bool(nan.any()):
            break
        stack = np.stack([np.roll(out, 1, 1), np.roll(out, -1, 1), np.vstack([out[:1], out[:-1]]), np.vstack([out[1:], out[-1:]])])
        cnt = (~np.isnan(stack)).sum(0)
        mean = np.nansum(stack, 0) / np.maximum(cnt, 1)
        mask = nan & (cnt > 0)
        out[mask] = mean[mask]
    return out


def sample_grid(g: Any, zf: Any, th: Any) -> Any:
    """Interpolación bilineal de ``g[nivel, dirección]`` con ángulo periódico."""
    nz, nt = g.shape
    zf_c = np.clip(zf, 0.0, nz - 1.0)
    z0 = np.floor(zf_c).astype(int)
    z1 = np.minimum(z0 + 1, nz - 1)
    wz = zf_c - z0
    tf = (th % (2.0 * np.pi)) / (2.0 * np.pi) * nt
    t0 = np.floor(tf).astype(int) % nt
    t1 = (t0 + 1) % nt
    wt = tf - np.floor(tf)
    sampled = (g[z0, t0] * (1.0 - wt) + g[z0, t1] * wt) * (1.0 - wz) + (g[z1, t0] * (1.0 - wt) + g[z1, t1] * wt) * wz
    return sampled


def nearest(points: Any, ref: Any, chunk: int = 400) -> tuple[Any, Any]:
    """Índice y distancia del punto de ``ref`` más cercano a cada punto de ``points`` (por bloques)."""
    idx = np.zeros(len(points), np.int64)
    dist = np.zeros(len(points))
    r2 = (ref**2).sum(1)
    for s in range(0, len(points), chunk):
        p = points[s : s + chunk]
        d2 = (p**2).sum(1)[:, None] - 2.0 * (p @ ref.T) + r2[None, :]
        i = d2.argmin(1)
        idx[s : s + chunk] = i
        dist[s : s + chunk] = np.sqrt(np.maximum(d2[np.arange(len(p)), i], 0.0))
    return (idx, dist)


def weld_map(pos: Any, tol: float = 0.02) -> Any:
    """Agrupa vértices coincidentes en posición (las prendas de Tripo llegan en islas de UV sin soldar)."""
    key = np.round(pos / tol).astype(np.int64)
    _, inv = np.unique(key, axis=0, return_inverse=True)
    result = inv.reshape(-1)
    return result


def adjacency(tris: Any, inv: Any) -> Any:
    """Pares únicos de aristas entre grupos de vértices soldados."""
    t = inv[tris]
    e = np.concatenate([t[:, [0, 1]], t[:, [1, 2]], t[:, [2, 0]]])
    e = e[e[:, 0] != e[:, 1]]
    result = np.unique(np.sort(e, 1), axis=0) if len(e) else np.zeros((0, 2), dtype=np.int64)
    return result


def laplacian_smooth(values: Any, edges: Any, n: int, passes: int, keep: float = 0.5) -> Any:
    """Suaviza un campo definido sobre ``n`` grupos soldados mediante sus aristas vecinas."""
    v = values.copy()
    if len(edges) > 0:
        deg = np.zeros(n)
        np.add.at(deg, edges[:, 0], 1.0)
        np.add.at(deg, edges[:, 1], 1.0)
        deg = np.maximum(deg, 1.0)
        for _ in range(passes):
            acc = np.zeros_like(v)
            np.add.at(acc, edges[:, 0], v[edges[:, 1]])
            np.add.at(acc, edges[:, 1], v[edges[:, 0]])
            denom = deg[:, None] if v.ndim > 1 else deg
            v = keep * v + (1.0 - keep) * acc / denom
    return v


def normalise(pos: Any, kind: str) -> tuple[Any, dict[str, Any]]:
    """Centra la prenda, alinea el ancho en X y orienta el frente hacia ``+Y``. @spec REQ-98"""
    p = pos - (pos.min(0) + pos.max(0)) / 2.0
    ext = p.max(0) - p.min(0)
    rot90 = bool(ext[1] > ext[0])
    info: dict[str, Any] = {"rot90": rot90}
    if rot90:
        p = np.stack([-p[:, 1], p[:, 0], p[:, 2]], 1)
    h = float(p[:, 2].max() - p[:, 2].min())
    w = float(p[:, 0].max() - p[:, 0].min())
    if kind == "top":
        c = np.abs(p[:, 0]) < 0.06 * w
        front_pts = p[c & (p[:, 1] > 0.0), 2]
        back_pts = p[c & (p[:, 1] < 0.0), 2]
        front_top = float(front_pts.max()) if len(front_pts) else 0.0
        back_top = float(back_pts.max()) if len(back_pts) else 0.0
        flip = front_top > back_top
        info["neck_front_back"] = (round(front_top, 1), round(back_top, 1))
    else:
        zt = float(p[:, 2].max())
        waist = p[p[:, 2] > zt - 0.06 * h]
        seat = p[(p[:, 2] < zt - 0.2 * h) & (p[:, 2] > zt - 0.45 * h)]
        bulge_plus = float(seat[:, 1].max() - waist[:, 1].max()) if (len(seat) and len(waist)) else 0.0
        bulge_minus = float(waist[:, 1].min() - seat[:, 1].min()) if (len(seat) and len(waist)) else 0.0
        flip = bulge_plus > bulge_minus
        info["bulge_plus_minus"] = (round(bulge_plus, 1), round(bulge_minus, 1))
    info["flip180"] = bool(flip)
    if flip:
        p = p * np.array([-1.0, -1.0, 1.0])
    return (p, info)


def _eval_centres(z: Any, z_lo: float, step: float, centres: Any) -> Any:
    """Evalúa el centro ``(0, cy)`` interpolado para cada cota ``z``."""
    nz = len(centres)
    zf = np.clip((z - z_lo) / step, 0.0, nz - 1.0)
    cy = np.interp(zf, np.arange(nz), centres[:, 1])
    result = np.stack([np.zeros_like(cy), cy], 1)
    return result


def polar_inner(p: Any, z_lo: float, z_hi: float, centres: Any, step: float = 1.0, nt: int = NT, pct: float = 10.0) -> Any:
    """Radio interior/mediano de la prenda por franja y dirección (``NaN`` donde no hay tela)."""
    nz = int(round((z_hi - z_lo) / step)) + 1
    g = np.full((nz, nt), np.nan)
    zi = np.round((p[:, 2] - z_lo) / step).astype(int)
    ok = (zi >= 0) & (zi < nz)
    c = _eval_centres(p[:, 2], z_lo, step, centres)
    r = np.hypot(p[:, 0] - c[:, 0], p[:, 1] - c[:, 1])
    ti = np.round(np.arctan2(p[:, 1] - c[:, 1], p[:, 0] - c[:, 0]) / (2.0 * np.pi) * nt).astype(int) % nt
    order = np.lexsort((r, ti, zi))
    zi, ti, r, ok = zi[order], ti[order], r[order], ok[order]
    zi, ti, r = zi[ok], ti[ok], r[ok]
    if len(zi) > 0:
        key = zi * nt + ti
        starts = np.r_[0, np.nonzero(np.diff(key))[0] + 1]
        ends = np.r_[starts[1:], len(key)]
        for s, e in zip(starts, ends):
            if e - s >= 2:
                g[zi[s], ti[s]] = r[s + int((e - s - 1) * pct / 100.0)]
    return g


def body_collider(body: Body, with_arms: bool = False) -> Collider:
    """Construye el colisionador del cuerpo excluyendo brazos y manos en prendas sin mangas. @spec REQ-98"""
    shoulder_x = abs(float(body.bone("upperarm_l")[0]))
    outside = np.abs(body.pos[:, 0]) > shoulder_x + 3.0
    mask = np.ones(len(body.pos), dtype=bool) if with_arms else ((body.arm_w < 0.5) & ~outside)
    pts, nrm, _ = body.surface(mask)
    return Collider(pts, nrm)


def garment_collider(pos: Any, tris: Any) -> tuple[Any, Any]:
    """Muestrea una prenda interior ya puesta como obstáculo con normales hacia fuera. @spec REQ-98"""
    a, b, c = (pos[tris[:, k]] for k in range(3))
    fn = np.cross(b - a, c - a)
    fn = fn / (np.linalg.norm(fn, axis=1, keepdims=True) + 1e-12)
    pts_list, nrm_list = [], []
    for w in ((1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0), (0.6, 0.2, 0.2), (0.2, 0.6, 0.2), (0.2, 0.2, 0.6)):
        pts_list.append(a * w[0] + b * w[1] + c * w[2])
        nrm_list.append(fn)
    pts = np.concatenate(pts_list)
    nrm = np.concatenate(nrm_list)
    radial = pts.copy()
    radial[:, 2] = 0.0
    sign = np.sign((nrm * radial).sum(1))
    sign[sign == 0.0] = 1.0
    return (pts, nrm * sign[:, None])


def landmarks_body(body: Body) -> dict[str, float]:
    """Puntos anatómicos clave del cuerpo (entrepierna, axila, pelvis y hombro). @spec REQ-98"""
    torso = body.arm_w < 0.5
    p = body.pos
    pelvis_z = float(body.bone("pelvis")[2])
    shoulder_x = abs(float(body.bone("upperarm_l")[0]))
    legs = p[torso & (p[:, 2] < pelvis_z + 2.0)]
    centre = legs[np.abs(legs[:, 0]) < 1.0]
    crotch = float(centre[:, 2].min()) if len(centre) else pelvis_z - 10.0
    mixed = (body.arm_w > 0.25) & (body.arm_w < 0.75) & (np.abs(p[:, 0]) < shoulder_x + 3.0)
    armpit = float(np.percentile(p[mixed, 2], 2)) if bool(mixed.any()) else pelvis_z + 35.0
    return {"crotch": crotch, "armpit": armpit, "pelvis": pelvis_z, "shoulder_x": shoulder_x}


def shoulder_top(body: Body, x_abs: float) -> float:
    """Altura máxima de la piel del hombro (sin brazos ni cabeza) en la vertical de ``|x|``."""
    m = (body.arm_w < 0.5) & (body.head_w < 0.5) & (np.abs(np.abs(body.pos[:, 0]) - x_abs) < 1.0)
    if not bool(m.any()):
        m = (body.arm_w < 0.5) & (body.head_w < 0.5)
    top_z = float(body.pos[m, 2].max())
    return top_z


def width_depth(pts: Any, z: float, half: float = 1.5) -> tuple[float, float, float]:
    """Ancho X, fondo Y y centro Y de ``pts`` en la franja ``[z - half, z + half]``."""
    q = pts[np.abs(pts[:, 2] - z) < half]
    if len(q) == 0:
        q = pts
    w = float(q[:, 0].max() - q[:, 0].min())
    d = float(q[:, 1].max() - q[:, 1].min())
    yc = float(q[:, 1].max() + q[:, 1].min()) / 2.0
    return (max(w, 1e-3), max(d, 1e-3), yc)


def garment_crotch(p: Any) -> float:
    """Altura de la entrepierna de un pantalón (primera franja con tela en el eje central)."""
    w = float(p[:, 0].max() - p[:, 0].min())
    zs = np.arange(float(p[:, 2].min()) + 2.0, float(p[:, 2].max()), 1.0)
    crotch = float(zs[0]) if len(zs) else float(p[:, 2].min())
    for z in zs:
        q = p[np.abs(p[:, 2] - z) < 0.75]
        if len(q) and float(np.abs(q[:, 0]).min()) < 0.01 * w:
            crotch = float(z)
            break
    return crotch


def garment_armhole(p: Any) -> float:
    """Altura más baja de la sisa de un top (por encima ya no hay tela cerrada en los costados)."""
    zt = float(p[:, 2].max())
    h = zt - float(p[:, 2].min())
    zs = np.arange(zt - 0.75 * h, zt - 0.03 * h, 1.0)
    armhole: float | None = None
    for z in zs:
        q = p[np.abs(p[:, 2] - z) < 0.75]
        if len(q) >= 8:
            depth = float(q[:, 1].max() - q[:, 1].min())
            yc = float(q[:, 1].max() + q[:, 1].min()) / 2.0
            side = q[np.abs(q[:, 1] - yc) < 0.12 * depth]
            if not len(side) or float(np.abs(side[:, 0]).max()) < 0.85 * float(np.abs(q[:, 0]).max()):
                armhole = float(z)
                break
    result = float(armhole if armhole is not None else zt - 0.35 * h)
    return result


def surface_samples(pos: Any, tris: Any, spacing: float = 1.0) -> Any:
    """Puntos repartidos por los triángulos de la prenda para no dejar celdas polares vacías."""
    a, b, c = (pos[tris[:, k]] for k in range(3))
    area = 0.5 * np.linalg.norm(np.cross(b - a, c - a), axis=1)
    n = np.clip(np.ceil(np.sqrt(area) / spacing).astype(int), 1, 8)
    out = [pos]
    for k in np.unique(n):
        m = n == k
        if int(k) == 1:
            out.append((a[m] + b[m] + c[m]) / 3.0)
        else:
            for i in range(int(k) + 1):
                for j in range(int(k) + 1 - i):
                    w0, w1 = i / float(k), j / float(k)
                    out.append(a[m] * w0 + b[m] * w1 + c[m] * (1.0 - w0 - w1))
    return np.concatenate(out)


def drop_inner_caps(
    pos: Any,
    tris: Any,
    col: Collider,
    deep: float = 4.0,
    flat: float = 0.7,
    z_max: float | None = None,
) -> tuple[Any, Any]:
    """Elimina las tapas horizontales interiores de modelos macizos de Tripo. @spec REQ-98"""
    a, b, c = (pos[tris[:, k]] for k in range(3))
    fn = np.cross(b - a, c - a)
    fn = fn / (np.linalg.norm(fn, axis=1, keepdims=True) + 1e-12)
    centres = (a + b + c) / 3.0
    d, _, _ = col.signed(centres)
    cand = (np.abs(fn[:, 2]) > flat) & (d < -0.5)
    if z_max is not None:
        cand = cand & (centres[:, 2] < z_max)
    seed = cand & (d < -deep)
    inv = weld_map(pos)
    t = inv[tris]
    reached = np.zeros(int(inv.max()) + 1, dtype=bool)
    cap = seed.copy()
    for _ in range(200):
        reached[t[cap].ravel()] = True
        grown = cand & reached[t].any(1)
        if int(grown.sum()) == int(cap.sum()):
            break
        cap = grown
    return (tris[~cap], cap)


def band_clamp_polar(
    p: Any,
    tris: Any,
    col: Collider,
    z_lo: float,
    z_hi: float,
    c_min: float,
    c_max_values: Any,
    fade_lo: tuple[float, float] | None = None,
    fade_hi: tuple[float, float] | None = None,
) -> tuple[Any, dict[str, float]]:
    """Lleva el radio mediano de la prenda a la banda ``[c_min, c_max]`` sobre la envolvente convexa. @spec REQ-98"""
    centres, hull = col.hull_grid(z_lo, z_hi)
    rin = polar_inner(surface_samples(p, tris), z_lo, z_hi, centres, STEP, pct=50.0)
    have = ~np.isnan(rin)
    rin_f = smooth_grid(fill_nan_grid(rin), 2)
    cmax = np.asarray(c_max_values, dtype=np.float64)[:, None]
    target = np.clip(rin_f, hull + c_min, hull + cmax)
    k = np.clip(target / np.maximum(rin_f, 1e-3), 0.7, 1.6)
    k = np.where(have, k, np.nan)
    k = smooth_grid(fill_nan_grid(k), 3)
    c = _eval_centres(p[:, 2], z_lo, STEP, centres)
    th = np.arctan2(p[:, 1] - c[:, 1], p[:, 0] - c[:, 0])
    kv = sample_grid(k, (p[:, 2] - z_lo) / STEP, th)
    w = np.ones(len(p))
    if fade_lo is not None:
        w = w * np.clip((p[:, 2] - fade_lo[0]) / (fade_lo[1] - fade_lo[0]), 0.0, 1.0)
    if fade_hi is not None:
        w = w * np.clip((fade_hi[1] - p[:, 2]) / (fade_hi[1] - fade_hi[0]), 0.0, 1.0)
    kv = 1.0 + (kv - 1.0) * w
    out = p.copy()
    out[:, 0] = c[:, 0] + (p[:, 0] - c[:, 0]) * kv
    out[:, 1] = c[:, 1] + (p[:, 1] - c[:, 1]) * kv
    k_min = float(np.nanmin(k)) if not bool(np.isnan(k).all()) else 1.0
    k_max = float(np.nanmax(k)) if not bool(np.isnan(k).all()) else 1.0
    return (out, {"k_min": round(k_min, 3), "k_max": round(k_max, 3)})


def band_clamp_distance(
    p: Any,
    tris: Any,
    col: Collider,
    lo: float,
    hi_limit: float | None = None,
    hi_z_start: float = 0.0,
    passes: int = 6,
    smooth: int = 6,
) -> tuple[Any, Any]:
    """Empuja fuera lo que queda a menos de ``lo`` de la piel y acerca hombros si ``hi_limit`` está activo. @spec REQ-98"""
    inv = weld_map(p)
    n = int(inv.max()) + 1
    edges = adjacency(tris, inv)
    cnt = np.bincount(inv, minlength=n).astype(float)
    out = p.copy()
    for it in range(passes):
        d, nr, _ = col.signed(out)
        if hi_limit is None:
            hi = np.full(len(out), 1e9)
        else:
            t = np.clip((out[:, 2] - hi_z_start) / 6.0, 0.0, 1.0)
            hi = np.where(t > 0.0, hi_limit - t * 4.2, 1e9)
        want = np.clip(d, lo, np.maximum(hi, lo)) - d
        disp = nr * want[:, None]
        g = np.zeros((n, 3))
        np.add.at(g, inv, disp)
        g = g / cnt[:, None]
        if it < passes - 1:
            sm = laplacian_smooth(g, edges, n, smooth, keep=0.4)
            need = np.zeros(n)
            np.maximum.at(need, inv, np.maximum(lo - d, 0.0))
            mag = np.linalg.norm(sm, axis=1)
            gm = np.linalg.norm(g, axis=1)
            short = (need > 0.0) & (mag < gm)
            sm[short] = g[short]
            g = sm
        out = out + g[inv]
    d_final, _, _ = col.signed(out)
    return (out, d_final)


def fit_pants(
    pos: Any,
    tris: Any,
    body: Body,
    col: Collider,
    clearance: float = 0.8,
    drop: float = 2.0,
) -> tuple[Any, Any, dict[str, Any]]:
    """Ajusta unos pantalones o shorts al cuerpo del MetaHuman. @spec REQ-81, REQ-98"""
    p, info = normalise(pos, "pants")
    marks = landmarks_body(body)
    zt_g = float(p[:, 2].max())
    h_g = zt_g - float(p[:, 2].min())
    cz_g = garment_crotch(p)
    rise_g = max(zt_g - cz_g, 1.0)
    wg, dg, _ = width_depth(p, zt_g - 0.04 * h_g, 0.03 * h_g)
    s = 0.5
    waist_z = marks["pelvis"]
    wb, db, yb = 30.0, 20.0, 0.0
    for _ in range(6):
        waist_z = min(marks["crotch"] - drop + rise_g * s, marks["pelvis"] + 12.0)
        wb, db, yb = width_depth(col.pts, waist_z - 2.0)
        s = (wb + 2.0 * clearance) * 1.02 / wg
    sz_rise = (waist_z - (marks["crotch"] - drop)) / max(rise_g * s, 1e-3)
    sy = s * float(np.clip((db / wb) / (dg / wg), 0.85, 1.2))
    q = p * np.array([s, sy, s])
    q[:, 2] -= cz_g * s
    q[:, 2] = np.where(q[:, 2] > 0.0, q[:, 2] * sz_rise, q[:, 2]) + (marks["crotch"] - drop)
    info["rise_squash"] = round(float(sz_rise), 3)
    _, _, yg = width_depth(q, waist_z - 2.0)
    q[:, 1] += yb - yg
    info.update(
        {
            "scale": round(float(s), 3),
            "scale_y": round(float(sy), 3),
            "waist_z": round(float(waist_z), 1),
            "crotch_b": round(marks["crotch"], 1),
            "rise_cm": round(float(rise_g * s), 1),
            "hem_z": round(float(q[:, 2].min()), 1),
        }
    )
    kept_tris, cap_mask = drop_inner_caps(q, tris, col)
    info["cap_mask"] = cap_mask
    z_lo = marks["crotch"] - 4.0
    z_hi = waist_z + 2.0
    nz = int(round((z_hi - z_lo) / STEP)) + 1
    zc = z_lo + np.arange(nz) * STEP
    cmax = 1.2 + np.clip((waist_z - zc) / 6.0, 0.0, 1.0) * 7.0
    q, pinfo = band_clamp_polar(
        q,
        kept_tris,
        col,
        z_lo,
        z_hi,
        clearance,
        cmax,
        fade_lo=(marks["crotch"] - 3.0, marks["crotch"] + 4.0),
    )
    info["polar"] = pinfo
    q, d = band_clamp_distance(q, kept_tris, col, clearance, hi_limit=None)
    info["inside_pct"] = round(float((d < 0.2).mean() * 100.0), 2)
    info["min_cm"] = round(float(d.min()), 2)
    info["median_cm"] = round(float(np.median(d)), 2)
    return (q, kept_tris, info)


def fit_top(
    pos: Any,
    tris: Any,
    body: Body,
    col: Collider,
    clearance: float = 1.0,
    limit_top_length: bool = False,
) -> tuple[Any, Any, dict[str, Any]]:
    """Ajusta un top o camiseta al cuerpo (y sobre el pantalón si forma parte de ``col``). @spec REQ-81, REQ-98"""
    p, info = normalise(pos, "top")
    marks = landmarks_body(body)
    zt_g = float(p[:, 2].max())
    ah_g = garment_armhole(p)
    h_g = max(zt_g - float(p[:, 2].min()), 1.0)
    wg, dg, _ = width_depth(p, ah_g - 0.03 * h_g, 0.02 * h_g)
    chest_z = marks["armpit"] - 3.0
    wb, db, yb = width_depth(col.pts, chest_z)
    s = (wb + 2.0 * clearance) * 1.03 / wg
    sy = s * float(np.clip((db / wb) / (dg / wg), 0.8, 1.2))
    q = p * np.array([s, sy, s])
    top_band = q[q[:, 2] > float(q[:, 2].max()) - 1.5]
    xs = float(np.abs(top_band[:, 0]).mean()) if len(top_band) else marks["shoulder_x"] * 0.6
    q[:, 2] += shoulder_top(body, xs) + clearance - float(q[:, 2].max())
    ah = ah_g * s + (float(q[:, 2].max()) - zt_g * s)
    _, _, yg = width_depth(q, ah - 3.0)
    q[:, 1] += yb - yg
    info.update(
        {
            "scale": round(float(s), 3),
            "scale_y": round(float(sy), 3),
            "strap_x": round(xs, 1),
            "armhole_z": round(float(ah), 1),
            "armpit_b": round(marks["armpit"], 1),
            "hem_z": round(float(q[:, 2].min()), 1),
            "top_z": round(float(q[:, 2].max()), 1),
            "limit_top_length": bool(limit_top_length),
        }
    )
    limit = marks["armpit"] - 1.5
    if ah > limit:
        zt = float(q[:, 2].max())
        f = (zt - limit) / max(zt - ah, 1e-3)
        up = q[:, 2] > ah
        q[up, 2] = zt - (zt - q[up, 2]) * f
        q[~up, 2] -= ah - limit
        ah = limit
        info["armhole_lowered_to"] = round(float(limit), 1)
    kept_tris, cap_mask = drop_inner_caps(q, tris, col, z_max=ah - 3.0)
    info["cap_mask"] = cap_mask
    if limit_top_length:
        hem_limit = marks["crotch"] + 3.0
        z_ref = marks["armpit"]
        hem = float(q[:, 2].min())
        if hem < hem_limit:
            f_len = (z_ref - hem_limit) / max(z_ref - hem, 1e-3)
            low = q[:, 2] < z_ref
            q[low, 2] = z_ref - (z_ref - q[low, 2]) * f_len
            ah = z_ref - (z_ref - ah) * f_len if ah < z_ref else ah
            info["length_squash"] = round(float(f_len), 3)
            info["hem_z"] = round(float(q[:, 2].min()), 1)
    z_lo = float(q[:, 2].min()) - 2.0
    z_hi = ah + 6.0
    nz = int(round((z_hi - z_lo) / STEP)) + 1
    cmax = np.full(nz, 6.0)
    q, pinfo = band_clamp_polar(q, kept_tris, col, z_lo, z_hi, clearance, cmax, fade_hi=(ah - 3.0, ah + 3.0))
    info["polar"] = pinfo
    q, d = band_clamp_distance(q, kept_tris, col, clearance, hi_limit=6.0, hi_z_start=ah - 4.0)
    info["inside_pct"] = round(float((d < 0.2).mean() * 100.0), 2)
    info["min_cm"] = round(float(d.min()), 2)
    info["median_cm"] = round(float(np.median(d)), 2)
    return (q, kept_tris, info)


# ---------------------------------------------------------------------------
# Vestidos de tela (REQ-87): se ajusta el talle, las mangas siguen a los brazos y la falda conserva el vuelo
# ---------------------------------------------------------------------------


def _central_halfwidths(x: Any, gap: float) -> tuple[float, float]:
    """Semianchos izquierdo y derecho del tramo continuo de tela que contiene ``x = 0``."""
    out: list[float] = []
    for side in (-1.0, 1.0):
        r = np.sort(side * x[side * x >= 0.0])
        brk = np.nonzero(np.diff(np.r_[0.0, r]) > gap)[0]
        if len(r) == 0 or (len(brk) > 0 and int(brk[0]) == 0):
            out.append(0.0)
        elif len(brk) == 0:
            out.append(float(r[-1]))
        else:
            out.append(float(r[int(brk[0]) - 1]))
    return (out[0], out[1])


def torso_profile(samples: Any, gap: float = 2.0, step: float = STEP) -> tuple[Any, Any, Any]:
    """Por franja de altura: cota, semiancho del tramo central y si hay tela separada a ambos lados. @spec REQ-87"""
    zs = np.arange(float(samples[:, 2].min()) + step, float(samples[:, 2].max()), step)
    cw = np.full(len(zs), np.nan)
    side = np.zeros(len(zs), dtype=bool)
    for i, z in enumerate(zs):
        q = samples[np.abs(samples[:, 2] - z) < step * 0.75]
        if len(q) >= 8:
            left, right = _central_halfwidths(q[:, 0], gap)
            cw[i] = (left + right) / 2.0
            reach_l, reach_r = float(-q[:, 0].min()), float(q[:, 0].max())
            # Un tramo central de pocos centímetros no es un talle: es una tapa interior poco densa.
            wide_enough = min(left, right) > max(3.0, 0.15 * max(reach_l, reach_r))
            side[i] = wide_enough and reach_l > left + gap and reach_r > right + gap
    return (zs, cw, side)


def _longest_run(flags: Any) -> tuple[int, int]:
    """Índices ``[inicio, fin)`` del tramo continuo más largo de ``flags`` verdaderos."""
    best = (0, 0)
    start = -1
    for i, flag in enumerate(list(flags) + [False]):
        if flag and start < 0:
            start = i
        elif not flag and start >= 0:
            if i - start > best[1] - best[0]:
                best = (start, i)
            start = -1
    return best


def dress_marks(p: Any, tris: Any) -> dict[str, Any]:
    """Sisa, cintura y anchos del talle de un vestido ya orientado; detecta si trae mangas. @spec REQ-87

    Con mangas, bajo la sisa hay tres tramos de tela por franja (manga, talle, manga): la sisa es el
    final de ese tramo. Sin mangas se usa la sisa de un top.
    """
    zs, cw, side = torso_profile(surface_samples(p, tris, 1.0))
    lo, hi = _longest_run(side)
    sleeved = hi - lo >= 5
    hem = float(p[:, 2].min())
    height = float(p[:, 2].max()) - hem
    armpit = float(zs[hi - 1]) + 0.5 if sleeved else garment_armhole(p)
    ok = ~np.isnan(cw)
    widest = float(np.nanmax(cw)) if bool(ok.any()) else 1.0
    smooth = np.convolve(np.pad(np.where(ok, cw, widest), 2, mode="edge"), np.ones(5) / 5.0, mode="valid")
    band = ok & (zs > hem + 0.25 * height) & (zs < armpit - 0.08 * height)
    waist = float(zs[band][int(np.argmin(smooth[band]))]) if bool(band.any()) else armpit - 0.25 * height
    near_chest = ok & (np.abs(zs - (armpit - 3.0)) <= 1.5)
    near_waist = ok & (np.abs(zs - waist) <= 1.5)
    marks = {
        "sleeved": bool(sleeved),
        "armpit": armpit,
        "waist": waist,
        "chest_hw": float(np.median(cw[near_chest])) if bool(near_chest.any()) else widest,
        "waist_hw": float(np.median(cw[near_waist])) if bool(near_waist.any()) else widest,
        "zs": zs,
        "cw": np.where(ok, cw, widest),
    }
    return marks


#: Diferencia mínima (cm) entre el hundimiento de cintura de los dos lados para fiarse de la silueta.
DRESS_SILHOUETTE_MARGIN_CM = 1.5


def dress_back_side(p: Any, tris: Any, garment: dict[str, Any]) -> tuple[float, dict[str, float]]:
    """Lado de la espalda de un vestido por su silueta: ``+1`` (``+Y``), ``-1`` (``-Y``) o ``0`` si no se distingue.

    La espalda es el lado donde la cintura se hunde entre la espalda alta y los glúteos (curva
    lumbar); el delantero cae casi recto. El escote no sirve para un vestido: con volantes en el
    cuello o la espalda baja, el borde delantero sale más alto y el vestido quedaba girado 180°
    (``docs/2026-11-12-dress-metahuman-neckline-orientation-flip.md``).
    @spec REQ-87
    """
    s = surface_samples(p, tris, 1.0)
    core = s[np.abs(s[:, 0]) < 0.8 * max(garment["waist_hw"], 1.0)]
    waist, armpit = garment["waist"], garment["armpit"]
    torso = max(armpit - waist, 4.0)
    seat_hi = waist - 0.8 * torso
    seat_lo = max(waist - 1.8 * torso, float(p[:, 2].min()) + 2.0)
    hollow: dict[str, float] = {}
    for side, key in ((1.0, "plus_y"), (-1.0, "minus_y")):
        y = side * core[:, 1]
        z = core[:, 2]
        at_waist = y[np.abs(z - waist) < 1.5]
        at_chest = y[(z > armpit - 8.0) & (z < armpit - 1.0)]
        at_seat = y[(z > seat_lo) & (z < seat_hi)]
        measurable = len(at_waist) > 0 and len(at_chest) > 0 and len(at_seat) > 0
        hollow[key] = round(float((at_chest.max() + at_seat.max()) / 2.0 - at_waist.max()), 1) if measurable else 0.0
    diff = hollow["plus_y"] - hollow["minus_y"]
    back = 0.0 if abs(diff) < DRESS_SILHOUETTE_MARGIN_CM else (1.0 if diff > 0.0 else -1.0)
    return (back, hollow)


def body_waist(col: Collider, marks: dict[str, float]) -> float:
    """Altura de la cintura del cuerpo: la franja más estrecha del torso entre la pelvis y la axila. @spec REQ-87"""
    zs = np.arange(marks["pelvis"] + 3.0, marks["armpit"] - 8.0, 1.0)
    waist_z = (marks["pelvis"] + marks["armpit"]) / 2.0
    if len(zs) > 0:
        widths = np.array([width_depth(col.pts, float(z))[0] for z in zs])
        waist_z = float(zs[int(np.argmin(widths))])
    return waist_z


def _rotation_between(u: Any, v: Any) -> Any:
    """Matriz del giro mínimo que lleva el vector unitario ``u`` a ``v`` (Rodrigues)."""
    axis = np.cross(u, v)
    s = float(np.linalg.norm(axis))
    rot = np.eye(3)
    if s > 1e-9:
        a = axis / s
        k = np.array([[0.0, -a[2], a[1]], [a[2], 0.0, -a[0]], [-a[1], a[0], 0.0]])
        rot = np.eye(3) + s * k + (1.0 - float(u @ v)) * (k @ k)
    return rot


def fit_sleeve(q: Any, sleeve: Any, body: Body, sign: float, x_root: float, blend: float = 8.0) -> tuple[Any, Any, dict[str, float]]:
    """Lleva una manga a la línea hombro-codo-mano del cuerpo conservando su volumen. @spec REQ-87

    Cada vértice se expresa como un punto del eje de la manga más un desvío; el eje se sustituye por
    los huesos del brazo y el desvío se gira con él. Cerca del hombro se mezcla con la posición
    original para no romper la unión con el talle. ``sign`` es ``+1`` para el lado ``+X`` (izquierdo).
    """
    side = "l" if sign > 0.0 else "r"
    m = sleeve & (sign * q[:, 0] > 0.0)
    out = q.copy()
    weight = np.zeros(len(q))
    info: dict[str, float] = {}
    if int(m.sum()) >= 30:
        pts = q[m]
        centre = pts.mean(0)
        axis = np.linalg.svd(pts - centre, full_matrices=False)[2][0]
        axis = axis if sign * axis[0] > 0.0 else -axis
        root = centre + axis * ((sign * x_root - centre[0]) / (axis[0] if abs(axis[0]) > 1e-6 else 1e-6))
        shoulder, elbow, hand = (body.bone(f"{name}_{side}") for name in ("upperarm", "lowerarm", "hand"))
        upper = (elbow - shoulder) / (np.linalg.norm(elbow - shoulder) + 1e-12)
        lower = (hand - elbow) / (np.linalg.norm(hand - elbow) + 1e-12)
        upper_len = float(np.linalg.norm(elbow - shoulder))
        t = (pts - root) @ axis
        off = (pts - root) - t[:, None] * axis[None, :]
        k = np.clip((t - (upper_len - 5.0)) / 10.0, 0.0, 1.0)[:, None]
        turned = (off @ _rotation_between(axis, upper).T) * (1.0 - k) + (off @ _rotation_between(axis, lower).T) * k
        along = np.clip(t, 0.0, None)[:, None]
        line = np.where(along < upper_len, shoulder[None, :] + upper[None, :] * along, elbow[None, :] + lower[None, :] * (along - upper_len))
        line = line + np.minimum(t, 0.0)[:, None] * upper[None, :]
        tt = np.clip(t / blend, 0.0, 1.0)
        w = tt * tt * (3.0 - 2.0 * tt)
        out[m] = pts * (1.0 - w[:, None]) + (line + turned) * w[:, None]
        weight[m] = w
        info = {
            "turn_deg": round(float(np.degrees(np.arccos(np.clip(float(axis @ upper), -1.0, 1.0)))), 1),
            "length_cm": round(float(t.max()), 1),
        }
    return (out, weight, info)


def segment_hits(a: Any, b: Any, c: Any, p0: Any, p1: Any) -> Any:
    """Triángulos ``(a, b, c)`` que corta el segmento ``p0``-``p1`` (Möller-Trumbore)."""
    d = p1 - p0
    e1, e2 = b - a, c - a
    h = np.cross(d[None, :], e2)
    det = (e1 * h).sum(1)
    ok = np.abs(det) > 1e-12
    inv = np.where(ok, 1.0 / np.where(ok, det, 1.0), 0.0)
    s = p0[None, :] - a
    u = (s * h).sum(1) * inv
    qv = np.cross(s, e1)
    v = (qv @ d) * inv
    t = (e2 * qv).sum(1) * inv
    hits = ok & (u >= 0.0) & (v >= 0.0) & (u + v <= 1.0) & (t >= 0.0) & (t <= 1.0)
    return hits


def _grow_region(cand: Any, seeds: Any, t: Any, groups: int) -> Any:
    """Triángulos candidatos alcanzables desde ``seeds`` a través de vértices soldados compartidos."""
    cur = seeds.copy()
    reached = np.zeros(groups, dtype=bool)
    for _ in range(600):
        reached[t[cur].ravel()] = True
        grown = (cand & reached[t].any(1)) | seeds
        if int(grown.sum()) == int(cur.sum()):
            break
        cur = grown
    return cur


def _planar_region(seed: int, fn: Any, cen: Any, area: Any, t: Any, groups: int, flat: float, slab: float) -> Any:
    """Región plana conexa que contiene el triángulo ``seed``; el plano se reestima con lo ya reunido."""
    normal, origin = fn[seed], cen[seed]
    cur = np.zeros(len(fn), dtype=bool)
    cur[seed] = True
    for _ in range(6):
        cand = (np.abs(fn @ normal) > flat) & (np.abs((cen - origin) @ normal) < slab)
        grown = _grow_region(cand, cur, t, groups)
        stable = int(grown.sum()) == int(cur.sum())
        cur = grown
        weights = area[cur] * np.sign(fn[cur] @ normal)
        normal = (fn[cur] * weights[:, None]).sum(0)
        normal = normal / (np.linalg.norm(normal) + 1e-12)
        origin = (cen[cur] * area[cur][:, None]).sum(0) / (float(area[cur].sum()) + 1e-12)
        if stable:
            break
    return cur


def drop_end_caps(
    pos: Any,
    tris: Any,
    segments: list[tuple[Any, Any, float]],
    col: Collider,
    flat: float = 0.8,
    slab: float = 2.0,
) -> tuple[Any, Any]:
    """Quita las tapas planas que el cuerpo atraviesa: cuello, puños y suelo de la falda. @spec REQ-87

    Tripo entrega el vestido como un sólido cerrado. Una tapa es la región plana conexa que contiene un
    triángulo perforado por un hueso y orientado de cara a él (``segments`` trae, por hueso, el
    alineamiento mínimo entre la normal y el hueso). Las tapas en tejado, de dos planos, se completan
    encadenando el plano vecino si está orientado como tapa y queda dentro del cuerpo. Las paredes de
    la prenda no se tocan: no las perfora ningún hueso ni son planas.
    """
    a, b, c = (pos[tris[:, k]] for k in range(3))
    fn = np.cross(b - a, c - a)
    area = 0.5 * np.linalg.norm(fn, axis=1)
    fn = fn / (np.linalg.norm(fn, axis=1, keepdims=True) + 1e-12)
    cen = (a + b + c) / 3.0
    inv = weld_map(pos)
    t = inv[tris]
    groups = int(inv.max()) + 1
    depth, _, _ = col.signed(cen)
    cap = np.zeros(len(tris), dtype=bool)
    for p0, p1, seed_align in segments:
        axis = (p1 - p0) / (np.linalg.norm(p1 - p0) + 1e-12)
        capish = np.abs(fn @ axis) > seed_align
        pending = [int(i) for i in np.nonzero(segment_hits(a, b, c, p0, p1) & capish)[0]]
        chained = False
        while pending:
            seed = pending.pop()
            region = None if cap[seed] else _planar_region(seed, fn, cen, area, t, groups, flat, slab)
            inside = region is not None and float((depth[region] * area[region]).sum()) < -0.5 * float(area[region].sum())
            if region is not None and (inside or not chained):
                cap |= region
                reached = np.zeros(groups, dtype=bool)
                reached[t[region].ravel()] = True
                pending += [int(i) for i in np.nonzero(capish & ~cap & reached[t].any(1) & (depth < -1.0))[0]]
            chained = True
    return (tris[~cap], cap)


def loose_parts(pos: Any, tris: Any, min_share: float = 0.05) -> Any:
    """Triángulos de piezas sueltas pequeñas, como los muñones del maniquí bajo la falda. @spec REQ-87"""
    t = weld_map(pos)[tris]
    label = np.arange(int(t.max()) + 1) if len(t) else np.zeros(0, dtype=np.int64)
    for _ in range(len(label) + 1):
        new = label.copy()
        lowest = label[t].min(1)
        for k in range(3):
            np.minimum.at(new, t[:, k], lowest)
        new = new[new]
        if bool((new == label).all()):
            break
        label = new
    loose = np.zeros(len(tris), dtype=bool)
    if len(t) > 0:
        a, b, c = (pos[tris[:, k]] for k in range(3))
        area = 0.5 * np.linalg.norm(np.cross(b - a, c - a), axis=1)
        _, idx = np.unique(label[t[:, 0]], return_inverse=True)
        total = np.bincount(idx.reshape(-1), weights=area)
        loose = (total < min_share * float(total.max()))[idx.reshape(-1)]
    return loose


#: Alineamiento mínimo normal-hueso para tomar un triángulo perforado como tapa. El brazo cae en
#: diagonal y roza la pared del talle de un vestido sin mangas (alineamiento ~0,6): se le exige más.
CAP_ALIGN_AXIAL = 0.5
CAP_ALIGN_ARM = 0.75


def _body_segments(body: Body) -> list[tuple[Any, Any, float]]:
    """Huesos que atraviesan las aberturas de un vestido: brazos, piernas y columna hasta la cabeza."""
    segments: list[tuple[Any, Any, float]] = []
    for side in ("l", "r"):
        shoulder, elbow, hand = (body.bone(f"{name}_{side}") for name in ("upperarm", "lowerarm", "hand"))
        thigh, calf, foot = (body.bone(f"{name}_{side}") for name in ("thigh", "calf", "foot"))
        segments += [(shoulder, elbow, CAP_ALIGN_ARM), (elbow, hand + (hand - elbow) * 0.8, CAP_ALIGN_ARM)]
        segments += [(thigh, calf, CAP_ALIGN_AXIAL), (calf, foot, CAP_ALIGN_AXIAL)]
    spine_lo, spine_hi, head = body.bone("spine_01"), body.bone("spine_05"), body.bone("head")
    segments += [(spine_lo, spine_hi, CAP_ALIGN_AXIAL), (spine_hi, head + np.array([0.0, 0.0, 15.0]), CAP_ALIGN_AXIAL)]
    # Entre las piernas: el suelo de la falda puede traer muñones del maniquí justo donde pasan los
    # muslos, y entonces el hueso del muslo no perfora el suelo sino el interior del muñón.
    pelvis = body.bone("pelvis")
    segments.append((pelvis, np.array([pelvis[0], pelvis[1], float(body.bone("foot_l")[2])]), CAP_ALIGN_AXIAL))
    return segments


#: La tela va sujeta hasta 5 cm por encima de la pelvis (sin pasar de la cintura de la prenda) y se
#: suelta del todo a una caída proporcional a la altura de la pelvis: 34 cm en una pelvis de 87 cm.
CLOTH_PIN_ABOVE_PELVIS_CM = 5.0
CLOTH_FREE_DROP_PER_PELVIS = 34.0 / 87.0


def cloth_free_weights(pos: Any, sleeve_w: Any, pin_z: float, free_z: float) -> Any:
    """Peso ``0..1`` de libertad de la tela por vértice: ``0`` sujeto al cuerpo, ``1`` suelto. @spec REQ-87

    Por encima de ``pin_z`` (talle) y en las mangas la tela sigue al cuerpo; baja suavemente hasta
    ``free_z``, desde donde la falda se mueve con toda su ``MaxDistance``.
    """
    t = np.clip((pin_z - pos[:, 2]) / max(pin_z - free_z, 1e-3), 0.0, 1.0)
    free = t * t * (3.0 - 2.0 * t) * (1.0 - np.clip(np.asarray(sleeve_w) * 2.0, 0.0, 1.0))
    return free


def fit_dress(
    pos: Any,
    tris: Any,
    body: Body,
    clearance: float = 1.0,
    flip_orientation: bool | None = None,
) -> tuple[Any, Any, dict[str, Any]]:
    """Ajusta un vestido de Tripo al cuerpo conservando su malla, sus UV y el vuelo de la falda. @spec REQ-87

    El frente se decide por la silueta (``dress_back_side``) y, solo si no se distingue, por el escote
    (``normalise``). ``flip_orientation=True`` gira 180° el resultado, para corregirlo a la vista.

    Escala uniforme por pecho y cintura (la mayor de las dos, para que el cuerpo quepa), sisa a la
    axila, cintura de la prenda a la del cuerpo, mangas a los brazos, tapas del sólido fuera, talle
    ceñido a la banda de holgura y, al final, empuje de todo lo que quede a menos de ``clearance`` de
    la piel. ``info`` lleva ``cap_mask`` (triángulos retirados), ``sleeve_w`` (peso de manga por
    vértice) y las cotas ``pin_z`` / ``free_z`` de la tela.
    """
    p, info = normalise(pos, "top")
    garment = dress_marks(p, tris)
    back, hollow = dress_back_side(p, tris, garment)
    info["front_by"] = "silhouette" if back != 0.0 else "neckline"
    info["waist_hollow"] = hollow
    info["flip_requested"] = bool(flip_orientation)
    # normalise deja el frente en +Y según el escote: se gira si la silueta dice que +Y es la espalda,
    # y otra vez si quien llama pide corregir el resultado.
    if (back > 0.0) != bool(flip_orientation):
        p = p * np.array([-1.0, -1.0, 1.0])
        info["flip180"] = not info["flip180"]
    marks = landmarks_body(body)
    torso = body_collider(body, with_arms=False)
    skin = body_collider(body, with_arms=True)
    chest_w, _, chest_y = width_depth(torso.pts, marks["armpit"] - 3.0)
    waist_b = body_waist(torso, marks)
    waist_w, _, _ = width_depth(torso.pts, waist_b)
    s_chest = (chest_w + 2.0 * clearance) * 1.03 / max(2.0 * garment["chest_hw"], 1e-3)
    s_waist = (waist_w + 2.0 * clearance) * 1.03 / max(2.0 * garment["waist_hw"], 1e-3)
    s = max(s_chest, s_waist)
    q = p * s
    armhole = marks["armpit"] - 1.5
    lift = armhole - garment["armpit"] * s
    q[:, 2] += lift
    waist_g = garment["waist"] * s + lift
    stretch = float(np.clip((armhole - waist_b) / max(armhole - waist_g, 1e-3), 0.8, 1.25))
    mid = (q[:, 2] <= armhole) & (q[:, 2] >= waist_g)
    low = q[:, 2] < waist_g
    q[mid, 2] = armhole - (armhole - q[mid, 2]) * stretch
    q[low, 2] -= (armhole - waist_g) * (stretch - 1.0)
    waist_g = armhole - (armhole - waist_g) * stretch
    x_root = garment["chest_hw"] * s
    sleeve = np.zeros(len(q), dtype=bool)
    if garment["sleeved"]:
        central = np.interp(p[:, 2], garment["zs"], garment["cw"]) * s
        sleeve = np.where(p[:, 2] < garment["armpit"], np.abs(q[:, 0]) > central + 1.0, np.abs(q[:, 0]) > x_root)
    _, _, chest_y_g = width_depth(q[~sleeve], armhole - 3.0)
    q[:, 1] += chest_y - chest_y_g
    sleeve_w = np.zeros(len(q))
    sleeves: dict[str, dict[str, float]] = {}
    if garment["sleeved"]:
        for sign in (1.0, -1.0):
            q, w, sleeve_info = fit_sleeve(q, sleeve, body, sign, x_root)
            sleeve_w = np.maximum(sleeve_w, w)
            sleeves["left" if sign > 0.0 else "right"] = sleeve_info
    _, cap = drop_end_caps(q, tris, _body_segments(body), skin)
    cap_count = int(cap.sum())
    rest = np.nonzero(~cap)[0]
    cap[rest[loose_parts(q, tris[rest])]] = True
    kept = tris[~cap]
    core = kept[(sleeve_w < 0.5)[kept].all(1)]
    z_lo, z_hi = waist_g - 6.0, armhole + 6.0
    levels = int(round((z_hi - z_lo) / STEP)) + 1
    tight, polar = band_clamp_polar(
        q, core, torso, z_lo, z_hi, clearance, np.full(levels, 5.0), fade_lo=(waist_g - 6.0, waist_g), fade_hi=(armhole - 3.0, armhole + 3.0)
    )
    q = q + (tight - q) * (1.0 - np.clip(sleeve_w * 2.0, 0.0, 1.0))[:, None]
    q, d = band_clamp_distance(q, kept, skin if garment["sleeved"] else torso, clearance)
    used = np.zeros(len(q), dtype=bool)
    used[kept.ravel()] = True
    pin_z = min(marks["pelvis"] + CLOTH_PIN_ABOVE_PELVIS_CM, waist_g - 2.0)
    free_z = pin_z - CLOTH_FREE_DROP_PER_PELVIS * marks["pelvis"]
    info.update(
        {
            "scale": round(float(s), 3),
            "scale_chest": round(float(s_chest), 3),
            "scale_waist": round(float(s_waist), 3),
            "sleeved": bool(garment["sleeved"]),
            "sleeves": sleeves,
            "armhole_z": round(float(armhole), 1),
            "waist_z": round(float(waist_g), 1),
            "body_waist_z": round(float(waist_b), 1),
            "waist_stretch": round(stretch, 3),
            "hem_z": round(float(q[used, 2].min()), 1),
            "top_z": round(float(q[used, 2].max()), 1),
            "caps_removed": cap_count,
            "loose_removed": int(cap.sum()) - cap_count,
            "polar": polar,
            "vertices": int(used.sum()),
            "inside_pct": round(float((d[used] < 0.2).mean() * 100.0), 2),
            "min_cm": round(float(d[used].min()), 2),
            "median_cm": round(float(np.median(d[used])), 2),
            "p95_cm": round(float(np.percentile(d[used], 95)), 2),
            "pin_z": round(float(pin_z), 1),
            "free_z": round(float(free_z), 1),
            "cap_mask": cap,
            "sleeve_w": sleeve_w,
        }
    )
    return (q, kept, info)


def skin_weights(body: Body, pos: Any, tris: Any, sleeveless: bool = True) -> tuple[Any, Any]:
    """Pesos de hueso por vértice de la prenda desde la piel más cercana, suavizados sobre la malla soldada. @spec REQ-98"""
    shoulder_x = abs(float(body.bone("upperarm_l")[0]))
    mask = ((body.arm_w < 0.5) & ~(np.abs(body.pos[:, 0]) > shoulder_x + 3.0)) if sleeveless else np.ones(len(body.pos), dtype=bool)
    pts, _, owner = body.surface(mask)
    lo, hi = pos.min(0) - 10.0, pos.max(0) + 10.0
    m = ((pts > lo) & (pts < hi)).all(1)
    if not bool(m.any()):
        m = np.ones(len(pts), dtype=bool)
    idx, _ = nearest(pos, pts[m])
    src = owner[m][idx]
    nb = len(body.names)
    inv = weld_map(pos)
    ng = int(inv.max()) + 1
    dense = np.zeros((ng, nb))
    rows = np.repeat(inv, body.wi.shape[1])
    np.add.at(dense, (rows, body.wi[src].ravel()), body.ww[src].ravel())
    edges = adjacency(tris, inv)
    dense = laplacian_smooth(dense, edges, ng, 8, keep=0.5)
    out = dense[inv]
    k_max = min(8, nb)
    top = np.argsort(-out, axis=1)[:, :k_max]
    w = np.take_along_axis(out, top, axis=1)
    w = w / (w.sum(1, keepdims=True) + 1e-12)
    return (top, w)


def welded_normals(pos: Any, tris: Any, ref: Any) -> Any:
    """Normales suaves a través de las costuras de las islas de UV, con el sentido de las originales. @spec REQ-98"""
    inv = weld_map(pos)
    a, b, c = (pos[tris[:, k]] for k in range(3))
    fn = np.cross(b - a, c - a)
    g = np.zeros((int(inv.max()) + 1, 3))
    for k in range(3):
        np.add.at(g, inv[tris[:, k]], fn)
    n = g[inv]
    n = n / (np.linalg.norm(n, axis=1, keepdims=True) + 1e-12)
    flip = (n * ref).sum(1) < 0.0
    if float(flip.mean()) > 0.5:
        n = -n
    return n


# ---------------------------------------------------------------------------
# Adaptador de Unreal Engine
# ---------------------------------------------------------------------------


def _first(value: Any) -> Any:
    """Desempaqueta tuplas ``(DynamicMesh, ...)`` devueltas por GeometryScript."""
    return value[0] if isinstance(value, tuple) else value


def _dyn_arrays(dyn: Any) -> tuple[Any, Any]:
    """Extrae matrices ``(pos, tris)`` desde un ``DynamicMesh``."""
    queries = unreal.GeometryScript_MeshQueries
    n = int(dyn.get_vertex_count())
    nt = int(dyn.get_triangle_count())
    pos = np.zeros((n, 3))
    for i in range(n):
        p = queries.get_vertex_position(dyn, i)[0]
        pos[i] = (float(p.x), float(p.y), float(p.z))
    tris = np.zeros((nt, 3), dtype=np.int64)
    for t in range(nt):
        idx = queries.get_triangle_indices(dyn, t)[0]
        tris[t] = (int(idx.x), int(idx.y), int(idx.z))
    return (pos, tris)


def body_model(full_skm: Any) -> tuple[Body, Any]:
    """Construye ``(Body, body_dyn)`` a partir del ``SkeletalMesh`` de cuerpo completo."""
    au = unreal.GeometryScript_AssetUtils
    bw = unreal.GeometryScript_BoneWeights
    dyn = unreal.DynamicMesh()
    dyn = au.copy_mesh_from_skeletal_mesh(
        full_skm,
        dyn,
        unreal.GeometryScriptCopyMeshFromAssetOptions(),
        unreal.GeometryScriptMeshReadLOD(),
    )[0]
    pos, tris = _dyn_arrays(dyn)
    info = bw.get_all_bones_info(dyn)[1]
    names = [str(b.name) for b in info]
    bpos = np.array(
        [(float(b.world_transform.translation.x), float(b.world_transform.translation.y), float(b.world_transform.translation.z)) for b in info]
    )
    k_max = 12
    wi = np.zeros((len(pos), k_max), dtype=np.int64)
    ww = np.zeros((len(pos), k_max))
    for i in range(len(pos)):
        for k, w in enumerate(list(bw.get_vertex_bone_weights(dyn, i)[1])[:k_max]):
            wi[i, k] = int(w.bone_index)
            ww[i, k] = float(w.weight)
    body = Body({"pos": pos, "tris": tris, "names": np.array(names), "wi": wi, "ww": ww, "bpos": bpos})
    return (body, dyn)


def garment_model(garment_sm: Any) -> tuple[Any, Any, Any, Any]:
    """Copia un ``StaticMesh`` de prenda a ``DynamicMesh`` y extrae ``(dyn, pos, tris, normals)``."""
    au = unreal.GeometryScript_AssetUtils
    dyn = unreal.DynamicMesh()
    dyn = au.copy_mesh_from_static_mesh_v2(
        garment_sm,
        dyn,
        unreal.GeometryScriptCopyMeshFromAssetOptions(),
        unreal.GeometryScriptMeshReadLOD(),
        True,
    )[0]
    pos, tris = _dyn_arrays(dyn)
    nl = unreal.GeometryScript_Normals.get_mesh_per_vertex_normals(dyn)[1]
    nrm = np.array([(float(v.x), float(v.y), float(v.z)) for v in unreal.GeometryScript_List.convert_vector_list_to_array(nl)])
    return (dyn, pos, tris, nrm)


def _skeletal_mesh_arrays(skm: Any) -> tuple[Any, Any] | None:
    """Extrae ``(pos, tris)`` de un ``SkeletalMesh`` ya equipado (p. ej. pantalón bajo el top)."""
    arrays: tuple[Any, Any] | None = None
    if skm is not None and hasattr(unreal, "GeometryScript_AssetUtils"):
        dyn = unreal.DynamicMesh()
        dyn = unreal.GeometryScript_AssetUtils.copy_mesh_from_skeletal_mesh(
            skm,
            dyn,
            unreal.GeometryScriptCopyMeshFromAssetOptions(),
            unreal.GeometryScriptMeshReadLOD(),
        )[0]
        if int(dyn.get_vertex_count()) > 0 and int(dyn.get_triangle_count()) > 0:
            arrays = _dyn_arrays(dyn)
    return arrays


def build_tailored_garment(
    full_skm: Any,
    garment_sm: Any,
    kind: str,
    folder: str,
    base_name: str,
    clearance_cm: float | None = None,
    limit_top_length: bool = False,
    under_skm: Any = None,
) -> dict[str, Any]:
    """Ajusta una prenda de Tripo con el método «sastre» y crea su ``SkeletalMesh``. @spec REQ-81, REQ-98"""
    result: dict[str, Any] = {"success": False, "method": "tailor", "garment_kind": kind, "warnings": []}
    skeleton = full_skm.get_editor_property("skeleton") if full_skm is not None and hasattr(full_skm, "get_editor_property") else None
    if kind not in TAILOR_KINDS:
        result["error"] = f"El ajuste de sastre solo admite top y pants, no «{kind}»."
    elif not HAS_NUMPY:
        result["error"] = "numpy no está disponible en el entorno Python de Unreal Engine (activa PythonFoundationPackages)."
    elif skeleton is None:
        result["error"] = "El cuerpo completo no tiene esqueleto."
    else:
        t0 = time.time()
        bw = unreal.GeometryScript_BoneWeights
        au = unreal.GeometryScript_AssetUtils
        eal = unreal.EditorAssetLibrary
        body, body_dyn = body_model(full_skm)
        col = body_collider(body)
        under_arrays = _skeletal_mesh_arrays(under_skm) if kind == "top" else None
        if under_arrays is not None:
            gp, gn = garment_collider(under_arrays[0], under_arrays[1])
            col = Collider(np.concatenate([col.pts, gp]), np.concatenate([col.nrm, gn]))
        dyn, pos, tris, nrm0 = garment_model(garment_sm)
        offset = resolve_clearance(kind, clearance_cm)
        if kind == "pants":
            new_pos, kept, info = fit_pants(pos, tris, body, col, clearance=offset)
        else:
            new_pos, kept, info = fit_top(pos, tris, body, col, clearance=offset, limit_top_length=limit_top_length)
        cap = info.pop("cap_mask")
        info["caps_removed"] = int(cap.sum())
        ref = nrm0.copy()
        if info["rot90"]:
            ref = np.stack([-ref[:, 1], ref[:, 0], ref[:, 2]], 1)
        if info["flip180"]:
            ref = ref * np.array([-1.0, -1.0, 1.0])
        normals = welded_normals(new_pos, kept, ref)
        wi, ww = skin_weights(body, new_pos, kept, sleeveless=True)
        vec = unreal.GeometryScript_List.convert_array_to_vector_list([unreal.Vector(float(p[0]), float(p[1]), float(p[2])) for p in new_pos])
        dyn = _first(unreal.GeometryScript_MeshEdits.set_all_mesh_vertex_positions(dyn, vec))
        nvec = unreal.GeometryScript_List.convert_array_to_vector_list([unreal.Vector(float(p[0]), float(p[1]), float(p[2])) for p in normals])
        dyn = _first(unreal.GeometryScript_Normals.set_per_vertex_normals(dyn))
        dyn = _first(unreal.GeometryScript_Normals.set_mesh_per_vertex_normals(dyn, nvec))
        if bool(cap.any()):
            ids = unreal.GeometryScript_List.convert_array_to_index_list(
                [int(i) for i in np.nonzero(cap)[0]],
                unreal.GeometryScriptIndexType.TRIANGLE,
            )
            dyn = _first(unreal.GeometryScript_MeshEdits.delete_triangles_from_mesh(dyn, ids))
        dyn = _first(bw.copy_bones_from_mesh(body_dyn, dyn, unreal.GeometryScriptCopyBonesFromMeshOptions()))
        dyn = _first(bw.mesh_create_bone_weights(dyn, True))
        for i in range(len(new_pos)):
            arr = [unreal.GeometryScriptBoneWeight(int(b), float(w)) for b, w in zip(wi[i], ww[i]) if w > 1e-4]
            bw.set_vertex_bone_weights(dyn, i, arr)
        if not eal.does_directory_exist(folder):
            eal.make_directory(folder)
        clean_name = base_name if base_name.startswith("SK_") else f"SK_{base_name}"
        dest = f"{folder}/{clean_name}"
        skm = (
            unreal.load_asset(dest)
            if eal.does_asset_exist(dest)
            else unreal.AssetToolsHelpers.get_asset_tools().duplicate_asset(clean_name, folder, full_skm)
        )
        material = None
        if hasattr(garment_sm, "get_material"):
            try:
                material = garment_sm.get_material(0)
            except Exception:
                material = None
        if material is None:
            static_mats = garment_sm.get_editor_property("static_materials") if hasattr(garment_sm, "get_editor_property") else getattr(garment_sm, "static_materials", None)
            material = static_mats[0].material_interface if static_mats and hasattr(static_mats[0], "material_interface") else None
        if skm is None:
            result["error"] = f"GeometryScript no pudo crear la malla esquelética en {dest}."
        else:
            opt = unreal.GeometryScriptCopyMeshToAssetOptions()
            opt.replace_materials = material is not None
            if material is not None:
                opt.new_materials = [material]
            opt.enable_recompute_normals = False
            opt.enable_recompute_tangents = True
            au.copy_mesh_to_skeletal_mesh(dyn, skm, opt, unreal.GeometryScriptMeshWriteLOD())
            if fit is not None:
                fit.apply_two_sided_materials(skm)
                fit.setup_garment_lods(skm, target_lod_count=4)
            info["asset"] = dest
            info["seconds"] = round(time.time() - t0, 1)
            result.update(
                {
                    "success": True,
                    "skeletal_mesh": skm,
                    "material": material,
                    "offset_cm": offset,
                    "placement": info,
                    "zone": info,
                    "fit_info": info,
                    "layered_over_legs": under_arrays is not None,
                }
            )
    return result


# ---------------------------------------------------------------------------
# Adaptador de Unreal Engine para prendas de tela
# ---------------------------------------------------------------------------

CLOTH_KINDS = ("dress",)

#: Recorrido máximo de la falda suelta respecto al cuerpo animado (cm).
CLOTH_MAX_DISTANCE_CM = 9.0


def _write_static_mesh(dyn: Any, path: str, material: Any) -> Any:
    """Guarda ``dyn`` como ``StaticMesh`` en ``path``; si ya existe se sobrescribe sin borrarlo.

    Un reintento no borra el asset: el Dataflow de la prenda lo referencia y borrarlo abre un diálogo.
    """
    eal = unreal.EditorAssetLibrary
    if eal.does_asset_exist(path):
        mesh = eal.load_asset(path)
        opts = unreal.GeometryScriptCopyMeshToAssetOptions()
        opts.enable_recompute_normals = False
        opts.enable_recompute_tangents = True
        unreal.GeometryScript_AssetUtils.copy_mesh_to_static_mesh(dyn, mesh, opts, unreal.GeometryScriptMeshWriteLOD())
    else:
        new_opts = unreal.GeometryScriptCreateNewStaticMeshAssetOptions()
        new_opts.enable_collision = False
        new_opts.enable_recompute_normals = False
        new_opts.enable_recompute_tangents = True
        mesh = _first(unreal.GeometryScript_NewAssetUtils.create_new_static_mesh_asset_from_mesh(dyn, path, new_opts))
    if mesh is not None and material is not None and hasattr(mesh, "set_material"):
        mesh.set_material(0, material)
    if mesh is not None:
        eal.save_asset(path)
    return mesh


def build_cloth_garment(
    full_skm: Any,
    garment_sm: Any,
    kind: str,
    folder: str,
    base_name: str,
    clearance_cm: float | None = None,
    flip_orientation: bool | None = None,
) -> dict[str, Any]:
    """Ajusta una prenda de tela al cuerpo y crea sus mallas de render y de simulación. @spec REQ-87

    La malla de render conserva vértices, UV y material de Tripo. La de simulación es la misma malla
    soldada (las costuras de UV rasgarían la tela). ``max_distance_weights`` va en el orden de
    vértices de la malla de simulación guardada, que es el que usa el Dataflow.
    """
    result: dict[str, Any] = {"success": False, "method": "tailor_cloth", "garment_kind": kind, "warnings": []}
    if kind not in CLOTH_KINDS:
        result["error"] = f"El ajuste de tela solo admite vestidos (dress), no «{kind}»."
    elif not HAS_NUMPY:
        result["error"] = "numpy no está disponible en el entorno Python de Unreal Engine (activa PythonFoundationPackages)."
    elif full_skm is None or garment_sm is None:
        result["error"] = "Faltan el cuerpo completo del MetaHuman o la malla de la prenda."
    else:
        t0 = time.time()
        eal = unreal.EditorAssetLibrary
        body, _ = body_model(full_skm)
        dyn, pos, tris, nrm0 = garment_model(garment_sm)
        new_pos, kept, info = fit_dress(pos, tris, body, clearance=resolve_clearance("top", clearance_cm), flip_orientation=flip_orientation)
        cap = info.pop("cap_mask")
        sleeve_w = info.pop("sleeve_w")
        ref = np.stack([-nrm0[:, 1], nrm0[:, 0], nrm0[:, 2]], 1) if info["rot90"] else nrm0.copy()
        ref = ref * np.array([-1.0, -1.0, 1.0]) if info["flip180"] else ref
        normals = welded_normals(new_pos, kept, ref)
        to_list = unreal.GeometryScript_List.convert_array_to_vector_list
        vec = to_list([unreal.Vector(float(p[0]), float(p[1]), float(p[2])) for p in new_pos])
        nvec = to_list([unreal.Vector(float(n[0]), float(n[1]), float(n[2])) for n in normals])
        dyn = _first(unreal.GeometryScript_MeshEdits.set_all_mesh_vertex_positions(dyn, vec))
        dyn = _first(unreal.GeometryScript_Normals.set_per_vertex_normals(dyn))
        dyn = _first(unreal.GeometryScript_Normals.set_mesh_per_vertex_normals(dyn, nvec))
        if bool(cap.any()):
            ids = unreal.GeometryScript_List.convert_array_to_index_list(
                [int(i) for i in np.nonzero(cap)[0]],
                unreal.GeometryScriptIndexType.TRIANGLE,
            )
            dyn = _first(unreal.GeometryScript_MeshEdits.delete_triangles_from_mesh(dyn, ids))
        dyn = _first(unreal.GeometryScript_MeshRepair.compact_mesh(dyn))
        if not eal.does_directory_exist(folder):
            eal.make_directory(folder)
        static_mats = getattr(garment_sm, "static_materials", None)
        material = static_mats[0].material_interface if static_mats else None
        render_path = f"{folder.rstrip('/')}/SM_{base_name}_Render"
        sim_path = f"{folder.rstrip('/')}/SM_{base_name}_Sim"
        render_sm = _write_static_mesh(dyn, render_path, material)
        dyn = _first(unreal.GeometryScript_MeshRepair.weld_mesh_edges(dyn, unreal.GeometryScriptWeldEdgesOptions()))
        dyn = _first(unreal.GeometryScript_MeshRepair.compact_mesh(dyn))
        sim_sm = _write_static_mesh(dyn, sim_path, None)
        if render_sm is None or sim_sm is None:
            result["error"] = f"GeometryScript no pudo crear las mallas de la prenda en {folder}."
        else:
            fit.apply_two_sided_materials(render_sm)
            _, sim_pos, sim_tris, _ = garment_model(sim_sm)
            used = np.zeros(len(new_pos), dtype=bool)
            used[kept.ravel()] = True
            src = np.nonzero(used)[0]
            near, _ = nearest(sim_pos, new_pos[src])
            weights = cloth_free_weights(sim_pos, sleeve_w[src][near], info["pin_z"], info["free_z"])
            info["sim_vertices"] = int(len(sim_pos))
            info["sim_triangles"] = int(len(sim_tris))
            info["pinned_pct"] = round(float((weights < 0.02).mean() * 100.0), 1)
            info["seconds"] = round(time.time() - t0, 1)
            result.update(
                {
                    "success": True,
                    "render_sm_path": render_path,
                    "sim_sm_path": sim_path,
                    "material_path": material.get_path_name().split(".")[0] if material is not None else None,
                    "max_distance_weights": [round(float(w), 4) for w in weights],
                    "max_distance_cm": CLOTH_MAX_DISTANCE_CM,
                    "fit_info": info,
                }
            )
    return result
