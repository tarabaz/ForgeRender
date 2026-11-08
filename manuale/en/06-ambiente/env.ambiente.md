---
id: env.ambiente
titolo: Environment (uniform, gradient sky, HDRI)
parole: environment, background, sky, hdri, hdr, exr, gradient, uniform, intensity, rotation, image based lighting, ibl
ui: panel.environment
correlate: light.tipi, render.esposizione, mat.metallic
---
# Environment (uniform, gradient sky, HDRI)

**Where:** Advanced mode → **Scene → Environment**; **Scene → Environment and HDRI…**.

**What it does:** the environment is the light that comes from everywhere around the scene, as
from the sky or the walls of a room. It lights the objects, it is reflected by glossy surfaces
and, if you want, it is the background of the image.

| Type | Controls | Use |
|---|---|---|
| Uniform | Color, Intensity (0 – 10) | The same light from every direction: neutral, flat |
| Gradient sky | Zenith, Horizon, Ground colors, Intensity (0 – 10) | Brighter above, darker below: natural and soft. It is the default (intensity 0.45) |
| HDRI map | File (.hdr or .exr), Intensity (0 – 20), Rotation (−180° – 180°) | A real photographed environment: the most realistic light and reflections |

The intensity sliders are logarithmic: the first part of the slider gives finer steps.

![Gradient intensity 0.1: the lights dominate, deep shadows](img/comune/env_strength_010.png) ![Intensity 0.45 (default): balanced](img/comune/env_strength_045.png) ![Intensity 1.5: bright and flat, few shadows](img/comune/env_strength_150.png)

*Gradient sky with intensity 0.1, 0.45 and 1.5, same studio lights.*

## HDRI

- **Library HDRIs:** the HDRIs downloaded from Poly Haven or saved in your library are
  listed with a preview; **Open folder** shows where they are.
- **Rotation** turns the environment around the vertical axis: use it to move the reflections
  on the product or the direction of the sun of the HDRI.
- The renderer samples the bright areas of the HDRI on purpose (importance sampling): sun and
  windows give clean shadows without extra grain.
- For interiors lit through a window add a **Portal** light on the window: see
  [Light types](light.tipi).

## Environment visible as background

Off, the background is black (or the solid color of the [render style](render.stili)) but the
environment **keeps lighting** the scene and appearing in the reflections. Useful to use an HDRI
for the light only.

## Tips

- Glossy metals and glass need something to reflect: an HDRI makes them come alive much more
  than a uniform color.
- Too much environment flattens the image (no shadows); too little leaves the shadows black.
