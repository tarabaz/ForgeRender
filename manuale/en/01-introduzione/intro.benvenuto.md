---
id: intro.benvenuto
titolo: Welcome to ForgeRender
parole: introduction, start, guide, manual, help, contents
ui: help.manual, help.welcome
correlate: intro.primo-render, intro.semplice, keys.elenco
---
# Welcome to ForgeRender

ForgeRender makes realistic **product photos** from a 3D model: you import the model, choose a
backdrop and lights, assign materials and start the render. The engine is a *path tracer*: it
simulates the path of light, bounce after bounce, just like in a real photo studio. It runs on the
graphics card (GPU) or on the processor (CPU).

## How to use this manual

- **Contents** on the left: the chapters and, inside each one, a page for every feature.
- **Search** at the top: type a word, even in everyday language ("matte", "grain", "move"), and
  you get the right page. Accents and a single typo do not matter.
- **F1** over an interface control opens its page; if the control has none, the page of the
  active panel opens.
- **Back / Forward** go through the pages you have already seen, **Home** brings you here.
- **Comparison images:** the pages about the main settings show the same scene rendered with
  the slider low, medium and high. Hover over an image to see it full size.

## Chapters

| Chapter | What you find |
|---|---|
| Introduction | [Your first render](intro.primo-render), [Simple mode](intro.semplice) |
| Interface | [The window](ui.finestra), [the viewport](ui.viewport), [the settings](ui.impostazioni) |
| Scene and objects | [Import](scene.importa), [move and resize](scene.oggetto), [projects and packages](scene.progetti) |
| Lights | [Light types](light.tipi), [size and shadows](light.dimensione), [light color](light.colore) |
| Materials | [The Materials panel](mat.pannello), [Roughness](mat.roughness), [Metallic](mat.metallic), [Specular](mat.specular), [IOR](mat.ior), [Transmission](mat.transmission), [Textures](mat.texture) |
| Environment | [Uniform, gradient sky, HDRI](env.ambiente) |
| Effects | [All effects](fx.effetti), [ambient haze](fx.foschia) |
| Camera | [Lens and focal length](cam.obiettivo), [depth of field](cam.dof), [tilt-shift](cam.tiltshift) |
| Rendering | [The Render panel](render.pannello), [samples](render.campioni), [denoiser](render.denoiser), [bounces](render.rimbalzi), [exposure](render.esposizione), [caustics and spectral](render.caustiche), [styles](render.stili) |
| Saving images | [Image and passes](export.immagine), [views and turntable](export.viste) |
| Animation | [Timeline](anim.timeline), [saving the animation](anim.video) |
| AI control | [MCP](ai.mcp) |

## Simple or Advanced

Choose the mode at the top right:

- **Simple**: a guided panel in 5 steps (product, look, materials, format and quality, render).
  The technical settings are already tuned for a good result. See [Simple mode](intro.semplice).
- **Advanced**: every control, in the Selection (what you picked), Scene (lights, environment,
  effects, camera, materials) and Render tabs.

The values are the same in both modes: you can prepare the scene in Simple and refine it in
Advanced.

> The comparison images were rendered by ForgeRender itself on the graphics card, on the
> program's test scene and on the material preview ball, at 640 × 480 pixels and at least 256
> samples (except the comparisons of the number of samples and of the noise threshold).
