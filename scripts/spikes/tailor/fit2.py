"""Ajuste v2: prenda de Tripo -> cuerpo, conservando la malla de la prenda."""
import numpy as np

from fitlab import (NT, adjacency, convex_hull, fill_nan_grid, hull_radii, laplacian_smooth, nearest, normalise, polar_inner,
                    sample_grid, smooth_grid, weld_map)

STEP = 1.0


class Collider:
    """Superficie contra la que se ajusta: piel (sin brazos) y prendas ya puestas debajo."""

    def __init__(self, pts, nrm):
        self.pts, self.nrm = pts, nrm

    def signed(self, p):
        box_lo, box_hi = p.min(0) - 12, p.max(0) + 12
        m = ((self.pts > box_lo) & (self.pts < box_hi)).all(1)
        ref, nr = self.pts[m], self.nrm[m]
        i, _ = nearest(p, ref)
        d = ((p - ref[i]) * nr[i]).sum(1)
        return d, nr[i], np.nonzero(m)[0][i]

    def hull_grid(self, z_lo, z_hi, half=1.5):
        """Envolvente convexa por franja: centro y radios por dirección."""
        nz = int(round((z_hi - z_lo) / STEP)) + 1
        centres = np.zeros((nz, 2))
        radii = np.full((nz, NT), np.nan)
        for k in range(nz):
            z = z_lo + k * STEP
            q = self.pts[np.abs(self.pts[:, 2] - z) < half][:, :2]
            if len(q) >= 6:
                hull = convex_hull(q)
                if len(hull) >= 3:
                    centres[k] = (0.0, (hull[:, 1].min() + hull[:, 1].max()) / 2)
                    radii[k] = hull_radii(hull, centres[k])
        # centro suave en altura
        valid = ~np.isnan(radii[:, 0])
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
        return centres, fill_nan_grid(radii)


def body_collider(B, with_arms=False):
    # En pose A, antebrazos y manos quedan por fuera de la línea del hombro: fuera también por
    # geometría, porque hay vértices de la mano con peso en «root» o en correctivos sin nombre de brazo.
    outside = np.abs(B.pos[:, 0]) > abs(B.bone("upperarm_l")[0]) + 3.0
    mask = np.ones(len(B.pos), bool) if with_arms else ((B.arm_w < 0.5) & ~outside)
    # los dedos de la mano cuelgan cerca de la cadera en pose A: fuera siempre que no haya brazos
    pts, nrm, _ = B.surface(mask)
    return Collider(pts, nrm)


def garment_collider(pos, tris):
    """Una prenda ya puesta como obstáculo: puntos densos y normales que se alejan del cuerpo."""
    a, b, c = (pos[tris[:, k]] for k in range(3))
    fn = np.cross(b - a, c - a)
    fn /= np.linalg.norm(fn, axis=1, keepdims=True) + 1e-12
    pts, nrm = [], []
    for w in ((1 / 3, 1 / 3, 1 / 3), (0.6, 0.2, 0.2), (0.2, 0.6, 0.2), (0.2, 0.2, 0.6)):
        pts.append(a * w[0] + b * w[1] + c * w[2])
        nrm.append(fn)
    pts, nrm = np.concatenate(pts), np.concatenate(nrm)
    radial = pts.copy()
    radial[:, 2] = 0
    sign = np.sign((nrm * radial).sum(1))
    sign[sign == 0] = 1
    return pts, nrm * sign[:, None]


def landmarks_body(B):
    torso = (B.arm_w < 0.5)
    P = B.pos
    legs = P[torso & (P[:, 2] < B.bone("pelvis")[2] + 2)]
    centre = legs[np.abs(legs[:, 0]) < 1.0]
    crotch = centre[:, 2].min()
    # axila: lo más bajo de la frontera brazo/torso
    mixed = (B.arm_w > 0.25) & (B.arm_w < 0.75) & (np.abs(P[:, 0]) < abs(B.bone("upperarm_l")[0]) + 3)
    armpit = np.percentile(P[mixed, 2], 2)
    return {"crotch": float(crotch), "armpit": float(armpit), "pelvis": float(B.bone("pelvis")[2]), "shoulder_x": float(abs(B.bone("upperarm_l")[0]))}


def shoulder_top(B, x_abs):
    """Altura de la piel (sin brazos ni cabeza) en la vertical de |x|."""
    m = (B.arm_w < 0.5) & (B.head_w < 0.5) & (np.abs(np.abs(B.pos[:, 0]) - x_abs) < 1.0)
    return float(B.pos[m, 2].max())


