---
id: export.immagine
titolo: Saving the image and render passes
parole: export, save image, png, jpg, jpeg, tiff, exr, 16 bit, passes, albedo, normals, depth, mask, alpha, cut-out, part id, compositing
ui: file.export, file.export_passes
correlate: render.esposizione, export.viste, render.stili
---
# Saving the image and render passes

## Export image

**Where:** **File → Export image…** (Ctrl+E); **Save image…** in Simple mode.

It saves the current render (what you see in the Render view, with the denoiser, exposure and
tone mapping). The format follows the extension chosen in the window:

| Format | Use |
|---|---|
| PNG (lossless, 8-bit) | Default: web, documents, no loss of quality |
| JPG (small, for web and social media) | Small files (quality 92) |
| 16-bit TIFF (for photo editing) | More shades per color: no banding when you edit the image a lot |
| EXR (linear values, for compositing) | The light as computed, with values above white, without tone mapping |

Files go by default into the `Rendering` folder next to the program (or `Documents\ForgeRender`
if the program folder cannot be written).

## Render passes

**Where:** **File → Export render passes…**.

Technical images of the last render, for post-production. They are not additional renders: they
describe the scene pixel by pixel.

| Pass | What it contains |
|---|---|
| Albedo | The color of the materials without lighting |
| Normals | The orientation of the surfaces |
| Depth | Distance from the camera: 16-bit PNG (white = near) and EXR in metres |
| Product mask | White where the product is: to cut it out from the background |
| Part IDs | One color per part, to select the parts in an image editor |

## Tips

- For a product on **pure white**, use a solid white background (see
  [Render styles](render.stili)) or cut it out with the product mask.
- For prints and heavy retouching prefer **16-bit TIFF**.
