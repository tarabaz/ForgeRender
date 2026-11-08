---
id: cam.tiltshift
titolo: Tilt-shift
parole: tilt-shift, tilt shift, miniature, sharp band, blur band, selective focus
ui: panel.camera.tiltshift
correlate: cam.dof, cam.obiettivo
---
# Tilt-shift

**Where:** **Scene → Camera** → **Tilt-shift**.

**What it does:** a sharp band on the final image with the blur growing on both sides, like the
tilt-shift lenses that make real scenes look like miniatures. It is **separate from the depth of
field**: aperture, focal length and focus do not change, and with both on the band adds to the
lens blur. Turning it on restarts the render (it needs the depth of each pixel); every other
value shows at once on the image, without rendering again.

| Control | Values | What it does |
|---|---|---|
| Position | −0.5 – 1.5 | Where the sharp band passes: 0 = top edge, 0.5 = center, 1 = bottom edge (left / right if the band is vertical) |
| Angle | −90° – 90° | 0° = horizontal band, 90° = vertical, in between tilted; buttons **Horizontal** and **Vertical** |
| Sharp width | 0 – 1 | Width of the fully sharp band, as a fraction of the image height (0 = only a line) |
| Upper / Lower transition | 0 – 1 | Space needed to go from sharp to fully blurred on each side (0 = hard edge); **Equal transitions** copies one to the other |
| Blur | 0 – 5 % | Maximum blur at the edges, as a radius in percent of the image width |
| Depth aware | on / off | A near sharp object does not bleed into the blurred background (no halos at the edges) |

The blur is gathered from progressively reduced copies of the image, so bright reflections
become a soft blur instead of repeated copies. It works the same on CPU and GPU.
