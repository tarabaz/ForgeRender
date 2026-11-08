---
id: mat.metallic
titolo: Metallic
parole: metal, metallic, metalness, conductor, steel, gold, copper, aluminium, chrome
ui: panel.materials.metallic
correlate: mat.roughness, mat.specular, mat.pannello
---
# Metallic

**Where:** Advanced mode → **Materials** tab → **Metallic** slider.

**What it does:** tells whether the surface is a metal or not. A non-metal (plastic, paint,
wood) shows its color in the diffuse light and has white, fairly weak reflections; a metal has
no diffuse color at all: everything it shows is reflection, **tinted by the base color**.

![Metallic 0: plastic, colored body and white highlights](img/comune/mat_metallic_000.png) ![Metallic 0.5: halfway, physically unusual](img/comune/mat_metallic_050.png) ![Metallic 1: copper metal, colored reflections](img/comune/mat_metallic_100.png)

*Same base color and roughness 0.2, Metallic 0, 0.5 and 1.*

- **Range:** 0 to 1.
- **Default:** 0.
- **Effect on the render:** at 1 the base color becomes the color of the reflections (gold,
  copper, brass); at 0 it is the color of the body.

## Which value to use

Real materials are either metals or not: use **0 or 1**. Values in between are only useful
with a **metalness texture** (paint chipped off a metal, rust on steel), where each point is
one or the other.

| Metal | Base color (approximate) |
|---|---|
| Steel, chrome | light grey 0.56 – 0.58 |
| Aluminium | almost white 0.91 |
| Gold | 1.0, 0.71, 0.29 |
| Copper | 0.95, 0.64, 0.54 |

## Common mistakes

- **Black metal:** a metal with a very dark base color reflects almost nothing. Metals are
  bright: use light colors.
- **Metal that looks like plastic:** Metallic left at 0. A chrome look needs Metallic 1 and low
  [Roughness](mat.roughness).
- **A metal needs something to reflect:** in front of a uniform background it looks flat. An
  HDRI environment or a few lights around give it life.
