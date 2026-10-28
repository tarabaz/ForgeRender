# Benchmark

Solo misure realmente eseguite. Comando: `forgerender-cli bench` (scena di test procedurale,
11 022 triangoli, una passata di riscaldamento esclusa dalla misura).

## Container di sviluppo (cloud), 27/10/2026

**Non è il PC di riferimento**: Intel Xeon @ 2.10 GHz, 4 vCPU, Ubuntu 24.04, build release.

`forgerender-cli bench --spp 8 --width 640 --height 360`

| Thread | Tempo (s) | ms/passata | Mcampioni/s | Speedup |
|---:|---:|---:|---:|---:|
| 1 | 1.625 | 203.1 | 1.13 | 1.00× |
| 2 | 0.877 | 109.6 | 2.10 | 1.85× |
| 4 | 0.431 | 53.9 | 4.28 | 3.77× |

Mesh STL da 1 000 000 di triangoli (toro generato), `render --mesh --spp 16 --width 640 --height 360`, 3 thread:
import + normali 1590 ms, costruzione BVH 914 ms (1 080 743 nodi), rendering 0.63 s (39.5 ms/passata).

## Ryzen 9 9900X + RTX 5070

Da eseguire sul PC: `forgerender-cli bench --json bench-9900x.json`.

## Nebbia volumetrica (ghiaccio secco), container di sviluppo, 28/10/2026

Stessa macchina (4 vCPU Xeon, 3 thread di rendering). Scena di test con pavimento scuro, nebbia
predefinita sopra il box (zona 3.2 × 0.95 × 3.2 m, 5 s simulati, collisioni attive),
`render --width 480 --height 270 --spp 64`:

| | Calcolo nebbia | ms/passata | Mcampioni/s |
|---|---:|---:|---:|
| Nebbia vuota (0 s simulati) | 0.13 s | 102.9 | 1.26 |
| Nebbia piena | 2.6 s (simulazione 64×24×64, 562 passi: 2.5 s; densità 160×60×160: 0.11 s) | 384.3 | 0.34 |

- Il calcolo della nebbia si fa una volta e resta in cache finché non cambiano i suoi parametri o
  la geometria; nell'app gira in background.
- Il costo del rendering dipende dalla densità e dai rimbalzi nella nebbia (scattering multiplo).
  Ottimizzazione prevista: majorant su una griglia 3D grossolana con attraversamento DDA (oggi è
  per strato orizzontale).

### Simulazione fluida della nebbia (seconda versione), 28/10/2026

Stessa macchina (4 vCPU Xeon). Solutore euleriano a griglia sfalsata, 6 s simulati,
risoluzione 160 (simulazione 112 celle sul lato lungo), zona 0.7 × 0.4–0.5 × 0.7 m:

| Scena | Calcolo nebbia | Note |
|---|---:|---|
| Tazza (misurino) che trabocca | 10 s | colate lungo la parete, tappeto sul tavolo |
| Collo stretto (beuta), risalita alta | 39 s | 674 passi; prima delle ottimizzazioni 309 s |

Ottimizzazioni misurate sulla scena "beuta": riquadro di lavoro attorno alla nebbia (309 → 200 s),
passo fino a 2 celle con controllo delle pareti (200 → 110 s), pressione rosso/nero sul posto
(pressione 40 → 10 s), trasporto di vapore e gas pesante insieme e solo vicino alla nebbia
(110 → 40 s). Il calcolo si fa una volta, in background, e resta in cache.
