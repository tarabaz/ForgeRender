"""Diagnostica del campionamento adattivo: dove vanno i campioni e quanto è l'errore per zona.

Uso, dalla radice del repository:
    python reports/strumenti/analisi_adattivo.py --rif ref.png ref2.png \
        --mappa mappa_ad04.png --img ad04.png [--img uni128.png ...] [--csv riquadri.csv]

- --rif: uno o due riferimenti PNG ad alti campioni (con due, seed diversi, il rumore del
  riferimento si stima e si sottrae dall'MSE: var = MSE(r1, r2) / 4 per la loro media).
- --mappa: mappa dei campioni per pixel scritta da `forgerender-cli render --adaptive-map`
  (PNG a 16 bit con i conteggi esatti). Facoltativa.
- --img: immagini da valutare (stessa risoluzione del riferimento).

Le zone sono fasce di luminanza del riferimento (immagine mostrata, 0..1):
scura < 0.1, media 0.1–0.5, chiara ≥ 0.5.

Per ogni zona: frazione di pixel, frazione dei campioni spesi e campioni medi per pixel (se c'è
la mappa); per ogni immagine MSE corretto per il rumore del riferimento e differenza media
rispetto al riferimento con il suo errore standard. Un |z| piccolo (< 3) vuol dire che la
differenza media non si distingue dal rumore: non va chiamata distorsione.

Con --csv scrive una riga per riquadro 16×16 (luminanza media del riferimento, campioni medi,
MSE di ogni immagine) per analisi successive.
"""
import argparse
import sys

import numpy as np
from PIL import Image

TILE = 16
BANDS = [("scura", 0.0, 0.1), ("media", 0.1, 0.5), ("chiara", 0.5, 1.01)]


def load_rgb(p):
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64) / 255.0


def luminance(img):
    return img @ np.array([0.2126, 0.7152, 0.0722])


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--rif", nargs="+", required=True)
    ap.add_argument("--mappa")
    ap.add_argument("--img", action="append", default=[])
    ap.add_argument("--csv")
    a = ap.parse_args()

    refs = [load_rgb(p) for p in a.rif[:2]]
    ref = sum(refs) / len(refs)
    h, w, _ = ref.shape
    # Rumore per pixel della media dei riferimenti (somma sui canali, come l'MSE sotto).
    ref_noise_px = ((refs[0] - refs[1]) ** 2).mean(axis=2) / 4.0 if len(refs) == 2 else np.zeros((h, w))
    lum = luminance(ref)

    counts = None
    if a.mappa:
        counts = np.asarray(Image.open(a.mappa), dtype=np.float64)
        if counts.shape != (h, w):
            sys.exit(f"mappa {counts.shape} diversa dal riferimento {(h, w)}")

    imgs = []
    for p in a.img:
        im = load_rgb(p)
        if im.shape != ref.shape:
            sys.exit(f"{p}: dimensione {im.shape[:2]} diversa dal riferimento {(h, w)}")
        imgs.append((p, im))

    print(f"riferimento: {' + '.join(a.rif[:2])}  ({w}×{h})")
    if len(refs) == 2:
        print(f"rumore stimato del riferimento (MSE): {ref_noise_px.mean():.6f}")
    if counts is not None:
        print(f"campioni per pixel: media {counts.mean():.1f}, min {counts.min():.0f}, max {counts.max():.0f}")
    print()

    total_samples = counts.sum() if counts is not None else None
    header = f"{'zona':<7} {'pixel':>7}"
    if counts is not None:
        header += f" {'campioni':>9} {'spp medi':>9}"
    print(header)
    for name, lo, hi in BANDS:
        m = (lum >= lo) & (lum < hi)
        row = f"{name:<7} {m.mean() * 100:6.1f}%"
        if counts is not None:
            spent = counts[m].sum() / total_samples * 100 if m.any() else 0.0
            row += f" {spent:8.1f}% {counts[m].mean() if m.any() else 0:9.1f}"
        print(row)
    print()

    for p, im in imgs:
        err_px = ((im - ref) ** 2).mean(axis=2)
        diff_px = (im - ref).mean(axis=2)
        print(f"{p}: MSE corretto {max(err_px.mean() - ref_noise_px.mean(), 0):.6f} (grezzo {err_px.mean():.6f})")
        for name, lo, hi in BANDS:
            m = (lum >= lo) & (lum < hi)
            n = int(m.sum())
            if n < 2:
                continue
            mse = max(err_px[m].mean() - ref_noise_px[m].mean(), 0.0)
            # Errore standard della differenza media: rumore dell'immagine + del riferimento,
            # pixel considerati indipendenti (ogni pixel ha i suoi campioni).
            se = np.sqrt(err_px[m].mean() / n)
            md = diff_px[m].mean()
            z = md / se if se > 0 else 0.0
            print(f"  {name:<7} MSE {mse:.6f}  differenza media {md:+.5f} ± {se:.5f} (z = {z:+.1f})")
        print()

    if a.csv:
        ty, tx = -(-h // TILE), -(-w // TILE)
        with open(a.csv, "w", encoding="utf-8") as f:
            cols = ["tx", "ty", "lum_rif", "rumore_rif"] + (["spp"] if counts is not None else [])
            cols += [f"mse:{p}" for p, _ in imgs]
            f.write(",".join(cols) + "\n")
            for j in range(ty):
                for i in range(tx):
                    sl = (slice(j * TILE, (j + 1) * TILE), slice(i * TILE, (i + 1) * TILE))
                    vals = [i, j, lum[sl].mean(), ref_noise_px[sl].mean()]
                    if counts is not None:
                        vals.append(counts[sl].mean())
                    vals += [((im[sl] - ref[sl]) ** 2).mean() for _, im in imgs]
                    f.write(",".join(f"{v:.6g}" if isinstance(v, float) else str(v) for v in vals) + "\n")
        print(f"riquadri -> {a.csv}")


if __name__ == "__main__":
    main()
