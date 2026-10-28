"""Genera la scena del bar (GLB) e le texture procedurali per la demo del cocktail.

Uso: python3 -I make_bar.py CARTELLA_USCITA
Unità: metri, Y in alto. Il piano del bancone è a y = 0, il bicchiere al centro.
"""
import json
import math
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image

OUT = Path(sys.argv[1])
rng = np.random.default_rng(7)


# ----------------------------------------------------------------------------
# Geometria
# ----------------------------------------------------------------------------

def lathe(profile, seg=96, close_axis=True):
    """Superficie di rivoluzione attorno a Y dal profilo [(r, y), …] (ordine: normali a destra
    del verso di percorrenza, cioè verso l'esterno se si sale lungo il lato esterno)."""
    prof = np.array(profile, dtype=np.float64)
    n = len(prof)
    # Normali 2D del profilo: media delle normali dei segmenti adiacenti.
    seg_n = []
    for i in range(n - 1):
        d = prof[i + 1] - prof[i]
        L = np.hypot(*d) or 1.0
        seg_n.append(np.array([d[1], -d[0]]) / L)
    nrm = []
    for i in range(n):
        a = seg_n[i - 1] if i > 0 else seg_n[0]
        b = seg_n[i] if i < n - 1 else seg_n[-1]
        v = a + b
        v /= np.hypot(*v) or 1.0
        nrm.append(v)
    pos, nor, uv, idx = [], [], [], []
    length = np.concatenate([[0], np.cumsum(np.hypot(*np.diff(prof, axis=0).T))])
    for s in range(seg + 1):
        a = s / seg * 2 * math.pi
        c, si = math.cos(a), math.sin(a)
        for i in range(n):
            r, y = prof[i]
            pos.append((r * c, y, -r * si))
            nr, ny = nrm[i]
            nor.append((nr * c, ny, -nr * si))
            uv.append((s / seg, length[i] / max(length[-1], 1e-9)))
    for s in range(seg):
        for i in range(n - 1):
            a = s * n + i
            b = (s + 1) * n + i
            idx += [(a, b, a + 1), (a + 1, b, b + 1)]
    return mesh(pos, nor, uv, idx)


def mesh(pos, nor, uv, idx):
    return {
        "pos": np.array(pos, np.float32),
        "nor": np.array(nor, np.float32),
        "uv": np.array(uv, np.float32),
        "idx": np.array(idx, np.uint32),
    }


def arc(cx, cy, rad, a0, a1, n):
    return [(cx + rad * math.cos(a0 + (a1 - a0) * k / n), cy + rad * math.sin(a0 + (a1 - a0) * k / n)) for k in range(n + 1)]


def box(sx, sy, sz, cx=0.0, cy=0.0, cz=0.0, uv_scale=1.0):
    pos, nor, uv, idx = [], [], [], []
    h = np.array([sx, sy, sz]) / 2
    c = np.array([cx, cy, cz])
    faces = [
        ((1, 0, 0), (0, 0, -1), (0, 1, 0)),
        ((-1, 0, 0), (0, 0, 1), (0, 1, 0)),
        ((0, 1, 0), (1, 0, 0), (0, 0, -1)),
        ((0, -1, 0), (1, 0, 0), (0, 0, 1)),
        ((0, 0, 1), (1, 0, 0), (0, 1, 0)),
        ((0, 0, -1), (-1, 0, 0), (0, 1, 0)),
    ]
    for n, u, v in faces:
        n, u, v = map(np.array, (n, u, v))
        base = len(pos)
        for du, dv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            p = c + (n + u * du + v * dv) * h
            pos.append(p)
            nor.append(n)
            uv.append(((np.dot(p, u)) * uv_scale, (np.dot(p, v)) * uv_scale))
        idx += [(base, base + 1, base + 2), (base, base + 2, base + 3)]
    return mesh(pos, nor, uv, idx)


