---
id: render.stili
titolo: Render styles, outlines and background
parole: style, cartoon, toon, cel shading, technical illustration, gooch, outlines, ink, lines, black and white, background, solid background, white background
ui: panel.render.style
correlate: render.pannello, env.ambiente, export.immagine
---
# Render styles, outlines and background

**Where:** at the top of the Render tab. Hovering over each style shows a small example.

![Photorealistic](img/comune/render_style_photoreal.png) ![Cartoon](img/comune/render_style_toon.png) ![Technical illustration](img/comune/render_style_technical.png) ![Photorealistic with ink outlines](img/comune/render_outlines_on.png)

| Style | What it does |
|---|---|
| Photorealistic | Physically based path tracing: reflections, refractions, soft shadows and bounced light |
| Cartoon | Flat material colors in three sharp bands (light, half shadow, shadow) from the scene lights, with their hard shadows and a crisp highlight on shiny materials |
| Technical illustration | Soft shading from warm in the light to cool in the shadow, with no black shadows, so shapes read clearly |

**Black and white:** the cartoon in greys, the technical illustration like a pencil drawing.

The styles act on the first surface seen by the camera, so glass and mirrors still reflect and
refract. They work on CPU and GPU, also with the Spectral render.

## Solid background

Where the camera sees nothing, a color (**White**, **Light grey**, **Black** or any) instead of
the environment, which keeps lighting the scene. Good for catalog photos on pure white.

## Ink outlines

Dark lines on silhouettes, borders between objects and depth jumps, drawn over the final image
(also after the denoiser), with any style.

| Control | What it does |
|---|---|
| Thickness | Line width in pixels (default 1.5) |
| Color | Line color |
| Sharp edges too | Lines also where two faces of the same object make a sharp angle; **Edge angle** sets from which angle (default 35°, lower = more lines) |
| Depth jump | Sensitivity to depth jumps inside the same object (default 3 %, lower = more lines) |
| Cleanup | Removes line pieces shorter than this (pixels) |
| No line on the floor edge | Leaves the edge of floor and backdrop against the background without a line |
