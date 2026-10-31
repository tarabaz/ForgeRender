# Roadmap e stato reale

🇬🇧 [English version](ROADMAP.md) (versione di riferimento)

Legenda: ✅ implementato e testato · 🟡 parziale · ⏳ non ancora implementato

## Sintesi: cosa viene dopo e quanto è complicato

La complessità è una stima di lavoro e rischio per questo codice:
**Bassa** (ore, modifica circoscritta) · **Media** (un giorno o due, più moduli) ·
**Alta** (più giorni, nuovo sottosistema con CPU + GPU + interfaccia + test) ·
**Molto alta** (settimane, ricerca o SDK esterni, difficile da verificare).

| # | Funzionalità | Complessità | Dipende da | Note |
|---|---|---|---|---|
| 1 | ✅ Gizmo per le luci: spostare puntiformi/spot come gli oggetti, cono dello spot visibile e modificabile (maniglie per apertura e sfumatura) | Media | gizmo | Fatto: sposta/ruota le luci, scala le luci ad area, maniglie del cono |
| 2 | ✅ Snap a contatto per oggetto: durante lo spostamento l'oggetto si ferma quando tocca un'altra mesh | Media | picking BVH | Fatto: impostazione dell'oggetto salvata nel progetto, si può scivolare sulle superfici |
| 3 | ✅ Campionamento adattivo: arresto quando il rumore scende sotto una soglia | Media | — | Fatto: riquadri 16×16, CPU e GPU, test sul rumore (docs/BENCHMARK.md) |
| 4 | ✅ Lingue dell'interfaccia in file esterni (`Lan_ita`, `Lan_eng`, chiunque può aggiungere `Lan_spa`) | Media | — | Fatto (0.3.0): `lang/Lan_*.json`, cambio immediato, test di copertura delle traduzioni |
| 5 | Decal / adesivi (immagine proiettata sulla superficie, piana e cilindrica, con posizione/scala/rotazione) | Media | texture | — |
| 6 | ✅ Nebbia volumetrica da ghiaccio secco, statica e procedurale (rumore con seed, path tracer CPU con delta tracking) | Alta | — | Fatto, vedi sotto |
| 7 | ✅ Collisione della nebbia con gli oggetti + emettitore puntiforme con durata simulata (calcolata una volta, senza timeline) | Alta | 6 | Fatto |
| 8 | ✅ Volumetrici sulla GPU | Alta | 6 | Fatto: stesso algoritmo, test statistico CPU/GPU |
| 9 | Nebbia animata (timeline, simulazione fluidodinamica) | Molto alta | 6, 7 | Architettura predisposta in 6–7 |
| 10 | Anteprima del displacement nella viewport di modellazione | Media | displacement ✅ | Oggi il displacement si vede solo nel render |
| 11 | Mip-mapping / filtraggio delle texture | Media | — | Meno sfarfallio sulle texture lontane |
| 12 | DLSS Ray Reconstruction come anteprima interattiva veloce | Molto alta | SDK NVIDIA | Solo RTX, interop Vulkan con wgpu |
| 13 | Ray tracing hardware (RT core) | Molto alta | OptiX o ray query Vulkan | — |
| 14 | Generatore di edera: rampicanti fotorealistici che crescono sugli oggetti | Alta | — | **Da fare più avanti**, non pianificato ora |
| 15 | Utilizzo GPU / VRAM tramite NVML | Bassa | ✅ 0.6.0 (solo NVIDIA) | — |
| 16 | Import di volumi OpenVDB (fumo/fuoco simulati in Blender Mantaflow, EmberGen, Houdini) | Media | renderer della nebbia ✅ | Lettore VDB in Rust puro; il renderer gestisce già griglie di densità |

| 17 | Modalità di rendering aggiuntive: silhouette, linee tecniche, cel-shading a colori e monocromatico, illustrazione stilizzata | Da stimare | renderer CPU/GPU, materiali, pipeline di output | Il rendering fotorealistico attuale resta invariato e sempre disponibile. Proposta da valutare prima di implementare; dettagli in `reports/chatgpt/CLAUDE_HANDOFF.md`. |

