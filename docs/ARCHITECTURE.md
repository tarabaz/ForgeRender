# ForgeRender Architecture

🇮🇹 [Leggi in italiano](ARCHITECTURE.it.md)

## Workspace

| Crate | Responsibility |
|---|---|
| `forge_math` | AABBs, rays (Wächter & Binder anti-acne offset), orthonormal bases, deterministic PCG32 RNG, sampling |
| `forge_scene` | Data model: meshes, component tree with stable `ComponentId` values, materials, lights, environment, camera, units, primitives |
| `forge_accel` | 16-bin SAH binary BVH, 32-byte `#[repr(C)]` nodes (GPU-compatible), Möller–Trumbore intersection |
| `forge_render_core` | `RenderSettings`, shared-BVH `RenderGeometry`, `RenderScene`, accumulator, tone mapping, `RenderBackend` trait, `RenderSession` |
| `forge_render_cpu` | CPU backend: BSDFs, light sampling, path-tracing integrator, dedicated Rayon pool |
| `forge_import` | OBJ/STL import, mesh validation, crease-angle normals, unit conversion and Z-up → Y-up conversion |
| `forge_project` | `.frproject` format, schema versioning, atomic saves, reports for missing components/files |
| `forge_image_io` | PNG, JPG, 16-bit TIFF and EXR export with atomic writes |
| `forge_telemetry` | Hardware detection (sysinfo), CPU/RAM monitoring, `tracing` logs buffered for the UI |
| `forge_cli` | `forgerender-cli`: headless rendering and thread-scaling benchmarks |
| `forge_app` | `forgerender`: egui/eframe GUI, off-screen wgpu viewport, orchestration |

Materials, lights and camera are modules of `forge_scene`, rather than separate crates:
they are small, tightly related data types, and splitting them would add overhead without
a meaningful benefit.

## Rendering pipeline

1. The UI edits the `Scene` and increments two revisions: `geometry_rev` (meshes and
   visibility) and `shading_rev` (materials, lights, environment and camera).
2. When geometry changes, the BVH (`RenderGeometry`) is rebuilt **on a separate thread**.
   Otherwise it is reused: moving the camera or changing a material takes only a few
   microseconds, even for assemblies containing millions of triangles.
3. `RenderScene::compile_with_geometry` prepares the integrator data and passes it to
   `RenderSession`, which runs on its own thread.
4. Each backend pass computes **one sample per pixel** into a separate buffer. Only
   completed passes are added to the accumulator. Pausing, stopping or restarting discards
   the pass in progress, so the image never contains partial or incompatible samples.
5. The UI reads the latest frame (linear average) and applies tone mapping and exposure
   without restarting the render.

The random-number generator depends only on `(seed, pixel, sample index)`: the same render
produces the same result with 1 or 24 threads (verified by tests). This also makes it
possible to distribute samples between CPU and GPU in the future without duplication.

## Integrator

Unidirectional path tracing:

- Next-event estimation (NEE) samples a light chosen uniformly. Multiple importance
  sampling (MIS), using the power heuristic, combines light and BSDF sampling for area
  lights; delta lights use weight 1.
- The environment and emissive surfaces are reached only through BSDF sampling (weight 1,
  unbiased estimator).
- Russian roulette starts at the third bounce. Indirect-contribution clamping is optional
  and disabled because it would introduce bias.

The **principled-lite BSDF** combines weighted Lambert diffuse `(1−F)(1−F)` with GGX,
height-correlated Smith masking-shadowing and Schlick Fresnel, using visible-normal
sampling (Heitz 2018). Smooth glass uses exact Fresnel and radiance scaling
`(n₁/n₂)²`.

### Automated checks

- *White furnace*: a Lambert sphere with albedo ρ in a uniform environment returns exactly
  ρ; glass returns 1; metals and plastics return ≤ 1 (no energy creation).
- Point and rectangular lights are compared with analytical solutions (form factor).
- BSDF `sample`, `eval` and `pdf` consistency; reciprocity; normalized PDFs.
- BVH intersections are compared with brute-force intersections over random rays.
- Convergence: RMSE at 128 samples is less than half the RMSE at 8 samples.

### Known Milestone 1 limitations

- Turquin multi-scattering compensation depends only on the outgoing direction:
  reciprocity is approximate (error below 1%).
- No depth of field, opacity/cut-outs or displacement.
- Caustics (light focused by glass or metal) converge slowly, as in any unidirectional
  path tracer.
- Geometry is flattened into world space (no instancing), so assemblies with many copies
  of the same part use more memory. Two-level instancing is planned with STEP import.

## Viewport

The viewport uses wgpu (DX12/Vulkan on Windows), preferring the high-performance GPU
(e.g. the RTX rather than the Ryzen integrated graphics). The scene is drawn to an
off-screen texture with 4× MSAA and a depth buffer, then displayed by egui. If GPU
initialization fails, the app automatically restarts in `--safe-mode` (OpenGL, viewport
disabled, CPU rendering enabled).

## Projects

A `.frproject` file is readable JSON containing `format`, `schema_version`, source-file
references (relative path, absolute fallback and import options), materials,
`ComponentId → MaterialId` assignments, visibility, lights, environment, camera and
render settings.

Geometry is not copied: the original CAD file is never modified. Assignments to components
that are no longer present are preserved and reported.

Textures (Phase 3) will follow the same strategy: relative/absolute external references,
with a future option for a “project package” that copies assets into a neighbouring folder.

## Plans for non-Rust components (Phases 4–5)

- **STEP**: a minimal external C++ converter using Open CASCADE (XCAF: hierarchy, names,
  colors, units and transforms; controlled-tolerance triangulation). It runs as a separate
  process, so an OCCT crash cannot close the app and the LGPL dependency remains isolated.
- **SolidWorks**: a C# bridge using the Interop DLLs installed with SolidWorks (COM). It
  opens the document read-only and exports STEP AP214/AP242 plus JSON containing structure,
  IDs and transforms. A licensed SolidWorks installation is required on the PC.
- **GPU**: first, a WGSL compute-shader path tracer through wgpu that reuses the
  `forge_accel` BVH layout; hardware ray tracing (Vulkan ray query or OptiX) will be
  evaluated after measurements.
- **Denoiser**: Intel Open Image Denoise (Apache 2.0), using albedo and normal buffers.

## Dependency choices

All maintained dependencies use permissive licenses (MIT/Apache/BSD/Zlib):
`glam`, `rayon`, `serde`, `tracing`, `image` (PNG only), `tobj`, `stl_io`,
`sysinfo`, `eframe`/`egui`/`wgpu` and `rfd`.

No unsupported or invented CUDA/OptiX bindings are used.
