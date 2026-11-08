---
id: mat.texture
titolo: Textures, mapping and relief
parole: texture, map, maps, pbr, uv, box projection, normal map, bump, height, displacement, ambient occlusion, ao, opacity, cutout, scale, decal
ui: panel.materials.textures
correlate: mat.pannello, mat.roughness, fx.effetti
---
# Textures, mapping and relief

**Where:** Advanced mode → **Selection** tab (material of the selected part) or **Scene → Materials** → **Texture** section; with textures, also **Texture
scale and placement** and **Relief (normal, bump, displacement)**.

## Texture slots

| Slot | What it does |
|---|---|
| Base color | The image of the surface; it is multiplied by the base color (set to white when a map is loaded) |
| Roughness | Roughness map (red channel); the slider multiplies it. **It is a glossiness map (gloss)** inverts it |
| Metalness | Metalness map; the slider multiplies it |
| Normal map | Fine relief without changing the shape. **DirectX format** for maps with the green channel inverted (Unreal, Quixel, *_NormalDX) |
| Relief (height) | Height map: bump, or real displacement of the geometry |
| Occlusion (AO) | Darkens cavities; **AO strength** 0 – 1. The path tracer already computes real occlusion: low values are more faithful |
| Opacity (cutout) | Black parts become holes (leaves, perforated sheet, labels) |

## Placement

For CAD models, which usually have no UV coordinates, the **Box** projection places the image
from three sides in real units: you set how large one repetition of the texture is (in mm, cm
or m), the rotation and the offset. The anchor can be the **Piece**, the **Assembly** or
the **World**: with Piece the texture moves with the part.

## Relief

- **Normal map** and **bump** change only the light, not the outline: cheap and good for fine
  detail.
- **Displacement** really moves the surface (the outline changes). It is good for stones,
  bricks and bark; on smooth metals it creates bumps. In **Settings → Library and import** you
  choose whether new materials get it on import (off by default).

## Labels and logos

To put an image (logo, label) on a part only, use a **decal** in **Scene → Effects**: see
[Effects](fx.effetti).
