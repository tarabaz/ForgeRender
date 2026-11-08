---
id: mat.specular
titolo: Specular
parole: specular, reflection strength, highlight, sheen, fresnel, gloss level
ui: panel.materials.specular
correlate: mat.ior, mat.roughness, mat.metallic
---
# Specular

**Where:** Advanced mode → **Materials** tab → **Specular** slider.

**What it does:** sets how strong the reflection of a **non-metallic** surface is. At 0.5 the
reflection is the physical one derived from the [IOR](mat.ior); lower values weaken it, higher
values strengthen it. It has no effect on metals (Metallic 1).

![Specular 0: no reflection, a dull black](img/comune/mat_specular_000.png) ![Specular 0.5: the physical reflection of a plastic](img/comune/mat_specular_050.png) ![Specular 1: stronger reflection, lacquer look](img/comune/mat_specular_100.png)

*Black plastic, roughness 0.15, Specular 0, 0.5 and 1.*

- **Range:** 0 to 1.
- **Default:** 0.5 (physical reflection from the IOR).

## When to change it

- **Leave it at 0.5** for almost every material: the IOR already gives the right reflection.
- **Lower it** for surfaces that reflect less than a plastic: chalk, fabric, unglazed clay.
- **Raise it** for a deep lacquer or piano black effect.

On dark materials the difference is the most visible, because there the reflection is almost
everything you see.
