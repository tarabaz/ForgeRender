---
id: render.denoiser
titolo: Denoiser (noise reduction)
parole: noise, grain, grainy, clean, oidn, cuda, preview
ui: panel.render.denoiser
correlate: intro.benvenuto
---
# Denoiser (noise reduction)

**Where:** Advanced mode → **Render** tab → **Noise reduction (denoiser)** box. In Simple mode
it is always on with the best values.

**What it does:** a path tracer starts with a grainy (noisy) image, and the grain only goes away
with many samples. The denoiser (Intel Open Image Denoise) cleans the image by recognizing edges,
colors and surfaces: a few hundred samples are enough for a clean photo.

## Controls

| Control | Values | Default | What it does |
|---|---|---|---|
| Mode | Off · Final image only · Fast preview + final | Final image only (Simple: Fast preview + final) | When to clean: never, only at the end, or during the render too |
| Use CUDA (NVIDIA GPU) | on / off | on | Uses the NVIDIA graphics card, much faster; without one, the denoiser uses the processor by itself |
| Final quality | Fast · Balanced · High | High | Cleaning quality of the final image |
| Preview quality | Fast · Balanced · High | Fast | Only with "Fast preview + final" |
| First preview at | 1 to 1024 samples | 8 | First cleaning during the render, then at every doubling (8, 16, 32…) |
| Intensity | 0 to 100 % | 100 % (Simple: 90 %) | Mix between the raw image (0 %) and the cleaned one (100 %) |
| Preserve details (albedo and normals) | on / off | on | Uses surface colors and orientation so textures and edges are not smudged |
| View | Final · Raw · Albedo · Normals | Final | Shows the result or the data used by the denoiser, for checking |

## Tips

- **For the final photo:** *Final image only* or *Fast preview + final* mode, High quality,
  details preserved.
- **Intensity below 100 %:** keeps a trace of natural grain; the 90 % of Simple mode avoids the
  "plastic" look on matte surfaces.
- **Smudged textures:** check that **Preserve details** is on.

## Common mistakes

- **"Open Image Denoise not found":** the denoiser libraries are missing next to the program.
  The render still works, but stays grainy.
- **"CUDA not available on this GPU":** the card is not NVIDIA or the driver is old; the
  denoiser uses the processor, slower but with the same result.
