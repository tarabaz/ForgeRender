---
id: mat.roughness
titolo: Roughness
parole: rough, glossy, matte, satin, reflection, surface
ui: panel.materials.roughness
correlate: mat.ior
---
# Roughness

**Where:** Advanced mode → **Materials** tab → **Roughness** slider.

**What it does:** sets how smooth or rough the surface is at a microscopic level. A smooth
surface reflects a sharp image, like a mirror; a rough one scatters light and the reflection
becomes soft, until it disappears.

| Value | Look | Examples |
|---|---|---|
| 0.0 | Mirror, sharp reflection | chrome, glass, glossy paint |
| 0.2 – 0.4 | Glossy but soft | glossy plastic, finely brushed metal |
| 0.5 | Satin (default) | ordinary plastic, varnished wood |
| 0.8 – 1.0 | Matte | rubber, cardboard, plaster |

- **Range:** 0 to 1.
- **Default:** 0.5.
- **Effect on the render:** it changes only the reflections, not the color. It works together
  with **Metallic**: a metal with roughness 0 is a mirror, with roughness 0.6 it is sand-blasted
  metal.

## With a texture

If the **Roughness** texture slot holds a map, the slider is set to 1 and **multiplies** the
map: at 1 the map is used as it is, lower values make everything glossier. Some sites provide a
*gloss* map instead of a roughness map: in that case turn on **It is a glossiness map (gloss)**, which
inverts it.

## Common mistakes

- **Everything looks like plastic:** this happens with high roughness (0.7 and above) on objects
  that should shine. Lower it.
- **Glass looks frosted:** clear glass wants roughness close to 0; higher values give frosted
  glass, which is only wanted for that effect.

See also [IOR](mat.ior).