def sphere(rad, cx, cy, cz, seg=16, ring=10, squash=1.0):
    pos, nor, uv, idx = [], [], [], []
    for j in range(ring + 1):
        t = math.pi * j / ring
        for i in range(seg + 1):
            p = 2 * math.pi * i / seg
            n = (math.sin(t) * math.cos(p), math.cos(t), math.sin(t) * math.sin(p))
            pos.append((cx + rad * n[0], cy + rad * n[1] * squash, cz + rad * n[2]))
            nn = np.array((n[0], n[1] / squash, n[2]))
            nor.append(nn / np.linalg.norm(nn))
            uv.append((i / seg, j / ring))
    for j in range(ring):
        for i in range(seg):
            a = j * (seg + 1) + i
            b = a + seg + 1
            idx += [(a, a + 1, b), (a + 1, b + 1, b)]
    return mesh(pos, nor, uv, idx)


def translate(m, dx, dy, dz):
    m = dict(m)
    m["pos"] = m["pos"] + np.array([dx, dy, dz], np.float32)
    return m


def merge(ms):
    pos, nor, uv, idx, off = [], [], [], [], 0
    for m in ms:
        pos.append(m["pos"])
        nor.append(m["nor"])
        uv.append(m["uv"])
        idx.append(m["idx"] + off)
        off += len(m["pos"])
    return {"pos": np.concatenate(pos), "nor": np.concatenate(nor), "uv": np.concatenate(uv), "idx": np.concatenate(idx)}


# --- Coppa da cocktail (coupe), misure in metri --------------------------------
T = 0.0016  # spessore del vetro
BASE_R, BASE_H = 0.040, 0.004
STEM_R, STEM_TOP = 0.0045, 0.100
RIM_R, RIM_Y = 0.052, 0.148
BOWL_BOTTOM = 0.106  # fondo interno della coppa


def bowl_outer(n=40):
    # Dal colletto sopra lo stelo al bordo: curva morbida (quarto di ellisse schiacciato).
    pts = []
    for k in range(n + 1):
        t = k / n
        a = t * math.pi / 2
        r = STEM_R + (RIM_R - STEM_R) * math.sin(a) ** 0.9
        y = STEM_TOP + (RIM_Y - STEM_TOP) * (1 - math.cos(a)) ** 1.15
        pts.append((r, y))
    return pts


def bowl_inner(n=40):
    pts = []
    for k in range(n, -1, -1):
        t = k / n
        a = t * math.pi / 2
        r = (RIM_R - T) * math.sin(a) ** 0.9
        y = BOWL_BOTTOM + (RIM_Y - BOWL_BOTTOM) * (1 - math.cos(a)) ** 1.15
        pts.append((r, y))
    return pts


glass_profile = (
    [(0.0, 0.0), (BASE_R - 0.002, 0.0)]
    + arc(BASE_R - 0.002, 0.002, 0.002, -math.pi / 2, math.pi / 2, 6)[1:]
    # Piede che sale verso lo stelo.
    + [(BASE_R * 0.6, BASE_H + 0.002), (STEM_R * 2.2, BASE_H + 0.010), (STEM_R, BASE_H + 0.020)]
    + [(STEM_R * 0.92, 0.055), (STEM_R, STEM_TOP - 0.006), (STEM_R * 1.25, STEM_TOP - 0.001)]
    + bowl_outer()[1:]
    + arc(RIM_R - T / 2, RIM_Y, T / 2, 0, math.pi, 8)[1:]
    + bowl_inner()[1:]
)
# Lo stelo è pieno: il profilo si chiude sull'asse sotto il fondo della coppa.
glass = lathe(glass_profile, seg=128)

