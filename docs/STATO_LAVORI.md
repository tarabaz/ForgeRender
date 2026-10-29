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

## Fatto nella nona parte
- Scheda Effetti, preset scenografici e cache a due livelli della nebbia; qualità della griglia
  (simulazione fino a 256 celle) e zona spostabile; nebbia ricalcolata solo se cambiano gli
  oggetti dentro la sua zona; niente gradini (zona appoggiata alla superficie sotto, campo
  ammorbidito).
- Menu Aggiungi (pavimento, fondale, parete, forme); render interattivo che non ricarica la scena
  sulla GPU quando si muove solo la camera; camera "Punta su oggetto".
- Portali di luce per gli interni (CPU e GPU); liquidi nei bicchieri: colore in volume e volumi
  annidati con priorità (CPU e GPU).
- BVH costruita su tutti i core; libreria online con catalogo completo in cache e
  `forgerender-cli libreria scarica-tutto`.
- Scena demo `samples/scene/cocktail-ghiaccio-secco`; benchmark sul PC di riferimento
  (Ryzen 9 9900X + RTX 5070) in docs/BENCHMARK.md.

## Piano di sviluppo incrementale

### Fase 1 — Versione nella barra del titolo: fatta, in attesa di conferma
- Titolo della finestra: `ForgeRender v<versione> (<commit>)`, es. `ForgeRender v0.1.0 (a6d8a99)`,
  seguito da ` — <progetto>[ *]` come prima.
- Versione da `CARGO_PKG_VERSION` (si cambia solo nel Cargo.toml del workspace a una release).
- Commit: `crates/forge_app/build.rs` lo calcola in compilazione (mai durante l'esecuzione):
  variabile `FORGE_GIT_SHA` se impostata, altrimenti `git rev-parse --short=7 HEAD`, altrimenti
  `dev`. Cargo lo ricalcola solo quando cambia il commit (HEAD, ref corrente, packed-refs).
- Stessa costante `APP_TITLE` in `ViewportBuilder::with_title`, `eframe::run_native` e nel titolo
  aggiornato a ogni frame (`app.rs`, che prima riscriveva "ForgeRender — …").
- `with_app_id("ForgeRender")`: l'archivio delle impostazioni di eframe resta lo stesso anche se
  il titolo cambia a ogni build.
- Verifiche: `cargo fmt --all --check` ok; `cargo build --release -p forge_app -p forge_cli` ok;
  titolo letto nel log di avvio e nell'eseguibile nei tre casi (commit reale, `FORGE_GIT_SHA`
  impostata, Git assente → `dev`).

### Fase 2 — Aggiornamento e compilazione locale con un comando: fatta, da collaudare sul PC
- `.\Build-ForgeRender.ps1` dalla radice del repository (oppure `.\Build-ForgeRender.cmd` se
  Windows blocca gli script; `-SkipInstaller` per il solo portable, `-NoOpen` per non aprire dist).
- Controlli: git e cargo presenti; radice Git = cartella dello script; remoto
  `tarabaz/RenderingEngine`; branch `claude/festive-heisenberg-d2ow6z` (non cambia branch da
  solo); nessuna modifica ai file tracciati (se ce ne sono si ferma senza toccarle).
- `git pull --ff-only origin claude/festive-heisenberg-d2ow6z`: se fallisce si ferma.
- `cargo build --release -p forge_app -p forge_cli` (cache riutilizzata, forge_step escluso),
  controllo degli eseguibili, poi `packaging\windows\package.ps1 -SkipBuild` in un processo
  separato; il pacchetto deve essere stato scritto da questa esecuzione.
- Niente GitHub Actions, artefatti, commit, push, reset o stash. Riepilogo finale con esito del
  pull (commit prima/dopo e nuovi commit), eseguibili e percorso del pacchetto.
- `package.ps1`: il convertitore STEP entra nel pacchetto solo con `-WithStep` (prima veniva
  copiato anche un `forgerender-step.exe` rimasto da compilazioni vecchie).
