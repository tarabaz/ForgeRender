---
id: render.rimbalzi
titolo: Bounces, Russian roulette, clamp and seed
parole: bounces, max bounces, light paths, depth, russian roulette, clamp, indirect clamp, fireflies, seed, path tracing
ui: panel.render.path_tracing
correlate: render.campioni, mat.transmission, render.caustiche
---
# Bounces, Russian roulette, clamp and seed

**Where:** Render tab → **Path tracing** (Max bounces); the other values are in the
**Advanced** section just below, closed by default.

## Max bounces

**What it does:** how many times a light path can bounce before it stops. Every bounce adds
light coming from other surfaces: the indirect light that fills shadows, the reflections of
reflections, and above all the light that crosses **glass**, which needs at least two bounces
for each glass surface it goes through.

![1 bounce: only direct light, glass and mirrors are black](img/comune/render_bounces_01.png) ![3 bounces: glass partly transparent, mirrors still dark inside](img/comune/render_bounces_03.png) ![12 bounces: complete light](img/comune/render_bounces_12.png)

*Max bounces 1, 3 and 12.*

- **Range:** 1 to 64.
- **Default:** 12 (Simple mode: Draft 8, Balanced 12, High 16).
- **Time:** more bounces cost a little more time per sample, but only where light really
  bounces many times; above 12 the difference is visible only with a lot of glass in a row
  (glasses with liquid, chandeliers, lenses).

## Russian roulette from

From this bounce on, dark paths that carry little light are stopped at random (and the
surviving ones count more, so the result stays correct). It saves time without changing the
image on average.

- **Range:** 1 to 16. **Default:** 3.
- Lower values = faster passes but a little more noise in indirect light.

## Indirect light limit (clamp)

Limits the brightness of the indirect light. It reduces **fireflies** (isolated very bright
pixels, typical of small lights seen through glass or in glossy reflections), but it removes
some energy, so the image is no longer physically exact.

- **Range:** 0 to 1000. **Default:** 0 = off (physically correct).
- If you need it, start from high values (10 – 20) and lower them only as far as needed.

## Random seed

The starting number of the random sequence. The same scene with the same seed gives exactly the
same image; changing it gives a different pattern of noise with the same quality. It is useful
for comparisons and to render the same frame again.
