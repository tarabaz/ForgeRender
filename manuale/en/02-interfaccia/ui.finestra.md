---
id: ui.finestra
titolo: The main window
parole: window, layout, menu, toolbar, panels, status bar, tree, components, tabs
ui: ui.window, ui.statusbar
correlate: ui.viewport, ui.impostazioni, intro.semplice
---
# The main window

| Area | Where | What it holds |
|---|---|---|
| Menu bar | top | File, Edit, Scene, Objects, View, Rendering, Library, Tools, Help; on the right the **Simple / Advanced** switch and the gear for **Settings** |
| Toolbar | under the menus | on the left New, Open, Save, Import and the **Add**, **Lights**, **Camera**, **Environment**, **Material** menus; on the right **Start render**, Stop, Restart, Export and the **AI MCP** switch |
| Scene panel | left | one tree: the objects of the model, then the **Lights**, **Cameras** and **Effects** groups |
| Central area | middle | the **Viewport** and **Render** tabs on top, with **Auto restart** on the right |
| Settings panel | right | Simple mode: the 5 steps. Advanced mode: the **Selection**, **Scene** and **Render** tabs |
| Bottom area | bottom | render status and counters; **Timeline** and **Console**, closed to one row until there is an animation |

## Menus

- **File:** new empty scene, demo scenes, open, recent projects, save, save as, save a complete
  package (`.frpack`), a folder or a ZIP with all files, import model, export image, export
  render passes, views and turntable, exit.
- **Edit:** undo, redo, duplicate, delete, hide, show all, **Settings…**, restore default
  settings.
- **Scene:** prepare product scene, fit the scene to the product, undo preset, look, place on
  the base, frame the product, environment, effects, add a light.
- **Objects:** position and dimensions, materials, lights.
- **View:** frame all, frame selection, perspective / orthographic, isometric, standard views,
  Viewport or Render, camera and lens.
- **Rendering:** start / pause / resume (F5), stop, restart, compute device, quality, format,
  automatic restart, render settings and denoiser, views and turntable, export.
- **Library:** my library, download from ambientCG / Poly Haven, import a material from
  Blender, library folder.
- **Tools:** AI MCP switch and **AI integration…** (see [AI control](ai.mcp)).
- **Help:** manual (F1), keyboard shortcuts, welcome and about.

## Panels

Panels can be resized by dragging their edges. Long rows scroll inside the panel instead of
widening it.

**Scene panel.** A click on an object selects it (Ctrl or Shift adds it to the selection), the
box next to the name hides or shows it, the right click has more commands. Below the objects the
**Lights**, **Cameras** and **Effects** groups open and close with the arrow; the **+** on the
right adds a light, a camera or an effect. What is selected is highlighted in orange.

**Settings panel (Advanced mode).** It has three tabs:

- **Selection** shows what you picked in the viewport or in the tree, with no need to look for
  the right tab: for an object its position and size and, below, its material; for a light,
  a camera or an effect its own properties.
- **Scene** holds what applies to the whole scene, in sub-tabs: **Lights** (presets and new
  lights), **Environment**, **Effects**, **Camera** and **Materials** (all the materials of the
  scene, also with nothing selected).
- **Render** holds the render settings. The technical values (Russian roulette, indirect light
  limit, random seed, CPU threads) are in the closed **Advanced** section.

**Bottom area.** With no animation the timeline stays closed to one row: click **Timeline** or
**Console** to open it, **Collapse** to close it again. It opens by itself when the timeline has
something.

## Status bar

It shows the progress of the render (samples done of the target, elapsed time, time remaining),
the computation of effects such as fog and smoke, and on the right the **backend** (the
processor is in its tooltip). The counters are configurable in **Edit → Settings… → Status bar**: up to 9 of 14
(viewport FPS, frames, time remaining, GPU, VRAM, RAM and others).