FILL_Y = 0.137
# Il liquido entra di 0.6 mm nello spessore del vetro (1.6 mm): nella parte comune vale il
# liquido (priorità più alta) e l'interfaccia vetro→liquido è quella fisica, senza aria in mezzo.
liq_inner = [(r + 0.0006 if r > 0 else 0.0, y) for r, y in bowl_inner() if y <= FILL_Y + 1e-4]
liq_inner = list(reversed(liq_inner))  # dal centro del fondo verso il bordo
# Raggio alla quota del pelo libero.
def inner_r_at(y):
    pts = bowl_inner()
    for (r0, y0), (r1, y1) in zip(pts[1:], pts[:-1]):
        if y0 <= y <= y1:
            t = (y - y0) / (y1 - y0 + 1e-12)
            return r0 + (r1 - r0) * t
    return RIM_R - T


r_top = inner_r_at(FILL_Y) + 0.0006
liquid_profile = [(0.0, BOWL_BOTTOM - 0.0006)] + [(r, y - 0.0006) for r, y in liq_inner[1:]] + [(r_top, FILL_Y), (r_top * 0.98, FILL_Y + 0.0006), (0.0, FILL_Y + 0.0006)]
# Il profilo del liquido va dal centro in basso verso l'esterno e poi torna all'asse in alto:
# le normali (a destra del verso) puntano verso l'esterno del volume.
liquid_profile = [(r, y) for r, y in liquid_profile]
liquid = lathe(liquid_profile, seg=128)

# Ciliegina sul bordo con il gambo (sfera rossa lucida).
cherry = sphere(0.009, RIM_R - 0.012, RIM_Y + 0.004, 0.010, seg=32, ring=20, squash=0.92)
stem_c = lathe([(0.0008, 0.0), (0.0006, 0.03)], seg=8)
stem_c["pos"] = stem_c["pos"] @ np.array(
    [[math.cos(0.5), math.sin(0.5), 0], [-math.sin(0.5), math.cos(0.5), 0], [0, 0, 1]], np.float32
)
stem_c = translate(stem_c, RIM_R - 0.012, RIM_Y + 0.010, 0.010)

# --- Shaker in acciaio e jigger --------------------------------------------------
shaker_profile = (
    [(0.0, 0.0), (0.040, 0.0), (0.0435, 0.002), (0.046, 0.14), (0.047, 0.142), (0.0465, 0.150),
     (0.042, 0.152), (0.040, 0.165), (0.030, 0.195), (0.014, 0.215), (0.0125, 0.232), (0.0135, 0.236), (0.011, 0.24), (0.0, 0.24)]
)
shaker = translate(lathe(shaker_profile, seg=96), -0.24, 0.0, -0.36)
jig = [(0.0, 0.0), (0.022, 0.0), (0.0225, 0.002), (0.012, 0.045), (0.0125, 0.048), (0.0, 0.048)]
jig2 = [(0.0, 0.048), (0.0125, 0.048), (0.024, 0.085), (0.0245, 0.087), (0.0, 0.087)]
jigger = translate(merge([lathe(jig, seg=64), lathe(jig2, seg=64)]), -0.12, 0.0, 0.12)

# --- Bancone e ambiente -----------------------------------------------------------
counter = box(2.4, 0.05, 0.9, 0.0, -0.025, -0.05, uv_scale=1.0)
front = box(2.4, 1.0, 0.04, 0.0, -0.55, 0.42, uv_scale=1.0)
wall = box(6.0, 3.0, 0.1, 0.0, 0.8, -2.4)
shelves = merge([box(3.2, 0.03, 0.28, 0.0, h, -2.2, uv_scale=1.0) for h in (0.05, 0.42, 0.79)])

# Bottiglie sullo scaffale (sfocate nella demo): profili diversi.
def bottle(h, r, neck_r, shoulder):
    return [(0.0, 0.0), (r - 0.003, 0.0), (r, 0.004), (r, h * shoulder), (neck_r * 1.3, h * (shoulder + 0.15)),
            (neck_r, h * 0.85), (neck_r * 1.15, h * 0.95), (neck_r, h), (0.0, h)]