- Script salvato in UTF-8 con BOM e a capo CRLF (Windows PowerShell 5.1 legge i .ps1 senza BOM
  come ANSI). Non eseguito nel container (PowerShell non disponibile): da collaudare sul PC.
### Fase 3 — Baseline visiva e profiling: da fare dopo la fase 2

### Ottimizzazione 2 — Campionamento adattivo (prima versione): fatto, spento di default
- Accumulatore con campioni per pixel e somma "B" delle passate alterne per stimare il rumore;
  maschera a riquadri 16×16 (`forge_render_core::adaptive`): un riquadro si ferma quando l'errore
  medio è sotto la soglia e lo sono anche i vicini; la sessione finisce quando tutti sono fermi
  (o al massimo di campioni). CPU e GPU saltano i riquadri fermi (`render_pass_masked`; GPU con
  un buffer di bit). Scheda Render: "Arresto a soglia di rumore", soglia e campioni minimi;
  barra di stato con la parte dell'immagine ancora in lavorazione; CLI `--noise-threshold`,
  `--adaptive-min`, `--seed`.
- Test: media uguale al riferimento, fermata automatica, efficienza su scena con cielo (CPU);
  GPU contro CPU; senza soglia tutti i pixel ricevono tutti i campioni (nessun cambiamento).
- Misure (docs/BENCHMARK.md): scena di test con cielo ≈ 1.8× più efficiente; **scena del
  cocktail meno efficiente** (1.25–2×) perché la stima relativa dell'errore tiene attive le zone
  scure con nebbia. Per questo resta spento di default.
- Prossimo passo proposto (da confermare): errore stimato nello spazio dell'immagine mostrata.