| 18 | Esplosi e documentazione visiva: esplosione selettiva XYZ/radiale, callout, BOM aggregata e tavole/PDF | Da stimare | gerarchia e metadati CAD, trasformazioni, output | Modulo leggero di documentazione; niente quote CAD. Specifica e criteri in `reports/chatgpt/CLAUDE_HANDOFF.md`. |
| 19 | Viste multiple automatiche e varianti di prodotto | 🟡 Viste fatte (0.22.0), varianti da fare | camere, configurazioni scena/materiali | Viste standard con inquadratura coerente; confronto di materiali/colori/configurazioni. |
| 20 | Sezioni e cutaway | Da valutare | geometria, clipping/sezioni, CPU/GPU | Valutare prima una sezione visiva tramite piano di taglio; distinguere da operazioni CAD. |
| 21 | Match fotografico e compositing | Da valutare | camera, HDRI/illuminazione, output | Allineamento prospettico a una foto e compositing; analisi separata per complessità. |
| 22 | Render passes / AOV | 🟡 Base fatta (0.22.0): albedo, normali, profondità, maschera, ID; ombre ed emissione separate da fare | pipeline renderer, output PNG | Pass separati per colore, ombre, normali, emissione, profondità e altri supportati. |
| 23 | Turntable e sequenze animate | 🟡 Turntable con video MP4 fatto (0.22.0); sequenze di esplosione da fare | camera, timeline/configurazioni | Rotazione automatica del prodotto; in seguito sequenze di esplosione e montaggio. |

Ordine consigliato per ciò che resta: 4 → 3 → 5, poi il resto secondo necessità.

## Nebbia volumetrica da ghiaccio secco: progetto (implementato)

Obiettivo: mezzi partecipanti 3D veri dentro il path tracer Monte Carlo esistente, non sprite
o piani con texture, senza toccare la qualità di ciò che già funziona.

**Punti di integrazione individuati nel codice attuale**
- `forge_render_cpu/src/integrator.rs` (`radiance`): il ciclo del percorso trova già la superficie
  successiva (`t_surf`). Un evento nel mezzo viene campionato in `[0, t_surf]` prima di
  ombreggiare la superficie; se avviene il percorso diffonde nella nebbia, altrimenti arriva alla
  superficie con la throughput invariata (il delta tracking lo rende non distorto).
- Next Event Estimation (stesso file): i raggi d'ombra usano `textures::occluded`; moltiplicheranno
  anche per la trasmittanza del mezzo (ratio tracking), così la nebbia proietta e riceve ombre.
- `forge_render_core/src/render_scene.rs` (`RenderScene::compile`): qui si "cuoce" la densità
  procedurale in una griglia 3D, deterministica dal seed, in cache come la geometria.
- `forge_render_gpu/src/layout.rs` + `path_tracer.wgsl`: la griglia va nello stesso buffer bindless
  della scena; il ciclo WGSL riceve lo stesso delta/ratio tracking.
- `forge_scene`: nuova entità `FogVolume` (come le luci) salvata nel progetto.

**Fasi**
1. Crate `forge_volume`: rumore a gradiente con seed, fBm, deformazione con curl noise (filamenti,
   vortici), base densa con decadimento in altezza, espansione laterale, bordi sfumati; cotto in
   una griglia con una griglia grossolana dei massimi (majorant). Test: determinismo col seed,
   intervalli dei valori, tempi di calcolo.
2. Trasporto su CPU: delta tracking (Woodcock) per i cammini liberi, funzione di fase
   Henyey–Greenstein (anisotropia g), albedo di scattering, NEE dai punti nel mezzo con MIS,
   ratio tracking per le ombre. Test: trasmittanza di Beer–Lambert in uno strato omogeneo,
   white furnace con albedo 1, conservazione dell'energia; render reali da più angolazioni.
3. Comportamento da ghiaccio secco: emettitore messo come un punto nella scena; la nebbia di CO₂
   cade, si deposita a terra e si allarga ai lati, scorrendo attorno agli ostacoli (le mesh della
   scena vengono voxelizzate nella griglia, niente nebbia dentro gli oggetti). L'utente imposta un
   *tempo simulato* (es. 4 s): si calcola lo stato a quel tempo, quindi niente simulazioni infinite
   e nessuna timeline obbligatoria.
