# Campionamento adattivo (prima versione)

- **Data**: 29/10/2026 · **Commit**: `7eccf8e` (prima: `951501d`, senza adattivo)
- **Macchina**: container di sviluppo, CPU 3 thread (vedi [indice](../README.md#macchine))

## Domanda

Fermare i pixel già puliti e dare i campioni alle zone difficili riduce il tempo per una data
qualità? Su quali scene?

## Cosa è stato implementato

- Accumulatore con campioni per pixel e una seconda somma "B" (passate di indice pari, circa
  metà dei campioni). Errore stimato del pixel = |media − media di B| (somma dei canali) /
  √(luminosità + 1e-4), come in Cycles.
- Maschera a riquadri 16×16 (`forge_render_core/src/adaptive.rs`): errore del riquadro = media
  dei pixel; il riquadro si ferma quando lui e gli 8 vicini sono sotto la soglia; un riquadro
  fermo non riparte. Valutazione dopo `adaptive_min_samples` (16), a ogni passata.
- CPU e GPU saltano i riquadri fermi; la sessione finisce quando tutti sono fermi o al massimo
  di campioni. Spento di default (`noise_threshold = 0`).

## Metodo

**Errore**: MSE contro un riferimento; costo per una data qualità = campioni (o tempo) × MSE,
costante per il Monte Carlo ideale, più basso = meglio.

1. *Scena di test con mezza inquadratura di cielo* (`Scene::test_scene` con camera alzata),
   96×54, riferimento uniforme 2048 campioni (seed 99), errore relativo alla luminosità in
   radianza lineare. Test `forge_render_cpu/tests/adaptive.rs`.
2. *Scena del cocktail*, progetto `bench_ad` (vedi `../strumenti/crea_progetti_bench.py`),
   320×400, CPU 3 thread; riferimento = media di due render uniformi a 1024 campioni (seed 99 e
   100) con il loro rumore sottratto; errore sui PNG (dopo tone mapping), con
   `../strumenti/confronta_immagini.py ref.png ref2.png -- uni32.png uni128.png ad08.png ad04.png`.

```
forgerender-cli render --project samples/scene/cocktail-ghiaccio-secco/bench_ad.frproject \
    --backend cpu --width 320 --height 400 --threads 3 --quiet --spp <N> --seed <S> \
    [--noise-threshold <T>] --out <file>.png
```

## Risultati

### Scena di test con cielo (96×54)

| | campioni/pixel | errore relativo | campioni × errore² |
|---|---:|---:|---:|
| Uniforme 64 | 64 | 0.0760 | 0.37 |
| Uniforme 128 | 128 | 0.0577 | 0.43 |
| Uniforme 256 | 256 | 0.0404 | 0.42 |
| Uniforme 512 | 512 | 0.0310 | 0.49 |
| Adattivo 0.08 | 50.2 (max 109) | 0.0662 | **0.22** |
| Adattivo 0.04 | 219.8 (max 467) | 0.0328 | **0.24** |

Circa **1.8×** più efficiente. Sulla scena di test standard (senza cielo, rumorosa ovunque) il
vantaggio era minimo: soglia 0.04 → 221 campioni/pixel, errore 0.040 contro 256 uniformi
0.043. Con soglie ≤ 0.02 l'errore toccava il rumore del riferimento (0.019) e non è confrontabile.

### Scena del cocktail (320×400)

Rumore stimato del riferimento combinato: MSE 0.000227.

| | tempo | campioni/pixel | media | MSE | RMSE | tempo × MSE |
|---|---:|---:|---:|---:|---:|---:|
| Riferimento (×2) | 445.4 + 431.5 s | 1024 | 0.1829 | — | — | — |
| Uniforme 32 | 12.9 s | 32 | 0.1711 | 0.00774 | 0.088 | **0.100** |
| Uniforme 128 | 55.0 s | 128 | 0.1797 | 0.00253 | 0.050 | **0.139** |
| Adattivo 0.08 | 170.7 s | 338 (max 1024) | 0.1811 | 0.00163 | 0.040 | 0.278 |
| Adattivo 0.04 | 272.8 s | 621 (max 1024) | 0.1827 | 0.00064 | 0.025 | 0.174 |

Qui l'adattivo è **meno efficiente** dell'uniforme (da 1.25× a 2.0×) e non si ferma prima
del massimo (alcuni riquadri arrivano a 1024).

Log: `log_cocktail.txt`, `log_scena_test_uniforme.txt`. Immagini: `ref.png`, `ref2.png`,
`uni32.png`, `uni128.png`, `ad04.png`, `ad08.png`.

## Interpretazione

> **Aggiornamento (diagnostica, [report](../2026-10-29_adattivo-diagnostica/REPORT.md))**:
> l'interpretazione qui sotto sulle zone scure è stata **smentita** dalla mappa dei campioni.
> I pixel scuri ricevono meno campioni; arrivano al massimo le zone chiare e medie (bicchiere
> con nebbia, bancone, riflessi). Inoltre "massimo su sottoblocchi 4×4" renderebbe il riquadro
> *più* sensibile ai pixel isolati, non meno.

- La stima d'errore relativa (divisa per √luminosità) pesa moltissimo le zone scure. Nella scena
  del cocktail lo sfondo è quasi nero con nebbia rada e scintille (lucine sfocate, riflessi):
  lì l'errore relativo resta sopra soglia anche quando nel PNG il rumore non si vede, e quei
  riquadri (più i loro vicini) lavorano fino al massimo.
- Anche l'uniforme non scala come 1/N (costo da 0.100 a 0.139 tra 32 e 128 campioni) e la media
  a 32 campioni è più bassa del 6 %: rumore a code lunghe (scintille rare e intense). È lo stesso
  fenomeno che inganna la stima a metà campioni.
- Nessuna distorsione: tutte le medie convergono a quella del riferimento.

## Prossimi passi proposti (uno alla volta, da confermare)

1. Stimare l'errore nello spazio dell'immagine mostrata (dopo tone mapping ed esposizione), dove
   il rumore delle zone scure pesa quanto si vede; rimisurare con queste due scene.
2. Errore del riquadro come massimo su sottoblocchi 4×4 invece che media (meno ostaggio dei
   singoli pixel con scintille) e vicinato solo 4-connesso.
3. Ripetere le misure sulla RTX 5070 (passate GPU da più campioni: la maschera si aggiorna meno
   spesso).