### Ottimizzazione 1 — Majorant 3D della nebbia: fatta, da provare sul PC
- Prima: un majorant per strato orizzontale (massimo dell'intera fetta di quota): il tappeto
  denso sul bancone alzava il majorant ovunque in quella fetta, con molti passi a vuoto.
- Dopo: griglia di blocchi 8³ celle (`forge_volume::MACRO_CELLS`), majorant = massimo delle celle
  del blocco più una di bordo (supporto dell'interpolazione trilineare); il raggio percorre i
  blocchi con un DDA (`BakedFog::majorant_segment` sulla CPU, `fog_segment` in WGSL, stessa
  logica). Delta tracking e ratio tracking restano non distorti.
- Misure (container, CPU 3 thread, scena cocktail, 320×400, 32 campioni, seed fisso):
  vista intera 20.25 → 11.91 s (1.70×), vista ravvicinata 26.55 → 13.44 s (1.98×);
  immagine uguale entro il rumore. Dettagli in docs/BENCHMARK.md.
- Test nuovi: majorant ≥ densità lungo i tratti di 2000 raggi casuali, blocchi vuoti presenti.
  Tutti i test del workspace verdi; GPU verificata con i test esistenti (furnace, Beer–Lambert,
  ghiaccio secco GPU contro CPU).

### Report delle misure e comandi di benchmark
- Nuova cartella `reports/`: un report per esperimento (`AAAA-MM-GG_argomento/REPORT.md` con
  comandi, tabelle, log originali e immagini), indice in `reports/README.md`, strumenti in
  `reports/strumenti/` (`crea_progetti_bench.py`, `confronta_immagini.py`).
- Report: esperimenti precedenti (riepilogo), majorant 3D della nebbia, campionamento adattivo.
- README (italiano e inglese): sezione "Benchmark: comandi da lanciare" con tutte le misure
  possibili e a cosa servono.

### Materiali della libreria: Salva, Salva con nome, Ripristina
- Problema: cliccando di nuovo un materiale scaricato, le modifiche fatte nel progetto si
  perdevano (il materiale veniva ricopiato dalla libreria).
- Ora il clic tiene le impostazioni già presenti nel progetto. Nell'editor del materiale:
  **Salva** (scrive le modifiche nella cartella del materiale e rifà la miniatura),
  **Salva con nome…** (copia nuova nella libreria con tutte le texture; il progetto passa a
  usare la copia, l'originale resta com'era), **Ripristina** (torna alla versione salvata) e
  l'avviso "modifiche non salvate nella libreria". Per i materiali del progetto (non della
  libreria) c'è "Salva nella libreria…".
- Le texture esterne alla cartella vengono copiate dentro, così il materiale resta
  autosufficiente. Test in `forge_library` (salva, texture esterna, salva con nome, nome vuoto).

### Benchmark pronti nello zip portable
- I progetti `bench_intera`, `bench_vicina` e `bench_ad` sono nel repository
  (`samples/scene/cocktail-ghiaccio-secco/`). Lo zip portable contiene tutta la cartella
  `samples/`, senza le texture estratte dai .glb, che si ricreano all'apertura.
- I benchmark si lanciano dalla cartella estratta, senza Python né repository. L'installer non
  li include: in Program Files il programma non può scrivere le texture estratte.
- Test `forge_project/tests/samples.rs`: copia `samples/` altrove, rende inesistenti i percorsi
  assoluti e verifica che ogni progetto si apra senza modelli, componenti o texture mancanti, e
  con le texture prese dalla copia.
- CI:
  - Linux controlla che i benchmark siano allineati alla demo;
  - Windows estrae lo zip, controlla i file e lancia un render di `bench_ad` (con la mappa
    dell'adattivo) dalla cartella estratta.
- CLI: avviso "file non trovato: modello …" se manca un modello del progetto.

### Materiali scaricati: displacement spento di default
- Segnalazione: Metal 049 B (ambientCG) ha una miniatura liscia sul sito, ma applicato diventava
  una sfera piena di bozzi. Causa: all'import il displacement veniva acceso in automatico al 3 %
  del lato della texture (30 mm per 1 m). Le height map dei metalli, delle vernici e delle
  plastiche contengono solo micro-variazioni stirate su tutta la scala di grigi.
- Ora l'import lascia la height map collegata ma il displacement a 0. I materiali già nella
  libreria (`materiale.json` versione 1) tornano a 0 al caricamento; da versione 2 il valore
  salvato con "Salva" resta. Le miniature si rifanno (`anteprima-v3.png`).
- Pannello Rilievo: pulsante "Valore proposto" (3 % del lato) per i materiali con rilievo vero.

### Libreria: riconoscimento delle mappe dal suffisso; eliminazione dei componenti importati
- **Bug**: con nomi come `metal_plate_02` (Poly Haven) le mappe `diff`, `disp` e `ao` venivano
  prese per metallicità ("metal" nel nome del materiale). Ne restava una sola (l'AO, usata come
  metallic) e il materiale perdeva colore e displacement.
- **Correzione**: ora decide l'ultima parola chiave del nome. Test con i nomi reali.
- **Materiali già importati male**: segnalati ("⚠ da riscaricare" nella Libreria, `libreria
  elenco`). Non contano come già scaricati: `scarica-tutto` e la scheda Online li riscaricano e
  la copia difettosa viene eliminata.
- **Canc** elimina dal progetto anche i componenti dei file importati, senza toccare il file. Il
  componente è segnato `removed` (salvato nel progetto): sparisce da viewport, render e albero.
  Si ripristina con Ctrl+Z o da "Eliminati dal progetto" in fondo all'albero.

### Materiali: EXR, Poly Haven completo, file MaterialX, origine dei valori
- **Poly Haven**: le mappe disponibili solo in EXR ora si scaricano (prima venivano saltate).
  Le mappe che il sito non offre alla risoluzione scelta finiscono come nota nel materiale e
  sono mostrate nella Libreria e nella CLI.
- **Texture**:
  - il colore base in EXR (lineare) viene codificato correttamente in sRGB (prima risultava
    decodificato due volte, molto più scuro);
  - la height del displacement da PNG a 16 bit o EXR viene letta a 16 bit.
- **MaterialX**: i file `.mtlx` di ambientCG (dentro lo zip) e di Poly Haven si conservano nella
  cartella del materiale. Il pannello mostra i valori scritti dalla fonte e permette di usare la
  scala del displacement del file.
  - Lettore minimo senza dipendenze, testato su esempi nello stile della specifica.
  - **Non ancora verificato su file reali**: servono gli esempi in `reports/esempi_materiali/`.
- **Pannello**: "× mappa" accanto a colore, metallic e roughness quando il valore viene da una
  mappa (il cursore la moltiplica).

### Menu Impostazioni
- Finestra **Impostazioni**, da File → Impostazioni… o dal pulsante nella barra. Le sezioni:
  - **Libreria e import**: risoluzione e sito proposti per i download; displacement all'import
    (spento, valore proposto o dal file MaterialX);
  - **Percorsi**: cartella della libreria modificabile;
  - **Rendering**: dispositivo, campioni e thread dei nuovi progetti, riavvio automatico;
  - **Informazioni**: versione, cartelle di libreria, log e impostazioni.
- Salvate subito in `impostazioni.json`: accanto al programma nella portable, in
  `%LOCALAPPDATA%\ForgeRender` nell'installata.
  - File mancante: valori predefiniti.
  - File non valido: valori predefiniti e copia `.non-valido` conservata.
  - "Ripristina predefiniti" riporta tutto ai valori iniziali.
- Solo controlli collegati a funzioni reali. La CLI usa i valori predefiniti.
- Test: salvataggio e rilettura, file non valido o parziale, politiche di displacement
  all'import.

### Import della libreria Poly Haven scaricata in locale (29/10/2026, fase 2)
- Comando `forgerender-cli libreria importa-polyhaven <cartella> [--max N] [--sposta]
  [--solo-elenco] [--libreria DIR]` per le cartelle dello script
  `tools/material-library-polyhaven` (modulo `forge_library::polyhaven_local`).
  - Solo le cartelle con `material.json` (complete); quelle in download non si toccano.
  - Nome "<nome> 2K", provenienza "Poly Haven <id> (2K)", scala reale dal catalogo Poly Haven.
  - Nella libreria vanno solo le mappe collegate agli slot, il .mtlx e `polyhaven_manifest.json`;
    nor_dx, arm, rough_ao, bump, spec, .blend/.gltf restano nei download. Non si cancella niente.
  - `--sposta`: spostamento con `rename`; se l'import fallisce i file tornano al loro posto.
  - Note nel materiale: mappe mancanti, stati di download anomali, risoluzioni diverse, scala
    non disponibile, slot base mancanti.
- Riconoscimento: la `disp` vince sulla `bump` (bark_brown_01/02), `rough_ao` ignorata; dalla
  ARM si estraggono solo i canali che mancano (niente PNG inutili, niente metallicità nera).
- `store::import_folder_with(…, Transfer::Copy | Transfer::Move)`.
- Prova reale: `--solo-elenco` (93 completi, 1 in download) e copia `--max 3` in una libreria
  temporanea. **Da fare**: prova `--max 3` in `Y:\ForgeRender\Libreria` controllata da Valerio
  nell'app, poi lo spostamento di tutto a download finito, con il suo ok.
- Prova `--max 3` in copia su `Y:\ForgeRender\Libreria` fatta (aerial_asphalt_01, aerial_beach_01/02).

### Categorie, anteprime grandi, EXR a un canale (29/10/2026)
- **Categorie e tag** della fonte in `materiale.json` (`categories`, `tags`): Poly Haven ne dà più
  d'una per materiale (media 3,6), ambientCG una. Salvate all'import locale e nei download
  dall'app; `libreria aggiorna-categorie` (e il pulsante "Aggiorna categorie") le aggiunge ai
  materiali già in libreria. Catalogo in cache con tutte le categorie (`*_v2.json`).
- **Finestra Libreria**: elenco delle categorie con i conteggi (nomi italiani solo
  nell'interfaccia, `forge_library::categories`), generiche in "Altre etichette", "Senza
  categoria"; la ricerca guarda nome, provenienza, categorie e tag. Si disegnano solo le righe
  visibili (prima tutte: con 869 materiali partivano centinaia di render).
- **Dimensione delle anteprime** 64–512 px (cursore, salvata in `impostazioni.json`). Miniature
  a 128/256/512 px secondo i pixel reali (`anteprima-v4[-256|-512].png`), con più campioni per le
  grandi; al massimo 2 render di miniature insieme.
- **EXR a un canale**: Poly Haven salva AO e rugosità con la sola luminanza "Y" e la height con
  "Y"+"A"; `image` non li legge ("non-deep rgb channels") e le mappe restavano scollegate. Ora si
  leggono con il crate `exr` (stessa versione già in uso). Miniature portate a v4 perché le v3 di
  quei materiali erano senza rugosità/AO/height.
- Nota: questi EXR di Poly Haven sono convertiti da PNG con compressione DWAA (con perdita),
  scritto nei loro metadati.

### Interfaccia Semplice / Avanzata — Fase 1: modello dei preset
- Nuovo `forge_app/src/presets.rs`. Gli aspetti (Sfondo chiaro, Sfondo scuro, Studio
  scenografico), la qualità (Bozza, Bilanciata, Alta) e "Prepara scena prodotto" scrivono negli
  stessi dati del renderer (luci e ambiente della scena, fondale, `RenderSettings`): niente
  parametri duplicati.
- **"Annulla preset"** ripristina solo i valori toccati dal preset.
- **"Personalizzato"**: c'è un controllo di corrispondenza con il preset.
- Il fondale del menu Aggiungi usa la stessa funzione (`presets::backdrop_for`).
- **Test**:
  - appoggio sulla base;
  - fondale non duplicato;
  - annulla che non tocca le altre modifiche;
  - render reale: luminosità media 0,773 con lo sfondo chiaro contro 0,039 con lo sfondo scuro.
- Scheda di configurazione con valori e decisioni: `docs/INTERFACCIA_SEMPLICE.md`.
- Il pannello della modalità Semplice arriva nelle fasi 2–3.

### Interfaccia Semplice / Avanzata — Fasi 2–3: selettore e pannello Semplice
- **Selettore Semplice | Avanzata** in alto a sinistra, ricordato nelle Impostazioni
  (predefinita: Semplice).
- **Pannello Semplice** "Foto prodotto in 5 passi" con pulsanti grandi:
  1. Importa / Appoggia sulla base / Inquadra il prodotto;
  2. Aspetto (3 scelte), Prepara scena prodotto, Annulla preset, indicatore "Personalizzato";
  3. Materiali (pezzo selezionato, libreria);
  4. Formato (4) e Qualità (3), GPU;
  5. Avvia render con avanzamento, Salva immagine.
- **Avanzata invariata**: stessi dati. Dopo "Prepara scena" la scheda Render mostra i valori
  impostati (verificato con screenshot).
- Menu File: tolte le voci grigie e false "Importa STEP/SolidWorks — non disponibile" (funzionano
  da "Importa"); "Importa modello…" con i formati veri.
- Opzioni di avvio per test e screenshot: `--modalita semplice|avanzata`,
  `--prepara chiaro|scuro|scenografico`.
- Problemi risolti durante la prova:
  - i simboli ★ ↶ ▶ non sono nel carattere dell'app e sono stati tolti;
  - l'avviso "Personalizzato" compariva su una scena appena aperta;
  - il messaggio di conferma era una finestra che copriva il pannello;
  - `gpu_backend()` bloccava l'interfaccia se chiamata durante la preparazione della GPU
    (aggiunto `gpu_ready()`, che non aspetta).
- Report con screenshot: `reports/2026-10-29_interfaccia-semplice/`. Scheda dei valori:
  `docs/INTERFACCIA_SEMPLICE.md`.

## Prossimi passi
Vedi il piano qui sopra e la tabella di sintesi in docs/ROADMAP.it.md.
