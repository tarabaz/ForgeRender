# ForgeRender Changelog

🇮🇹 [Leggi il changelog in italiano](CHANGELOG.md)

The workspace version is defined once, in `version` under `[workspace.package]` in the root `Cargo.toml`. All components inherit it. The window title displays `ForgeRender v<version> (<commit>)`.

## Versioning policy (0.x)

- **0.X.0** (minor): a group of new features is delivered. Increment the version in the commit that completes the group and update this file.
- **0.X.Y** (patch): bug fixes only, with no new features.
- Each release adds a dated section here. The version is set in `Cargo.toml` and, as a fallback, in `packaging/windows/forgerender.iss`.

## Unreleased (development branch, not on main)

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
