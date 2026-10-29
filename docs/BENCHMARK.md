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

## Ryzen 9 9900X + RTX 5070 (PC di riferimento), 28/10/2026

Windows 11 (build 26100), AMD Ryzen 9 9900X (12 core, 24 thread), 93.6 GB di RAM,
NVIDIA GeForce RTX 5070 (Vulkan, driver 617.42), build release compilata in locale.

`forgerender-cli bench --json bench-9900x.json` (640×360, 16 campioni/pixel; compilazione
scena + BVH 4.6 ms)

| Dispositivo | Tempo (s) | ms/passata | Mcampioni/s | Speedup |
|---|---:|---:|---:|---:|
| CPU, 1 thread | 1.814 | 113.36 | 2.03 | 1.00× |
| CPU, 2 thread | 0.944 | 58.98 | 3.91 | 1.92× |
| CPU, 4 thread | 0.498 | 31.10 | 7.41 | 3.65× |
| CPU, 8 thread | 0.244 | 15.23 | 15.13 | 7.44× |
| CPU, 16 thread | 0.151 | 9.42 | 24.47 | 12.04× |
| CPU, 24 thread | 0.121 | 7.55 | 30.50 | 15.01× |
| **GPU RTX 5070** | **0.026** | **1.65** | **139.36** | **68.57×** |

Denoiser Open Image Denoise 2.3.3, 640×360, qualità alta:

| Dispositivo | ms per immagine |
|---|---:|
| CPU | 50.8 |
| CUDA (RTX 5070) | 18.1 |

- La CPU scala quasi linearmente fino ai 12 core fisici (7.4× con 8 thread); i thread SMT
  aggiungono un altro 25% circa (12× → 15× da 16 a 24 thread).
- La GPU è 4.6 volte più veloce della CPU a pieno carico e 68 volte più veloce di un singolo core.
  Questa scena è piccola: sulle scene pesanti (texture, displacement, nebbia) il divario va misurato
  con `render --project … --backend gpu|cpu`.
- Con CUDA il denoiser è 2.8 volte più veloce che sulla CPU: per l'anteprima progressiva è
  conveniente attivarlo.

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
  Il majorant per strato orizzontale è stato sostituito da una griglia 3D (vedi sotto).

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

### Majorant 3D della nebbia (griglia di blocchi 8³ con DDA), 29/10/2026

Container di sviluppo (Xeon 4 vCPU, **3 thread**, solo CPU: la GPU software del container non
regge il buffer di questa scena, 182 MB contro un limite di 134 MB). Scena demo
`samples/scene/cocktail-ghiaccio-secco`, senza profondità di campo, seed 12345, 320×400,
32 campioni, `render --backend cpu --threads 3`. Stessi parametri prima e dopo; la simulazione
della nebbia (circa 45 s, fuori dal tempo di rendering) non cambia.

| Vista | Prima (majorant per strato) | Dopo (majorant per blocco) | Guadagno |
|---|---:|---:|---:|
| Intera (demo) | 20.25 s · 632.7 ms/passata | 11.91 s · 372.2 ms/passata | 1.70× |
| Ravvicinata sulla nebbia | 26.55 s · 829.7 ms/passata | 13.44 s · 419.8 ms/passata | 1.98× |

Controllo dell'immagine: luminosità media per canale uguale entro lo 0.3 % (rumore Monte Carlo:
con lo stesso seed la sequenza casuale cambia perché cambiano i passi), nessuna differenza
visibile. Test: majorant ≥ densità in ogni punto dei tratti lungo 2000 raggi casuali, blocchi
vuoti presenti; furnace, Beer–Lambert, ratio tracking, ombra e GPU contro CPU invariati.
Da misurare sul PC di riferimento (RTX 5070) con la stessa scena.
