# Stato dei lavori (punto di ripresa)

## Milestone 1 — fatto e verificato
- Workspace Cargo: forge_math, forge_scene, forge_accel (BVH SAH), forge_render_core
  (settings, RenderGeometry/RenderScene, accumulatore, tone mapping, RenderSession asincrona),
  forge_render_cpu (path tracer: Lambert + GGX VNDF + vetro Fresnel, NEE + MIS, Russian roulette),
  forge_import (OBJ/STL), forge_project (.frproject JSON versionato, salvataggio atomico),
  forge_image_io (PNG), forge_telemetry (hw, log), forge_cli (render/bench), forge_app (GUI egui + viewport wgpu).
- Test automatici verdi (white furnace, luce puntiforme e area vs analitico, convergenza, BVH vs brute force, progetto round-trip, sessione pausa/ripresa/stop).
- GUI provata sotto Xvfb + lavapipe (screenshot viewport e render progressivo OK).
- Cross-compilazione Windows (x86_64-pc-windows-gnu) OK: exe senza DLL esterne.

## Da fare alla ripresa
1. Sistemare 2 warning clippy in forge_app (app.rs new_empty_scene, inspector.rs if annidato).
2. Packaging: `.cargo/config.toml` con crt-static per MSVC, script `packaging/build-windows.ps1`
   (portable zip + installer Inno Setup), workflow GitHub Actions Windows.
3. Documentazione: README, docs/ARCHITECTURE.md, docs/ROADMAP.md, istruzioni build Windows.
4. Benchmark nel container (`forgerender-cli bench`) e poi sul PC reale.
5. Poi Fase 3 (texture, HDRI, editor materiali avanzato) e Fase 4 (backend GPU wgpu compute).
