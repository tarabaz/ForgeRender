---
id: cam.obiettivo
titolo: Camera, lens and focal length
parole: camera, lens, focal length, mm, zoom, field of view, fov, wide angle, telephoto, perspective, orthographic, render camera, shift
ui: panel.camera
correlate: cam.dof, cam.tiltshift, ui.viewport, export.viste
---
# Camera, lens and focal length

**Where:** Advanced mode → **Scene → Camera** (a selected render camera also shows in the **Selection** tab); **View → Camera and lens…**.

## Viewport camera and render cameras

The image is taken from the **viewport view** or from a **render camera**. **+ New camera from
view** saves the current point of view as a camera; cameras are listed in the Scene panel,
above the lights. A selected camera shows a line to its target (a diamond): the gizmo moves
camera and target together, **Rotate** orbits the camera around the target, and after clicking
the diamond only the target moves. **Aim at object** makes the camera frame an object, focus
on it and follow it when it moves.

Quick views: **Front**, **Right**, **Top**, **Isometric**; **Frame all**, **Frame selection**.

## Lens (focal length)

**Lens** sets the focal length in millimetres, full frame equivalent (36 × 24 mm sensor), from 4
to 2000 mm; the buttons give 14, 24, 35, 50, 85, 135 and 200 mm. Next to it you see the
vertical angle of view.

![24 mm: wide angle, strong perspective](img/comune/cam_focal_024.png) ![50 mm: natural perspective](img/comune/cam_focal_050.png) ![135 mm: telephoto, flattened perspective](img/comune/cam_focal_135.png)

*The camera moves back as the focal length grows, so the objects keep about the same size: what
changes is the perspective.*

| Focal length | Effect | Use |
|---|---|---|
| 14 – 24 mm | Wide angle: near objects big, far ones small, lines that converge | Interiors, dramatic shots |
| 35 – 50 mm | Similar to the human eye | General use |
| 85 – 135 mm | Telephoto: flat perspective, background close | Product photos without distortion |
| 200 mm and more | Very flat, almost orthographic | Technical-looking product shots |

## Projection

- **Perspective:** like a camera, far objects are smaller.
- **Orthographic:** no perspective, as in a technical drawing; **Frame height** is automatic
  (follows the zoom) or set in metres.

## Shift

**Shift** (horizontal and vertical, −100 % to 100 %) moves the image without turning the camera,
like the shift lens of architecture photographers: vertical lines stay vertical when you frame
a tall object from below.
