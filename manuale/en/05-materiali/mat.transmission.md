---
id: mat.transmission
titolo: Transmission (glass and liquids)
parole: transmission, transparent, transparency, glass, liquid, water, see-through, dispersion, abbe, volume color, priority
ui: panel.materials.transmission
correlate: mat.ior, render.caustiche, mat.roughness
---
# Transmission (glass and liquids)

**Where:** Advanced mode → **Materials** tab → **Transmission** slider.

**What it does:** makes the material transparent like glass or a liquid: light goes through it
and bends according to the [IOR](mat.ior). At 0 the material is opaque, at 1 it is fully
transparent.

![Transmission 0: opaque light blue plastic](img/comune/mat_transmission_000.png) ![Transmission 0.8: mostly glass, still milky](img/comune/mat_transmission_080.png) ![Transmission 1: clear tinted glass](img/comune/mat_transmission_100.png)

*Light blue material, roughness 0.05, IOR 1.5, Transmission 0, 0.8 and 1.*

- **Range:** 0 to 1.
- **Default:** 0.
- **Values in between** mix an opaque surface and glass: they look milky. For real glass use 1
  and control the look with roughness (frosted glass) and the color controls below.

## Controls that appear above 0

| Control | Values | What it does |
|---|---|---|
| Dispersion | None, Crown glass (BK7), Flint glass, Lead crystal, Polycarbonate, Diamond, or the **Abbe number** (0 – 200) | Splits white light into colors (rainbows in jewels and prisms). Lower Abbe = stronger. Visible only with the **Spectral** render: see [Caustics and spectral](render.caustiche) |
| Volume color | color | The color the light takes after travelling **Color depth** inside the object. For liquids use this and leave the base color white: darker where it is deep, lighter at the edges, as in reality |
| Color depth | 0.01 cm and up (default 10 cm) | The distance at which the volume color is reached |
| Volume priority | −100 – 100 | Liquid in a glass: model the liquid a little larger than the cavity and give it a higher priority than the glass: the engine then uses the correct glass-liquid interface |

## Tips

- **Clear glass:** Transmission 1, roughness 0 – 0.02, IOR 1.5, base color white.
- **Frosted glass:** the same with roughness 0.2 – 0.5.
- **Colored glass:** prefer the volume color to the base color: thick parts get darker.
- Glass needs enough [bounces](render.rimbalzi): with too few it turns black.