def width_depth(pts, z, half=1.5):
    q = pts[np.abs(pts[:, 2] - z) < half]
    return (q[:, 0].max() - q[:, 0].min(), q[:, 1].max() - q[:, 1].min(), (q[:, 1].max() + q[:, 1].min()) / 2)


def garment_crotch(p):
    w = p[:, 0].max() - p[:, 0].min()
    zs = np.arange(p[:, 2].min() + 2, p[:, 2].max(), 1.0)
    crotch = zs[0]
    for z in zs:
        q = p[np.abs(p[:, 2] - z) < 0.75]
        if len(q) and np.abs(q[:, 0]).min() < 0.01 * w:
            crotch = z
            break
    return float(crotch)


def garment_armhole(p):
    """Altura más baja de la sisa: por encima no hay tela en los costados."""
    zt = p[:, 2].max()
    h = zt - p[:, 2].min()
    zs = np.arange(zt - 0.75 * h, zt - 0.03 * h, 1.0)
    armhole = None
    for z in zs:
        q = p[np.abs(p[:, 2] - z) < 0.75]
        if len(q) < 8:
            continue
        depth = q[:, 1].max() - q[:, 1].min()
        yc = (q[:, 1].max() + q[:, 1].min()) / 2
        side = q[np.abs(q[:, 1] - yc) < 0.12 * depth]
        # costado abierto: ya no hay tela en el extremo lateral, a media profundidad
        if not len(side) or np.abs(side[:, 0]).max() < 0.85 * np.abs(q[:, 0]).max():
            armhole = z
            break
    return float(armhole if armhole is not None else zt - 0.35 * h)


def surface_samples(pos, tris, spacing=1.0):
    """Puntos repartidos por los triángulos (los de Tripo son grandes: solo vértices deja huecos)."""
    a, b, c = (pos[tris[:, k]] for k in range(3))
    area = 0.5 * np.linalg.norm(np.cross(b - a, c - a), axis=1)
    n = np.clip(np.ceil(np.sqrt(area) / spacing).astype(int), 1, 8)
    out = [pos]
    for k in np.unique(n):
        m = n == k
        if k == 1:
            out.append((a[m] + b[m] + c[m]) / 3)
            continue
        for i in range(k + 1):
            for j in range(k + 1 - i):
                w0, w1 = i / k, j / k
                out.append(a[m] * w0 + b[m] * w1 + c[m] * (1 - w0 - w1))
    return np.concatenate(out)


def drop_inner_caps(pos, tris, col, deep=4.0, flat=0.7, z_max=None):
    """Quita las tapas que Tripo pone dentro de la prenda (modelo macizo).

    Una tapa es un grupo conexo de triángulos casi horizontales metidos en el cuerpo, con al
    menos uno muy dentro (cruza el tronco). Un tirante sobre el hombro también es horizontal,
    pero nunca llega tan dentro, así que no se toca.
    """
    a, b, c = (pos[tris[:, k]] for k in range(3))
    fn = np.cross(b - a, c - a)
    fn /= np.linalg.norm(fn, axis=1, keepdims=True) + 1e-12
    d, _, _ = col.signed((a + b + c) / 3)
    cand = (np.abs(fn[:, 2]) > flat) & (d < -0.5)
    if z_max is not None:
        # por encima de la sisa la tela de los hombros es horizontal y aún no está asentada
        cand &= ((a + b + c) / 3)[:, 2] < z_max
    seed = cand & (d < -deep)
    inv = weld_map(pos)
    t = inv[tris]
    reached = np.zeros(inv.max() + 1, bool)
    cap = seed.copy()
    for _ in range(200):
        reached[t[cap].ravel()] = True
        grown = cand & reached[t].any(1)
        if grown.sum() == cap.sum():
            break
        cap = grown
    return tris[~cap], cap


