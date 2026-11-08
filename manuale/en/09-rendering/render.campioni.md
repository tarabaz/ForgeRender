---
id: render.campioni
titolo: Samples per pixel and noise threshold
parole: samples, spp, passes, quality, noisy, adaptive sampling, noise threshold, unlimited, render time, convergence
ui: panel.render.samples
correlate: render.denoiser, render.rimbalzi, render.pannello
---
# Samples per pixel and noise threshold

**Where:** Render tab → **Samples per pixel**.

**What it does:** every sample is one light path traced for each pixel. The image is the
average of the samples: with few of them it is noisy (grainy), with many it becomes clean. The
noise halves every time the samples are multiplied by four, so the last steps cost the most.

![16 samples: very grainy](img/comune/render_samples_016.png) ![64 samples: grain still visible](img/comune/render_samples_064.png) ![256 samples: fine grain in glass and shadows](img/comune/render_samples_256.png) ![1024 samples: clean](img/comune/render_samples_1024.png)

*Denoiser off, to show the real noise: 16, 64, 256 and 1024 samples. Glass and soft shadows
are the last to become clean.*

| Choice | Values |
|---|---|
| Presets | 16, 32, 64, 128, 256, 512, 1024 |
| Custom | from 1 to 1,000,000 |
| Unlimited | the render goes on until you stop it |

- **Default:** 128.
- **It can be changed while rendering:** the accumulation goes on without restarting.
- **Time:** grows in proportion to the samples: 1024 samples take about four times as long as
  256.

## How many?

| Scene | With the denoiser |
|---|---|
| Opaque product in a studio | 128 – 256 |
| Glass, liquids, glossy metals | 512 – 1024 |
| Smoke, fog, caustics, dark interiors | 1024 and more |

With the [denoiser](render.denoiser) on, a few hundred samples are usually enough: it removes
the remaining grain.

## Stop at noise threshold (adaptive sampling)

**Stop at noise threshold** makes each area of the image stop when its estimated noise falls
below the threshold: the time goes to the difficult areas (glass, shadows, fog) instead of the
background. The number of samples above stays the **maximum**.

![Threshold 0.08: fast, still noisy](img/comune/render_noisethreshold_0080.png) ![Threshold 0.04: about the quality of 256 samples](img/comune/render_noisethreshold_0040.png) ![Threshold 0.01: clean, slower](img/comune/render_noisethreshold_0010.png)

*Maximum 1024 samples, denoiser off, thresholds 0.08, 0.04 and 0.01.*

| Control | Range | Default | What it does |
|---|---|---|---|
| Noise threshold | 0.002 – 0.1 | 0.03 when turned on | Lower = cleaner and slower. On the test scene 0.08 ≈ 64 samples, 0.04 ≈ 256, 0.02 ≈ 1000 |
| Minimum samples | 4 – 4096 | 16 | Samples per pixel before the noise is measured (avoids stopping too early where light is rare) |

It pays off with backgrounds and easy areas; in dark scenes with fog and sparkles it can cost
more than uniform sampling.
