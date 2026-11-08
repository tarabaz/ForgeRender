---
id: scene.oggetto
titolo: Move, rotate and resize (Object tab)
parole: object, position, dimensions, size, move, rotate, scale, drop, floor, duplicate, delete, snap, contact
ui: panel.object, edit.duplicate, edit.delete, edit.hide, edit.isolate, edit.show_all
correlate: ui.viewport, scene.importa, mat.pannello
---
# Move, rotate and resize (Object tab)

**Where:** Advanced mode → **Object** tab, with one or more components selected.

| Control | What it does |
|---|---|
| Duplicate / Delete | Copies or removes the selection (Delete removes it from the project, not from the imported file; Ctrl+Z brings it back) |
| Drop | Places the selection exactly on the floor or on what is under it |
| Center at the origin | Moves the center to X = 0, Z = 0 |
| Reset | Back to the position of the original file |
| Contact snap | While moving, the object stops when it touches another one |
| Rotate 90° | Quick rotations around X, Y and Z |
| Dimensions (mm) | Real size of the selection; with **Keep proportions** the other sides follow |
| Offset (mm) | Position |
| Rotation | Angles in degrees |
| Scale | Factor; quick buttons ×0.1, ×10 and ×25.4 (inches to millimetres) |

All values can be undone with Ctrl+Z. The same moves are possible in the viewport with the gizmo:
see [Viewport](ui.viewport).

## Hide, duplicate, delete

| Keys | Command |
|---|---|
| Ctrl+D | Duplicate the selection |
| Delete | Delete the selection |
| H | Hide the selection |
| Shift+H | Hide everything else |
| Alt+H | Show all |

## Tip

**Scene → Place on the base** and **Scene → Fit the scene to the product** work on the whole
product: they are the fastest way to rest it on the backdrop and resize lights and backdrop
around it.
