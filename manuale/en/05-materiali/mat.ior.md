---
id: mat.ior
titolo: IOR (index of refraction)
parole: refraction, index, glass, water, diamond, transparency, reflection
ui: panel.materials.ior
correlate: mat.roughness
---
# IOR (index of refraction)

**Where:** Advanced mode → **Materials** tab → **IOR** slider.

**What it does:** sets how much light bends when it enters the material and how much the
material reflects when seen head-on. It matters most for transparent materials
(**Transmission** above 0) and for the reflections of non-metallic surfaces.

| Material | Typical IOR |
|---|---|
| Air | 1.0 |
| Water | 1.33 |
| Plastic, ordinary glass | 1.45 – 1.5 |
| Lead glass, crystal | 1.6 – 1.7 |
| Diamond | 2.42 |

- **Range:** 1 to 3.
- **Default:** 1.5 (plastic and ordinary glass).
- **Effect on the render:** with a high IOR transparent objects distort what is behind them more
  and their edges reflect more. With **Specular** at 0.5 (default) the reflection of opaque
  surfaces follows the IOR too.

## When to change it

- **Liquids:** 1.33 for water, about 1.36 for spirits.
- **Jewels and stones:** high values (diamond 2.42) give the typical sparkling reflections.
- **Metals:** the IOR matters little; their look depends on **Metallic** and
  [Roughness](mat.roughness).

## Common mistakes

- **Glass that looks like a warped mirror:** IOR too high for ordinary glass. Go back to 1.5.
- **IOR 1.0 on a transparent object:** the object becomes almost invisible, because it neither
  bends nor reflects light.
