# Benchmarks

🇮🇹 [Leggi in italiano](BENCHMARK.it.md)

Only measurements actually performed are listed here. Command: `forgerender-cli bench` (procedural test scene, 11,022 triangles; one warm-up pass excluded).

## Cloud development container — 27 Oct 2026

**Not the reference PC:** Intel Xeon @ 2.10 GHz, 4 vCPUs, Ubuntu 24.04, release build.

`forgerender-cli bench --spp 8 --width 640 --height 360`

| Threads | Time (s) | ms/pass | Msamples/s | Speedup |
|---:|---:|---:|---:|---:|
| 1 | 1.625 | 203.1 | 1.13 | 1.00× |
| 2 | 0.877 | 109.6 | 2.10 | 1.85× |
| 4 | 0.431 | 53.9 | 4.28 | 3.77× |

For a generated 1,000,000-triangle torus STL, using `render --mesh --spp 16 --width 640 --height 360` with 3 threads: import + normals 1,590 ms; BVH build 914 ms (1,080,743 nodes); rendering 0.63 s (39.5 ms/pass).

## Ryzen 9 9900X + RTX 5070 reference PC — 28 Oct 2026

Windows 11 (build 26100), AMD Ryzen 9 9900X (12 cores, 24 threads), 93.6 GB RAM, NVIDIA GeForce RTX 5070 (Vulkan, driver 617.42), locally compiled release build.

`forgerender-cli bench --json bench-9900x.json` (640×360, 16 samples/pixel; scene compilation + BVH: 4.6 ms).

| Device | Time (s) | ms/pass | Msamples/s | Speedup |
|---|---:|---:|---:|---:|
| CPU, 1 thread | 1.814 | 113.36 | 2.03 | 1.00× |
| CPU, 2 threads | 0.944 | 58.98 | 3.91 | 1.92× |
| CPU, 4 threads | 0.498 | 31.10 | 7.41 | 3.65× |
| CPU, 8 threads | 0.244 | 15.23 | 15.13 | 7.44× |
| CPU, 16 threads | 0.151 | 9.42 | 24.47 | 12.04× |
| CPU, 24 threads | 0.121 | 7.55 | 30.50 | 15.01× |
| **RTX 5070 GPU** | **0.026** | **1.65** | **139.36** | **68.57×** |

Open Image Denoise 2.3.3, 640×360, high quality:

| Device | ms per image |
|---|---:|
| CPU | 50.8 |
| CUDA (RTX 5070) | 18.1 |

- CPU scaling is almost linear up to 12 physical cores (7.4× with 8 threads); SMT adds about another 25% (12× to 15× from 16 to 24 threads).
- The GPU is 4.6× faster than the fully loaded CPU and 68× faster than one CPU core. This is a small scene; heavy scenes (textures, displacement, fog) still need measurement with `render --project … --backend gpu|cpu`.
- CUDA denoising is 2.8× faster than CPU denoising.

## Volumetric fog (dry ice) — cloud development container, 28 Oct 2026

Same machine (4 Xeon vCPUs, 3 render threads). Test scene with a dark floor and default fog above the box (3.2 × 0.95 × 3.2 m region, 5 simulated seconds, collisions enabled). Command: `render --width 480 --height 270 --spp 64`.

| | Fog computation | ms/pass | Msamples/s |
|---|---:|---:|---:|
| Empty fog (0 simulated seconds) | 0.13 s | 102.9 | 1.26 |
| Full fog | 2.6 s (64×24×64 simulation, 562 steps: 2.5 s; 160×60×160 density: 0.11 s) | 384.3 | 0.34 |

Fog is computed once and cached until its parameters or geometry change; in the app it runs in the background. Render cost depends on density and scattering bounces. The former horizontal-layer majorant was replaced by a 3D grid.

### Fluid fog simulation — second version, 28 Oct 2026

Same machine (4 Xeon vCPUs). Staggered-grid Eulerian solver, 6 simulated seconds, resolution 160 (112 cells along the long side), region 0.7 × 0.4–0.5 × 0.7 m.

| Scene | Fog computation | Notes |
|---|---:|---|
| Overflowing measuring cup | 10 s | Flow down the wall, forming a layer on the table |
| Narrow-neck flask, high rise | 39 s | 674 steps; 309 s before optimizations |

Measured flask optimizations: work region around fog (309 → 200 s); up to 2-cell steps with wall checks (200 → 110 s); in-place red-black pressure solve (40 → 10 s for pressure); transporting vapour and heavy gas together and only near fog (110 → 40 s). The computation runs once in the background and is cached.

### 3D fog majorant (8³ block grid with DDA) — 29 Oct 2026

Development container (Xeon, 4 vCPUs, **3 threads**), CPU only: the software GPU cannot handle this scene's 182 MB buffer against a 134 MB limit. Demo scene `samples/scene/cocktail-ghiaccio-secco`, no depth of field, seed 12345, 320×400, 32 samples, `render --backend cpu --threads 3`. Same settings before and after; fog simulation (~45 s, excluded from render time) is unchanged.

| View | Before (layer majorant) | After (block majorant) | Gain |
|---|---:|---:|---:|
| Full demo scene | 20.25 s · 632.7 ms/pass | 11.91 s · 372.2 ms/pass | 1.70× |
| Fog close-up | 26.55 s · 829.7 ms/pass | 13.44 s · 419.8 ms/pass | 1.98× |

Average image brightness per channel matches within 0.3%; no visible difference. With the same seed, the random sequence changes because the number of steps changes. Tests check that the majorant is at least the density along 2,000 random ray segments, empty blocks are present, and furnace, Beer–Lambert, ratio tracking, shadow and GPU-vs-CPU tests remain unchanged. Still to be measured on the reference PC (RTX 5070).

### Adaptive sampling — first version, 29 Oct 2026

Development container, CPU, 3 threads. Error = RMSE against a reference; cost = samples (or time) × error², i.e. work required for a given quality (lower is better).

**Test scene with half the frame showing sky** (96×54, 2,048-sample reference; test `forge_render_cpu/tests/adaptive.rs`, luminance-relative error):

| | Samples/pixel | Error | Cost (samples × error²) |
|---|---:|---:|---:|
| Uniform 64 / 128 / 256 / 512 | 64 / 128 / 256 / 512 | 0.0760 / 0.0577 / 0.0404 / 0.0310 | 0.37 / 0.43 / 0.42 / 0.49 |
| Adaptive threshold 0.08 | 50.2 (max 109) | 0.0662 | 0.22 |
| Adaptive threshold 0.04 | 219.8 (max 467) | 0.0328 | 0.24 |

About 1.8× more efficient; image average matches the reference within 1%.

**Cocktail scene** (320×400, no depth of field; reference is the average of two 1,024-sample renders with different seeds, with reference noise subtracted; error measured on the PNG):

| | Time | Samples/pixel | Error | Time × error² |
|---|---:|---:|---:|---:|
| Uniform 32 | 12.9 s | 32 | 0.088 | 0.100 |
| Uniform 128 | 55.0 s | 128 | 0.050 | 0.139 |
| Adaptive threshold 0.08 | 170.7 s | 338 (max 1024) | 0.040 | 0.278 |
| Adaptive threshold 0.04 | 272.8 s | 621 (max 1024) | 0.025 | 0.174 |

Here adaptive sampling is **less** efficient (1.25× to 2× higher cost): the luminance-relative error estimate (as in Cycles) remains high in dark fog and spark regions even when noise is not visible, so those tiles keep rendering up to the maximum. The feature is disabled by default. A proposed next step is to estimate error in display space (after tone mapping) and measure again on these same scenes.
