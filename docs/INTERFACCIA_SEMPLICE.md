# Simple Mode: visible controls and default settings

🇮🇹 [Leggi in italiano](INTERFACCIA_SEMPLICE.it.md)

Configuration reference for ForgeRender **Simple Mode**. It documents the controls shown to users and the values applied by the presets. Values are taken from `crates/forge_app/src/presets.rs`.

Status: **implemented** (phases 1–3; report: `reports/2026-10-29_interfaccia-semplice/`). The Simple/Advanced selector is at the top left; Simple is the default mode.

## 1. What appears in Simple Mode

The right-hand panel uses large, labelled buttons rather than hidden icons. Every button has a tooltip.

| # | Section | Control | Actual behaviour |
|---|---|---|---|
| 0 | Top bar | **Simple / Advanced** (default: Simple; remembered in Settings) | Changes only the right-hand panel; values remain the same |
| 1 | Product | **Import model…** | Same as File menu: GLB, STEP, OBJ, STL, SolidWorks on Windows |
| 1 | Product | **Place on base** | Moves the product's lowest point to height 0, without horizontal movement |
| 1 | Product | **Frame product** | Frames the product without floor or backdrop; also moves the active render camera |
| 1 | Product | **Fit scene to product** | Resizes backdrop and lights to product dimensions, preserves backdrop material and light colours, then frames it; undo with "Undo preset" |
| 2 | Look | **Photo studio** (default), **Light background**, **Dark background**, **Set studio** | Applies the values in table 2 |
| 2 | Look | **Light colour**: Warm 3200 K, Neutral 5500 K, Cool 7500 K, plus an individual selector for each light | Changes colour only; does not switch the look to "Custom" |
| 2 | Look | **Prepare product scene** | Places product on base, adds a backdrop if missing, applies look and quality, then frames it |
| 2 | Look | **Undo preset** | Restores lights, environment, display, quality, backdrop and position to their state before the latest preset |
| 2 | Look | **Custom** indicator | Appears when lights, environment or display change, including in Advanced Mode |
| 3 | Materials | **Open material library** | Opens the same library as Advanced Mode |
| 4 | Format | **Landscape 16:9**, **Square 1:1**, **Portrait 4:5**, **Stories 9:16** | Sets output resolution |
| 4 | Quality | **Draft**, **Balanced**, **High** | Sets samples, bounces and denoiser |
| 4 | Compute device | **GPU** and **CPU** | GPU is selected by default once confirmed working; user/Claude choice is then respected. Disabled if unavailable |
| 5 | Render | **Start render** | Same as F5 |
| 5 | Render | **Save image…** | PNG, JPG, 16-bit TIFF or EXR |

Advanced Mode also exposes numeric transforms, gizmo, individual lights, HDRI, fog and effects, lens and depth of field, threads, seed, bounces, Russian roulette, clamping, tone mapping, exposure and material assignment rules.

## Settings always active in Simple Mode

These are not shown in Simple Mode, but are always applied. In Advanced Mode they are in the Render tab; switching back to Simple reapplies them through `presets::apply_simple_base`.

| Setting | Value |
|---|---|
| Noise-threshold stopping | Enabled, threshold 0.030 (since 0.23.0; previously 0.050), minimum 32 samples |
| Denoiser mode | Fast preview + final |
| Denoiser CUDA | Enabled; automatically falls back to CPU if unsupported |
| Denoiser final / preview quality | High / Fast |
| First denoiser preview | At 8 samples |
| Denoiser strength | 100% (since 0.23.0; previously 90%) |
| Preserve detail (albedo and normals) | Yes |

On the cocktail scene (fog, dark scene), adaptive sampling was slower than uniform rendering (`reports/2026-10-29_campionamento-adattivo/`). It is usually beneficial for product photos on a light background. In Advanced Mode it remains disabled by default.

## 2. Looks: applied values

Lights use `forge_scene::light::preset`, scaled to the product bounds (centre and radius; floor and backdrop excluded).

| Parameter | Photo studio | Light background | Dark background | Set studio |
|---|---|---|---|---|
| Light preset | `studio-fotografico`: key softbox, fill, top light, backdrop light (hidden from camera) | `studio`: key softbox + fill | `tre-punti`: key, fill, rim | `cinematografica`: warm 3200 K key + cool 9000 K backlight |
| Backdrop | Slanted canvas: floor + curve + wall tilted 10° backwards | Curved | Curved | Curved |
| Environment | Uniform white, intensity 0.12 | Uniform white, intensity 0.9 | Uniform, intensity 0 (black) | Very dark gradient, intensity 0.5 |
| Environment visible to camera | Yes | Yes | Yes | Yes |
| Backdrop colour | Canvas grey 0.80 | Light grey 0.85 | Near black 0.03 | Dark brown 0.05 |
| Backdrop roughness | 0.95 | 0.65 | 0.35 | 0.30 |
| Tone mapping | ACES filmic | ACES filmic | ACES filmic | ACES filmic |
| Exposure | 0 EV | 0 EV | 0 EV | +0.3 EV |

Canvas dimensions relative to the product (`e` = bounds radius): height 2.5·e (minimum 2.5 m), width 7·e (minimum 6 m), curve 0.3·height, rear foot 0.35·e behind the product.

**Measured:** test `looks_change_the_rendered_image`, sphere on backdrop, CPU render 48×32, 8 samples: average brightness 0.773 with "Light background", 0.039 with "Dark background".

## 3. Quality presets

| Parameter | Draft | Balanced | High |
|---|---:|---:|---:|
| Samples per pixel | 64 | 256 | 1024 |
| Maximum bounces | 8 | 12 | 16 |
| Denoiser | Preview + final | Preview + final | Preview + final |

Threads (automatic), Russian roulette (from bounce 3), indirect clamp (0) and seed remain unchanged and can be adjusted in Advanced Mode.

## 4. Output formats

| Name | Aspect ratio | Resolution |
|---|---:|---:|
| Landscape | 16:9 | 1920 × 1080 |
| Square | 1:1 | 1080 × 1080 (Instagram) |
| Portrait | 4:5 | 1080 × 1350 (Instagram) |
| Stories | 9:16 | 1080 × 1920 |

## 5. "Prepare product scene", step by step

1. Saves the values that are about to change, so "Undo preset" can restore them.
2. Places the product on the base (height 0), without horizontal movement.
3. Creates a product-sized backdrop if none exists; otherwise reuses the existing one.
4. Applies the selected look (table 2) and quality preset (table 3).
5. Frames the product.

Running it again does not add a second backdrop. The backdrop material is "Backdrop (preset)" and can be changed through the library like any other material.

## 6. Open questions

- Would a fourth look be useful, for example **"On a wooden table"** (floor with a wood material from the library)? It requires that material to be available in the user's library.
- Draft quality uses 128 samples since 0.21.0 (previously 64, considered too low by Valerio).
- The "Dark background" look sets environment intensity to 0, so metals reflect only lights and backdrop. Should a subtle environment reflection be used instead (intensity 0.05)?