bottles = {"verde": [], "ambra": [], "chiaro": []}
x = -1.45
k = 0
while x < 1.45:
    for shelf_y in (0.065, 0.435, 0.805):
        if rng.random() < 0.18:
            continue
        h = rng.uniform(0.22, 0.34)
        r = rng.uniform(0.032, 0.045)
        prof = bottle(h, r, rng.uniform(0.011, 0.015), rng.uniform(0.55, 0.7))
        color = ["verde", "ambra", "chiaro"][int(rng.integers(0, 3))]
        bottles[color].append(translate(lathe(prof, seg=32), x + rng.uniform(-0.02, 0.02), shelf_y, -2.2 + rng.uniform(-0.05, 0.05)))
    x += rng.uniform(0.11, 0.17)
    k += 1

# Lucine calde (bokeh) davanti al muro e una fila sopra lo scaffale.
bulbs = []
for i in range(46):
    t = i / 45
    bx = -2.0 + 4.0 * t
    by = 1.25 + 0.12 * math.sin(t * 9.0) - 0.15 * (2 * t - 1) ** 2
    bulbs.append(sphere(0.012, bx, by, -2.05 + 0.05 * math.sin(t * 13), seg=10, ring=6))
for i in range(70):
    bulbs.append(sphere(0.008, rng.uniform(-0.9, 0.9), rng.uniform(0.02, 0.75), rng.uniform(-1.95, -1.5), seg=10, ring=6))


# ----------------------------------------------------------------------------
# Materiali glTF (metallic/roughness + trasmissione e IOR)
# ----------------------------------------------------------------------------
materials = [
    {"name": "Vetro coppa", "pbrMetallicRoughness": {"baseColorFactor": [1, 1, 1, 1], "metallicFactor": 0, "roughnessFactor": 0.0},
     "extensions": {"KHR_materials_transmission": {"transmissionFactor": 1.0}, "KHR_materials_ior": {"ior": 1.5}}},
    {"name": "Cocktail", "pbrMetallicRoughness": {"baseColorFactor": [1, 1, 1, 1], "metallicFactor": 0, "roughnessFactor": 0.0},
     "extensions": {"KHR_materials_transmission": {"transmissionFactor": 1.0}, "KHR_materials_ior": {"ior": 1.34}}},
    {"name": "Ciliegia", "pbrMetallicRoughness": {"baseColorFactor": [0.45, 0.01, 0.02, 1], "metallicFactor": 0, "roughnessFactor": 0.12}},
    {"name": "Gambo", "pbrMetallicRoughness": {"baseColorFactor": [0.25, 0.12, 0.03, 1], "metallicFactor": 0, "roughnessFactor": 0.6}},
    {"name": "Acciaio spazzolato", "pbrMetallicRoughness": {"baseColorFactor": [0.92, 0.92, 0.93, 1], "metallicFactor": 1, "roughnessFactor": 0.18}},
    {"name": "Bancone legno", "pbrMetallicRoughness": {"baseColorFactor": [0.12, 0.06, 0.03, 1], "metallicFactor": 0, "roughnessFactor": 0.35}},
    {"name": "Muro", "pbrMetallicRoughness": {"baseColorFactor": [0.05, 0.035, 0.03, 1], "metallicFactor": 0, "roughnessFactor": 0.9}},
    {"name": "Bottiglia verde", "pbrMetallicRoughness": {"baseColorFactor": [0.25, 0.7, 0.3, 1], "metallicFactor": 0, "roughnessFactor": 0.05},
     "extensions": {"KHR_materials_transmission": {"transmissionFactor": 1.0}, "KHR_materials_ior": {"ior": 1.5}}},
    {"name": "Bottiglia ambra", "pbrMetallicRoughness": {"baseColorFactor": [0.9, 0.5, 0.15, 1], "metallicFactor": 0, "roughnessFactor": 0.05},
     "extensions": {"KHR_materials_transmission": {"transmissionFactor": 1.0}, "KHR_materials_ior": {"ior": 1.5}}},
    {"name": "Bottiglia chiara", "pbrMetallicRoughness": {"baseColorFactor": [0.9, 0.95, 0.95, 1], "metallicFactor": 0, "roughnessFactor": 0.05},
     "extensions": {"KHR_materials_transmission": {"transmissionFactor": 1.0}, "KHR_materials_ior": {"ior": 1.5}}},
    {"name": "Lucine", "pbrMetallicRoughness": {"baseColorFactor": [0, 0, 0, 1], "metallicFactor": 0, "roughnessFactor": 1},
     "emissiveFactor": [1.0, 0.55, 0.22], "extensions": {"KHR_materials_emissive_strength": {"emissiveStrength": 9.0}}},
]
MAT = {m["name"]: i for i, m in enumerate(materials)}

