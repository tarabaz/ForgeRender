# Architettura di ForgeRender

## Workspace

| Crate | Ruolo |
|---|---|
| `forge_math` | AABB, raggi (offset anti-acne di Wächter & Binder), basi ortonormali, RNG PCG32 deterministico, campionamento |
| `forge_scene` | Modello dati: mesh, albero componenti con `ComponentId` stabili, materiali, luci, ambiente, camera, unità, primitive |
| `forge_accel` | BVH binaria SAH a 16 bin, nodi `#[repr(C)]` da 32 byte (riutilizzabili sulla GPU), intersezione Möller–Trumbore |
| `forge_render_core` | `RenderSettings`, `RenderGeometry` (BVH condivisa), `RenderScene`, accumulatore, tone mapping, trait `RenderBackend`, `RenderSession` |
| `forge_render_cpu` | Backend CPU: BSDF, campionamento luci, integratore path tracing, pool Rayon dedicato |
| `forge_import` | Import OBJ/STL, validazione mesh, normali con angolo di piega, conversione unità e Z-up → Y-up |
| `forge_project` | Formato `.frproject`, versionamento schema, salvataggio atomico, report componenti/file mancanti |
| `forge_image_io` | Export PNG, JPG, TIFF 16 bit ed EXR con scrittura atomica |
| `forge_telemetry` | Rilevamento hardware (sysinfo), monitor CPU/RAM, log `tracing` con buffer per la UI |
| `forge_cli` | `forgerender-cli`: render headless, benchmark scalabilità thread |
| `forge_app` | `forgerender`: GUI egui/eframe, viewport wgpu fuori schermo, orchestrazione |

Materiali, luci e camera sono moduli di `forge_scene` invece che crate separati: sono
tipi di dati piccoli e strettamente legati, separarli avrebbe solo aggiunto attrito.

## Flusso di rendering

1. La UI modifica la `Scene` e incrementa due revisioni: `geometry_rev` (mesh, visibilità)
   e `shading_rev` (materiali, luci, ambiente, camera).
2. Se cambia la geometria, la BVH (`RenderGeometry`) viene ricostruita **su un thread
   separato**; altrimenti si riusa: spostare la camera o cambiare un materiale costa
   pochi microsecondi anche su assiemi da milioni di triangoli.
3. `RenderScene::compile_with_geometry` produce i dati per l'integratore e li passa a
   `RenderSession`, che gira su un thread proprio.
4. Ogni passata del backend calcola **un campione per ogni pixel** in un buffer separato.
   Solo le passate complete vengono sommate all'accumulatore: pausa, stop o riavvio
   scartano la passata in corso, quindi l'immagine non contiene mai campioni parziali
   o incompatibili.
5. La UI legge l'ultimo frame (media lineare) e applica tone mapping ed esposizione
   senza riavviare il render.

Il generatore casuale dipende solo da `(seed, pixel, indice campione)`: lo stesso render
dà lo stesso risultato con 1 o 24 thread (verificato da test), il che rende possibile
in futuro distribuire i campioni tra CPU e GPU senza duplicazioni.

## Integratore

Path tracing unidirezionale:
- NEE verso una luce scelta uniformemente; MIS con euristica della potenza tra
  campionamento della luce e della BSDF per le luci ad area; peso 1 per luci delta;
- l'ambiente e le superfici emissive sono raggiunti solo via BSDF (peso 1, stimatore corretto);
- Russian roulette dal 3° rimbalzo; clamp dei contributi indiretti opzionale e disattivato
  (introdurrebbe bias).

BSDF "principled lite": Lambert pesato (1−F)(1−F) + GGX con Smith height-correlated e
Schlick, campionamento delle normali visibili (Heitz 2018); vetro liscio con Fresnel
esatto e scala della radianza (n₁/n₂)².

### Verifiche automatiche

- *White furnace*: sfera Lambert con albedo ρ in ambiente uniforme = ρ esatto; vetro = 1;
  metalli/plastiche ≤ 1 (nessuna energia creata).
- Luce puntiforme e luce rettangolare confrontate con le soluzioni analitiche (fattore di forma).
- Coerenza tra `sample`, `eval` e `pdf` delle BSDF; reciprocità; PDF normalizzate.
- BVH contro intersezione a forza bruta su raggi casuali.
- Convergenza: RMSE a 128 campioni < metà di quello a 8.

### Limiti noti della Milestone 1

- La compensazione multi-scattering di Turquin dipende solo dalla direzione d'uscita:
  reciprocità approssimata (scarto < 1%).
- Niente profondità di campo, opacità/ritagli, displacement.
- Le caustiche (luce focalizzata da vetro/metallo) convergono lentamente, come in ogni
  path tracer unidirezionale.
- Geometria appiattita in spazio mondo (niente instancing): assiemi con molte copie dello
  stesso pezzo occupano più memoria. Instancing a due livelli previsto con l'import STEP.

## Viewport

wgpu (DX12/Vulkan su Windows, GPU ad alte prestazioni preferita, quindi la RTX invece
della grafica integrata del Ryzen). La scena è disegnata in una texture fuori schermo
con MSAA 4× e depth buffer, poi mostrata da egui. Se l'inizializzazione GPU fallisce,
l'app si riavvia da sola con `--safe-mode` (OpenGL, viewport disattivata, render CPU attivo).

## Progetti

`.frproject` è JSON leggibile con `format`, `schema_version`, riferimenti ai file sorgente
(percorso relativo + assoluto di riserva + opzioni di import), materiali, assegnazioni
`ComponentId → MaterialId`, visibilità, luci, ambiente, camera e impostazioni di render.
La geometria non è copiata: il file CAD originale non viene mai modificato.
Assegnazioni a componenti spariti vengono conservate e segnalate.
Le texture (Fase 3) seguiranno la stessa strategia: riferimento esterno relativo/assoluto,
con opzione futura di "pacchetto progetto" che copia gli asset in una cartella accanto.

## Piani per le parti non-Rust (Fasi 4–5)

- **STEP**: convertitore esterno minimo in C++ con Open CASCADE (XCAF: gerarchia, nomi,
  colori, unità, trasformazioni; triangolazione con tolleranza controllata), eseguito come
  processo separato: un crash di OCCT non chiude l'app e la licenza LGPL resta isolata.
- **SolidWorks**: bridge C# con le Interop DLL installate con SolidWorks (COM), che apre il
  documento in sola lettura ed esporta STEP AP214/AP242 + JSON con struttura, id e
  trasformazioni. Richiede SolidWorks con licenza sul PC.
- **GPU**: prima un path tracer in compute shader WGSL via wgpu che riusa il layout BVH di
  `forge_accel`; ray tracing hardware (Vulkan ray query o OptiX) valutato dopo le misure.
- **Denoiser**: Intel Open Image Denoise (Apache 2.0), con buffer albedo e normali.

## Scelte sulle dipendenze

Tutte mantenute e con licenze permissive (MIT/Apache/BSD/Zlib): `glam`, `rayon`, `serde`,
`tracing`, `image` (solo PNG), `tobj`, `stl_io`, `sysinfo`, `eframe`/`egui`/`wgpu`, `rfd`.
Nessun binding inventato verso CUDA/OptiX.
