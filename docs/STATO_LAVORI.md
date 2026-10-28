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

## Fatto nella sesta parte
- Gizmo sposta/ruota/scala, annulla/ripeti, H per nascondere, duplica, appoggio esatto,
  navigatore degli assi, camere di rendering con obiettivo professionale e profondità di campo.
- Libreria materiali/HDRI con download ambientCG e Poly Haven, mappe PBR complete.
- Displacement vero dalla height map (suddivisione al render, CPU e GPU) e pannello "Scala e
  posizionamento texture" nel materiale.
- README e ROADMAP in inglese con copie italiane; sintesi delle prossime funzionalità con la
  complessità (vedi docs/ROADMAP.it.md).

## Fatto nella settima parte
- Nebbia volumetrica da ghiaccio secco: crate forge_volume (rumore con seed, simulazione di caduta,
  deposito ed espansione come corrente di gravità con ostacoli, turbolenza da curl noise, bake su
  griglia a 16 bit con majorant per strato), trasporto nel path tracer CPU (delta tracking, ratio
  tracking, Henyey–Greenstein, NEE con MIS) e lo stesso in WGSL sulla GPU, calcolo in background
  nell'app, pannello nella scheda Ambiente, zona ed emettitore nella viewport, guide del denoiser.
- Gizmo per luci (sposta/ruota, dimensioni delle luci ad area) ed emettitori di nebbia; cono dello
  spot nella viewport con maniglie per apertura e sfumatura; selezione col clic sulle icone.
- Snap a contatto per oggetto (salvato nel progetto): lo spostamento si ferma al primo contatto,
  si può scivolare lungo le superfici.

## Fatto nell'ottava parte
- Nebbia: simulazione fluida vera (Fedkiw 2001) al posto del modello a diffusione: pressione con
  gli oggetti come pareti, vorticità, MacCormack, attrito col suolo, sorgente di volume, gas
  pesante che si mescola e risale ("Risalita"); preset bicchiere/tavolo/palco; la zona si estende
  fino al pavimento della scena; test su tazza che trabocca e risalita; ottimizzazioni (8×).

## Prossimi passi
Vedi la tabella di sintesi in docs/ROADMAP.it.md (gizmo luci, snap a contatto, lingue,
campionamento adattivo, nebbia volumetrica…).
