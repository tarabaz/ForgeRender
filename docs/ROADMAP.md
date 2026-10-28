# Roadmap and real status

🇮🇹 [Versione italiana](ROADMAP.it.md)

Legend: ✅ implemented and tested · 🟡 partial · ⏳ not implemented yet

## Summary: what comes next and how hard it is

Complexity is a rough estimate of the effort and risk for this codebase:
**Low** (hours, contained change) · **Medium** (a day or two, several modules) ·
**High** (several days, new subsystem with CPU + GPU + UI + tests) ·
**Very high** (weeks, research or external SDKs, hard to verify).

| # | Feature | Complexity | Depends on | Notes |
|---|---|---|---|---|
| 1 | Light gizmo: move point/spot lights like objects, visible and editable spot cone (angle + edge softness handles) | Medium | gizmo | In progress: panel sliders done |
| 2 | Per-object contact snap: while moving, the object stops when it touches another mesh | Medium | BVH picking | Per-object setting, saved in the project |
| 3 | Adaptive sampling: stop when noise is below a threshold | Medium | — | The "noise threshold stop" placeholder in the Render tab |
| 4 | UI localization with external language files (`Lan_ita`, `Lan_eng`, anyone can add `Lan_spa`) | Medium | — | Mechanical but touches every UI string |
| 5 | Decals / stickers (image projected on a surface, planar and cylindrical, with position/scale/rotation) | Medium | textures | — |
| 6 | Volumetric dry-ice fog, static and procedural (seeded noise, CPU path tracer with delta tracking) | High | — | Plan below |
| 7 | Fog collision with objects + point emitter with simulated duration (baked, no timeline) | High | 6 | Plan below |
| 8 | Volumetrics on the GPU | High | 6 | Same algorithm as CPU, statistical test CPU vs GPU |
| 9 | Animated fog (timeline, fluid simulation) | Very high | 6, 7 | Architecture prepared in 6–7 |
| 10 | Displacement preview in the modelling viewport | Medium | displacement ✅ | Today displacement shows in the render only |
| 11 | Texture mip-mapping / filtering | Medium | — | Less aliasing on distant textures |
| 12 | DLSS Ray Reconstruction as fast interactive preview | Very high | NVIDIA SDK | RTX only, Vulkan interop with wgpu |
| 13 | Hardware ray tracing (RT cores) | Very high | OptiX or Vulkan ray query | — |
| 14 | Ivy generator: photorealistic climbing plants growing on objects | High | — | **To do later**, not scheduled now |
| 15 | GPU usage / VRAM readout via NVML | Low | — | — |

Suggested order: 1 → 2 → 4 → 3 → 6 → 7 → 8 → 5, then the rest by need.

## Volumetric dry-ice fog: technical plan

Goal: real 3D participating media inside the existing Monte Carlo path tracer,
not sprites or textured planes, without changing the quality of what already works.

**Integration points found in the current code**
- `forge_render_cpu/src/integrator.rs` (`radiance`): the path loop already finds the
  next surface hit (`t_surf`). A medium event is sampled in `[0, t_surf]` before the
  surface is shaded; if it happens the path scatters in the medium, otherwise it reaches
  the surface with the throughput unchanged (delta tracking makes this unbiased).
- Next Event Estimation (same file): shadow rays use `textures::occluded`; they will also
  multiply by the medium transmittance (ratio tracking), so fog casts and receives shadows.
- `forge_render_core/src/render_scene.rs` (`RenderScene::compile`): the place to bake the
  procedural density into a 3D grid, deterministic from the seed, cached like the geometry.
- `forge_render_gpu/src/layout.rs` + `path_tracer.wgsl`: the grid goes into the same bindless
  scene buffer; the WGSL loop gets the same delta/ratio tracking.
- `forge_scene`: a new `FogVolume` entity (like lights) saved in the project.

**Phases**
1. `forge_volume` crate: seeded gradient noise, fBm, curl-noise domain warping (filaments,
   swirls), dense base with height falloff, lateral spread, soft edges; baked into a grid with
   a coarse majorant grid. Tests: determinism by seed, value ranges, performance of the bake.
2. CPU transport: delta tracking (Woodcock) for free paths, Henyey–Greenstein phase function
   (anisotropy g), scattering albedo, NEE from points in the medium with MIS, ratio tracking
   for shadows. Tests: Beer–Lambert transmittance through a homogeneous slab, white furnace
   with albedo 1, energy conservation; real renders from several angles.
3. Dry-ice behaviour: emitter placed as a point in the scene; the CO₂ fog falls, pools on the
   ground and spreads sideways, flowing around obstacles (scene meshes voxelized into the grid,
   no fog inside objects). The user sets a *simulated time* (e.g. 4 s): the state at that time
   is baked, so nothing runs forever and no timeline is required.
4. GPU port of the same algorithm, statistical CPU/GPU comparison test, benchmark.
5. UI: fog panel (seed, density, scale, turbulence, scattering, anisotropy, size, simulated
   time, collision on/off), bounding box and emitter drawn in the viewport, project save.
