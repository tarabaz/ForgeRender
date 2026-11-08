---
id: render.esposizione
titolo: Exposure and tone mapping
parole: exposure, ev, stops, brightness, too dark, too bright, tone mapping, aces, filmic, standard, clamp, highlights, contrast
ui: panel.render.display
correlate: env.ambiente, light.tipi, export.immagine
---
# Exposure and tone mapping

**Where:** Render tab → **Display (does not restart the render)**.

Both settings change only how the computed light becomes the colors of the image: they apply at
once, without restarting the render, and they are applied to the exported PNG, JPG and TIFF.

## Exposure (EV)

**What it does:** brightens or darkens the whole image, like the exposure of a camera. Each
step (EV, *stop*) doubles or halves the light.

![Exposure −1 EV: half the light](img/comune/render_exposure_minus1.png) ![Exposure 0 EV: as computed](img/comune/render_exposure_0.png) ![Exposure +1 EV: twice the light](img/comune/render_exposure_plus1.png)

- **Range:** −6 to +6 EV.
- **Default:** 0.
- **Use it** to correct the whole image. If only one area is wrong, change the lights instead.

## Tone mapping

**What it does:** the light computed by the renderer has no upper limit (a softbox is thousands
of times brighter than a shadow), while an image goes from black to white. Tone mapping decides
how to squeeze it.

![Standard (clamp): saturated colors, highlights cut to white](img/comune/render_tonemapping_standard.png) ![ACES filmic: highlights roll off softly, more natural](img/comune/render_tonemapping_aces.png)

*Both at +0.5 EV.*

| Choice | Effect |
|---|---|
| ACES filmic | Default. A film-like curve: highlights fade gently to white, colors of very bright areas desaturate as on film |
| Standard (clamp) | No compression: what is above white is cut. Colors more saturated and contrasty, highlights burnt sooner |

## Tips

- Keep **ACES** for photos with bright reflections, glass and lights in view.
- Use **Standard** when the exact color of a surface matters more than the highlights (color
  samples, flat illustrations).
- The **EXR** export stores linear light (with the exposure, without tone mapping): the curve
  can be decided later in a compositing program.
