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
| Menu bar | top | File, Edit, Scene, Objects, View, Rendering, Library, Tools, Help |
| Toolbar | under the menus | Simple / Advanced switch, the most used commands, **Add** menu, **Start render**, **AI MCP** switch |
| Scene panel | left | the tree of the components of the model, then render cameras, lights and effects |
| Central area | middle | **Viewport** (real-time view) or **Render** (the path traced image) |
| Settings panel | right | Simple mode: the 5 steps. Advanced mode: the tabs Object, Materials, Lights, Environment, Effects, Camera, Render |
| Timeline / Console | bottom | animation keys (see [Timeline](anim.timeline)) and program messages |
| Status bar | very bottom | samples, time, time remaining, effects being computed, device |

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
widening it. In the Scene panel a click selects a component (Ctrl or Shift adds it to the
selection), the box next to the name hides or shows it, the right click has more commands.

## Status bar

It shows the progress of the render (samples done of the target, elapsed time, time remaining),
the computation of effects such as fog and smoke, and on the right the **backend** and the
processor. The counters are configurable in **Edit → Settings… → Status bar**: up to 9 of 15
(viewport FPS, frames, time remaining, GPU, VRAM, RAM and others).