4. Porting GPU dello stesso algoritmo, test statistico CPU/GPU, benchmark.
5. Interfaccia: pannello nebbia (seed, densità, scala, turbolenza, scattering, anisotropia,
   dimensioni, tempo simulato, collisione sì/no), riquadro ed emettitore disegnati nella viewport,
   salvataggio nel progetto.
6. Più avanti: timeline e calcolo per fotogramma (il generatore riceve già un parametro di tempo),
   poi un vero solutore fluidodinamico che scrive lo stesso formato di griglia.

**Simulazione (seconda versione):** vero solutore euleriano per il fumo (Fedkiw et al. 2001)
su griglia sfalsata: trasporto semi-lagrangiano della velocità, proiezione della pressione
(SOR rosso/nero) con gli oggetti della scena come pareti, confinamento della vorticità per i
riccioli, trasporto MacCormack per filamenti netti, attrito col suolo (strato limite), sorgente
di volume nell'emettitore (il gas nuovo spinge fuori il vecchio: i contenitori si riempiono e
traboccano), CO₂ pesante che si mescola prima del vapore visibile (la nebbia vecchia diventa
leggera e risale a sbuffi, cursore "Risalita"). Verificata su una tazza (si riempie, trabocca,
cola lungo la parete, tappeto sul tavolo) e su un collo stretto (sbuffo sopra l'imboccatura,
colate lungo i lati); preset di scala per bicchiere, tavolo e palco.

