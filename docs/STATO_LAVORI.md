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

## Fatto nella quarta parte
- Trasformazioni non distruttive (scheda Oggetto, mm, Alt+trascina), libreria di 124 materiali.
- Import STEP con Open CASCADE (convertitore separato forgerender-step, test end-to-end),
  bridge SolidWorks via COM (da collaudare sul PC con SolidWorks).
- Import glTF/GLB e .blend (via Blender in background), verificato con Blender 4.0.
- Fase 3 completata: compensazione multi-scattering, emissivi come luci, vetro satinato.
- Fase 4: backend GPU WGSL via wgpu con test fisici e confronto statistico con la CPU.

## Fatto nella quinta parte
- Build Windows completa in CI (Open CASCADE statico con MSVC: OCCT_STATIC_BUILD, uscita con _exit).
- Collaudo su hardware reale: Xeon E-2176G + Quadro P5000 (Vulkan), GPU ≈ 6× la CPU.
- Fase 6 (denoiser): Intel Open Image Denoise 2.3.3 caricato a runtime (crate forge_denoise),
  dispositivo CPU o CUDA (spunta "Usa CUDA", ripiego automatico sulla CPU), buffer guida
  albedo/normali calcolati sulla CPU (anche attraverso il vetro), modalità "solo finale" e
  "anteprima veloce" (8, 16, 32… campioni), qualità, intensità, viste di controllo. Errore
  ridotto ~10× a 8 campioni nei test (MSE contro riferimento a 512 campioni).

## Prossimi passi
1. Prova sul PC con RTX 5070: denoiser CUDA, render GPU, benchmark, file STEP/SLDASM reali.
2. DLSS Ray Reconstruction come anteprima interattiva su RTX (vedi docs/ROADMAP.md, "NVIDIA").
3. Arresto a soglia di rumore; NVML per utilizzo/VRAM della GPU.
