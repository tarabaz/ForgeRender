---
id: fx.foschia
titolo: Ambient haze
parole: haze, fog, mist, light rays, god rays, beams, volumetric light, density, atmosphere
ui: panel.effects.haze
correlate: fx.effetti, light.tipi, render.campioni
---
# Ambient haze

**Where:** **Scene → Effects** → **Add effect** → **Haze**.

**What it does:** fills the scene (or a layer of it) with a light fog that every light crosses:
spots, sun and lamps draw visible rays, with the shadows of the objects inside them.

## Density

**Density** (0.001 – 50, logarithmic) is how thick the air is, in 1/m. Next to the slider the
panel shows the distance at which an object would disappear in the haze ("visible up to ~").

![Density 0.3: faint beam, visible up to about 10 m](img/comune/fx_haze_density_low.png) ![Density 1: clear beam, about 3 m](img/comune/fx_haze_density_mid.png) ![Density 3: thick beam, about 1 m](img/comune/fx_haze_density_high.png)

*Low fog lit by a spot from above, density 0.3, 1 and 3.*

## Kinds

| Kind | Look |
|---|---|
| Low fog | Lies on the ground and thins out going up, with banks and curls |
| Gas clouds | Almost clear air with dense clumps here and there |
| Toxic smoke on the floor | A layer a few centimetres thick hovering over the floor, gathered into filaments |
| Floating layers | Horizontal veils at different heights, wavy and full of holes |
| Vapour curls | Swirls inside swirls with high contrast |

## Other controls

| Control | What it does |
|---|---|
| Light rays only | On (default): the haze shows only the light of spots, sun and lamps that crosses it, without veiling or darkening the objects. Off: physical haze that veils everything with the environment light |
| Clumping (0 – 2) | 0 = same everywhere, 1 = marked clumps and gaps, 2 = few very concentrated clumps |
| Start height / Maximum height | A suspended band, for example under a ceiling |
| Puffs | A few random puffs rising above the maximum height |
| Color | White = neutral air, warm = dust, cool = mist |
| Backlit rays | How bright the rays are when you look towards the light |
| Swirl size, Swirl strength, Fractal detail, Roughness | The shape of the banks and curls |
| Quality (1 – 4) | From a light draft grid to 24 million cells for fine detail |
| Shape: New | Another arrangement of the swirls |

## Tips

- The rays show best with a **spot** or the **sun** and a dim environment.
- Thick haze needs more samples: volumes are the noisiest part of a render.