**Stato:** le fasi 1–5 sono implementate e testate (crate `forge_volume`, `medium.rs` sulla CPU,
lo stesso codice in WGSL sulla GPU, pannello nebbia nella scheda Ambiente, emettitore e zona
disegnati nella viewport, gizmo sull'emettitore). Test: determinismo col seed e indipendenza dal
numero di thread, deposito ed espansione laterale, muro che ferma la nebbia, majorant, campionamento
Henyey–Greenstein, Beer–Lambert, white furnace con nebbia che diffonde, ratio tracking non distorto,
ombra attraverso la nebbia, GPU contro CPU su una nebbia simulata. Misure in
[BENCHMARK.md](BENCHMARK.md).

## Fase 1 — Fondamenta ✅
Workspace, app desktop, viewport con camera orbitale, mesh di test, test e documentazione.

## Fase 2 — Primo renderer CPU ✅
Ray tracing con BVH, normali, materiale diffuso, luci, ombre, path tracing progressivo,
accumulo corretto, export PNG.

## Criteri di accettazione della prima versione
| # | Criterio | Stato |
|---|---|---|
| 1 | Avviare l'applicazione Windows | ✅ zip portable e installer prodotti dalla CI |
| 2 | Importare una mesh di test | ✅ OBJ/STL/glTF + scena di test procedurale |
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

## Fase 3 — Materiali e illuminazione ✅
- ✅ BRDF GGX, metalli, dielettrici, riflessi, rifrazioni, Fresnel, editor dei materiali
- ✅ HDRI equirettangolari (.hdr/.exr) con importance sampling e MIS, rotazione e intensità
- ✅ Portali di luce per gli interni: rettangoli sulle finestre attraverso cui si campiona
  l'ambiente (con MIS); stessa immagine e circa 6 volte meno rumore a parità di campioni in una
  stanza illuminata da una finestra (test CPU e GPU)
- ✅ Set PBR completo: colore base, rugosità (o lucidità), metallicità, normal map (OpenGL o
  DirectX), height (bump), occlusione ambientale, ritaglio con opacità; mapping UV o proiezione
  box in metri per CAD senza UV; dimensione della ripetizione, rotazione, spostamento
- ✅ Displacement vero dalla height map: suddivisione al momento del render, lati condivisi senza
  crepe, coordinate texture originali conservate, budget di triangoli; testato su CPU e GPU
- ✅ Compensazione multi-scattering GGX (Turquin 2019)
- ✅ Superfici emissive campionate come luci (NEE + MIS)
- ✅ Vetro satinato (trasmissione microfacet, Walter 2007)
- ⏳ Mip-mapping, decal

## Fase 4 — GPU 🟡
- ✅ Path tracer in compute shader WGSL via wgpu (Vulkan/DX12), stesso algoritmo della CPU, un solo
  buffer bindless della scena, passate adattive (niente timeout TDR)
- ✅ Test GPU (lavapipe in CI): white furnace, luce ad area ed emissivi contro formule analitiche,
  HDRI, texture, displacement, profondità di campo, convergenza statistica all'immagine CPU
- ✅ Selettore CPU/GPU nell'interfaccia e nella CLI, ripiego automatico sulla CPU
- ✅ Misure sulla RTX 5070 ([BENCHMARK.md](BENCHMARK.md)): 139 Mcampioni/s, 4,6 volte il 9900X a 24 thread
- ✅ NVML (utilizzo e VRAM, schede NVIDIA) · ⏳ ray tracing hardware, rendering ibrido CPU+GPU

## Fase 5 — CAD 🟡
- ✅ glTF/GLB (esportazione consigliata da SolidWorks), OBJ, STL, Blender `.blend`
- 🟡 STEP tramite Open CASCADE (convertitore separato, per ora escluso dai pacchetti della CI:
  esportare GLB)
- 🟡 SolidWorks SLDPRT/SLDASM tramite esportazione COM (da provare su un PC con SolidWorks)
- ⏳ Instancing nel renderer

## Import da Blender e glTF ✅
Gerarchia, mesh, materiali Principled e texture, camere, luci `KHR_lights_punctual`; i file
`.blend` vengono esportati tramite Blender avviato in background.

## Fase 6 — Denoising e qualità 🟡
- ✅ Intel Open Image Denoise 2.3.3 caricato a runtime, dispositivo CPU o CUDA, guide albedo/normali,
  modalità solo finale o anteprima progressiva, intensità
- ✅ Campionamento adattivo (arresto a soglia di rumore, CPU e GPU) con test sul rumore contro un riferimento
- ⏳ Benchmark completi

## Fase 7 — Editor professionale 🟡
- ✅ Libreria materiali (~120 preset + materiali e HDRI scaricati da ambientCG / Poly Haven,
  cartelle locali `Libreria` con anteprime)
- ✅ Gizmo di trasformazione (sposta / ruota / scala, snap), annulla/ripeti, nascondi (H), duplica,
  appoggio esatto sul pavimento, navigatore degli assi stile Blender
- ✅ Camere di rendering con obiettivo professionale: focale in mm, diaframma f/, profondità di
  campo con piano di fuoco e limiti vicino/lontano nella viewport, lamelle, decentramento
- ✅ Gizmo per le luci (sposta/ruota, dimensioni delle luci ad area, maniglie del cono dello spot), snap a contatto per oggetto
- ✅ Nebbia volumetrica da ghiaccio secco (procedurale, con seed, collisioni, CPU e GPU)
- ⏳ Decal, lingue, regole di assegnazione avanzate

## NVIDIA: CUDA, OptiX e DLSS (piano)

| Passo | Cosa | Requisiti | Stato |
|---|---|---|---|
| 1 | Denoiser OIDN su CUDA | driver NVIDIA, GPU Volta o successiva | fatto, collaudato sulla RTX 5070: 18 ms contro 51 ms della CPU (640×360) |
| 2 | DLSS Ray Reconstruction come anteprima veloce della vista Render | NVIDIA Streamline/NGX su Vulkan, interop con le risorse wgpu, solo RTX | da fare |
| 3 | Menu DLSS: versione del modello (una cartella di DLL per versione, menu costruito da un manifest), risoluzione interna, nitidezza, miscela con l'immagine accumulata | passo 2 | da fare |
| 4 | Ray tracing hardware (RT core) | OptiX o ray query Vulkan | da valutare |

Note:
- I "CUDA core" sono le unità di calcolo della GPU: il path tracer WGSL li usa già.
- DLSS Ray Reconstruction è pensato per il tempo reale; per l'immagine finale ferma si usa OIDN.
