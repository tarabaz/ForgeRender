---
id: render.pannello
titolo: Render panel, device and resolution
parole: render settings, device, gpu, cpu, graphics card, processor, resolution, size, pixels, threads, rt cores, start, stop, restart, automatic restart
ui: panel.render, render.stop, render.restart, render.auto_restart
correlate: render.campioni, render.denoiser, render.rimbalzi, render.esposizione
---
# Render panel, device and resolution

**Where:** Advanced mode → **Render** tab; **Rendering → Render settings and denoiser…**.

From top to bottom the tab holds: [render style and outlines](render.stili), the render camera,
the device, color (RGB or spectral) and [caustics](render.caustiche), resolution,
[samples per pixel](render.campioni), [path tracing](render.rimbalzi),
[display](render.esposizione) and [noise reduction](render.denoiser).

## Device

| Device | Notes |
|---|---|
| GPU (compute shader) | Default. The graphics card through Vulkan or DirectX 12. When the card has ray tracing units (**RT cores**) the panel says so: the rays are traced in hardware, same result, many times faster |
| CPU (all cores) | Works on every PC. **CPU threads** (in **Path tracing → Advanced**): 0 = automatic (logical threads − 1, to keep the interface responsive) |

If the GPU cannot be used, the panel says why and the render runs on the CPU. CPU and GPU give
the same image.

## Resolution

Buttons **HD** (1280 × 720), **Full HD** (1920 × 1080), **QHD** (2560 × 1440), **4K**
(3840 × 2160), **Square** (1080 × 1080), **Preview** (640 × 360), or any size from 16 to 16384
pixels per side. Twice the width and height means four times the pixels and about four times
the time for the same quality.

## Start, pause, stop

| Command | Keys | What it does |
|---|---|---|
| Start render / Pause / Resume | F5 | Starts, pauses or resumes the path tracing |
| Stop | | Stops and keeps the current image |
| Restart | | Starts again from zero samples |
| Automatic restart | | When materials, lights or camera change, the render starts again by itself (only in the Render view). The box is above the central area, on the right |

Some settings do not restart the render: the number of samples, exposure, tone mapping and the
denoiser apply to the image being computed. The others (resolution, bounces, clamp, seed,
color mode, caustics, style) need a restart: the panel says so.
