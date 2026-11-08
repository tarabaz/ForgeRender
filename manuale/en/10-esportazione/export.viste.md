---
id: export.viste
titolo: Views and turntable
parole: views, multiple views, turntable, spin, rotation video, 360, front, side, three quarter, top, batch
ui: render.sequences
correlate: export.immagine, anim.video, cam.obiettivo
---
# Views and turntable

**Where:** **File → Views and turntable…** (also in the Rendering menu).

## Multiple views

Renders the product from several automatic points of view, one image each: **Front**, **Three
quarters right**, **Right**, **Back**, **Left**, **Three quarters left**, **From above**.

| Control | What it does |
|---|---|
| Views | Tick the ones you want (at least one) |
| Frame the product (always fully inside) | Each view frames the whole product |
| Samples per image | Quality of each image |
| Render passes for every image | Also albedo, normals, depth, product mask and part IDs for each view |
| File name, Folder… | Where and with which name the images are saved |
| Start / Stop | Stopping keeps the images already done; the scene goes back as it was |

## Turntable (video)

The product turns on itself while camera, lights and backdrop stay still.

| Control | What it does |
|---|---|
| Rotation between two images | Smaller = smoother movement and more images to compute |
| Frames per second, Duration of one turn | Changing the duration recomputes the rotation between images |
| Turns in the video | The turn is repeated without rendering again: the video loops without jumps |
| Direction | Counter-clockwise or clockwise |
| Samples per image | Below 256 samples the video may flicker |
| Camera | The current view, or **Create a camera from this view** to keep the framing in the project |
| Create the video, Video settings… | MP4 and other formats: see [Saving the animation](anim.video) |