def band_clamp_polar(p, tris, col, z_lo, z_hi, c_min, c_max_fn, fade_lo=None, fade_hi=None):
    """Lleva el radio interior de la prenda a la banda [c_min, c_max] sobre la envolvente."""
    centres, hull = col.hull_grid(z_lo, z_hi)
    nz = hull.shape[0]

    def centre_fn(z):
        zf = np.clip((z - z_lo) / STEP, 0, nz - 1)
        cy = np.interp(zf, np.arange(nz), centres[:, 1])
        return np.stack([np.zeros_like(cy), cy], 1)

    # Mediana, no mínimo: forros, bolsillos y la tela de la entrepierna tienen radios pequeños
    # y con el mínimo disparaban el factor (picos de 15 cm en la primera prueba).
    rin = polar_inner(surface_samples(p, tris), centre_fn, z_lo, z_hi, STEP, pct=50)
    have = ~np.isnan(rin)
    rin_f = smooth_grid(fill_nan_grid(rin), 2)
    zc = z_lo + np.arange(nz) * STEP
    cmax = np.array([c_max_fn(z) for z in zc])[:, None]
    target = np.clip(rin_f, hull + c_min, hull + cmax)
    k = np.clip(target / np.maximum(rin_f, 1e-3), 0.7, 1.6)
    k = np.where(have, k, np.nan)
    k = smooth_grid(fill_nan_grid(k), 3)
    c = centre_fn(p[:, 2])
    th = np.arctan2(p[:, 1] - c[:, 1], p[:, 0] - c[:, 0])
    kv = sample_grid(k, (p[:, 2] - z_lo) / STEP, th)
    w = np.ones(len(p))
    if fade_lo is not None:
        w *= np.clip((p[:, 2] - fade_lo[0]) / (fade_lo[1] - fade_lo[0]), 0, 1)
    if fade_hi is not None:
        w *= np.clip((fade_hi[1] - p[:, 2]) / (fade_hi[1] - fade_hi[0]), 0, 1)
    kv = 1 + (kv - 1) * w
    out = p.copy()
    out[:, 0] = c[:, 0] + (p[:, 0] - c[:, 0]) * kv
    out[:, 1] = c[:, 1] + (p[:, 1] - c[:, 1]) * kv
    return out, {"k_min": float(np.nanmin(k)), "k_max": float(np.nanmax(k))}


def band_clamp_distance(p, tris, col, lo, hi_fn, passes=6, smooth=6):
    """Empuja fuera lo que está a menos de `lo` de la superficie y acerca lo que pasa de `hi`.

    El desplazamiento se suaviza sobre la malla soldada para no arrugar la prenda.
    """
    inv = weld_map(p)
    n = inv.max() + 1
    edges = adjacency(tris, inv)
    cnt = np.bincount(inv, minlength=n).astype(float)
    out = p.copy()
    for it in range(passes):
        d, nr, _ = col.signed(out)
        hi = hi_fn(out)
        want = np.clip(d, lo, np.maximum(hi, lo)) - d
        disp = nr * want[:, None]
        g = np.zeros((n, 3))
        np.add.at(g, inv, disp)
        g /= cnt[:, None]
        if it < passes - 1:
            sm = laplacian_smooth(g, edges, n, smooth, keep=0.4)
            # nunca empujar menos de lo que pide salir de la piel
            need = np.zeros(n)
            np.maximum.at(need, inv, np.maximum(lo - d, 0))
            mag = np.linalg.norm(sm, axis=1)
            gm = np.linalg.norm(g, axis=1)
            short = (need > 0) & (mag < gm)
            sm[short] = g[short]
            g = sm
        out = out + g[inv]
    d, _, _ = col.signed(out)
    return out, d


def fit_pants(pos, tris, B, col, clearance=0.8, drop=2.0):
    p, info = normalise(pos, "pants")
    L = landmarks_body(B)
    zt_g = p[:, 2].max()
    h_g = zt_g - p[:, 2].min()
    cz_g = garment_crotch(p)
    rise_g = zt_g - cz_g
    wg, dg, _ = width_depth(p, zt_g - 0.04 * h_g, 0.03 * h_g)
    s = 0.5
    for _ in range(6):
        waist_z = min(L["crotch"] - drop + rise_g * s, L["pelvis"] + 12.0)
        wb, db, yb = width_depth(col.pts, waist_z - 2.0)
        s = (wb + 2 * clearance) * 1.02 / wg
    # si el tiro de la prenda es más largo que el del cuerpo, se acorta solo en vertical
    sz_rise = (waist_z - (L["crotch"] - drop)) / (rise_g * s)
    sy = s * np.clip((db / wb) / (dg / wg), 0.85, 1.2)
    q = p * np.array([s, sy, s])
    q[:, 2] -= cz_g * s
    q[:, 2] = np.where(q[:, 2] > 0, q[:, 2] * sz_rise, q[:, 2]) + (L["crotch"] - drop)
    info["rise_squash"] = round(float(sz_rise), 3)
    # centrar en Y a la altura de la cinturilla
    _, _, yg = width_depth(q, waist_z - 2.0)
    q[:, 1] += yb - yg
    info.update({"scale": round(float(s), 3), "scale_y": round(float(sy), 3), "waist_z": round(float(waist_z), 1), "crotch_b": round(L["crotch"], 1),
                 "rise_cm": round(float(rise_g * s), 1), "hem_z": round(float(q[:, 2].min()), 1)})
    band_top = waist_z

    def c_max(z):
        # cinturilla elástica ceñida; más abajo la tela conserva su holgura
        t = np.clip((band_top - z) / 6.0, 0, 1)
        return 1.2 + t * 7.0

    tris, info["cap_mask"] = drop_inner_caps(q, tris, col)
    q, pinfo = band_clamp_polar(q, tris, col, L["crotch"] - 4, waist_z + 2, clearance, c_max, fade_lo=(L["crotch"] - 3, L["crotch"] + 4))
    info["polar"] = pinfo
    q, d = band_clamp_distance(q, tris, col, clearance, lambda x: np.full(len(x), 1e9))
    info["inside_pct"] = round(float((d < 0.2).mean() * 100), 2)
    info["min_cm"] = round(float(d.min()), 2)
    return q, tris, info


