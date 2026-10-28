# ForgeRender

🇮🇹 [Leggi in italiano](README.it.md)

A photorealistic renderer for CAD models, mechanical assemblies and general 3D scenes,
written in Rust. Physically based progressive path tracing on the CPU and on the GPU,
desktop interface, designed for Windows 11 (developed for a Ryzen 9 9900X + RTX 5070)
but also working without a dedicated GPU.

> See [docs/ROADMAP.md](docs/ROADMAP.md) for what really works today, what comes next and
> how complex each next step is.

## What it does today

- **Path tracing** on the CPU (all cores) and on the GPU (wgpu compute shaders, Vulkan/DX12)
  with real progressive accumulation (16…1024 samples, custom or unlimited, changeable while
  rendering). Next Event Estimation + Multiple Importance Sampling, Russian roulette, SAH BVH.
- **Denoiser**: Intel Open Image Denoise loaded at runtime, CPU or CUDA device, albedo/normal
  guide buffers, final-only or progressive preview mode.
- **Materials**: diffuse, GGX metals and plastics with multiple-scattering compensation, glass
  with exact Fresnel, frosted glass, emissive surfaces sampled as lights.
  Full PBR maps: base colour, roughness/gloss, metallic, normal (OpenGL/DirectX), height (bump),
  **true displacement**, ambient occlusion, opacity cutout; UV mapping or world-space box
  projection in metres for CAD models without UVs, with repeat size, rotation and offset.
- **Library**: ~120 ready-made materials, plus materials and HDRIs downloaded from
  ambientCG and Poly Haven or imported from folders/zip files, kept in a local `Library`
  folder with previews.
- **Lights and environment**: area, point, spot and directional lights, studio presets;
  uniform, gradient or HDRI environment (.hdr/.exr) with importance sampling.
- **Cameras**: orbit viewport camera plus render cameras with a professional lens: focal length
  in mm, f-stop, depth of field (focus plane and near/far limits drawn in the viewport),
  aperture blades, lens shift.
- **Import**: glTF/GLB (recommended export from SolidWorks), OBJ (with .mtl), STL, Blender
  `.blend` (through an installed Blender); STEP via Open CASCADE and SolidWorks SLDPRT/SLDASM
  are experimental.
- **Editing**: move/rotate/scale gizmo with snapping, undo/redo, hide (H), duplicate,
  exact drop to floor, Blender-style axis navigator, component tree, persistent material
  assignment by stable id.
- Start / pause / resume / stop / automatic restart when the scene changes, PNG export.
- `.frproject` projects (versioned JSON, atomic save, relative paths).
- CLI for headless rendering, benchmarks and library import.

The interface is currently in Italian; external language files (Italian and English first)
are on the roadmap.

## Download the Windows build

Every push is built by GitHub Actions (job *Build e pacchetti (Windows)*); two packages are
available in the *Artifacts* section of the run:

| Package | Contents |
|---|---|
| `ForgeRender-portable-x64` | zip: extract and run `forgerender.exe`, no installation |
| `ForgeRender-setup-x64` | installer (Start menu, uninstaller, `.frproject` association) |

Both are self-contained: static CRT, only Windows system DLLs plus the bundled Open Image
Denoise libraries.

## Build locally

Requirements: stable Rust (`rustup`); on Windows the *Visual Studio Build Tools* (C++).

```
cargo test --workspace --release
cargo run --release -p forge_app            # graphical interface
cargo run --release -p forge_cli -- bench   # benchmark
```

Windows packages locally: `powershell -ExecutionPolicy Bypass -File packaging\windows\package.ps1`
(the installer needs [Inno Setup 6](https://jrsoftware.org/isdl.php)).

## Documentation

- [docs/ROADMAP.md](docs/ROADMAP.md) — status, next steps with complexity, technical plans
  ([italiano](docs/ROADMAP.it.md))
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — architecture, technical choices, risks (Italian)
- [docs/BENCHMARK.md](docs/BENCHMARK.md) — measurements actually performed (Italian)
- [docs/STATO_LAVORI.md](docs/STATO_LAVORI.md) — development log (Italian)
