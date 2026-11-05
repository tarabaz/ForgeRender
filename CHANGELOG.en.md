# ForgeRender Changelog

🇮🇹 [Leggi il changelog in italiano](CHANGELOG.md)

The workspace version is defined once, in `version` under `[workspace.package]` in the root `Cargo.toml`. All components inherit it. The window title displays `ForgeRender v<version> (<commit>)`.

## Versioning policy (0.x)

- **0.X.0** (minor): a group of new features is delivered. Increment the version in the commit that completes the group and update this file.
- **0.X.Y** (patch): bug fixes only, with no new features.
- Each release adds a dated section here. The version is set in `Cargo.toml` and, as a fallback, in `packaging/windows/forgerender.iss`.

## 0.42.1 — 5 Nov 2026

**Colour picker**
- The saturation/value square and the hue bar of the colour picker were blank (only the marker circle and triangle showed): egui 0.37 draws them as a texture created on the fly, which did not appear in the program. They are now drawn in `color_picker.rs` as meshes with per-vertex colours, without textures.
- Every colour button (lights, environment, haze, smoke, dust, rendering styles, Blender import) now opens the same picker as the materials (RGB, CMYK, RAL palette) instead of the egui one (`color_picker::rgb_edit`); the palette code is not kept there, only the colour.

**Fog not restarted by the camera**
- The fingerprint that decides whether to recompute fog, smoke and haze included the render camera and the lights even with the haze off, so turning the view during a dry-ice or smoke simulation cancelled it and started again from zero. Camera and lights now count only when the haze is on, and when only the haze inputs change the running job stops only if it is computing the haze: a dry-ice or smoke simulation finishes, then only the haze is redone.

## 0.42.0 — 5 Nov 2026

**Replace material**
- `Scene::replace_material(from, to)`: every assignment of `from` (so also the components inheriting it from a parent) and the scene default switch to `to`; dust and decal nodes are left alone (test `replace_material_changes_every_user_and_the_default`, 100 objects). Materials panel: "Replace [combo] with «selected» (N)" and right-click on a scene material → "Replace with «selected»"; "Undo replacement" restores the assignments from before the last replacement.

**Dust: two colours, opacity, size variation, flakes**
- `Dust::color2` and `color2_share`: the grains of the second colour go in a second node and material (`effetto:polvere:2`, `effects::is_dust_id`), chosen with a separate random generator so the grains of existing projects stay where they were (test `second_color_splits_the_grains`).
- `Dust::size_spread` (0–1, 0.5 = from half to double as before, 0 = all the same, 1 = up to triple) (test `size_spread_controls_the_variation`).
- `Dust::opacity`: scales the light scattered by the grains; a shadow ray is blocked by a grain with a probability equal to the opacity (per ray on CPU with a hash, with `rnd()` on GPU), so the shadow is partial on average.
- `Dust::shape` = `Grains` (icosahedra, as before) or `Flakes` (flat hexagons with a random normal) with `sparkle` and `flake_roughness`: `DustLook::response` gives the grains the forward scattering of before and the flakes a diffuse part plus a two-sided GGX reflection tinted by their colour, so only the flakes facing the right way between light and camera sparkle (test `flakes_sparkle_unevenly`). GPU: `dust_response` in the shader, parameters 88–90 (`PARAM_WORDS` 92) and bit 3 of `P_HAS_DUST` for the flakes.

**Esc deselects everything**
- In the viewport, Esc clears the whole selection (objects, lights, fogs, smokes, dust, cameras) when it is not cancelling something else (a gizmo or spot-cone drag, the choice of the object to frame or of the focus point); the colour picker keeps its own Esc. Shortcut list and manual updated.

**Simulated smoke as a preview only**
- `SmokeCloud::preview_only`: the simulated smoke stays as the point preview in the viewport (drawn also when not selected, with a "preview only" note) and the render skips it; the cache key ignores the flag, so turning the full computation back on reuses what was already simulated (test `preview_only_smoke_is_skipped_by_the_render`). Panel: "Full computation for the render".

## 0.41.0 — 5 Nov 2026

**Smoke and fog are not computed again for nothing**
- The key of the caches that depend on the geometry (`RenderGeometry::id`) was a counter that changed at every rebuild of the geometry, even an identical one: the simulated smoke, the baked smoke, the dry-ice fog obstacles and the haze already computed were done again at the render although nothing had changed. Now it is a fingerprint of the content (triangles, normals, UVs, nodes): an identical scene gives the same key (test `same_scene_same_geometry_key`).

**Faster smoke simulation**
- Like the dry-ice fog, the simulation works only in the **active zone**: the box of cells with smoke or heat, enlarged by how far the fluid can move in a step plus a margin; outside, the air is still and empty, nothing is computed there and the pressure is 0 (open boundary). Measured (`forge_volume` example `fumo_tempi`): hot smoke 0.8 × 1.4 × 0.8 m 13.5 → 6.2 s, heavy gas 1.6 × 0.5 × 1.6 m 3.9 → 3.0 s, large box 3 × 2 × 3 m 25.5 → 6.1 s, with the same amount of smoke.

**Smoke contrast**
- `SmokeCloud::contrast` (0.5–4, 1 = as before): the relative density is raised to a power, so thin parts become transparent while the clumps stay dense and shade themselves; a light smoke keeps the relief and the detail of a dense one instead of becoming a uniform veil (test `contrast_thins_the_wisps_more_than_the_core`). For a transparent smoke with contrast: lower the density, raise the contrast to 2–3.

## 0.40.0 — 5 Nov 2026