objects = [
    ("Coppa", glass, "Vetro coppa"),
    ("Cocktail", liquid, "Cocktail"),
    ("Ciliegia", cherry, "Ciliegia"),
    ("Gambo ciliegia", stem_c, "Gambo"),
    ("Shaker", shaker, "Acciaio spazzolato"),
    ("Jigger", jigger, "Acciaio spazzolato"),
    ("Bancone", counter, "Bancone legno"),
    ("Fronte bancone", front, "Bancone legno"),
    ("Muro", wall, "Muro"),
    ("Mensole", shelves, "Bancone legno"),
    ("Bottiglie verdi", merge(bottles["verde"]), "Bottiglia verde"),
    ("Bottiglie ambra", merge(bottles["ambra"]), "Bottiglia ambra"),
    ("Bottiglie chiare", merge(bottles["chiaro"]), "Bottiglia chiara"),
    ("Lucine", merge(bulbs), "Lucine"),
]


# ----------------------------------------------------------------------------
# Scrittura GLB
# ----------------------------------------------------------------------------
def write_glb(path, objects):
    bin_ = bytearray()
    views, accessors, meshes, nodes = [], [], [], []

    def add_view(data, target):
        while len(bin_) % 4:
            bin_.append(0)
        off = len(bin_)
        bin_.extend(data)
        views.append({"buffer": 0, "byteOffset": off, "byteLength": len(data), "target": target})
        return len(views) - 1

    for name, m, mat in objects:
        pos, nor, uv, idx = m["pos"], m["nor"], m["uv"], m["idx"].reshape(-1)
        a = {}
        for key, arr, typ in (("POSITION", pos, "VEC3"), ("NORMAL", nor, "VEC3"), ("TEXCOORD_0", uv, "VEC2")):
            v = add_view(arr.astype(np.float32).tobytes(), 34962)
            acc = {"bufferView": v, "componentType": 5126, "count": len(arr), "type": typ}
            if key == "POSITION":
                acc["min"] = arr.min(axis=0).tolist()
                acc["max"] = arr.max(axis=0).tolist()
            accessors.append(acc)
            a[key] = len(accessors) - 1
        v = add_view(idx.astype(np.uint32).tobytes(), 34963)
        accessors.append({"bufferView": v, "componentType": 5125, "count": len(idx), "type": "SCALAR"})
        meshes.append({"name": name, "primitives": [{"attributes": a, "indices": len(accessors) - 1, "material": MAT[mat]}]})
        nodes.append({"name": name, "mesh": len(meshes) - 1})

    gltf = {
        "asset": {"version": "2.0", "generator": "ForgeRender demo"},
        "extensionsUsed": ["KHR_materials_transmission", "KHR_materials_ior", "KHR_materials_emissive_strength"],
        "scene": 0,
        "scenes": [{"name": "Bar", "nodes": [len(nodes)]}],
        "nodes": nodes + [{"name": "Bar", "children": list(range(len(nodes)))}],
        "meshes": meshes,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": len(bin_)}],
    }
    js = json.dumps(gltf, separators=(",", ":")).encode()
    while len(js) % 4:
        js += b" "
    while len(bin_) % 4:
        bin_.append(0)
    total = 12 + 8 + len(js) + 8 + len(bin_)
    with open(path, "wb") as f:
        f.write(struct.pack("<III", 0x46546C67, 2, total))
        f.write(struct.pack("<II", len(js), 0x4E4F534A))
        f.write(js)
        f.write(struct.pack("<II", len(bin_), 0x004E4942))
        f.write(bin_)


