---
id: light.tipi
titolo: Lights panel and light types
parole: light, lamp, area, softbox, point, bulb, spot, directional, sun, portal, laser, intensity, presets, range, shadows, beam
ui: panel.lights, scene.add_light
correlate: light.dimensione, light.colore, env.ambiente, fx.foschia
---
# Lights panel and light types

**Where:** Advanced mode → **Scene → Lights** (presets and new lights); a selected light shows in the **Selection** tab. The lights are listed in the Scene panel on the left:
click one to edit it.

## Presets

**Preset:** adds a ready set of lights, sized to the scene, **next to the lights that are
already there** (it never removes any):

| Preset | Lights |
|---|---|
| Neutral studio | Main softbox and fill |
| Photo studio (4 lights) | Main softbox, fill, top light and backdrop light |
| Three-point lighting | Key, fill and rim |
| Softbox | One large softbox from above |
| Industrial lighting | Three ceiling lights |
| Cinematic | Warm key and cool backlight |

## Light types

| Type | Intensity in | Use |
|---|---|---|
| Area (rectangular) | Radiance (W/sr/m²) | A softbox: soft shadows, the light of product photography |
| Point | Intensity (W/sr) | A small bulb: sharp shadows |
| Spot | Intensity (W/sr) | A cone of light; can show a visible beam in the air |
| Directional | Irradiance (W/m²) | The sun: parallel rays, same strength everywhere |
| Portal (window) | — | Emits nothing: tells the renderer where the HDRI light comes in through a window, much less grain in interiors |
| Laser | Power (W) | Red, green, blue, violet or white lab laser with a visible beam, refracted by glass and prisms |

## Common controls

| Control | Values | What it does |
|---|---|---|
| On | on / off | Switches the light off without deleting it |
| Aim at | object | The light follows an object (**Hook to selection**, **Unlock**) |
| Color and Temperature | 2700 K … 9000 K | See [Light color](light.colore) |
| Intensity | 0 and up | Brightness, in the unit of the type |
| Width / Height (m) | area and portal | Size of the rectangle: see [Size and soft shadows](light.dimensione) |
| Cone angle (°) | 1 – 170 | Spot opening |
| Edge softness (%) | 0 – 100 | 0 = sharp edge, 100 = fades from the center |
| Visible beam | spot | Volumetric beam with **beam intensity** (0.02 – 5) and **beam length** |
| Range | Limited: maximum distance (m) and range falloff (%) | Point and spot: the light stops at a distance, drawn as a bubble in the viewport |
| Position / Target | metres | Where the light is and where it points; **Aim at the scene center** |
| Cast shadows | on / off | Off: the light passes through objects |
| Visible to camera | on / off | Whether the light itself appears in the image |
| Duplicate / Delete | | |

## Aiming in the viewport

Select a spot or directional light: a line goes to its target, drawn as a **diamond**. Drag the
diamond to aim the light while the source stays still; the fields follow and the move can be
undone.
