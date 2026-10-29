# Majorant 3D della nebbia (griglia di blocchi con DDA)

- **Data**: 29/10/2026 · **Commit**: prima `951501d^` (majorant per strato), dopo `951501d`
- **Macchina**: container di sviluppo, CPU 3 thread (vedi [indice](../README.md#macchine))

## Domanda

Il delta tracking nella nebbia usava un majorant per strato orizzontale (il massimo di σt in
tutta la fetta di quota y). Nella scena del cocktail il tappeto denso sul bancone alza il
majorant dell'intera fetta, anche dove la nebbia non c'è: molte collisioni nulle e letture
trilineari inutili, sia per i percorsi (delta tracking) sia per le ombre (ratio tracking).
Quanto si guadagna con un majorant per blocchi 3D?

## Cosa è cambiato

- `forge_volume`: griglia di blocchi di 8³ celle (`MACRO_CELLS`), majorant del blocco = massimo
  delle sue celle più una di bordo per lato (il supporto dell'interpolazione trilineare per
  ogni punto del blocco, fino a mezza cella oltre il bordo), × 1.0001.
- `BakedFog::majorant_segment`: il raggio percorre i blocchi con un DDA senza stato (blocco in
  cui sta entrando, uscita alla prima faccia); i blocchi vuoti si saltano in un colpo.
- CPU (`forge_render_cpu/src/medium.rs`) e GPU (`fog_segment` in `path_tracer.wgsl`) usano gli
  stessi tratti. Algoritmi di tracking invariati, quindi stimatori non distorti come prima.

## Metodo

Progetti `bench_intera` e `bench_vicina` (vedi `../strumenti/crea_progetti_bench.py`): demo del
cocktail, senza profondità di campo, seed 12345.

```
forgerender-cli render --project samples/scene/cocktail-ghiaccio-secco/bench_<vista>.frproject \
    --backend cpu --width 320 --height 400 --spp 32 --threads 3 --quiet --out <file>.png
```

La simulazione della nebbia (~45 s) è fuori dal tempo di rendering e non cambia.

## Risultati

| Vista | Prima | Dopo | Guadagno |
|---|---:|---:|---:|
| Intera (demo) | 20.25 s · 632.7 ms/passata | 11.91 s · 372.2 ms/passata | **1.70×** |
| Ravvicinata sulla nebbia | 26.55 s · 829.7 ms/passata | 13.44 s · 419.8 ms/passata | **1.98×** |

Log originali: `log_prima.txt`, `log_dopo.txt`. Immagini: `prima_*.png`, `dopo_*.png`.

## Controlli di qualità

- Luminosità media per canale uguale entro lo 0.3 % (vista intera: 0.2076/0.1689/0.1376 prima,
  0.2076/0.1690/0.1376 dopo). Con lo stesso seed i pixel non sono identici perché cambia il
  numero di numeri casuali consumati; la differenza è rumore Monte Carlo.
- Test nuovo `macro_majorant_bounds_density_along_rays` (forge_volume): lungo 2000 raggi casuali
  i tratti coprono l'intervallo senza buchi e in 8 punti di ogni tratto la densità è ≤ majorant;
  ci sono blocchi vuoti. Test esistenti verdi: Beer–Lambert, white furnace con nebbia, ratio
  tracking non distorto, ombra attraverso la nebbia, GPU contro CPU sulla nebbia.

## Limiti

- GPU non misurata: nel container la GPU software non accetta il buffer di questa scena (182 MB
  contro 134 MB). Da misurare sulla RTX 5070 (comandi nel README del repository).
- Una sola scena, bassa risoluzione, 32 campioni.

## Possibili migliorie da valutare

- Blocchi di 4³ o 16³ (compromesso tra salti e precisione del majorant): misurare.
- Majorant per blocco anche per i raggi d'ombra delle luci lontane: già incluso (stessa funzione).
