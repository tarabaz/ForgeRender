---
id: cam.dof
titolo: Depth of field, aperture and focus
parole: depth of field, dof, aperture, f-stop, f number, focus, blur, bokeh, blades, sharp zone, background blur
ui: panel.camera.dof
correlate: cam.obiettivo, cam.tiltshift
---
# Depth of field, aperture and focus

**Where:** Camera tab → **Depth of field: on**.

**What it does:** a real optical blur (thin lens): sharp on the focus plane, blurred in front of
it and behind it. It detaches the product from the background, as in a real photo.

## Aperture

**Aperture** is the f-number: **lower = more blur** (narrow sharp zone), **higher = sharper**
everywhere. The list has f/1.2, 1.4, 2, 2.8, 4, 5.6, 8, 11, 16, 22 and 32; the field accepts
from 0.7 to 64.

![f/1.2: background strongly blurred](img/comune/cam_fstop_12.png) ![f/4: moderate blur](img/comune/cam_fstop_40.png) ![f/16: almost everything sharp](img/comune/cam_fstop_160.png)

*50 mm lens focused on the copper ring, f/1.2, f/4 and f/16.*

## Focus

| Control | What it does |
|---|---|
| Focus | Distance of the sharp plane from the camera, in metres |
| Pick with a click | Click the point to focus in the viewport |
| On the selection | Focuses on the center of the selected parts |
| Sharp zone | Near and far limits of what looks sharp, computed as on a full frame camera (circle of confusion 0.03 mm); they are also drawn in the viewport |
| Blades | Shape of the bokeh: blurred highlights become polygons, as in real lenses, with the **rotation** of the diaphragm |

## How much blur?

Blur grows with: a **lower f-number**, a **longer focal length**, a **closer** subject, and a
background **further** behind it. A small product photographed from close by gets a lot of blur
even at f/4; a whole room needs f/1.4 to show any.

## Tips

- Turn on the depth of field only when you need it: the image needs more samples to become
  clean in the blurred areas.
- For a sharp band on the finished image, without a lens, use the
  [tilt-shift](cam.tiltshift).
