# Roadmap e stato reale

Legenda: ✅ implementato e testato · 🟡 parziale · ⏳ non ancora implementato

## Fase 1 — Fondamenta ✅
Workspace, app desktop, viewport con camera orbitale, mesh di test, test e documentazione.

## Fase 2 — Primo renderer CPU ✅
Ray tracing con BVH, normali, materiale diffuso, luci, ombre, path tracing progressivo,
accumulo corretto, export PNG.

## Criteri di accettazione della prima versione
| # | Criterio | Stato |
|---|---|---|
| 1 | Avviare l'applicazione Windows | 🟡 compila per Windows (MSVC in CI, mingw in locale); da provare sul PC reale |
| 2 | Importare una mesh di test | ✅ OBJ/STL + scena di test procedurale |
| 3 | Visualizzare e ruotare il modello | ✅ |
| 4 | Impostare un materiale | ✅ |
| 5 | Impostare luce e camera | ✅ |
| 6 | Rendering CPU reale | ✅ |
| 7 | Miglioramento progressivo con i campioni | ✅ |
| 8 | Modificare il numero massimo di campioni | ✅ anche durante il render |
| 9 | Fermare e vedere il risultato corrente | ✅ |
| 10 | Esportare PNG | ✅ |
| 11 | Salvare e riaprire un progetto | ✅ |
| 12 | Test automatici del motore | ✅ |

## Fase 3 — Materiali e illuminazione 🟡
- ✅ BRDF GGX, metalli, dielettrici, riflessi, rifrazioni, Fresnel, persistenza materiali, editor base
- ✅ HDRI equirettangolari (.hdr/.exr) con importance sampling e MIS, rotazione e intensità,
  verificate contro integrazione di riferimento
- ✅ texture colore base / roughness / normal map (PNG, JPEG, EXR, HDR), mapping UV o
  proiezione box in metri per CAD senza UV, ripetizione, rotazione, spostamento; percorsi
  ritrovati se il progetto viene spostato
- ✅ compensazione multi-scattering GGX (Turquin 2019): i metalli ruvidi conservano l'energia
- ✅ superfici emissive campionate come luci (NEE proporzionale alla potenza + MIS), verificate
  contro il fattore di forma analitico
- ✅ vetro satinato (trasmissione microfacet GGX, Walter 2007) con white furnace
- ⏳ bump/displacement, opacità, mip-mapping

## Fase 4 — GPU 🟡
- ✅ Path tracer in compute shader WGSL via wgpu (Vulkan/DX12), stesso algoritmo del CPU:
  BSDF con multi-scattering, vetro liscio e satinato, NEE+MIS su luci, HDRI ed emissivi,
  texture, Russian roulette. Scena in un unico buffer "bindless".
- ✅ Passate adattive con più campioni per pixel (≈60 ms per passata, niente timeout TDR),
  dispositivo e shader preparati in background all'avvio e riutilizzati.
- ✅ Test su GPU (lavapipe in CI): white furnace, luce ad area ed emissivi contro formule
  analitiche, HDRI con sole contro CPU, texture, convergenza statistica alla stessa immagine CPU.
- ✅ Selettore CPU/GPU nell'interfaccia e nella CLI (`--backend gpu`), riga GPU nel benchmark,
  ritorno automatico alla CPU se la GPU non è disponibile.
- ⏳ misure sulla RTX 5070, utilizzo GPU/VRAM via NVML, ray tracing hardware (RT core),
  rendering ibrido CPU+GPU. CUDA/OptiX esclusi per ora.

## Fase 5 — CAD 🟡
- ✅ Import STEP/STP tramite `forgerender-step` (Open CASCADE 7.8.1, processo separato):
  gerarchia dell'assieme, nomi delle istanze, trasformazioni, unità del file, colori per
  pezzo e per faccia, triangolazione a qualità regolabile, mesh condivise tra istanze uguali,
  id stabili. Test end-to-end con un assieme STEP generato da OCCT.
- ✅ Importazione in background con avanzamento; opzioni (unità, asse verticale, qualità,
  appoggio sul pavimento).
- 🟡 SolidWorks SLDPRT/SLDASM: esportazione STEP tramite le API COM di SolidWorks (script
  PowerShell, sola lettura). **Implementato ma non verificabile qui**: da provare sul PC con SolidWorks.
- ⏳ test con i file STEP/SLDASM reali dell'utente, instancing nel renderer (oggi le istanze
  condividono la mesh in memoria ma la BVH è appiattita).

## Import da Blender e glTF ✅
- `.glb`/`.gltf`: gerarchia, mesh, materiali Principled (colore, metallic, roughness, IOR,
  trasmissione, emissione), texture incorporate o esterne, camera, luci `KHR_lights_punctual`
  (unità fotometriche convertite)
- `.blend`: Blender installato viene avviato in background per esportare glTF; le luci ad area
  (non supportate da glTF) vengono esportate a parte. Verificato con Blender 4.0 su una scena
  di prova generata da script. I nodi shader complessi di Cycles non hanno equivalente: vanno
  "cotti" in texture (bake) in Blender.

## Fase 6 — Denoising e qualità ⏳
Open Image Denoise con buffer albedo/normali, arresto a soglia di rumore, benchmark completi.

## Fase 7 — Editor professionale 🟡
- ✅ libreria di ~120 materiali in 11 categorie con ricerca, editor luci con preset, viste camera,
  salvataggio progetti
- ✅ trasformazioni non distruttive (spostamento, rotazione, scala, dimensioni in mm,
  appoggia sul pavimento, centra, rotazioni di 90°, Alt+trascina nella viewport)
- ⏳ regole di assegnazione avanzate, rimappatura componenti rinominati, profondità di campo,
  ripresa del render dopo la chiusura, file di log su disco, lingue aggiuntive, annulla/ripeti
