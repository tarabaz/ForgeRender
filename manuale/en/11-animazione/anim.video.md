---
id: anim.video
titolo: Saving the animation and video settings
parole: save animation, video, mp4, mov, mkv, avi, h264, h265, prores, ffv1, codec, crf, bitrate, ffmpeg, frames, export video, background export, resume
ui: anim.save, anim.video_settings
correlate: anim.timeline, export.viste, render.campioni
---
# Saving the animation and video settings

**Where:** Timeline → **Save animation**.

| Choice | What it saves |
|---|---|
| Render every frame | The photorealistic render of each frame of the work area, with the current samples |
| Preview (viewport) | A quick viewport recording, to check the motion (smoke and fog are not in it) |
| View | The viewport or a render camera |
| Also the video | Besides the PNG frames, a video file |

**Save** writes the frames into a new folder inside **Rendering** for every animation.

## Video settings

The gear (**Video settings…**) opens container, codec, compression (constant quality **CRF** or
**bitrate**), pixel format, frame rate, resolution and destination file.

| Preset | Result |
|---|---|
| Standard | MP4: small files, high compression, to share the video |
| High quality | MP4 with light compression: files about three times larger |
| Very high | MP4 with minimal compression: almost no visible defect |
| For editing | ProRes in MOV: large files, smooth to edit |
| Lossless | FFV1 in MKV: every pixel identical to the frames, for archiving |
| Uncompressed | AVI: huge files, read by any editing program |

MP4 H.264 and uncompressed AVI work out of the box; the other formats need **ffmpeg** next to the
program or in the PATH, and are disabled without it. The video file appears only when finished,
and the notice at the end reports the real parameters.

## Long exports

- **Maximum speed** uses all the power; **Keep the PC usable** gives the render a low priority
  and pauses on the GPU, so other programs stay smooth (the render takes longer).
- The export **keeps going with the window minimized** and the PC does not go to sleep meanwhile.
- An export that stopped **resumes from the frames already done**. Closing during an export asks
  first.
- The status bar counts frames and estimates the time left for the render and then for the
  encoding.