**Simulated smoke that does not end at the box**
- **Soft edges** (`SmokeSim::edge_fade`, 0–0.4 of the side, default 0.2): in a band near the walls and the ceiling of the box (not the bottom) smoke and heat dissolve in the simulation, as if they left the box, instead of piling up against it and being cut off sharply by the render (`smoke_sim`, test `soft_edges_keep_smoke_off_the_box`). The depth of the band changes from point to point with a fractal noise, and in the band the render makes swirls, puffs and wisps stronger (`SmokeCloud::edge_keep`), so the smoke frays into tongues and holes rather than fading evenly. 0 = full up to the edge, as before.
- **Box rotation** (`SmokeCloud::rotation_deg`, around the vertical only, since heat still rises straight up): `to_local`, `to_world`, `corners`, `bounds` (world box containing the rotated one), source turned with the box; the simulation grid covers the rotated box and the smoke outside it is lost (`SmokeSimParams::box_center`, `box_yaw`, `box_half`). Test `rotated_box_carries_the_source`.
- **Gizmo of the smoke:** it follows the axes of the box; Rotate turns box and source together (also the "Rotation" field); Scale enlarges box, source position and source radius together. With the gizmo and with the "Size" field the bottom stays fixed: the box grows upwards only and no longer goes below the floor.
- **"Steam from a cup" preset:** now simulated (wide source like the cup, little heat, strong swirls and fine turbulence, quick fade, fine detail with many wisps), instead of the procedural plume that looked like a chimney (test `cup_steam_rises_in_thin_wisps`).
- **Viewport preview:** it is no longer thrown away when the smoke is deselected (for example by a click in the viewport while turning the camera), so selecting it again does not recompute it; a label at the bottom left shows the time being played (the loop starting again from 0 is not a new computation).

**Material previews at 512 px**
- Every preview is rendered once at 512 px (`store::THUMBNAIL_RENDER`, file `anteprima-v6-512.png`, the existing 512 px previews stay valid) and reduced with a Lanczos filter for smaller cards and the inspector. The "Minimum resolution" preference is removed (Low / Medium / High change the samples only).

**Fixes**
- **Black and white styles:** with "Black and white" on, the whole sample of each pixel becomes grey (`forge_render_cpu::style_gray`, the same in WGSL), including what is seen through coloured glass and liquids or in mirrors: before, only the drawn surfaces were grey and the orange glass ball of Valerio's render stayed orange (test `style_mono.rs`, cartoon and technical, RGB and spectral).

## 0.39.0 — 4 Nov 2026

**Simulated smoke you can place**
- The source of the simulation is drawn as a sphere of its real size, selectable with a click or "Select the source" (`ForgeApp::smoke_source_handle`); the gizmo moves it (`gizmo::DragStart::SmokeSource`, offset from the bottom of the box) and Scale changes its radius.
- **Viewport preview** (`ui::smoke_preview`, `forge_render_core::smoke::smoke_preview`): the same simulation on a coarse grid (about 36,000 cells, under a second) in a background thread whenever the settings of the selected smoke change, with 48 frames (`forge_volume::smoke_sim::simulate_smoke_frames`) shown as point clouds (up to 5,000 points), in a loop or paused at a chosen time. Not saved with the project.
- **Cells follow the source** (`SmokeCloud::sim_cells`, `sim_cell`): the quality's cells per side, made finer when a cell is larger than the source (about three cells across it), within the quality's total cell count; `sim_warning` and a message in the panel when the source is still smaller than a cell. Cause of the invisible smoke reported by Valerio: a 10 × 1.5 × 5 m box at quality 2 gave 14 cm cells for a 7 cm source, and the heavy gas lay in a layer thinner than a cell, cleared by the floor.

**Colour picker (RGB / CMYK / Palette)**
- `ui::color_picker`: one panel opened from the swatch of the material colours (base, volume, emission) with RGB (egui wheel, R G B, hex), CMYK (simple mathematical conversion, documented as not colorimetric) and Palette; live preview, Apply, Cancel/Esc restoring colour and code.
- **RAL:** the official RAL CLASSIC digital values are sold by RAL gGmbH as libraries to import into design software. With Valerio's go-ahead the palette tab includes `assets/colori/RAL_Classic.json` (225 codes, Italian and English names, **indicative** sRGB values: the average of the English and German Wikipedia tables, the German one where they differ by more than 20/255; prepared by `tools/colori/ral_classic.js`), labelled as not official. In addition it reads files imported by the user into `<Library>/Colori`: Adobe Swatch Exchange `.ase` (RGB, Lab D50 → sRGB with Bradford adaptation, CMYK, grey), `.json`, `.csv`.
- `Material::color_codes` (field → palette code, not written when empty): the code is shown next to the swatch with the full name as a tooltip; editing the colour in RGB or CMYK removes it.