# ----------------------------------------------------------------------------
# Texture procedurali: legno scuro (noce) per il bancone
# ----------------------------------------------------------------------------
def value_noise(shape, cells, seed):
    g = np.random.default_rng(seed).random((cells[0] + 1, cells[1] + 1))
    y = np.linspace(0, cells[0], shape[0], endpoint=False)
    x = np.linspace(0, cells[1], shape[1], endpoint=False)
    yi, xi = y.astype(int), x.astype(int)
    fy, fx = y - yi, x - xi
    fy, fx = fy * fy * (3 - 2 * fy), fx * fx * (3 - 2 * fx)
    a = g[yi][:, xi]
    b = g[yi][:, xi + 1]
    c = g[yi + 1][:, xi]
    d = g[yi + 1][:, xi + 1]
    return (a * (1 - fx) + b * fx) * (1 - fy)[:, None] + (c * (1 - fx) + d * fx) * fy[:, None]


def wood(size=1024):
    H = W = size
    n = sum(value_noise((H, W), (4 * 2**o, 1 * 2**o), 10 + o) / 2**o for o in range(5))
    y = np.linspace(0, 1, H)[:, None]
    x = np.linspace(0, 1, W)[None, :]
    # Venature lungo X (le assi del bancone), deformate dal rumore; tavole larghe 1/4.
    plank = np.floor(y * 4)
    rings = np.sin((y * 4 - plank) * 38.0 + n * 9.0 + plank * 2.1 + np.sin(x * 6.0 + plank) * 0.8)
    fine = value_noise((H, W), (220, 8), 3)
    t = 0.5 + 0.5 * rings
    t = 0.65 * t ** 2.2 + 0.35 * fine
    gap = np.broadcast_to((np.abs((y * 4 - plank) - 0.0) < 0.004) | (np.abs((y * 4 - plank) - 1.0) < 0.004), (H, W))
    dark = np.array([0.055, 0.025, 0.012])
    light = np.array([0.22, 0.11, 0.05])
    col = dark + (light - dark) * t[..., None]
    col *= (0.85 + 0.3 * value_noise((H, W), (3, 3), 99))[..., None] * np.broadcast_to(0.8 + 0.2 * plank / 3, (H, W))[..., None]
    col[gap] *= 0.25
    srgb = np.where(col <= 0.0031308, col * 12.92, 1.055 * np.power(col, 1 / 2.4) - 0.055)
    Image.fromarray((np.clip(srgb, 0, 1) * 255).astype(np.uint8)).save(OUT / "textures" / "legno_noce_colore.png")
    rough = np.clip(0.28 + 0.25 * (1 - t) + 0.08 * fine, 0, 1)
    rough[gap] = 0.9
    Image.fromarray((rough * 255).astype(np.uint8)).save(OUT / "textures" / "legno_noce_rugosita.png")
    hgt = np.clip(0.6 + 0.25 * t - 0.6 * gap, 0, 1)
    Image.fromarray((hgt * 255).astype(np.uint8)).save(OUT / "textures" / "legno_noce_rilievo.png")


(OUT / "modelli").mkdir(parents=True, exist_ok=True)
(OUT / "textures").mkdir(parents=True, exist_ok=True)
write_glb(OUT / "modelli" / "bar_cocktail.glb", objects)
wood()
print("ok", sum(len(m["idx"]) for _, m, _ in objects), "triangoli")
