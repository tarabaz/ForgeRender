"""Metriche di confronto tra render PNG (stessa risoluzione) usate nei report.

Uso:
    python reports/strumenti/confronta_immagini.py RIF1.png [RIF2.png] -- A.png B.png ...

- Con due riferimenti (stessa scena, seed diversi) il riferimento è la loro media e il suo
  rumore stimato, MSE(rif1, rif2) / 4, viene sottratto dall'errore delle immagini: così il
  confronto non dipende da quanto è pulito il riferimento.
- Stampa per ogni immagine: luminosità media, MSE corretto e RMSE (valori PNG 0..1, dopo il
  tone mapping), e la differenza media su blocchi 16x16 (scostamenti locali di luminosità).
- Il "costo" per una data qualità è tempo × MSE (o campioni × MSE): con il Monte Carlo ideale
  resta costante al variare dei campioni; più basso = più efficiente.
"""
import sys

import numpy as np
from PIL import Image


def load(p):
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64) / 255.0


def blocks(x, n=16):
    h, w = x.shape[0] // n * n, x.shape[1] // n * n
    return x[:h, :w].reshape(h // n, n, w // n, n, 3).mean((1, 3))


args = sys.argv[1:]
if "--" not in args:
    sys.exit(__doc__)
k = args.index("--")
refs, imgs = [load(p) for p in args[:k]], args[k + 1:]
ref = sum(refs) / len(refs)
var_ref = ((refs[0] - refs[1]) ** 2).mean() / 4 if len(refs) == 2 else 0.0
print(f"riferimento: media {ref.mean():.4f}, rumore stimato (MSE) {var_ref:.6f}")
for p in imgs:
    a = load(p)
    mse = ((a - ref) ** 2).mean() - var_ref
    dblk = np.abs(blocks(a) - blocks(ref)).mean()
    print(f"{p}: media {a.mean():.4f}  MSE {mse:.6f}  RMSE {np.sqrt(max(mse, 0)):.4f}  diff. blocchi {dblk:.4f}")