**Fixes**
- **Ink outlines and tilt-shift no longer delay the render:** they were computed on the render thread before the first sample (0.3–0.5 s at 1080p on 0.1–1.6 million triangles, for every restart, and queued restarts added up while dragging). Now `Overlays` are computed in a separate thread and published when ready, reused while geometry, camera, resolution and outline settings stay the same, and a run of queued starts keeps only the last one (test `overlays.rs`).
- **Tilt-shift without repeated copies:** with 48 gather taps on a radius of 20+ pixels, bright reflections showed as separate copies (on the torus in Valerio's render). The blur is now gathered from a pyramid of reduced images, about 3 pixels of radius on the chosen level, blending two levels (test `large_blur_has_no_ghost_copies`).
- **Cartoon highlights (white patches):** the cel highlight was a fixed +0.6 white on any non-rough material, placed where the half vector of the strongest light passed 0.985 (about 10°), using the centre of area lights. Now it appears only where the mirror direction really sees the light (the softbox shape for area lights, a small circle for the others), with the strength of the material's reflectance (about 0.15 on dark plastic, full on metals), on CPU and GPU (test `toon_highlight_follows_the_reflectance`). Matches the photoreal reflections of the test scene.
- **Black dots in the outlines:** not reproduced with 0.37 and later (test scene, 640×360 and 1600×900, with and without cleanup): the fix of 0.37 (isolated sub-samples in triangle seams) covers it.
- **Missing glyphs:** the square after "U8" in egui's colour picker was its copy button 📋 (U+1F4CB): since egui 0.37 the symbol fonts are optional (`monochrome_emoji_fonts`), now enabled. A new test checks that every non-ASCII character of the interface strings and of `Lan_eng.json` has a glyph; it found and replaced ⊙ (camera aimed at an object), ● (unsaved changes), ✓ (in library) and ⧉ (detach window).
- **Manual in the interface language:** it follows the active interface language (also after a language change, forgetting the choice made in the manual window); a language without a manual falls back to English, then Italian.
- **Duplicate toolbar button:** "Library" and "Material" opened the same view; "Library" is removed (the Library menu with download, Blender import and folder stays).
- **Material preview:** dragging to the right turns the object to the right.

## 0.38.0 — 4 Nov 2026

**New built-in material library**
- The built-in materials are data, not code: `assets/materiali/<NN Category>/<id>/materiale.json` plus maps (the "Export zip…" format), read once by `forge_scene::material::presets()` (`library_presets`, cached). 125 materials in 13 categories (Metals, Anodised, Paints, Plastics, Rubbers, Glass, Liquids, Ceramics and stone, Woods, Fabrics and leather, Asphalt, Roofing, Emissive): every plain material, plus at most five textured ones per category with maps reduced to 1024 px (108 MB in all). Without the folder a few plain materials remain (the test scene ids).
- Built from the material zips prepared by Valerio, following `tools/libreria-serie/tabella.json` (id, Italian name, category, dispersion, included or additional) with the `forge_render_core` example `libreria_serie`. Fixed on the way: duplicate ids (anodised, glossy, matte and soft-touch plastics, seals), wrong categories (water was in Glass and was not treated as a liquid; plasters in Woods; Poly Haven materials all in "Libreria"; the one-material "Classici" category merged into Plastics), English names.
- **Dispersion:** every transparent material has an Abbe number (glass 58, lead crystal 32, optical glass 64, diamond 55, PMMA 57, polycarbonate 30, clear resin 45, silicone 50, water 56, wines and spirits 55, olive oil 47); the build fails on a transparent material without one.
- **Map conversion without changing the render:** EXR and PNG maps become JPEG (quality 95), because the engine reads roughness, normal, metalness and AO at 8 bits; height maps become 16-bit PNG, because it reads them at 16 bits; linear EXR colour becomes sRGB. Measured on the converted maps: PSNR 41–44 dB against the originals.
- **BlenderKit removed:** the 29 `bk-` materials and the 19 materials imported from BlenderKit (anodised, gold, fabrics) are not in the program; they are replaced by CC0 materials downloaded with the `forge_library` example `sostituti_cc0`: 21 from ambientCG (brushed and scratched steel, turned and scratched aluminium, diamond plate, perforated and corrugated sheet, gold, copper, verdigris, titanium, nickel, antique bronze, raw and rusty iron, black powder coat, red painted metal, the brushed base of the anodised aluminium) and 12 from Poly Haven (jersey, linen, velvet, corduroy, suede, terry, fleece, gingham, piqué, white and red leather, floral jacquard). Sources in `assets/materiali/LEGGIMI.md`.
- **Additional pack:** 109 more textured materials at 2K (about 1.6 GB), not in the repository: read from `materiali-aggiuntivi` next to the program, `assets/materiali-aggiuntivi` (local build) or `FORGE_EXTRA_MATERIALS_DIR` (`extra_materials_dir`); a material with the same id replaces the included one.
- Saved projects: the old map folders (`legno-rovere`, `marmo-bianco`, `pelle-nera`) stay in `assets/materiali/_vecchi`, and the maps of built-in materials are found again in every category folder (`builtin_map_dirs`). Projects keep their materials as saved.

## 0.37.0 — 4 Nov 2026

**Smoke simulation**
- Volumetric smoke gets a **Simulated** mode (`SmokeCloud::mode`, `SmokeSim`): a Eulerian fluid on a cell-centred grid with temperature and buoyancy (`forge_volume::smoke_sim`: sphere source with smoke, heat and exit speed; force (β·T − α·ρ)·ŷ; vorticity confinement; curl-noise turbulence; wind; semi-Lagrangian advection; red-black Gauss–Seidel pressure; fading and cooling; objects as solid cells; open box). Deterministic. Frozen at the chosen instant; the field (normalised to its 90th percentile, softened curve) replaces the procedural shape and the procedural puffs and filaments are added on its edge. Cached separately from the detail. Simulation quality 48–136 cells per side. Presets: simulated hot smoke, simulated heavy gas. The source is drawn in the viewport.

**Camera as a component**
- Left panel: **Cameras** list above **Lights** (select, make active, add from the view). A selected render camera shows a dashed line to its target (a diamond) in the viewport; click the camera icon or the diamond to select it. Gizmo: Move = camera and target together, Rotate = orbit around the target, the diamond = target only (`gizmo::Target::Camera`, `ForgeApp::selected_camera`, `camera_target_handle`). `RenderCamera::aim`: the target can follow an object (`follow_camera_aims`). If the view is looking through the camera, it follows it.

**Tilt-shift**
- `forge_render_core::tiltshift`: a sharp band on the final image (position, width, angle, two independent transitions, maximum blur), depth-aware gather blur from a per-pixel depth buffer computed at render start, applied in `Frame::display` (CPU and GPU, after the denoiser and the outlines). Separate from the physical depth of field; with both on they add up. Turning it on restarts the render (depth buffer); the other values apply live (`Frame::with_tilt`), also on export.

**Styles and outlines**
- **Black and white** (`RenderSettings::style_mono`) for cartoon (bands in greys) and technical illustration (grey ramp, no material colours), CPU and GPU.
- **Solid background** (`RenderSettings::background`): colour for camera rays that hit nothing, the environment still lights the scene (CPU, GPU, spectral, denoiser guides).
- Outlines: the stray dots came from single sub-sample rays slipping through the seam between two triangles of the same object and hitting its back; an isolated sample that disagrees with its four agreeing neighbours now takes their value (test `no_isolated_dots_on_a_smooth_sphere`). New: depth-jump sensitivity (default 3% as before), cleanup of line pieces shorter than N pixels (default 2), no line on the floor/backdrop edge against the background.

**Dust**
- Its own box (`Dust::region`, gizmo `Target::Dust`, dashed box in the viewport, "Fit to the scene"): the old automatic box was centred between the scene and the camera and cut at 6 m, so the specks could stop in mid-air and the beam looked cut. New dust gets a box around the objects up to the spots.
- Amount from 1 per m³, size up to 18 mm; **Cast shadows** (`Dust::shadows`): specks block shadow rays on CPU and GPU.

**Command line**
- `render`: `--mono`, `--background R,G,B`, `--outline-clean PX`, `--tilt C,W,T`, `--tilt-angle`, `--tilt-blur`, `--dust-shadows`, `--dust-box`; smoke presets 6–7 are simulated.

Not done yet: undo/redo for camera, smoke and dust boxes; MCP tools for smoke and the new options; transparent background.

## 0.36.0 — 4 Nov 2026

**Volumetric smoke**
- New effect **Volumetric smoke** (Effects tab): real smoke clouds in a box (`forge_scene::SmokeCloud`, `Scene::smokes`, saved in the project). The box decides where the smoke exists: select its centre dot in the viewport and move or resize it with the gizmo (Move / Scale, `gizmo::Target::Smoke`), or type centre and size; the rendered smoke follows it exactly.
- Four shapes (cloud, plume, incense thread, bank) and six presets that add a new cloud on top of the selection: suspended cloud, incense, steam from a cup, plume from a pipe, dense industrial smoke, smoke bank.
- Density field (`forge_render_core::smoke::smoke_density_at`) in box coordinates: shape outline twisted by domain-warped noise, cauliflower puffs on the edge (fractal of |noise|), filaments from noise ridges, slow inner variation; plumes and incense laminar at the bottom and turbulent going up. Start and maximum height inside the box, soft edges on the box faces.
- Controls: density (0.5–2000 1/m) with an "opaque beyond ~X" scale, colour, backlight, curl size, swirl strength (0–8), fractal detail (1–10), roughness, puffs, filaments, edge softness, inner variation, widening / meandering / fading, start and maximum height, no smoke inside objects, quality 1–4 (3–24 M cells), variant.
- Physical medium (`BakedFog`, delta tracking with shadows on CPU and GPU); the inside of objects is cleared with `voxelize` + `interior_cells`. Computed ahead of time with the other effects on the same bar (`Phase::Smoke`), with a cache.
- Procedural first version, no fluid simulation yet (see `docs/studio-fumo-fuoco.md`).

**Haze**
- **Start height** (`Haze::bottom_height`): the haze begins above the floor, with a soft start and a few puffs hanging below it; with the maximum height it makes a suspended band (e.g. 2.2–2.8 m). Layers, thinning and the floor smoke are measured from the start height; the grid starts just below it (finer cells).
- Wider sliders: density 0.001–50, clumping up to 2 (few, very concentrated clumps), swirl size 0.01–20 m, swirl strength up to 8, fractal detail up to 10, roughness 0.2–0.85, forward scattering −0.5–0.95, heights up to 20 m, layer thickness and spacing ranges ×4.

**Command line**
- `render`: `--haze-bottom`, `--smoke 0..5|NAME`, `--smoke-at X,Z`, `--smoke-y`, `--smoke-scale`, `--smoke-quality`, `--smoke-density`.

## 0.35.0 — 4 Nov 2026

**Render styles**
- At the top of the Render tab: **Photorealistic**, **Cartoon** (cel shading: material colour in three bands from the scene lights, with their shadows, and a crisp highlight on shiny materials) and **Technical illustration** (Gooch: warm to cool with an illustrator's light, no black shadows), with an example image on hover (`assets/stili/`). Applied at the first non-specular surface seen by the camera (glass and mirrors stay real); CPU and GPU, RGB and spectral (`forge_render_cpu::style`, `style_shade` in the shader).
- **Ink outlines** (with any style): silhouettes, object borders, depth jumps and sharp edges from 3×3 rays per pixel, drawn over the final image also after the denoiser (`forge_render_core::outline`, `Frame::outline`). Thickness, colour, edges and angle. Changing style or outlines restarts the render.

**Haze**
- **Light rays only** (default): the haze adds to camera rays the light of spots, sun and lamps scattered once, without veiling or darkening the objects (`BakedFog::additive`); off = physical haze as before.
- **Clumping**, **Maximum height** above the floor with randomly rising **Puffs**, and next to the density the scale "visible up to ~X m".
- Floor found with 5×5 downward rays (before, the lowest point of the scene was used: the floor smoke ended up inside the thickness of the floor).
- Grid also tied to the size of the scene (quality 2: at least 160 cells per side; before ~60 on the test scene), 2.5× stronger swirls, more contrast in the banks, lower default densities. Tuned with test renders (`forgerender-cli render --dark --spot --haze …`).

**Dust**
- Lit only by spots and sun (the beams); larger (2.5 mm) and sparser default specks, adjustable **Brightness**; the specks are in the denoiser guides.

**Command line**
- `render`: `--style`, `--outlines`, `--haze`, `--haze-density`, `--haze-quality`, `--haze-clumping`, `--haze-top`, `--dust`, `--dust-size`, `--dust-amount`, `--dark`, `--spot`.

## 0.34.0 — 4 Nov 2026

**Haze in five kinds, with fractal and quality controls**
- Ambient haze has a **Type**: *Low fog* (on the ground, thinning out going up), *Gas clouds* (almost clear air with dense clumps), *Toxic smoke on the floor* (a layer a few cm thick on the floor, gathered into filaments), *Floating layers* (horizontal veils at different heights, wavy and full of holes), *Vapour curls* (swirls inside swirls, high contrast). In the Effects tab the **Haze** button opens the menu of types; the type can be changed afterwards.
- Fractal controls: **Swirl strength** (domain warp, 0–4), **Fractal detail** (1–8 octaves), **Roughness** (gain between octaves), plus swirl amount and size. The floor smoke uses a low box (fine cells only where needed).
- **Quality** 1–4: from 1 to 24 million cells (48 MB), up to 512 per side.
- Removed the Light veil, Light rays, Dusty attic and Thick haze presets.

**Floating dust as its own effect, only in beams**
- **Floating dust** button in the Effects tab, no haze needed.
- The specks are transparent to every ray; camera rays add only the direct light of the scene lights (spot, sun, point, area), with shadows and forward scattering: no environment or bounce light, so they show only inside a beam. CPU (RGB and spectral) and GPU (RGB and spectral). Test `tests/dust.rs`.

**Interface**
- The ✕ buttons and the decal arrows showed a square (symbols missing from the font): they are now icons (bin, X, arrows). A test forbids those symbols in the interface code.
- Video error message without the missing arrow.

## 0.33.0 — 3 Nov 2026

**Ambient haze and floating dust**
- Effects tab, **Ambient haze**: hazy air throughout the scene, with banks, curls and gaps (domain-warped fractal noise), lit like the fog (multiple scattering, object shadows in the rays), on CPU and GPU. Density, colour, backlit rays, swirls, swirl size, thinning with height, shape. Presets: Light veil, Light rays, Dusty attic, Thick haze, Low fog.
- **No haze inside objects**: path segments inside glass and liquids do not cross any fog (lenses and glasses stay clear), on CPU and GPU; single-sheet glass without a back face does not count. The haze is also cleared from the solid interior of the parts, found by voxelising the objects and filling their inside (`fog::interior_cells`): the same grid will be used to simulate the haze. The air of a closed room stays air.
- **Progress bar for the haze too**: it is computed ahead of time like the fog. The effects are computed **one at a time** on the same bar, which starts again from zero for each one ("Effect 1 of 2: simulation…", then "Effect 2 of 2: ambient haze…"); the ✕ stops them all. Moving the camera or the lights recomputes the haze (its box changes).
- **Floating dust**: real sub-millimetre specks (up to 150,000) that sparkle where a ray or bounced light hits them. Amount, size, colour.

**Decals**
- Effects tab, **Decal**: an image on the selected object, where the camera is looking; it wraps curved surfaces and moves with the object. A transparent PNG lets the object show through (the same image as colour and cut-out: its alpha channel is used). Width, rotation, offset, depth, roughness, metalness, "Reposition from the view".
- Saved in the project and in `.frpack` packages (with the image).

**STEP: saved conversion**
- A STEP is converted once: the conversion stays in a user cache and is saved with the project (`modelli-convertiti/`) and in packages; on opening it is read back instead of converting again. Optical bench demo: from 27 s to 0.2 s. It is converted again if the STEP, an assembly part or the quality changes.
- Packages also include the parts of multi-file STEP assemblies. Fixed the **Converging and diverging lenses** demo: the package only held the empty assembly (only the lasers showed); it now has lenses, mounts and lasers.

**Dry-ice fog**
- **✕** button next to the fog progress bar: cancels the background calculation (with a warning that it will be redone when the render starts).
- Changing the fog (quality included) or the scene during the calculation stops it at once and restarts it from scratch with the new values.

**Other**
- Rendering starts on the **GPU** by default; without a suitable graphics card it falls back to the CPU (command line too).
- AI integration: the MCP bridge updates even while an assistant is using it (the file in use is renamed), no more "Access denied".

## 0.32.0 — 3 Nov 2026

**Dry-ice fog: much faster simulation**
- Each simulation step now works only where the fog is: arrays sized to the fog's box instead of the whole grid, forces, transport and pressure on the cells within 6 cells of the fog, a 2-byte neighbour table for the pressure, reused buffers, and the maximum velocity measured in the pass that already updates the velocities. Physics unchanged (same forces, MacCormack transport, pressure solver and collisions); the pressure is now solved on the fog's shape with open air beyond 6 cells.
- Measured on the cocktail demo (Ryzen 9 9900X, median of 3 runs, first computation without cache): Medium 20.9 → 8.0 s (2.6×), High 11.4 → 3.7 s (3.1×), Very high 60.9 → 14.5 s (4.2×), Ultra 182 → 37 s (4.9×). Same fog mass; overall shape within 3.5–6% (block averages). Full report: `reports/2026-11-03_ottimizzazione-simulazione-fumo/REPORT.md`; benchmark: `cargo run --release -p forge_project --example fog_bench`.
- **Computed in advance**: shortly after the last change to the fog or the scene the fog is computed in the background, so it is usually ready when you press Render (the Render button stays available meanwhile).
- **Progress bar** in the bottom bar, next to the samples: "Fog: simulation 45%" (obstacles, simulation, detail). "Building BVH" now appears only while the BVH is really being built.

**AI MCP switch**
- The top bar button is now **AI MCP: ON / OFF**, with a **gear** next to it that opens the AI integration settings (connect or disconnect Claude Desktop, Claude Code, Codex).

**Laser: partial reflections**
- New **Partial reflections** option in the laser card (off by default): like a real laser, at every glass face part of the light (the Fresnel fraction) is reflected and becomes a beam of its own with its own spot, so many dots appear around a crystal or a diamond. Branches below 1% of the laser's power are dropped; up to 64 stretches and 16 spots per laser. CPU and GPU, RGB and Spectral (checked by tests: same image within 4%). Off, the laser follows a single path as before.

**Demo scenes**
- New **Converging and diverging lenses** (Light and dispersion): Valerio's bench with three lasers (two white lab lasers and one UV violet) going through a row of lenses, crossing at the focus and opening again towards the screen.
- New **Optical bench: lasers and prism** (Light and dispersion): Valerio's laboratory optical bench, with a white lab laser that opens into a spectrum and a blue laser that does not split, lenses and a prism in dispersive crystal, Spectral render with caustics. It is a `.frpack` package in `samples/`: it is extracted into the user's folder, and Save asks where to save it (never inside the program's files).

## 0.31.0 — 2 Nov 2026

**Projects with all their files (package)**
- **File → Save complete package (.frpack)…**: a single file with everything inside: the project, the imported models (with the files they refer to: `.mtl` and textures of OBJ files, `.bin` and images of glTF files), the textures of the materials in use and the HDRI. It opens with File → Open on any computer (also from the recent projects, by dragging it onto the window or with a double click once installed); **Save** then updates the package itself.
- **File → Save to a folder with all files…**: the project with, next to it, `modelli/`, `textures/` and `hdri/`; just copy the folder.
- **File → Save as ZIP with all files…**: the same folder compressed into a ZIP.

**Materials library**
- The library and the material card open in a **separate window** that can be moved outside the program or onto another screen; "⧉ Back into the program" puts them back over the scene.
- **Apply to selection** is a large coloured button right under the preview, with the number of selected parts; it also appears at the top of the library as soon as a material is chosen.

**Lasers**
- New **UV violet (405 nm)** laser.
- The **blue** laser looks blue (it used to look violet: the part of a pure wavelength that a screen cannot show was simply cut off, which shifted the hue). Lasers now use the colours they have in photos.

**Wide laser beam**: each stretch of the visible beam now ends exactly on the glass faces it crosses. Before, it ended square on the axis and could stick up to three radii out of a prism: with a wide beam you could see a piece of beam that could not exist.

**Simple mode**: choosing an aspect (Light, Dark, Dramatic…) no longer deletes the lights you added: only the lights of the previous aspect are replaced.

**Welcome screen**: new image (optical bench rendered with ForgeRender) with the large logo, the version under the logo and the "A new dimension of light" title.

## 0.30.0 — 2 Nov 2026

**Laser: its own light, in red, green, blue or white**
- **Laser** is now in the list of lights you can add (Lights panel, Add), no longer only by changing the type of another light.
- **Laser type**: **Red (650 nm)**, **Green (532 nm)**, **Blue (450 nm)** or **White lab laser**. Red, green and blue have a single wavelength like real lasers: their path uses the index of that wavelength and they never split, not even in a prism or a diamond. The white lab laser contains every colour and, in the Spectral render, dispersive glass opens it into a rainbow. Projects saved before keep a white laser; a new laser starts red.

**Light presets add, they don't replace**
- The light presets of the Lights panel add their lights to the ones already in the scene instead of deleting them.

**Fixes**
- The visible laser beam was not drawn inside a liquid modelled slightly larger than the cavity of its glass (the right way): the camera ray skipped the inner face of the glass that lies inside the liquid and only looked for the beam after that face, so the whole stretch through the drink was lost. Now the skipped stretch is checked too (CPU and GPU, RGB and Spectral).

## 0.29.0 — 1 Nov 2026

**Light through glasses with liquid**
- The laser beam and the caustic photons now handle a liquid inside a glass like the path tracer does: at the glass-liquid boundary they bend with the real pair of indices (e.g. 1.52 → 1.35) instead of acting as if there were air on the other side, and the faces of the liquid that sit inside the thickness of the glass no longer count as surfaces. Before, the beam bent far too much in a drink and the colours opened up much more than in reality.
- The visible beam fades gradually inside a coloured liquid (power halfway along each stretch) instead of being drawn with the colour it has when it leaves.

**Liquids category**
- New **Liquids** category in the material library: Water (moved from Glass), Red wine, White wine, Whisky, Red cocktail, Olive oil, with real refractive index and dispersion (Abbe 55) and the colour of the volume.
- A material of the Liquids category wins over its container where they overlap, with no other settings: just model the liquid slightly larger than the cavity. An explicit higher Priority still wins.

**Note on the Laser light**: it is a thin beam of **white** light (like the sunbeam of the prism demo). A real laser has a single colour and never splits into a rainbow; the strong rainbow in a cocktail render came from a glass with Abbe 10.5 (extremely dispersive; ordinary glass is about 58) together with the wrong glass-liquid bending fixed here.

## 0.28.1 — 1 Nov 2026

**Fixes**
- Demo scenes saved as a project and reopened lost their objects (diamond, backdrop, floor, prism…) and showed "components with an assigned material are no longer present": the shapes built by the demos were not written to the project. They are now created like the objects of the Add menu, so the project saves them and recreates them on opening. Projects saved from a demo with 0.26–0.28.0 have to be saved again from the demo.

## 0.28.0 — 1 Nov 2026

**Caustics (progressive photon mapping)**
- Render tab → **Caustics**: light passing through smooth glass, crystals and gems gathers on the surfaces around them (bright focus spots, the rainbows of hanging crystals). Works with **every light**: point, spot, directional (sun) and area. With point, spot and sun lights plain path tracing cannot find these paths at all.
- How it works: in every pass photons are emitted from the lights towards the smooth glass of the scene, follow it (Fresnel choice between reflection and refraction, Snell's law, total internal reflection, glass tint, Beer–Lambert absorption) and land on the first non-specular surface. The first non-specular point seen by the camera gathers them within a radius that **shrinks with every pass** (Knaus–Zwicker), so caustics become sharper and sharper and converge to the correct value; the "glass → light" paths are no longer counted by the path tracer, so nothing is counted twice.
- In the **Spectral** render every photon carries its own wavelength (the glass index depends on it): caustics split into rainbows.
- Settings: photons per pass (default 200 k) and initial radius (0 = automatic, 1% of the scene).
- CPU and GPU: on the GPU photons are traced on the CPU and gathered by the shader (RGB and spectral, RT cores and shader BVH). Checked by tests: with an area light the photon-mapped image matches pure path tracing (0.4% difference); CPU and GPU agree within 2%.

**Demo scenes**
- **File → Demo scenes**, grouped by what they show: *Light and dispersion* (Prism and light, **Laser in a diamond**), *Caustics* (**Crystal pendants in the sun**, **Glass and caustics on the table**), *Volumetric effects* (Cocktail with dry ice), *Studio and materials* (Test scene). The test scene and the prism demo moved here from the File menu.
- Opening a demo shows a **card** that explains what the scene shows, which engine features it uses and how to recreate it step by step (materials, lights, settings), with a "Start the render" button.
- The cocktail project is now also included in the installer; it is copied into the user's folder the first time it is opened.

**New shapes**
- **Add** menu: **Prism**, **Diamond (brilliant cut)** (Tolkowsky proportions, 57 facets) and **Crystal pendant** (faceted drop).

**Fixes**
- Adding a floor, backdrop or wall with objects already in the scene places it **under all the objects**, not only under the selected one (before it could end up above or through some objects).

## 0.27.0 — 1 Nov 2026

**Lights aimed at an object**
- Lights panel → **Aim at → Hook to selection** (spot, area, laser, directional): the light stays aimed at the object. The target follows the object when it moves; moving the light keeps it aimed, and rotating it with the Rotate gizmo **orbits around the object**. "Unhook" frees the target again.

**Laser**
- The beam is now visible **inside glass** too (real glass is never perfectly pure): it enters white, crosses the prism or gem and leaves as a rainbow.

**Interface fixes**
- Left panel: the **Lights** list has its own area with a scroll bar and a **draggable border** to set its height; "Removed from project" moved above the scene tree. Before, with several lights they ended up below the panel and could not be selected, and the panel border ran down into the status bar.

## 0.26.0 — 1 Nov 2026

**Laser light**
- New light type **Laser** (Lights panel): a thin beam (diameter in mm, power in W) from the light towards its target. Its path is traced from the light through smooth glass (refraction, total internal reflection, Fresnel losses, volume absorption) and mirrors up to the first opaque surface, where it lights a Gaussian spot: exact and noise-free, also behind glass, which a camera path tracer could never find.
- **Visible beam** on/off ("In the air"): single scattering in a uniform haze inside the beam, integrated analytically.
- In the **Spectral** render the path is traced for 64 wavelengths (interpolated in between): after a dispersive prism or gem the beam opens into a rainbow fan and draws a continuous spectrum on the floor. CPU and GPU (RT cores and shader BVH) give the same result.
- **File → New demo scene: prism and light**: a white beam enters a glass prism on a black background and leaves as a rainbow (spectral render, already set up). New primitive: triangular prism.

## 0.25.0 — 1 Nov 2026

**Hardware ray tracing (RT cores)**
- On GPUs with ray tracing units (NVIDIA RTX, AMD RX 6000 and newer, with Vulkan), the GPU renderer finds the triangles hit by each ray with the hardware (wgpu ray queries) instead of the shader's BVH. Same image: 99.9% of pixels identical to the BVH on the same GPU, and all CPU/GPU tests unchanged.
- Measured on an RTX 5070, 256 samples at 1280×720: test scene 2.62 s → 0.25 s (**10.6×**), 1.6 M triangles 7.69 s → 0.59 s (**13.0×**), 3.8 M triangles 8.86 s → 0.59 s (**14.9×**); spectral 7.7–12.6×.
- Automatic: the Render tab shows "RT cores active". Without support (or with `FORGE_GPU_RT=0`) the shader BVH is used as before. Cutout (opacity) textures work: the ray restarts just past transparent points.
- Benchmark: `cargo run --release -p forge_render_gpu --example rt_bench`.

## 0.24.0 — 1 Nov 2026

**Spectral rendering (optional)**
- Render → Colour: **RGB** (default, unchanged) or **Spectral**: four wavelengths per path (hero wavelength, PBRT v4 visible sampling), RGB colours converted to smooth spectra (sigmoid polynomial, own coefficient table), CIE 1931 reconstruction with the scene's white kept white. CPU and GPU, same result. About 2× slower than RGB, with more colour noise.
- Transparent materials: **Dispersion** (Abbe number, Cauchy's equation) with presets for crown, flint, lead crystal, polycarbonate and diamond. Visible only in spectral mode.
- AI control: `set_render_settings` accepts `color` (rgb / spettrale), `create_material` accepts `abbe`.
- Details and validation: [docs/SPECTRAL.md](docs/SPECTRAL.md).

**Startup mode choice**
- At startup, after the welcome screen, a "How do you want to work?" window offers Simple or Advanced mode, with a short description of each and a note that you can switch later with the "Mode: Simple | Advanced" selector at the top left. It appears at every launch until "Don't show at startup again" is ticked; it can be turned back on in Settings, Information ("Ask for the mode at startup").

**BlenderKit materials removed**
- The 38 BlenderKit metals (Royalty Free, personal use only) are no longer bundled or listed among the presets. The `.blend` / BlenderKit material importer is still available.

## 0.23.0 — 31 Oct 2026

**Default render settings**
- Denoiser strength in Simple Mode increased to 100% (previously 90%).
- Noise threshold set to 0.03 (previously 0.05) in Simple Mode and in newly created Advanced Mode projects (threshold stopping was previously disabled for new Advanced projects). Images are cleaner, with somewhat longer render times.

## 0.22.0 — 31 Oct 2026

**Multiple views and MP4 turntable** (Render → Views and turntable…)
- Automatic product views: front, front-left and front-right three-quarter, right, left, rear and top. The product rotates while camera, lights and backdrop remain fixed, like a photo studio. Each view is framed independently.
- Turntable animation: set the angular step between frames or the duration of one full rotation. The dialog shows frame count and video duration. Configure 24–60 fps, repeated rotations and clockwise/counter-clockwise direction. The loop does not duplicate the first frame.
- H.264 MP4 encoding uses the Windows encoder, including hardware encoding when available. No extra installation is needed; ffmpeg is used as a fallback when installed.
- Flicker reduction: identical samples per pixel, fixed sample count (no adaptive sampling), and denoising only on the final image. The static backdrop measured 61 dB PSNR between consecutive frames, with less than one level out of 255 average difference.
- Uses the active render camera, or the current viewport with **Create camera from this view**. The scene, camera and settings are restored after completion or cancellation.

**Render passes** (File → Export render passes… or for each sequence frame)
- Albedo, normals, depth (16-bit PNG, white = near, plus EXR in metres), product mask for cut-outs and per-part IDs (one colour per part). Computed on the CPU using the same camera framing.

## 0.21.0 — 31 Oct 2026

**Material editor and ZIP export**
- A single click on a library material opens **Edit material**; double-click still applies it. Edit values and maps, then apply to the selection, save to the library, export a ZIP or discard changes.
- ZIP export contains `material.json` with all settings and the texture maps using relative file names. Extracting it into a folder creates a ready-to-use material preset.
- The material preview can be rotated by dragging; it temporarily uses a lighter preview quality while dragging.

**Fixes**
- Draft quality increased to 128 samples (previously 64).
- Studio and photographic backdrops are now sized to the product rather than to a previously selected backdrop, preventing repeated additions from growing progressively.

## 0.20.0 — 31 Oct 2026

**Object-scaled materials**
- Box-projected materials can use metre-based scale or **Fit to object**. In object mode, texture repetition follows the longest dimension of each part (or the assembly when assembly anchoring is selected).
- BlenderKit-derived presets and imported Blender Mapping nodes use object-relative repetition, avoiding oversized, blurry textures on small parts.
- Bump and displacement use the same scale; CPU and GPU results match.

## 0.19.0 — 31 Oct 2026

**Volumetric spotlights**
- Spotlights can display a visible volumetric beam in the air. Beam density and length are adjustable; cone shape, edge softness, colour and shadows follow the light settings.
- The camera sees the beam, including through glass and mirrors. It uses a single scattering event and does not add noisy indirect illumination to surfaces. Simulated dry-ice fog remains physically rendered as before.
- CPU and GPU produce matching results. The MCP light-edit tool exposes `beam_density` and `beam_length_m`.

## 0.18.0 — 31 Oct 2026

**One-click AI integration**
- Tools → AI Integration… shows the MCP server and assistant connection status, with Connect, Update and Disconnect actions.
- Supports Claude Desktop (including Microsoft Store and Claude-Data locations), Claude Code (terminal and app) and OpenAI Codex. ChatGPT desktop connection is not supported by this local integration.
- The `forgerender-mcp.exe` bridge is copied to `%LOCALAPPDATA%\ForgeRender\mcp`, so links remain valid when the application is moved or updated.
- Configuration files are backed up before changes; only the ForgeRender entry is written, and malformed files are left untouched.
- The AI Control indicator turns on after a successful connection and identifies the connected assistant.

## 0.17.1 — 31 Oct 2026

**Viewport and camera fixes**
- Automatic render restart can be limited to the Render view, allowing object, light and camera edits in the viewport without switching views or restarting the render immediately.
- Camera framing now shows the complete render frame, including wide images. Mouse-wheel zoom changes the viewport framing without moving the camera (unless Pilot mode is active).

## 0.17.0 — 31 Oct 2026

**Box texture anchoring and placement**
- Box projection can be anchored to **Part**, **Assembly** or **World**. Assembly anchoring keeps the projection continuous across differently rotated parts; CPU and GPU agree.
- Projects saved before 0.15 are migrated to World anchoring to preserve their previous appearance.
- Place multiple selected parts independently, from lowest to highest, so they settle and stack without interpenetration. One undo reverses the complete operation.

## 0.16.0 — 31 Oct 2026

**Material and texture correctness**
- Smoothed Box-projection displacement and removed hard seams on curved surfaces; CPU and GPU results match.
- Blender Displacement and Bump nodes are imported with their supported parameters, including connected normal maps.
- Updated BlenderKit-derived preset values; original source files are retained in the development reports.

## 0.15.0 — 31 Oct 2026

**Box-projected textures and material previews**
- Textures follow object axes when parts move or rotate; triplanar projections blend smoothly on curved surfaces.
- The same behaviour applies to normal, bump, cut-out and displacement maps on CPU and GPU.
- Material preview object size can be set to Real (9 cm), 30 cm (default) or 1 m.
- Added **Place** to lower selected parts until they touch the object below or the floor.

## 0.14.0 — 31 Oct 2026

**Object-axis dimensions and GPU-first rendering**
- Object dimensions and scaling follow local component axes rather than world-aligned bounds.
- GPU is the default render and material-preview device when supported (NVIDIA, AMD and Intel through Vulkan or DirectX 12); CPU remains available as fallback or user choice.
- Improved Blender material import preview, viewport display modes and generated primitives.

## 0.13.0 — 31 Oct 2026

**Blender material importer**
- Added an in-app `.blend` material importer that reads files in the background without launching Blender.
- Shows source materials, shaders, found and missing maps, warnings and a conversion report.
- Imports supported material values and texture maps into the local library without changing the open scene. Procedural-only materials are skipped.

## 0.12.0 — 31 Oct 2026

**Shader-ball material previews**
- Library thumbnails and material previews use the Khronos USD Shader Ball sample scene (CC-BY 4.0), including its lights, camera and environment.
- Added preview quality, resolution and sample settings, plus a command to clear the preview cache.
- The Materials panel focuses on materials used by the current scene.

## 0.11.0 — 30 Oct 2026

**Unified material library**
- Official presets and downloaded/imported materials appear in one searchable library.
- Added BlenderKit-derived metal and wood presets during development. Those source assets have personal-use-only terms and are excluded from redistributable release packages.
- Added data-driven material presets and tools for local library import.

## 0.10.0 — 30 Oct 2026

**Mapped built-in materials**
- Added mapped presets for oak, black leather and polished white marble.
- Material maps are bundled separately and remain optional; presets still work without the map folder.

## 0.9.0 — 30 Oct 2026

**AI component inspection**
- Added MCP tool `inspect_components` to create numbered viewport-style previews of assembly parts, identify repeated components and report IDs, dimensions, materials and colours.

## 0.8.3 — 30 Oct 2026

**Rendering and placement fixes**
- Fixed self-shadowing artefacts in orthographic views and improved accurate placement on the base.

## 0.8.2 — 0.8.0

Stability and rendering fixes, the first integrated manual, build improvements, interface review and the initial Simple Mode workflow.

## 0.7.0 — 0.1.0

- **0.7.0:** menu and toolbar separation, new interface actions and first manual phase.
- **0.6.0:** Simple Mode, first-run experience, statistics, welcome screen and AI controls.
- **0.5.0:** separated AI control bridge.
- **0.4.0:** first Claude/MCP natural-language control integration.
- **0.3.0:** product-photo workflow, viewport, camera and lighting improvements.
- **0.2.0:** first workflow and material library, rendering improvements and portable build tools.
- **0.1.0:** initial CPU/GPU path tracer, CAD/3D import, material library, OIDN denoiser, professional camera, lights, volumetric fog, projects, portable package and installer.

For measured performance results, see [Benchmarks](docs/BENCHMARK.md). For project architecture and current plans, see the [documentation index](docs/README.md).
