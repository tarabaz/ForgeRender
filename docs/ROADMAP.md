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
- ⏳ texture e UV, normal/bump map, HDRI equirettangolari con importance sampling,
  trasmissione ruvida, compensazione multi-scattering, NEE sulle superfici emissive

## Fase 4 — GPU ⏳
Path tracer WGSL in compute shader via wgpu, confronto CPU/GPU sulla stessa scena,
NVML per utilizzo GPU/VRAM. CUDA/OptiX esclusi per ora.

## Fase 5 — CAD ⏳
STEP via Open CASCADE (processo separato), bridge SolidWorks (C#/COM), instancing,
test con i file STEP/SLDASM reali.

## Fase 6 — Denoising e qualità ⏳
Open Image Denoise con buffer albedo/normali, arresto a soglia di rumore, benchmark completi.

## Fase 7 — Editor professionale 🟡
- ✅ libreria materiali, editor luci con preset, viste camera, salvataggio progetti
- ⏳ regole di assegnazione avanzate, rimappatura componenti rinominati, profondità di campo,
  ripresa del render dopo la chiusura, file di log su disco, lingue aggiuntive, annulla/ripeti
