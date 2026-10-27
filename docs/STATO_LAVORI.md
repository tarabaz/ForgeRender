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

## Fatto nella seconda sessione
- Warning clippy risolti; CI GitHub Actions (Linux: fmt, clippy -D warnings, test; Windows: test, build MSVC,
  benchmark indicativo, pacchetti portable zip + installer Inno Setup come artifact).
- `.cargo/config.toml`: CRT statico per MSVC (nessun redistributable richiesto).
- Documentazione: README, ARCHITECTURE, ROADMAP, BENCHMARK.

## Fatto nella terza parte
- CI verde end-to-end (run #2): test su Windows MSVC, build, pacchetti portable (7,8 MB) e
  installer (6,7 MB) caricati come artifact.
- Log su file (portable: .\logs, installato: %LOCALAPPDATA%\ForgeRender\logs).
- Fase 3: HDRI .hdr/.exr con importance sampling + MIS; texture colore/roughness/normal map
  con mapping UV o box (CAD senza UV). Verificate con test analitici e render visivi.

## Prossimi passi
1. Prova sul PC reale: scaricare ForgeRender-portable-x64 dagli artifact della CI.
2. Benchmark sul 9900X (`forgerender-cli bench --json bench-9900x.json`).
3. Fase 3 restante: compensazione multi-scattering GGX, NEE sulle superfici emissive,
   trasmissione ruvida, mip-mapping.
4. Fase 4: backend GPU wgpu compute (riusa il layout BVH di forge_accel).
5. Fase 5: convertitore STEP (Open CASCADE) e bridge SolidWorks (C#/COM).
