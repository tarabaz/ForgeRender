---
id: render.caustiche
titolo: Caustics and spectral render
parole: caustics, photon mapping, photons, focused light, spectral, wavelength, dispersion, rainbow, prism, abbe, rgb
ui: panel.render.caustics, panel.render.color
correlate: mat.transmission, render.rimbalzi, light.tipi
---
# Caustics and spectral render

**Where:** Render tab → **Colour** (RGB / Spectral) and **Caustics**.

## Caustics

**What it does:** the light that passes through smooth glass, crystals and gems concentrates on
the surfaces around them: bright spots under a glass, the light focused by a lens, the rainbows
of a pendant. Path tracing alone rarely finds these paths, especially with small lights; the
**caustics** option computes them with progressive photon mapping and works with every light,
also point lights, spots and the sun.

![Caustics off: the shadow of the glass sphere is uniform](img/comune/render_caustics_off.png) ![Caustics on: the sphere focuses the light into a bright spot](img/comune/render_caustics_on.png)

*Point light behind the glass sphere, 512 samples.*

| Control | Range | Default | What it does |
|---|---|---|---|
| Caustics | on / off | off | Turns photon mapping on |
| Photons per pass | 20k – 2000k | 200k | More photons = less noisy caustics at every pass, slower passes |
| Initial radius | 0 – 100 mm | 0 = automatic (1 % of the scene) | Radius in which a point gathers photons at the first pass; it shrinks by itself at every pass, so the caustics get sharper and sharper |

Caustics come from smooth glass (transmission, low roughness). Photon bounces are the render's
**Max bounces**. On the GPU the photon paths are built on the CPU and gathered by the GPU.

## RGB or Spectral

| Choice | What it does |
|---|---|
| RGB | Default. Three channels, red, green and blue: fast |
| Spectral | Light travels as wavelengths (4 per ray): **dispersion** in glass and jewels (rainbows), more faithful colors with saturated lights. Slower and with more color noise: it needs more samples |

Dispersion is set in the material (**Dispersion**, Abbe number): see
[Transmission](mat.transmission). It is visible only with the Spectral render; caustics and
spectral can be combined for the rainbows of prisms and gems.
