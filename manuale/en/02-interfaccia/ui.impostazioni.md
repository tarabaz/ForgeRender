---
id: ui.impostazioni
titolo: Settings
parole: settings, preferences, options, language, library folder, paths, new projects, status bar, viewport background, background color, anthracite, grid, metric grid, cell size, denoiser, previews
ui: edit.preferences, edit.reset_preferences
correlate: ui.finestra, mat.pannello, render.pannello
---
# Settings

**Where:** **Edit → Settings…**. **Edit → Restore default settings** brings every setting back
to its initial value.

The settings belong to the program, not to the project: they stay the same whatever you open.

| Section | What it holds |
|---|---|
| Language | Language of the whole interface (Italian, English, or a `lang/Lan_*.json` file added next to the program). It switches immediately. The manual follows it. |
| Viewport | **Viewport background**: Anthracite (default, RGB 40, 44, 52, #282C34), Blender-like grey (#30343B), Technical dark (#222831), Light grey (#353A42) or **Custom**: one click selects it, the custom colour (picked with the colour picker) is kept while you try the other presets. **Grid**: show or hide, unit (mm, cm, m), cell size (1, 5, 10, 20, 50, 100 mm, 50 cm, 1 m or a custom value), a major line every 5, 10, 20 or N cells, intensity. Background and grid change the viewport and the STEP preview immediately; the final render keeps its own background (environment and backdrop) and never shows the grid. |
| Library and import | Resolution of the downloads, proposed site (ambientCG or Poly Haven), what to do with the height map when importing (**Off (recommended)**; a proposed value of 3 % of the texture side; or the value of the source's MaterialX file), quality, samples and object size of the material previews, **Reduce noise** (the denoiser of the final render on the previews), **Clear preview cache** |
| Paths | Library folder (HDRI, materials, textures). The environment variable `FORGE_LIBRARY_DIR` takes precedence. |
| Rendering | Values for **new** projects: device, samples per pixel (0 = no limit), CPU threads (0 = automatic), automatic restart. Saved projects keep their own. |
| Status bar | Which counters to show at the bottom (up to 9). |
| Information | Version, **Show at startup** for the welcome screen, **Ask for the mode at startup** (Simple or Advanced). |

## Where files go

What you save or export goes by default into the `Rendering` folder next to the program, or into
`Documents\ForgeRender` if the program folder cannot be written.
