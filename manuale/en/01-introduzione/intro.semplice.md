---
id: intro.semplice
titolo: Simple mode (product photo in 5 steps)
parole: simple, easy, guided, wizard, steps, product photo, look, format
ui: mode.simple, scene.prepare, scene.fit, scene.undo_preset, scene.drop_to_base, scene.frame_product
correlate: intro.primo-render, light.colore, render.campioni
---
# Simple mode (product photo in 5 steps)

**Where:** **Simple** switch at the top right. It is the default mode.

**What it does:** a single column with large buttons that takes you from the model to the photo.
Every choice writes the same values you find in Advanced mode, so you can switch at any time.

## 1. Product

- **Import model…** opens the 3D file.
- **Place on the base** puts the product exactly on the floor or the backdrop.
- **Frame the product** frames it without floor and backdrop.
- **Fit the scene to the product** remakes backdrop and lights on the current size of the
  product.

## 2. Look

| Look | Result |
|---|---|
| Light background | White backdrop and soft light: catalog or e-commerce photo |
| Dark background | Glossy dark backdrop and three-point lighting: the product stands out from black |
| Dramatic studio | Warm side light and cool backlight: an effect photo |
| Photo studio | Tilted cloth backdrop and four studio lights, as in a real studio |

**Light color** sets all the lights at once: **Warm** (3200 K, yellow like a bulb), **Neutral**
(5500 K, studio white), **Cool** (7500 K, bluish). See [Light color](light.colore).

**Prepare product scene** applies the look: it places the product, adds the backdrop if missing,
applies look and quality and frames. **Undo preset** brings lights, environment, quality,
backdrop and product position back to how they were before the last preset. If you change lights
or environment by hand, the look shows as *Custom*.

## 3. Materials

Click a part in the view, then **Open the material library** and click a material: it is applied
to the selected parts.

## 4. Format and quality

| Format | Size |
|---|---|
| Landscape | 16:9, 1920 × 1080 |
| Square | 1:1, 1080 × 1080 (Instagram) |
| Portrait | 4:5, 1080 × 1350 (Instagram) |
| Stories | 9:16, 1080 × 1920 |

| Quality | Samples | Bounces | When |
|---|---|---|---|
| Draft | 128 | 8 | To try framing and materials |
| Balanced | 256 | 12 | Good for most photos |
| High | 1024 | 16 | Final photo: glass and reflections cleaner |

**Compute device:** **Graphics card (GPU)**, much faster with a recent card, or **Processor
(CPU)**, which works on every PC. If no usable GPU is found the GPU button is disabled.

## 5. Render

**Start render** (then **Pause** / **Resume**) and **Save image…** (PNG, JPG, 16-bit TIFF or
EXR). In Simple mode the denoiser is always on with the best values.
