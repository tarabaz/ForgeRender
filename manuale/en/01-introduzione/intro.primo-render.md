---
id: intro.primo-render
titolo: Your first render
parole: quick start, first, steps, beginner, tutorial, how to start
ui: render.start
correlate: intro.semplice, scene.importa, export.immagine
---
# Your first render

Five steps from a 3D file to a finished photo. They work the same in Simple and Advanced mode.

1. **Import the model:** **File → Import model…** (Ctrl+I). glTF/GLB, OBJ, STL, STEP and
   Blender files are accepted; see [Import](scene.importa).
2. **Prepare the scene:** **Scene → Prepare product scene** places the product on the base, adds
   a backdrop if there is none, applies the chosen look and quality and frames the product. It
   can be undone with **Undo preset**.
3. **Assign the materials:** click a part in the viewport, open the **material library** and
   click a material. See [The Materials panel](mat.pannello).
4. **Start the render:** **Rendering → Start render** (F5). The image starts grainy and becomes
   cleaner pass after pass; at the end the denoiser removes the remaining grain.
5. **Save the image:** **File → Export image…** (Ctrl+E): PNG, JPG, 16-bit TIFF or EXR. See
   [Saving the image](export.immagine).

## Viewport and Render view

The central area has two views:

- **Viewport:** the real-time working view. Here you move objects, lights and camera.
- **Render:** the image of the path tracer, which is the real photo.

With **Automatic restart** on, every change to materials, lights or camera restarts the render
by itself, but only in the Render view: in the Viewport you can work without it starting.
**Stop** keeps the current image, **Restart** starts again from zero samples.

## How long does it take?

It depends on the number of **samples per pixel** (see [Samples](render.campioni)): 128–256
samples with the denoiser are enough for most photos; glass, liquids and dark scenes need more.
The status bar at the bottom shows samples, time and time remaining.
