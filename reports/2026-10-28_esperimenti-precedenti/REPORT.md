# Esperimenti precedenti (riepilogo)

Riassunto delle misure fatte prima che esistesse questa cartella. I numeri vengono dai log
originali delle sessioni di sviluppo; i log completi non sono stati conservati, quindi qui ci
sono solo i valori riportati. Le tabelle complete del PC di riferimento stanno in
[`docs/BENCHMARK.md`](../../docs/BENCHMARK.md).

## 1. Portali di luce (interni illuminati da una finestra)

- **Macchina**: container di sviluppo (vedi [indice](../README.md#macchine)).
- **Test**: `cargo test --release -p forge_render_cpu portal -- --nocapture`. La GPU usa il test
  equivalente in `forge_render_gpu/tests/gpu.rs`.
- **Scena**: stanza chiusa illuminata dal cielo attraverso una finestra; errore a 64 campioni
  contro un riferimento.

| Backend | Errore senza portale | Errore con portale | Riduzione |
|---|---:|---:|---:|
| CPU | 0.0639 | 0.0098 | 6.5× |
| GPU | 0.0629 | 0.0101 | 6.2× |

**Controllo di distorsione**: a 64 rimbalzi le medie dell'immagine coincidono (circa 0.1051
senza portale e 0.1053 con il portale, entro il rumore). A 8 rimbalzi c'era una differenza, ma
era dovuta al troncamento del percorso e non al portale: per questo il test usa 64 rimbalzi e
`russian_roulette_start` alto.

## 2. Costruzione della BVH in parallelo

- **Macchina**: container, 4 vCPU.
- **Test**: test temporaneo (non nel repository) con
  `TriangleAccel::build` su 1 000 000 di triangoli casuali, due costruzioni di fila.

| Thread | Prima costruzione | Seconda costruzione |
|---|---:|---:|
| 1 (`RAYON_NUM_THREADS=1`) | 1994 ms | 1854 ms |
| 4 | 628 ms | 563 ms |

Guadagno: circa 3.2–3.3× su 4 core. Il test `BVH vs brute force` verifica che il risultato
sia lo stesso.

## 3. Bug della risoluzione della griglia della nebbia

- **Scena**: demo del cocktail; container, 3 thread.
- **Il bug**: la simulazione girava su una griglia di 56×128×56 celle da 1.1 cm (risoluzione
  presa sull'asse sbagliato), quindi da vicino si vedevano i bordi delle celle.
- **Dopo la correzione**: 128×90×128 celle da 4.7 mm, più l'ammorbidimento del campo.

| | Griglia | Passi | Simulazione | Densità |
|---|---|---:|---:|---|
| Prima | 56×128×56 | 242 | 5.2 s | 87×200×87, 198 ms |
| Dopo | 128×90×128 | 297 | 32.7–36.6 s | 200×140×200, ~0.8 s |

Il costo della simulazione cresce di circa 6–7× perché le celle sono 4.6 volte di più. La
simulazione si fa una volta sola ed è in cache. Il rendering per passata resta simile:
1233.5 ms a 48 campioni prima, 1280.6 ms dopo (480×300).

## 4. PC di riferimento: Ryzen 9 9900X + RTX 5070 (28/10/2026)

- **Comando**: `forgerender-cli bench --json bench-9900x.json` (640×360, 16 campioni/pixel).
- **Scena**: scena di test procedurale di 11 022 triangoli.

| Dispositivo | Mcampioni/s | Rispetto a 1 thread |
|---|---:|---:|
| CPU, 1 thread | 2.03 | 1.00× |
| CPU, 8 thread | 15.13 | 7.44× |
| CPU, 24 thread | 30.50 | 15.01× |
| GPU RTX 5070 | 139.36 | 68.57× |

Denoiser OIDN 2.3.3 (640×360, qualità alta): 50.8 ms sulla CPU, 18.1 ms con CUDA.

## Cosa manca (da misurare sul PC di riferimento)

- Scene pesanti (nebbia, texture, displacement) sulla GPU contro la CPU. Ora è possibile con i
  progetti `bench_*` (vedi il README del repository, sezione benchmark).
- Majorant 3D e campionamento adattivo sulla RTX 5070.
