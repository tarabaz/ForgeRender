---
id: ui.viewport
titolo: Viewport, navigation and gizmo
parole: viewport, navigate, orbit, pan, zoom, gizmo, move, rotate, scale, select, snap, orthographic, isometric, navigator, grid, metric grid, floor grid, measure
ui: view.viewport, view.ortho, view.isometric, view.frame_selection, view.frame_all, tool.select, tool.move, tool.rotate, tool.scale
correlate: scene.oggetto, keys.elenco, cam.obiettivo
---
# Viewport, navigation and gizmo

**Where:** central area, **Viewport** view.

The viewport is the real-time working view: it shows the shapes and the colors of the
materials, not the final light. The photo is in the **Render** view.

## Moving around

| Action | Effect |
|---|---|
| Drag with the left button | Orbit around the scene |
| Shift + drag, or right / middle button | Pan sideways |
| Wheel | Zoom in or out |
| Home | Frame all |
| F | Frame the selection |
| 5 | Perspective / orthographic |

The **axis navigator** in the corner (Blender style) shows X, Y and Z: click an axis to look
along it. Next to it are the icons for the orthographic and the **isometric** view (orthographic,
45° from the side). **View → Standard views** has front, back, sides, top and bottom.

## Metric grid

The floor (height 0) has a grid in real sizes: the scene is in metres, so a 1 cm cell is 1 cm
whatever the zoom. Every few cells a stronger major line; the red line is the X axis, the blue
one the Z axis. When the cells become too small on screen the minor lines fade out and the major
ones stay, so the grid stays readable from close up and from far away. It stays behind the
objects and does not get in the way of selection and gizmo. Unit, cell, major lines and
intensity are in **Edit → Settings… → Viewport**; the same grid is in the STEP import preview.
It never appears in the render.

## Selecting and transforming

Choose the tool in the toolbar or with the keys:

| Key | Tool |
|---|---|
| Q | Select |
| W | Move |
| E | Rotate |
| R | Scale |

- **Click** a part to select it, **Ctrl+click** to add or remove, **Esc** to deselect.
- Drag the **gizmo** arrows, rings or squares. Hold **Ctrl** to snap to fixed steps; the steps
  (move, rotation, scale) and **Global / Local** axes are in the tool row above the viewport.
- **Alt + drag** moves the selection instead of the camera.
- Lights, spot cone handles, effect boxes and fog emitters have their own gizmo too. Spot and
  directional lights are aimed by dragging the **diamond** of their target.
- **Undo / Redo:** Ctrl+Z, Ctrl+Y.

For exact numbers use the **Selection** tab: see [Move, rotate and resize](scene.oggetto).

## Hiding

**H** hides the selection, **Shift+H** hides everything else, **Alt+H** shows everything again.
Hidden objects are not rendered.
