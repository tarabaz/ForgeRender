# Fumo e fuoco in ForgeRender — studio e piano

🇬🇧 [English version below](#smoke-and-fire-in-forgerender--study-and-plan)

Stato: **studio**, nessuna modifica al motore. L'implementazione parte quando Valerio ha letto e approvato il piano.

## 1. Obiettivo

Portare in ForgeRender un simulatore di fumo e fuoco nello stile di **Chaos Phoenix** (ex Phoenix FD) e **FumeFX** (Sitni Sati), pensato per la fotografia di prodotto: la candela accanto al flacone di profumo, il vapore della tazza di caffè, il fumo d'incenso, la fiamma del camino che si riflette nel vetro, il barbecue. Si parte da quello che c'è già: la nebbia da ghiaccio secco simulata (`forge_volume`) e il delta tracking nel path tracer CPU e GPU.

## 2. Come lavorano Phoenix e FumeFX

Le fonti sono la documentazione pubblica dei due prodotti e gli articoli su cui si basano (elenco nel § 8). I nomi dei parametri sono quelli dei manuali; i numeri di tempo e risoluzione sono ordini di grandezza da manuali, tutorial ed esempi, non misure nostre.

### 2.1 Griglia

- **Euleriana, a celle cubiche.** Entrambi simulano su una griglia regolare: velocità e grandezze scalari per cella. Il parametro principale è la dimensione della cella (*Cell Size* in Phoenix, *Spacing* in FumeFX); la risoluzione segue dalla dimensione del contenitore.
- **Contenitore adattivo.** Phoenix ha l'*Adaptive Grid*: il riquadro cresce e si restringe seguendo i canali sopra una soglia (fumo, temperatura), con un limite massimo. FumeFX ha l'*Adaptive* del contenitore, con lo stesso principio. Così si paga solo il volume dove c'è fumo.
- **Sparsità.** Le versioni recenti di FumeFX dichiarano un solver a griglia sparsa (memoria e calcolo solo nei blocchi attivi). Phoenix resta soprattutto a riquadro adattivo; legge e scrive OpenVDB, che è sparso per natura. L'idea comune è la stessa: lavorare solo dove c'è qualcosa.
- **Passi.** Il passo di tempo segue la condizione di Courant (CFL): FumeFX espone *CFL* e *Maximum Iterations*, Phoenix *Steps per Frame* e *Time Scale*. Fiamme veloci richiedono più sottopassi per fotogramma.

### 2.2 Campi (canali)

| Canale | Phoenix | FumeFX | A cosa serve |
|---|---|---|---|
| Velocità | Velocity | Velocity | trasporta tutto il resto |
| Fumo / densità | Smoke | Smoke | opacità e diffusione della luce |
| Temperatura | Temperature | Temperature | spinta verso l'alto, colore e luce del fuoco |
| Combustibile | Fuel | Fuel | brucia: produce calore, fumo, espansione |
| Colore | RGB | Color | fumo colorato (scenografico) |
| Coordinate di tessitura | UVW / Wavelet | Texture coords | dettagli che seguono il fluido |

Ogni canale è trasportato dalla velocità (semi-lagrangiano, con varianti di ordine più alto tipo MacCormack/BFECC) e decade nel tempo (*Dissipation*, *Cooling*).

### 2.3 Spinta e combustione

- **Spinta di galleggiamento** (Fedkiw, Stam, Jensen 2001): f = (−α·fumo + β·(T − T_amb))·ŷ. Il fumo pesa, il caldo sale. Phoenix ha *Buoyancy* e *Smoke Buoyancy*; FumeFX *Buoyancy* e *Gravity*.
- **Raffreddamento**: la temperatura torna verso quella dell'ambiente (*Cooling*), così la fiamma si ferma a un'altezza e sopra resta il fumo.
- **Combustione** (modello a combustibile, semplificazione di Nguyen, Fedkiw, Jensen 2002): dove T supera la *Ignition Temperature* e c'è combustibile, una parte brucia a ogni passo (*Burn Rate*). Il combustibile bruciato:
  - libera calore (*Heat Production*): T sale;
  - produce fumo (*Smoke Density* / *Smoke Amount*);
  - si espande (*Expansion*): una sorgente di divergenza nella pressione, la fiamma "gonfia" e spinge l'aria intorno.
- Il risultato è il ciclo che rende viva una fiamma: il combustibile esce freddo, si accende, il calore lo accelera verso l'alto, si stira in lingue, si raffredda, diventa fumo.

### 2.4 Vortici e dettaglio

- **Confinamento della vorticità** (*Vorticity*): reinserisce la rotazione persa per la diffusione numerica della griglia. Dà i riccioli e le lingue.
- **Turbolenza procedurale** (*Turbulence*, campi di rumore): agitazione aggiuntiva, più forte dove la velocità è alta.
- **Wavelet turbulence** (Kim, Thürey, James, Gross 2008): si simula grosso e poi si ricalcola a risoluzione 2–4× più fine, aggiungendo rumore wavelet con l'energia presa dalla simulazione grossa e trasportato da coordinate di tessitura che seguono il fluido. FumeFX ha la *Wavelet Turbulence* come secondo passaggio; Phoenix ha la *Resimulation* con *Amplify Resolution* (dettaglio "wavelet" o "smoothing"). Vantaggio: la forma generale resta quella approvata, il dettaglio costa una frazione di una simulazione fine.

### 2.5 Sorgenti e collisioni

- **Sorgenti su oggetti.** Phoenix: *Fire Source* (PHXSource) con modalità *Emit* (portata uscente dalla superficie, con velocità di uscita *Discharge*) e *Inject* (riempie il volume dell'oggetto); per ogni canale si sceglie il valore (temperatura, fumo, combustibile, colore). FumeFX: *Object Source*, *Simple Source* (sfera, riquadro), *Particle Source*. Entrambi permettono di modulare l'emissione con una mappa o un rumore (fiamma che sfarfalla, bruciatore a fori) e di emettere da una parte sola della superficie.
- **Collisioni.** Gli oggetti della scena sono voxelizzati; la velocità non entra nei solidi (con la loro velocità, se si muovono). Opzionale: un oggetto non emette ma blocca.
- **Velocità ereditata** dagli oggetti in movimento (non ci serve per le foto ferme).

### 2.6 Resa del fuoco

- **Emissione da corpo nero.** Il colore del fuoco viene dalla temperatura: legge di Planck → colore. Phoenix ha la modalità *Physically Based* per il fuoco (temperatura in kelvin, intensità regolabile) accanto a una curva colore libera; FumeFX usa una sfumatura per temperatura/combustibile. In pratica entrambi danno all'artista: intensità, curva, opacità del fuoco separata da quella del fumo.
- **Fumo.** Opacità dal canale fumo, colore e albedo propri; fumo scuro = più assorbimento (fuliggine), fumo chiaro = più diffusione.
- **Il fuoco come luce.** È la parte che fa la differenza in una foto: la fiamma illumina gli oggetti, si riflette, fa ombre. Phoenix crea *Fire Lights*: luci puntiformi automatiche distribuite nella griglia, pesate dall'emissione (poco rumore, approssimate). FumeFX ha l'*Illumination Map* (luce precalcolata in una griglia) e il fuoco come sorgente nel GI del renderer. Con path tracer moderni l'alternativa esatta è campionare il volume emissivo come una luce (scelta dei voxel in proporzione alla potenza).
- **Diffusione multipla** nel fumo: costosa; spesso si approssima (Phoenix: *Approximate* vs *Ray-traced*). Nel nostro path tracer è già esatta (delta tracking).

### 2.7 Tempi e risoluzioni tipiche (ordini di grandezza)

| Effetto | Celle | Simulazione (CPU moderna) |
|---|---|---|
| Candela, fiammifero | 50–100 per lato, < 1 M celle | secondi |
| Fumo di sigaretta/incenso | 100×300×100, ~3 M | da secondi a un minuto per secondo simulato |
| Falò, camino | 150–300 per lato, 5–30 M | minuti per secondo simulato |
| Esplosioni, grandi incendi | 100 M e oltre, con wavelet | ore |

Il dettaglio wavelet ×2 moltiplica le celle per 8 ma costa molto meno di una simulazione fine, perché non risolve la pressione.

## 3. Che cosa c'è già in ForgeRender

### 3.1 Solver (`crates/forge_volume/src/sim.rs`)

- Fumo euleriano alla Fedkiw 2001 su **griglia sfalsata (MAC)**: velocità sulle facce, grandezze al centro.
- Campi: vapore visibile `ρ`, gas pesante `c` (CO₂ freddo), velocità, pressione. **Manca la temperatura** come campo: il "caldo che sale" è un parametro di risalita legato alla densità.
- Forze: peso del gas pesante, risalita del vapore, curl noise (due campi alternati), **confinamento della vorticità**, attrito con l'aria e con le pareti.
- Trasporto: semi-lagrangiano per la velocità, **MacCormack limitato** per gli scalari.
- Pressione: Gauss–Seidel rosso/nero con SOR, **16 iterazioni**, in place. La sorgente ha già una **divergenza imposta** (`src_div`): è esattamente il meccanismo che serve per l'espansione della combustione.
- **Lavoro solo vicino alla nebbia**: riquadro di lavoro con margine attorno alla nebbia, maschere dilatate, righe/span. È un contenitore adattivo, non ancora sparso a blocchi.
- Una **sorgente a disco** (la bocca della ciotola).
- Collisioni da `voxelize` (BVH → guscio) e `interior_cells` (riempimento dell'interno) in `forge_render_core/src/fog.rs`.
- Deterministico (doppio buffer, nessuna dipendenza dai thread), con barra di avanzamento (`progress.rs`: Ostacoli, Simulazione, Dettaglio, Foschia) e cache dei risultati.
- Lato massimo della simulazione 256 celle (`SIM_MAX`); misure: 10–40 s per 6 s simulati a ~112 celle su 4 vCPU.

### 3.2 Dettaglio e dati per il renderer (`lib.rs`)

- `bake` porta la simulazione su una griglia fine con deformazione e tessitura procedurale ("cavolfiore", bordi a filamenti): è già una forma di upresing, ma procedurale, non guidata dall'energia del fluido.
- `BakedFog`: una sola grandezza, σt quantizzata a 16 bit, **albedo e anisotropia uniche**, majorant per blocchi 8³ (DDA), flag `camera_only` e `additive`.

### 3.3 Rendering

- CPU (`forge_render_cpu/src/medium.rs`): delta tracking con majorant a tratti, ratio tracking per le ombre, Henyey–Greenstein, raggi solo-luci additivi.
- GPU (`path_tracer.wgsl`, funzioni `fog_*`): stesse tecniche, intestazione di 16 parole per nebbia (`FOG_WORDS`), densità trilineare.
- **Non c'è emissione nei volumi**: un punto di collisione può solo diffondere o assorbire. Le superfici emissive esistono, ma il campionamento delle luci non conosce volumi luminosi.
- Esiste `kelvin_to_rgb` (`forge_scene/src/light.rs`) per il colore delle luci: approssimazione normalizzata, utile come riferimento ma per il fuoco serve la radianza assoluta (Planck ∝ T⁴).
- Modalità spettrale (4 lunghezze d'onda): qui Planck si valuta direttamente per lunghezza d'onda, senza tabelle RGB.

### 3.4 Confronto

| Aspetto | Phoenix / FumeFX | ForgeRender oggi | Distanza |
|---|---|---|---|
| Griglia | adattiva / sparsa | MAC con riquadro di lavoro attorno alla nebbia | piccola: aggiungere blocchi attivi |
| Pressione | solver accurati (CG precondizionato, multigrid) | 16 iterazioni SOR | **media**: per fiamme veloci serve un solver migliore |
| Canali | velocità, fumo, T, fuel, colore, UVW | velocità, vapore, gas pesante | aggiungere T e fuel |
| Spinta | Boussinesq con T | peso + risalita parametrica | piccola |
| Combustione | ignizione, calore, fumo, espansione | — (ma la divergenza imposta c'è) | media |
| Vorticità | sì | sì | — |
| Dettaglio | wavelet guidato dall'energia | procedurale | media |
| Sorgenti | oggetti, superficie/volume, mappe | un disco | media |
| Collisioni | voxelizzazione | voxelizzazione + interno | — |
| Fuoco nel render | corpo nero, fuoco come luce | — | **grande**: emissione + campionamento |
| Diffusione multipla | spesso approssimata | esatta (delta tracking) | vantaggio nostro |
| GPU | anteprima / render GPU | render GPU, solver solo CPU | possibile fase futura |

## 4. Scelte di fondo

1. **Un solo solver di gas** con canali: la nebbia da ghiaccio secco diventa un caso particolare (gas pesante + vapore), fumo e fuoco aggiungono temperatura e combustibile. Niente secondo motore da mantenere.
2. **Foto ferme**: si simula fino al tempo scelto (`sim_time`) e si rende quel fotogramma. Animazione possibile più avanti (il solver riceve già il tempo).
3. **Determinismo e cache** restano: stesso seme, stesso fumo; si ricalcola solo se cambiano parametri o oggetti nella zona.
4. **CPU prima**, poi eventualmente un solver su GPU (wgpu compute), quando i passi sono stabili e verificati.
5. **Dati per il renderer**: `BakedFog` riceve un secondo canale opzionale (temperatura o emissione già convertita), sempre a 16 bit, con il suo majorant di potenza per il campionamento delle luci.

## 5. Piano a fasi

Costi: **lavoro** in sessioni di sviluppo (una sessione ≈ una giornata di lavoro con un agente e la revisione di Valerio); **calcolo** stimato sulle misure del § 3.1, da verificare.

### Fase 0 — Basi del solver (2–3 sessioni)

- Separare in `sim.rs` il nucleo del fluido (MAC, trasporto, pressione, vorticità, riquadro di lavoro) dalla fisica del ghiaccio secco: un elenco di canali scalari trasportati e una funzione "forze e reazioni" per tipo di gas.
- **Pressione migliore**: gradiente coniugato precondizionato (MIC(0)) o un V-ciclo multigrid come precondizionatore, con tolleranza invece di 16 iterazioni fisse. Le colonne di fumo veloci perdono volume e "derivano" con una pressione approssimata.
- **Blocchi attivi** 8³ (gli stessi dei majorant) al posto del solo riquadro: una colonna di fumo alta e sottile e un fumo che si allarga costano solo dove c'è qualcosa.
- Riuso: MAC, MacCormack, vorticità, curl noise, `voxelize`/`interior_cells`, barra, cache. Test: i risultati della nebbia attuale restano visivamente uguali (confronto immagini) e i test di `tests.rs` passano.

### Fase 1 — Fumo con temperatura e spinta (2 sessioni)

- Nuovo campo **temperatura** (scalare al centro, trasportato con MacCormack), raffreddamento verso l'ambiente (esponenziale, più un termine ∝ T⁴ opzionale per le fiamme).
- Spinta di Boussinesq: (−α·fumo + β·(T − T_amb))·ŷ.
- Nuovo tipo nel `FogVolume`: `kind = GhiaccioSecco | Fumo | Fuoco` (i progetti vecchi si aprono come GhiaccioSecco).
- Parametri: temperatura della sorgente, spinta, raffreddamento, densità del fumo, dissipazione, vorticità, turbolenza, scala dei vortici.
- Primo caso d'uso: **vapore del caffè** e **fumo d'incenso** (pennacchio laminare che si rompe in riccioli). Sono anche i più utili per le foto di prodotto.
- Calcolo atteso: 96×192×96 (~1,8 M celle), 3 s simulati, qualche centinaio di passi → 30–90 s su CPU a 16 thread.

### Fase 2 — Sorgenti sugli oggetti (2 sessioni)

- Sorgente = **oggetto della scena** (uno o più), in due modalità come in Phoenix: **superficie** (guscio da `voxelize`, emissione con velocità di uscita lungo la normale) e **volume** (celle da `interior_cells`).
- Restano le sorgenti semplici: disco (quella attuale), sfera, punto (stoppino).
- Per ogni sorgente: temperatura, fumo, combustibile, velocità di uscita, percentuale di superficie, **rumore che modula l'emissione** (sfarfallio, bruciatore a fori), seme.
- Più sorgenti nello stesso volume; un oggetto può essere solo ostacolo.
- Interfaccia: scelta dell'oggetto dalla scena (come per gli adesivi), anteprima delle celle sorgente in viewport.

### Fase 3 — Fuoco: combustione ed emissione nel path tracer (4–5 sessioni)

**3a. Combustione nel solver**

- Campo **combustibile**; dove T > T_ignizione: brucia `min(fuel, velocità·dt)`; il bruciato aggiunge calore, fumo ed **espansione** (divergenza nella pressione, come `src_div`).
- Opzione "fiamma semplice" senza combustibile per candele e fiammiferi: la temperatura stessa fa da fiamma.

**3b. Dati per il renderer**

- `BakedFog` riceve il canale **temperatura** (u16) accanto a σt; il fumo dà σt e albedo (scuro = fuliggine assorbente), il fuoco dà l'emissione.
- **Tabella corpo nero**: Planck integrato con le curve CIE → RGB lineare, 256 valori tra ~500 e ~4000 K, calcolata una volta; in modalità spettrale Planck si valuta per lunghezza d'onda. Parametri artistici: intensità (esposizione del fuoco), spostamento di temperatura, saturazione, opacità del fuoco separata.
- Per le fiamme blu dei bruciatori a gas (non sono corpo nero: emissione di CH e C₂) un'opzione "nucleo blu" con colore scelto.

**3c. Emissione nel path tracer (CPU e GPU)**

- Nel delta tracking, a ogni collisione (anche fittizia) si aggiunge σe·Le/σ̄ (stimatore di collisione): non distorto e già compatibile con i majorant a blocchi. Il majorant deve coprire anche la parte emissiva dove il fumo è sottile ma la fiamma è forte.
- GPU: nuove parole nell'intestazione `FOG_WORDS` (offset del canale temperatura, tabella, intensità), stesse funzioni `fog_*` con la lettura in più.

**3d. Il fuoco illumina la scena**

- Primo passo, economico e poco rumoroso: **luci automatiche** (come le *Fire Lights* di Phoenix): k luci puntiformi (8–64) scelte per raggruppamento dei voxel più luminosi, colore e potenza sommati. Il resto del motore le gestisce già (ombre, riflessi, denoiser).
- Passo esatto: **volume come luce** nel campionamento diretto: tabella alias dei blocchi 8³ pesati dalla potenza emessa, punto nel blocco, raggio d'ombra con la trasmittanza esistente, MIS con la diffusione. Le luci automatiche restano come opzione veloce.
- Attenzione ai *fireflies*: i valori del fuoco sono molto alti; clamp opzionale solo sui rimbalzi indiretti, verifica con il denoiser.

**3e. Dettaglio più fine**

- Estensione di `bake`: al posto della sola tessitura procedurale, **rumore guidato dall'energia** della simulazione (wavelet turbulence semplificata: banda di rumore curl con ampiezza ∝ energia turbolenta locale, coordinate di tessitura trasportate). Upres ×2–×4 della temperatura e del fumo.

Calcolo atteso: candela 64³ in pochi secondi; camino 160×240×160 (~6 M celle) in qualche minuto su CPU. Rendering: il fuoco è "luce nel volume", costo simile alla nebbia attuale; con le luci automatiche quasi gratis.

### Fase 4 — Qualità e barra di avanzamento (1–2 sessioni)

- **Qualità** con nomi semplici (come la nebbia): Bozza / Media / Alta / Massima → celle (64 / 128 / 192 / 256+), tolleranza della pressione, sottopassi, upres del dettaglio. Tempo stimato mostrato accanto alla scelta.
- **Barra**: nuove fasi in `progress::Phase` (Sorgenti, Simulazione fumo, Combustione, Dettaglio, Luci del fuoco), frazione per passo simulato, pulsante Annulla.
- **Anteprima veloce**: mentre si regolano i parametri parte da sola una simulazione a Bozza; la qualità scelta si calcola al render finale (con cache).
- Benchmark in `BENCHMARK.md` (CPU e GPU di riferimento) e voce nel changelog.

### Fase 5 — Preset (1–2 sessioni)

I preset **aggiungono** un volume, non cancellano né sostituiscono quello che l'utente ha impostato.

| Preset | Sorgente | Note |
|---|---|---|
| Candela | punto sullo stoppino | fiamma semplice, 64³, fumo quasi nullo, luce automatica calda |
| Fiammifero | punto | più fumo, fiamma più bassa |
| Vapore del caffè | disco sulla tazza | senza fuoco, fumo bianco, dissipazione alta |
| Incenso | punto | pennacchio sottile laminare che si rompe in riccioli |
| Sigaro / sigaretta | punto | fumo bluastro, riccioli lenti |
| Camino / falò | volume dei ceppi | combustibile, fiamme alte, fumo scuro, luci del fuoco |
| Bruciatore a gas | superficie ad anello con rumore | nucleo blu |
| Fumo denso nero | volume | fumo assorbente, spinta forte |

Ogni preset: voce nel pannello, strumento MCP (`add_object`/nuovo strumento "add_smoke"), capitolo nel README (inglese e italiano, con immagini di Valerio), manuale.

### Fase 6 (opzionale) — Solver su GPU

- I passi del solver (trasporto, forze, pressione multigrid, combustione) sono tutti per cella: si portano su wgpu compute. Guadagno atteso 10–30× sul solver, che rende pratiche griglie da 20–50 M celle.
- Da fare solo dopo che la versione CPU è il riferimento (stessi risultati entro una tolleranza).

## 6. Riepilogo dei costi

| Fase | Lavoro | Calcolo tipico |
|---|---|---|
| 0 Basi del solver | 2–3 sessioni | nebbia attuale invariata o più veloce |
| 1 Fumo + temperatura | 2 | 30–90 s per un pennacchio |
| 2 Sorgenti sugli oggetti | 2 | trascurabile |
| 3 Fuoco + emissione + luce | 4–5 | secondi (candela) – minuti (camino) |
| 4 Qualità e barra | 1–2 | — |
| 5 Preset | 1–2 | — |
| 6 Solver GPU (opz.) | 4–6 | 10–30× più veloce |

Ordine consigliato: 0 → 1 → 2 → 3a/3b/3c → 3d (luci automatiche) → 4 → 5 → 3d (volume come luce) → 3e → 6. Dopo la fase 1 c'è già qualcosa di utile (vapore e incenso); dopo 3d il fuoco che illumina il prodotto.

## 7. Rischi

- **Rumore**: un volume emissivo piccolo e luminoso è difficile per un path tracer; le luci automatiche sono la rete di sicurezza.
- **Aspetto "da computer"**: senza dettaglio fine il fuoco sembra gelatina; la fase 3e è necessaria per la qualità Alta/Massima.
- **Tempi**: oltre ~10 M celle la CPU diventa lenta; serve la fase 6 per i camini grandi.
- **Memoria GPU**: due canali a 16 bit per 256³ ≈ 64 MB, accettabile; oltre serve la griglia sparsa anche nei dati del renderer.
- **Compatibilità**: la nebbia da ghiaccio secco non deve cambiare aspetto con il refactoring della fase 0 (test di confronto).

## 8. Fonti

- Chaos, documentazione di Phoenix: Grid (Cell Size, Adaptive Grid), Simulation (Conservation, Steps per Frame, Vorticity), Fuel (Ignition, Burn Rate, Energy, Smoke Amount, Propagation), Fire Source, Resimulation (Amplify Resolution), Volumetric Shader (Fire Based On Temperature, Physically Based, Fire Lights).
- Sitni Sati, documentazione di FumeFX: General (Spacing, Adaptive), Simulation (Quality, Maximum Iterations, CFL, Vorticity, Turbulence), Fuel (Ignition Temperature, Burn Rate, Heat Production, Expansion), Wavelet Turbulence, Sources, Rendering (Fire/Smoke color, Illumination Map).
- R. Fedkiw, J. Stam, H. W. Jensen, *Visual Simulation of Smoke*, SIGGRAPH 2001.
- D. Nguyen, R. Fedkiw, H. W. Jensen, *Physically Based Modeling and Animation of Fire*, SIGGRAPH 2002.
- T. Kim, N. Thürey, D. James, M. Gross, *Wavelet Turbulence for Fluid Simulation*, SIGGRAPH 2008.
- A. Selle et al., *An Unconditionally Stable MacCormack Method*, 2008.
- R. Bridson, *Fluid Simulation for Computer Graphics*, 2ª ed., 2015 (pressione con MICCG, griglie MAC).
- K. Museth, *VDB: High-Resolution Sparse Volumes with Dynamic Topology*, ACM TOG 2013.
- J. Novák et al., *Monte Carlo Methods for Volumetric Light Transport Simulation*, Eurographics STAR 2018 (stimatori di collisione ed emissione).
- V. Pegoraro, S. Parker, *Physically-Based Realistic Fire Rendering*, 2006 (corpo nero spettrale).

---

# Smoke and fire in ForgeRender — study and plan

🇮🇹 [Versione italiana sopra](#fumo-e-fuoco-in-forgerender--studio-e-piano)

Status: **study**, no changes to the engine. Implementation starts once Valerio has read and approved the plan.

## 1. Goal

Bring to ForgeRender a smoke-and-fire simulator in the style of **Chaos Phoenix** (formerly Phoenix FD) and **FumeFX** (Sitni Sati), aimed at product photography: a candle beside a perfume bottle, steam from a coffee cup, incense smoke, a fireplace flame reflected in glass, a barbecue. It builds on what already exists: the simulated dry-ice fog (`forge_volume`) and delta tracking in the CPU and GPU path tracers.

## 2. How Phoenix and FumeFX work

Sources are the public documentation of both products and the papers they build on (list in § 8). Parameter names are taken from the manuals; time and resolution figures are orders of magnitude from manuals, tutorials and examples, not our own measurements.

### 2.1 Grid

- **Eulerian, cubic cells.** Both simulate on a regular grid: velocity and scalar quantities per cell. The main setting is cell size (*Cell Size* in Phoenix, *Spacing* in FumeFX); resolution follows from container size.
- **Adaptive container.** Phoenix has the *Adaptive Grid*: the box grows and shrinks to follow channels above a threshold (smoke, temperature), up to a limit. FumeFX's container has *Adaptive* with the same idea. You only pay for the volume where smoke is.
- **Sparsity.** Recent FumeFX versions advertise a sparse-grid solver (memory and work only in active blocks). Phoenix stays mostly with the adaptive box; it reads and writes OpenVDB, which is sparse by design. The common idea: work only where something is.
- **Steps.** The time step follows the Courant (CFL) condition: FumeFX exposes *CFL* and *Maximum Iterations*, Phoenix *Steps per Frame* and *Time Scale*. Fast flames need more substeps per frame.

### 2.2 Fields (channels)

| Channel | Phoenix | FumeFX | Purpose |
|---|---|---|---|
| Velocity | Velocity | Velocity | carries everything else |
| Smoke / density | Smoke | Smoke | opacity and light scattering |
| Temperature | Temperature | Temperature | upward push, fire colour and light |
| Fuel | Fuel | Fuel | burns: produces heat, smoke, expansion |
| Colour | RGB | Color | coloured smoke (stylised) |
| Texture coordinates | UVW / Wavelet | Texture coords | detail that follows the fluid |

Each channel is advected by the velocity (semi-Lagrangian, with higher-order variants such as MacCormack/BFECC) and decays over time (*Dissipation*, *Cooling*).

### 2.3 Buoyancy and combustion

- **Buoyancy** (Fedkiw, Stam, Jensen 2001): f = (−α·smoke + β·(T − T_amb))·ŷ. Smoke is heavy, heat rises. Phoenix has *Buoyancy* and *Smoke Buoyancy*; FumeFX *Buoyancy* and *Gravity*.
- **Cooling**: temperature relaxes towards ambient (*Cooling*), so the flame stops at some height and smoke remains above.
- **Combustion** (fuel model, a simplification of Nguyen, Fedkiw, Jensen 2002): where T exceeds the *Ignition Temperature* and fuel is present, part of it burns each step (*Burn Rate*). Burnt fuel:
  - releases heat (*Heat Production*): T rises;
  - produces smoke (*Smoke Density* / *Smoke Amount*);
  - expands (*Expansion*): a divergence source in the pressure solve, the flame "swells" and pushes the surrounding air.
- This is the cycle that makes a flame alive: fuel leaves cold, ignites, heat accelerates it upwards, it stretches into tongues, cools and turns into smoke.

### 2.4 Vortices and detail

- **Vorticity confinement** (*Vorticity*): puts back the rotation lost to the grid's numerical diffusion. It gives curls and tongues.
- **Procedural turbulence** (*Turbulence*, noise fields): extra agitation, stronger where velocity is high.
- **Wavelet turbulence** (Kim, Thürey, James, Gross 2008): simulate coarse, then recompute at 2–4× finer resolution, adding wavelet noise scaled by the energy of the coarse simulation and carried by texture coordinates that follow the fluid. FumeFX has *Wavelet Turbulence* as a second pass; Phoenix has *Resimulation* with *Amplify Resolution* ("wavelet" or "smoothing" detail). Benefit: the approved overall shape stays, and the detail costs a fraction of a fine simulation.

### 2.5 Sources and collisions

- **Sources on objects.** Phoenix: *Fire Source* (PHXSource) with *Emit* mode (outflow from the surface, with exit speed *Discharge*) and *Inject* mode (fills the object's volume); each channel gets its value (temperature, smoke, fuel, colour). FumeFX: *Object Source*, *Simple Source* (sphere, box), *Particle Source*. Both let you modulate emission with a map or noise (flickering flame, perforated burner) and emit from part of the surface only.
- **Collisions.** Scene objects are voxelised; velocity does not enter solids (or matches their velocity if they move). Optional: an object that blocks but does not emit.
- **Inherited velocity** from moving objects (not needed for still photos).

### 2.6 Fire shading

- **Blackbody emission.** Fire colour comes from temperature: Planck's law → colour. Phoenix offers a *Physically Based* fire mode (temperature in kelvin, adjustable intensity) next to a free colour curve; FumeFX uses a gradient over temperature/fuel. In practice both give the artist intensity, a curve, and fire opacity separate from smoke opacity.
- **Smoke.** Opacity from the smoke channel, its own colour and albedo; dark smoke = more absorption (soot), light smoke = more scattering.
- **Fire as a light.** This is what makes the difference in a photo: the flame lights objects, shows in reflections, casts shadows. Phoenix creates *Fire Lights*: automatic point lights spread through the grid, weighted by emission (little noise, approximate). FumeFX has the *Illumination Map* (light cached in a grid) and fire as a source in the renderer's GI. With modern path tracers the exact alternative is to sample the emissive volume as a light (voxels chosen in proportion to power).
- **Multiple scattering** in smoke: expensive; often approximated (Phoenix: *Approximate* vs *Ray-traced*). Our path tracer already does it exactly (delta tracking).

### 2.7 Typical times and resolutions (orders of magnitude)

| Effect | Cells | Simulation (modern CPU) |
|---|---|---|
| Candle, match | 50–100 per side, < 1 M cells | seconds |
| Cigarette/incense smoke | 100×300×100, ~3 M | seconds to a minute per simulated second |
| Campfire, fireplace | 150–300 per side, 5–30 M | minutes per simulated second |
| Explosions, large fires | 100 M and more, with wavelets | hours |

×2 wavelet detail multiplies cells by 8 but costs far less than a fine simulation, because it does not solve pressure.

## 3. What ForgeRender already has

### 3.1 Solver (`crates/forge_volume/src/sim.rs`)

- Eulerian smoke after Fedkiw 2001 on a **staggered (MAC) grid**: velocity on faces, quantities at centres.
- Fields: visible vapour `ρ`, heavy gas `c` (cold CO₂), velocity, pressure. **No temperature field**: "warm air rises" is a rise parameter tied to density.
- Forces: heavy-gas weight, vapour rise, curl noise (two alternating fields), **vorticity confinement**, air and wall drag.
- Advection: semi-Lagrangian for velocity, **limited MacCormack** for scalars.
- Pressure: in-place red-black Gauss–Seidel with SOR, **16 iterations**. The source already imposes a **divergence** (`src_div`): exactly the mechanism combustion expansion needs.
- **Work only near the fog**: a work box with a margin around the fog, dilated masks, rows/spans. An adaptive container, not yet block-sparse.
- A single **disc source** (the bowl's mouth).
- Collisions from `voxelize` (BVH → shell) and `interior_cells` (interior fill) in `forge_render_core/src/fog.rs`.
- Deterministic (double buffering, no thread dependence), with a progress bar (`progress.rs`: Obstacles, Simulation, Detail, Haze) and a result cache.
- Max simulation side 256 cells (`SIM_MAX`); measured: 10–40 s for 6 simulated seconds at ~112 cells on 4 vCPUs.

### 3.2 Detail and renderer data (`lib.rs`)

- `bake` moves the simulation onto a fine grid with warping and procedural texture ("cauliflower" clumps, filament edges): already a form of up-resing, but procedural, not driven by the fluid's energy.
- `BakedFog`: one quantity, σt quantised to 16 bits, **single albedo and anisotropy**, majorants per 8³ block (DDA), `camera_only` and `additive` flags.

### 3.3 Rendering

- CPU (`forge_render_cpu/src/medium.rs`): delta tracking with piecewise majorants, ratio tracking for shadows, Henyey–Greenstein, additive light-only rays.
- GPU (`path_tracer.wgsl`, `fog_*` functions): same techniques, 16-word header per fog (`FOG_WORDS`), trilinear density.
- **No volume emission**: a collision can only scatter or absorb. Emissive surfaces exist, but light sampling knows nothing about glowing volumes.
- `kelvin_to_rgb` exists (`forge_scene/src/light.rs`) for light colour: a normalised approximation, useful as reference, but fire needs absolute radiance (Planck ∝ T⁴).
- Spectral mode (4 wavelengths): here Planck is evaluated per wavelength, with no RGB table.

### 3.4 Comparison

| Aspect | Phoenix / FumeFX | ForgeRender today | Gap |
|---|---|---|---|
| Grid | adaptive / sparse | MAC with work box around the fog | small: add active blocks |
| Pressure | accurate solvers (preconditioned CG, multigrid) | 16 SOR iterations | **medium**: fast flames need a better solver |
| Channels | velocity, smoke, T, fuel, colour, UVW | velocity, vapour, heavy gas | add T and fuel |
| Buoyancy | Boussinesq with T | weight + parametric rise | small |
| Combustion | ignition, heat, smoke, expansion | — (imposed divergence exists) | medium |
| Vorticity | yes | yes | — |
| Detail | energy-driven wavelets | procedural | medium |
| Sources | objects, surface/volume, maps | one disc | medium |
| Collisions | voxelisation | voxelisation + interior | — |
| Fire in render | blackbody, fire as light | — | **large**: emission + sampling |
| Multiple scattering | often approximated | exact (delta tracking) | our advantage |
| GPU | GPU preview / render | GPU render, CPU-only solver | possible future phase |

## 4. Core decisions

1. **One gas solver** with channels: dry-ice fog becomes a special case (heavy gas + vapour); smoke and fire add temperature and fuel. No second engine to maintain.
2. **Still photos**: simulate up to the chosen time (`sim_time`) and render that frame. Animation is possible later (the solver already takes time as input).
3. **Determinism and cache** stay: same seed, same smoke; recompute only when parameters or objects in the region change.
4. **CPU first**, then possibly a GPU solver (wgpu compute), once the steps are stable and verified.
5. **Renderer data**: `BakedFog` gets an optional second channel (temperature, or emission already converted), also 16-bit, with its own power majorant for light sampling.

## 5. Phased plan

Costs: **work** in development sessions (one session ≈ one working day with an agent plus Valerio's review); **compute** estimated from the measurements in § 3.1, to be verified.

### Phase 0 — Solver foundations (2–3 sessions)

- In `sim.rs`, separate the fluid core (MAC, advection, pressure, vorticity, work box) from dry-ice physics: a list of advected scalar channels and a "forces and reactions" function per gas type.
- **Better pressure**: preconditioned conjugate gradient (MIC(0)) or a multigrid V-cycle as preconditioner, with a tolerance instead of a fixed 16 iterations. Fast smoke columns lose volume and "drift" with approximate pressure.
- **Active 8³ blocks** (the same as the majorant blocks) instead of only a box: a tall thin smoke column or spreading smoke costs only where something is.
- Reuse: MAC, MacCormack, vorticity, curl noise, `voxelize`/`interior_cells`, progress bar, cache. Tests: the current fog looks the same (image comparison) and `tests.rs` passes.

### Phase 1 — Smoke with temperature and buoyancy (2 sessions)

- New **temperature** field (cell-centred scalar, MacCormack-advected), cooling towards ambient (exponential, plus an optional ∝ T⁴ term for flames).
- Boussinesq buoyancy: (−α·smoke + β·(T − T_amb))·ŷ.
- New `FogVolume` type: `kind = DryIce | Smoke | Fire` (old projects open as DryIce).
- Parameters: source temperature, buoyancy, cooling, smoke density, dissipation, vorticity, turbulence, vortex scale.
- First use cases: **coffee steam** and **incense smoke** (a laminar plume that breaks into curls). They are also the most useful for product photos.
- Expected compute: 96×192×96 (~1.8 M cells), 3 simulated seconds, a few hundred steps → 30–90 s on a 16-thread CPU.

### Phase 2 — Sources on objects (2 sessions)

- Source = **scene object** (one or more), in two modes as in Phoenix: **surface** (shell from `voxelize`, emission with exit speed along the normal) and **volume** (cells from `interior_cells`).
- Simple sources remain: disc (the current one), sphere, point (wick).
- Per source: temperature, smoke, fuel, exit speed, surface percentage, **noise modulating emission** (flicker, perforated burner), seed.
- Several sources in one volume; an object can be an obstacle only.
- UI: pick the object from the scene (as for stickers), preview source cells in the viewport.

### Phase 3 — Fire: combustion and emission in the path tracer (4–5 sessions)

**3a. Combustion in the solver**

- **Fuel** field; where T > T_ignition: burn `min(fuel, rate·dt)`; burnt fuel adds heat, smoke and **expansion** (divergence in the pressure solve, like `src_div`).
- A "simple flame" option without fuel for candles and matches: temperature itself acts as the flame.

**3b. Renderer data**

- `BakedFog` gets a **temperature** channel (u16) next to σt; smoke gives σt and albedo (dark = absorbing soot), fire gives emission.
- **Blackbody table**: Planck integrated against CIE curves → linear RGB, 256 entries from ~500 to ~4000 K, computed once; in spectral mode Planck is evaluated per wavelength. Artistic controls: intensity (fire exposure), temperature shift, saturation, separate fire opacity.
- For the blue flames of gas burners (not blackbody: CH and C₂ emission) a "blue core" option with a chosen colour.

**3c. Emission in the path tracer (CPU and GPU)**

- In delta tracking, at each collision (including null ones) add σe·Le/σ̄ (collision estimator): unbiased and already compatible with block majorants. The majorant must also cover the emissive part where smoke is thin but the flame is strong.
- GPU: new words in the `FOG_WORDS` header (temperature channel offset, table, intensity), the same `fog_*` functions with one extra read.

**3d. Fire lights the scene**

- First, cheap and low-noise step: **automatic lights** (like Phoenix's *Fire Lights*): k point lights (8–64) chosen by clustering the brightest voxels, with summed colour and power. The rest of the engine already handles them (shadows, reflections, denoiser).
- Exact step: **volume as a light** in direct sampling: an alias table of 8³ blocks weighted by emitted power, a point in the block, a shadow ray with the existing transmittance, MIS with scattering. Automatic lights stay as a fast option.
- Watch out for fireflies: fire values are very high; optional clamping on indirect bounces only, checked with the denoiser.

**3e. Finer detail**

- Extend `bake`: instead of procedural texture alone, **energy-driven noise** from the simulation (simplified wavelet turbulence: a curl-noise band with amplitude ∝ local turbulent energy, advected texture coordinates). ×2–×4 up-res of temperature and smoke.

Expected compute: candle at 64³ in a few seconds; fireplace at 160×240×160 (~6 M cells) in a few minutes on CPU. Rendering: fire is "light in the volume", cost similar to the current fog; with automatic lights almost free.

### Phase 4 — Quality and progress bar (1–2 sessions)

- **Quality** with simple names (as for the fog): Draft / Medium / High / Maximum → cells (64 / 128 / 192 / 256+), pressure tolerance, substeps, detail up-res. Estimated time shown next to the choice.
- **Progress bar**: new phases in `progress::Phase` (Sources, Smoke simulation, Combustion, Detail, Fire lights), fraction per simulated step, Cancel button.
- **Quick preview**: while parameters are being tuned, a Draft simulation starts by itself; the chosen quality is computed for the final render (cached).
- Benchmarks in `BENCHMARK.md` (reference CPU and GPU) and a changelog entry.

### Phase 5 — Presets (1–2 sessions)

Presets **add** a volume; they never delete or replace what the user has set.

| Preset | Source | Notes |
|---|---|---|
| Candle | point on the wick | simple flame, 64³, almost no smoke, warm automatic light |
| Match | point | more smoke, lower flame |
| Coffee steam | disc on the cup | no fire, white smoke, high dissipation |
| Incense | point | thin laminar plume breaking into curls |
| Cigar / cigarette | point | bluish smoke, slow curls |
| Fireplace / campfire | volume of the logs | fuel, tall flames, dark smoke, fire lights |
| Gas burner | ring surface with noise | blue core |
| Thick black smoke | volume | absorbing smoke, strong buoyancy |

Each preset: an entry in the panel, an MCP tool (`add_object` / a new "add_smoke" tool), a README chapter (English and Italian, with Valerio's images), manual pages.

### Phase 6 (optional) — GPU solver

- The solver steps (advection, forces, multigrid pressure, combustion) are all per cell: they move to wgpu compute. Expected gain 10–30× on the solver, making 20–50 M-cell grids practical.
- Only after the CPU version is the reference (same results within a tolerance).

## 6. Cost summary

| Phase | Work | Typical compute |
|---|---|---|
| 0 Solver foundations | 2–3 sessions | current fog unchanged or faster |
| 1 Smoke + temperature | 2 | 30–90 s for a plume |
| 2 Sources on objects | 2 | negligible |
| 3 Fire + emission + light | 4–5 | seconds (candle) – minutes (fireplace) |
| 4 Quality and progress | 1–2 | — |
| 5 Presets | 1–2 | — |
| 6 GPU solver (opt.) | 4–6 | 10–30× faster |

Recommended order: 0 → 1 → 2 → 3a/3b/3c → 3d (automatic lights) → 4 → 5 → 3d (volume as light) → 3e → 6. After phase 1 there is already something useful (steam and incense); after 3d, fire that lights the product.

## 7. Risks

- **Noise**: a small bright emissive volume is hard for a path tracer; automatic lights are the safety net.
- **"CG" look**: without fine detail fire looks like jelly; phase 3e is required for High/Maximum quality.
- **Time**: beyond ~10 M cells the CPU gets slow; large fireplaces need phase 6.
- **GPU memory**: two 16-bit channels at 256³ ≈ 64 MB, acceptable; beyond that, renderer data needs a sparse grid too.
- **Compatibility**: dry-ice fog must not change appearance with the phase 0 refactor (comparison tests).

## 8. Sources

- Chaos, Phoenix documentation: Grid (Cell Size, Adaptive Grid), Simulation (Conservation, Steps per Frame, Vorticity), Fuel (Ignition, Burn Rate, Energy, Smoke Amount, Propagation), Fire Source, Resimulation (Amplify Resolution), Volumetric Shader (Fire Based On Temperature, Physically Based, Fire Lights).
- Sitni Sati, FumeFX documentation: General (Spacing, Adaptive), Simulation (Quality, Maximum Iterations, CFL, Vorticity, Turbulence), Fuel (Ignition Temperature, Burn Rate, Heat Production, Expansion), Wavelet Turbulence, Sources, Rendering (Fire/Smoke colour, Illumination Map).
- R. Fedkiw, J. Stam, H. W. Jensen, *Visual Simulation of Smoke*, SIGGRAPH 2001.
- D. Nguyen, R. Fedkiw, H. W. Jensen, *Physically Based Modeling and Animation of Fire*, SIGGRAPH 2002.
- T. Kim, N. Thürey, D. James, M. Gross, *Wavelet Turbulence for Fluid Simulation*, SIGGRAPH 2008.
- A. Selle et al., *An Unconditionally Stable MacCormack Method*, 2008.
- R. Bridson, *Fluid Simulation for Computer Graphics*, 2nd ed., 2015 (MICCG pressure, MAC grids).
- K. Museth, *VDB: High-Resolution Sparse Volumes with Dynamic Topology*, ACM TOG 2013.
- J. Novák et al., *Monte Carlo Methods for Volumetric Light Transport Simulation*, Eurographics STAR 2018 (collision and emission estimators).
- V. Pegoraro, S. Parker, *Physically-Based Realistic Fire Rendering*, 2006 (spectral blackbody).