def fit_top(pos, tris, B, col, clearance=1.0, limit_top_length=False):
    p, info = normalise(pos, "top")
    L = landmarks_body(B)
    zt_g = p[:, 2].max()
    ah_g = garment_armhole(p)
    h_g = zt_g - p[:, 2].min()
    wg, dg, _ = width_depth(p, ah_g - 0.03 * h_g, 0.02 * h_g)
    chest_z = L["armpit"] - 3.0
    wb, db, yb = width_depth(col.pts, chest_z)
    s = (wb + 2 * clearance) * 1.03 / wg
    sy = s * np.clip((db / wb) / (dg / wg), 0.8, 1.2)
    q = p * np.array([s, sy, s])
    # los tirantes descansan en el hombro
    top = q[q[:, 2] > q[:, 2].max() - 1.5]
    xs = float(np.abs(top[:, 0]).mean())
    q[:, 2] += shoulder_top(B, xs) + clearance - q[:, 2].max()
    ah = ah_g * s + (q[:, 2].max() - zt_g * s)
    _, _, yg = width_depth(q, ah - 3.0)
    q[:, 1] += yb - yg
    info.update({"scale": round(float(s), 3), "scale_y": round(float(sy), 3), "strap_x": round(xs, 1), "armhole_z": round(float(ah), 1),
                 "armpit_b": round(L["armpit"], 1), "hem_z": round(float(q[:, 2].min()), 1), "top_z": round(float(q[:, 2].max()), 1)})
    # la sisa no puede quedar por encima de la axila
    limit = L["armpit"] - 1.5
    if ah > limit:
        zt = q[:, 2].max()
        f = (zt - limit) / (zt - ah)
        up = q[:, 2] > ah
        q[up, 2] = zt - (zt - q[up, 2]) * f
        q[~up, 2] -= ah - limit
        ah = limit
        info["armhole_lowered_to"] = round(float(limit), 1)
    tris, info["cap_mask"] = drop_inner_caps(q, tris, col, z_max=ah - 3)
    # Largo opcional: si limit_top_length=True, el bajo de un top no pasa de la entrepierna;
    # si la prenda es más larga, se acorta en vertical desde el pecho (la tela se comprime, no se corta).
    hem_limit = L["crotch"] + 3.0
    z_ref = L["armpit"]
    hem = q[:, 2].min()
    if limit_top_length and hem < hem_limit:
        f = (z_ref - hem_limit) / (z_ref - hem)
        low = q[:, 2] < z_ref
        q[low, 2] = z_ref - (z_ref - q[low, 2]) * f
        ah = z_ref - (z_ref - ah) * f if ah < z_ref else ah
        info["length_squash"] = round(float(f), 3)
    q, pinfo = band_clamp_polar(q, tris, col, q[:, 2].min() - 2, ah + 6, clearance, lambda z: 6.0, fade_hi=(ah - 3, ah + 3))
    info["polar"] = pinfo

    def hi(x):
        # arriba (hombros, pecho alto) la tela descansa sobre el cuerpo; abajo cuelga
        t = np.clip((x[:, 2] - (ah - 4)) / 6.0, 0, 1)
        return np.where(t > 0, 6.0 - t * 4.2, 1e9)

    q, d = band_clamp_distance(q, tris, col, clearance, hi)
    info["inside_pct"] = round(float((d < 0.2).mean() * 100), 2)
    info["min_cm"] = round(float(d.min()), 2)
    info["median_cm"] = round(float(np.median(d)), 2)
    return q, tris, info