6. Later: timeline and per-frame bake (the generator already takes a time parameter), then a
   real fluid solver writing the same grid format.

## Phase 1 — Foundations ✅
Workspace, desktop app, viewport with orbit camera, test meshes, tests and documentation.

## Phase 2 — First CPU renderer ✅
Ray tracing with BVH, normals, diffuse material, lights, shadows, progressive path tracing,
correct accumulation, PNG export.

## First version acceptance criteria
| # | Criterion | Status |
|---|---|---|
| 1 | Start the Windows application | ✅ portable zip and installer built by CI |
| 2 | Import a test mesh | ✅ OBJ/STL/glTF + procedural test scene |
| 3 | Display and rotate the model | ✅ |
| 4 | Set a material | ✅ |
| 5 | Set light and camera | ✅ |
| 6 | Real CPU rendering | ✅ |
| 7 | Progressive improvement with samples | ✅ |
| 8 | Change the maximum sample count | ✅ also while rendering |
| 9 | Stop and see the current result | ✅ |
| 10 | Export PNG | ✅ |
| 11 | Save and reopen a project | ✅ |
| 12 | Automated engine tests | ✅ |

## Phase 3 — Materials and lighting ✅
- ✅ GGX BRDF, metals, dielectrics, reflections, refractions, Fresnel, material editor
- ✅ Equirectangular HDRI (.hdr/.exr) with importance sampling and MIS, rotation and strength
- ✅ Full PBR texture set: base colour, roughness (or gloss), metallic, normal map (OpenGL or
  DirectX), height (bump), ambient occlusion, opacity cutout; UV mapping or world-space box
  projection in metres for CAD without UVs; repeat size, rotation, offset
- ✅ True displacement from the height map: render-time subdivision, crack-free shared edges,
  original texture coordinates preserved, triangle budget; CPU and GPU tested
- ✅ GGX multiple-scattering compensation (Turquin 2019)
- ✅ Emissive surfaces sampled as lights (NEE + MIS)
- ✅ Frosted glass (microfacet transmission, Walter 2007)
- ⏳ Mip-mapping, decals

## Phase 4 — GPU 🟡
- ✅ WGSL compute path tracer via wgpu (Vulkan/DX12), same algorithm as the CPU, single
  bindless scene buffer, adaptive passes (no TDR timeouts)
- ✅ GPU tests (lavapipe in CI): white furnace, area light and emissives against analytic
  formulas, HDRI, textures, displacement, depth of field, statistical convergence to the CPU image
- ✅ CPU/GPU selector in the UI and CLI, automatic fallback to CPU
- ⏳ Measurements on the RTX 5070, NVML, hardware ray tracing, hybrid CPU+GPU

## Phase 5 — CAD 🟡
- ✅ glTF/GLB (recommended export from SolidWorks), OBJ, STL, Blender `.blend`
- 🟡 STEP via Open CASCADE (separate converter, left out of the CI packages for now: export GLB instead)
- 🟡 SolidWorks SLDPRT/SLDASM via COM export (to be tested on a PC with SolidWorks)
- ⏳ Renderer instancing

## Import from Blender and glTF ✅
Hierarchy, meshes, Principled materials and textures, cameras, `KHR_lights_punctual` lights;
`.blend` files are exported through Blender running in the background.

## Phase 6 — Denoising and quality 🟡
- ✅ Intel Open Image Denoise 2.3.3 loaded at runtime, CPU or CUDA device, albedo/normal guides,
  final-only or progressive preview mode, strength
- ⏳ Adaptive sampling / noise threshold stop, complete benchmarks

## Phase 7 — Professional editor 🟡
- ✅ Material library (~120 presets + downloaded ambientCG / Poly Haven materials and HDRIs,
  local `Library` folders with previews)
- ✅ Transform gizmo (move / rotate / scale, snapping), undo/redo, hide (H), duplicate, exact
  drop to floor, Blender-style axis navigator
- ✅ Render cameras with professional lens: focal length in mm, f-stop, depth of field with focus
  plane and near/far limits shown in the viewport, aperture blades, lens shift
- ⏳ Light gizmo, contact snap, decals, localization, advanced assignment rules

## NVIDIA: CUDA, OptiX and DLSS (plan)

| Step | What | Requirements | Status |
|---|---|---|---|
| 1 | OIDN denoiser on CUDA | NVIDIA driver, Volta or newer GPU | done, to be tested on the RTX 5070 |
| 2 | DLSS Ray Reconstruction as fast preview of the Render view | NVIDIA Streamline/NGX on Vulkan, interop with wgpu resources, RTX only | to do |
| 3 | DLSS menu: model version (one DLL folder per version, menu built from a manifest), internal resolution, sharpness, blend with the accumulated image | step 2 | to do |
| 4 | Hardware ray tracing (RT cores) | OptiX or Vulkan ray query | to evaluate |

Notes:
- "CUDA cores" are the GPU's compute units: the WGSL path tracer already uses them.
- DLSS Ray Reconstruction is designed for real time; for the final still image OIDN is used.
