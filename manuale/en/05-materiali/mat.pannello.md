---
id: mat.pannello
titolo: Materials panel and library
parole: material, materials, library, assign, apply, replace, preview, new material, duplicate, default material, emission, color picker, ral, ambientcg, poly haven
ui: panel.materials, library.mine, library.online, library.import_blend, library.folder
correlate: mat.roughness, mat.metallic, mat.transmission, mat.texture
---
# Materials panel and library

**Where:** Advanced mode → **Selection** tab (the material of the selected part) or **Scene → Materials**; **Library** menu; in Simple mode **Open the
material library**.

## Assigning materials

1. Select one or more parts (click in the viewport or in the Scene panel).
2. Click a material in the **Material library…** or in the list of the scene materials, then
   **Assign to selection**.

| Command | What it does |
|---|---|
| Assign to selection | The selected parts take the material |
| Remove assignment | The part goes back to inheriting the material of its parent or the default one |
| Name contains | Assigns the material to every part whose name contains the text |
| Use as default for components without an assignment | The scene default material |
| Replace [material] with «selected» | Every part with that material (also inherited, and the default) switches to the selected one; **Undo replacement** brings it back |
| New / Duplicate / Delete | Your own materials (in the *Custom* category) |

## The library

- **Built-in library:** 125 ready materials in 13 categories (metals, anodised aluminium, paints,
  plastics, rubbers, glass, liquids, ceramics and stone, woods, fabrics and leather, asphalt,
  roofing, emissive). An additional pack adds 109 textured materials.
- **My library:** materials with textures and HDRIs saved on the computer.
- **Download from ambientCG / Poly Haven…:** free materials and HDRIs (CC0).
- **Import material from Blender…:** maps and values of a `.blend` material, with preview.
- Every preview is rendered with the path tracer, so it shows the real material.

## Material values

| Control | Range | Default | Page |
|---|---|---|---|
| Base color | color | 0.8 grey | the color of the surface (RGB, CMYK or palette, RAL included) |
| Metallic | 0 – 1 | 0 | [Metallic](mat.metallic) |
| Roughness | 0 – 1 | 0.5 | [Roughness](mat.roughness) |
| Specular | 0 – 1 | 0.5 | [Specular](mat.specular) |
| IOR | 1 – 3 | 1.5 | [IOR](mat.ior) |
| Transmission | 0 – 1 | 0 | [Transmission](mat.transmission) |
| Emission and Emission strength | color, 0 – 10000 | black, 0 | the surface gives off light and lights the scene (LEDs, displays) |
| Textures | maps | none | [Textures and relief](mat.texture) |

The **color picker** opens by clicking a color swatch: **RGB** (wheel, values, hex code),
**CMYK** and **Palette** (RAL Classic with indicative values, or your own `.ase`, `.json`,
`.csv` palettes). **Apply** keeps the color, **Cancel** or Esc restores the previous one.
