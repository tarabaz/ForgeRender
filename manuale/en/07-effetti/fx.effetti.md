---
id: fx.effetti
titolo: Effects (smoke, fog, dust, fireflies, decals)
parole: effects, smoke, fog, dry ice, haze, dust, fireflies, decal, sticker, label, logo, volumetric, plume, incense, steam
ui: panel.effects
correlate: fx.foschia, light.tipi, anim.timeline
---
# Effects (smoke, fog, dust, fireflies, decals)

**Where:** Advanced mode → **Scene → Effects** → **Add effect**, or the **+** of the **Effects** group in the Scene panel; a selected effect shows in the **Selection** tab; **Scene → Effects and fog…**.

Volumetric effects are computed **ahead of time**, one at a time, with the progress shown in the
status bar; then the render uses them on CPU and GPU. Each effect has a box (or an emitter) you
select in the viewport and move or resize with the gizmo. **Active in the render** switches one
off without deleting it.

| Effect | What it is |
|---|---|
| Haze | Air with fog in the whole scene or a layer of it, where the light beams become visible. See [Ambient haze](fx.foschia) |
| Smoke | Volumetric clouds in a box: *Cloud*, *Plume*, *Incense thread*, *Bank*, or a physical simulation with heat and buoyancy; presets such as incense, steam from a cup, plume from a pipe, industrial smoke |
| Dry ice | Cold, heavy fog that falls from an emitter, settles, spreads and flows around objects (fluid simulation) |
| Floating dust | Specks that sparkle only inside the beams of spots and sun, like dust in a ray through a window |
| Fireflies | A flock that flies and lights up at intervals; lit fireflies are real lights |
| Sticker (decal) | An image (logo, label, PNG with transparency) projected on the selected object where the camera looks; it wraps curved surfaces and moves with the object |

## Smoke and dry ice: the main controls

| Control | What it does |
|---|---|
| Density | How opaque the full smoke is; the panel shows the distance beyond which it becomes opaque |
| Color / Scattering | White steam, grey smoke, dark soot |
| Detail scale, swirls, turbulence | Size and strength of puffs and curls |
| Rise, fading | How fast it rises and dissolves |
| Quality | Size of the grid: more cells = finer detail, more memory and time |
| Seed / New | Another shape with the same parameters |

Simulated smoke and dry ice follow the timeline: their row in the [Timeline](anim.timeline)
decides when the emission starts and stops.

## Tips

- Effects are visible only where there is **light** to scatter: a spot or the sun crossing the
  smoke or the haze makes it stand out; with only the environment it looks flat.
- Volumetric effects need more [samples](render.campioni) than a clean scene.
