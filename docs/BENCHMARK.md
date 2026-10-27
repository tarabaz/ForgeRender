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
