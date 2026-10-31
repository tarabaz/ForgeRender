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

## Fatto nella decima parte (versioni 0.2.0 e 0.3.0)
- Interfaccia Semplice/Avanzata, preset di aspetto e qualità con "Annulla preset"
  (`crates/forge_app/src/presets.rs`, scheda `docs/INTERFACCIA_SEMPLICE.md`).
- Studio fotografico (tela inclinata + 4 luci) come scena di partenza, colore delle luci,
  "Adatta la scena al prodotto"; luci e oggetti nuovi vicino al prodotto.
- Vista ortografica/isometrica (viewport e camera di render, CPU e GPU: parole 27 e 31 dei
  parametri GPU).
- Raggio d'azione con sfumatura per puntiformi e spot (CPU e GPU: parole c.x, c.y della luce).
- Lingue: `crates/forge_app/src/i18n.rs` (`tr`, `tf`, `tk`), file `lang/Lan_*.json`; il test
  `english_covers_every_interface_text` fallisce se un testo nuovo non ha la traduzione inglese:
  ogni scritta nuova va passata a `tr("…")` (o `tf("…", &[…])` se ha valori) e aggiunta a
  `lang/Lan_eng.json`.
- Schermata di benvenuto (`crates/forge_app/src/ui/splash.rs`, immagine `assets/splash.png`).

## Fatto nell'undicesima parte (versione 0.4.0)
- Controllo da Claude con MCP: `forgerender --mcp`, codice in `crates/forge_app/src/mcp/`
  (protocollo, ponte con la finestra, 24 strumenti, operazioni pure con test). Guida
  `docs/MCP.md`, report `reports/2026-10-30_mcp/`. Ogni strumento nuovo: definizione in
  `tools::list`, ramo in `dispatch`, test della parte pura in `ops.rs`.

## Fatto nella dodicesima parte (versione 0.5.0)
- Controllo IA separato: crate `forge_mcp` (protocollo, strumenti, collegamento locale) e
  `forge_mcp_bridge` (`forgerender-mcp.exe`, avviato da Claude); nella finestra l'interruttore
  "Controllo IA" e l'indicatore "Controllato da IA". `render_status` per i render lunghi.
  Report `reports/2026-10-30_controllo-ia-ponte/`.

## Fatto nella tredicesima parte (versione 0.6.0)
- Modalità Semplice: valori di base sempre attivi (`presets::apply_simple_base`, applicata a ogni
  frame in Semplice: soglia di rumore 0,05/32, denoiser in anteprima + finale con CUDA, qualità
  alta/veloce, 8 campioni, 90 %, dettagli) e selettore GPU/CPU a due pulsanti (GPU predefinita
  una volta sola, `simple_gpu_checked`). Valori in `docs/INTERFACCIA_SEMPLICE.md`.
- Fondali larghi 12 volte il prodotto e alti 3,5 (niente bordi nelle inquadrature).
- Statistiche: utilizzo GPU e VRAM usata/totale/libera via NVML (`forge_telemetry::gpu`,
  caricata a runtime, solo NVIDIA). Da collaudare su una scheda NVIDIA.
- Schermata di benvenuto dal mockup di Valerio: `assets/splash.png` (suo render) e
  `assets/logo.png`, versione sotto il logo, licenze open source in piccolo sotto le informazioni.
  "Mostra all'avvio" in Impostazioni → Informazioni.
- Guide MCP con screenshot e render d'esempio puliti (denoiser), script in `tools/guida-mcp/`
  (`scatta.py`, `annota.py`, `galleria.py`).

## Fatto nella quattordicesima parte (versione 0.7.0)
- Piano interfaccia + manuale approvato (`docs/PIANO_INTERFACCIA_E_MANUALE.md`).
- Fase 1: inventario automatico `tools/manuale/inventario.py` → `manuale/` (366 controlli).
- Fase 2: `commands.rs` (elenco unico dei comandi con id, testo, icona, scorciatoia, azione),
  `ui/menubar.rs`, `ui/quickbar.rs`, `ui/icons.rs` (carattere Phosphor), tolto `ui/toolbar.rs`.
  Esportazione PNG/JPG/TIFF 16/EXR (`forge_image_io::save_frame`), progetti recenti, Esci e
  conferma delle modifiche non salvate, viste standard, luci da menu (`ForgeApp::add_light`).
  Corretto Maiusc+H/Alt+H. Screenshot in `reports/2026-10-30_nuova-interfaccia/`.
- Prossimo: fase 3 (finestra del manuale, ricerca, F1, "?", test di copertura). Gli screenshot
  della guida MCP vanno rifatti con la nuova barra (`tools/guida-mcp/scatta.py`, `annota.py`).

## Fatto nella quindicesima parte (versione 0.8.0)
- Fase 3 del manuale: crate `forge_manual` (pagine, Markdown, ricerca, `check_all`),
  finestra `ui/manual.rs` (indice, ricerca, Home/Indietro/Avanti, lingua), F1 e icona "?"
  (`HelpExt::help`, `help_button`), cartella `manuale/` nei pacchetti.
- Come aggiungere una pagina: file `manuale/<lingua>/<capitolo>/<id>.md` in **tutte** le lingue,
  id in `manuale/indice.json`, collegamento dal controllo con `.help("<id>")`; `cargo test`
  controlla il resto.
- `Build-ForgeRender.ps1` ora compila anche `forge_step` (serve CMake; `-SkipStep` per saltarlo).
- F1 e la ricerca non si possono provare con tasti simulati sotto Xvfb (winit non riceve il
  testo): provati il clic sull'icona e la navigazione, la ricerca con i test.

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
- **Fase 4 fatta (29–30/10/2026)**: download finito (868 completi, 1 errore:
  medieval_blocks_05), spostati 865 materiali con `--sposta` in 197 s, 0 errori; in libreria
  868 materiali Poly Haven (15,28 GB), nei download restano 13,17 GB non cancellati.
  4 materiali importati male e riparati (Rough Linen: rugosità = anisotropia; Book Pattern,
  Leather Red 02/03: solo varianti `col1`/`coll1`): `detect` ora ignora anisotropy/translucent/ior
  e riconosce `col1..3`/`coll1..3`. Report: `reports/2026-10-30_import-polyhaven-locale/REPORT.md`.
  Da fare: riscaricare medieval_blocks_05; decidere sui residui e su EXR o PNG.

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

### Versione 0.2.0
- Versione portata da 0.1.0 a **0.2.0** (`Cargo.toml`, valore di riserva dell'installer).
- Novità e regola per i prossimi incrementi in `CHANGELOG.md`:
  - 0.X.0 per un gruppo di funzioni consegnato;
  - 0.X.Y per sole correzioni.

### Controllo IA provato con Claude Code vero (30/10/2026)
- Con la portable 0.8.2 e l'app desktop di Claude Code su Windows funziona: Claude esegue una
  richiesta dopo l'altra, anche complesse (luci, materiali, spostamenti, camera).
- Con il ponte registrato in un `.mcp.json` di cartella funzionava solo la prima risposta: a
  fine risposta Claude Code chiude il ponte ("Closing MCP server stdin" nei suoi log, in
  `%LOCALAPPDATA%\claude-cli-nodejs\Cache\…\mcp-logs-forgerender`) e poi non lo riavvia. Il ponte
  non c'entra: si chiude solo a fine stdin.
- Soluzione: `claude mcp add --scope user forgerender -- "<percorso>\forgerender-mcp.exe"`.
  `docs/MCP.md` e `MCP.en.md` la indicano ora come metodo principale, con il caso "funziona solo
  il primo comando" in "Se qualcosa non va".
- Da valutare: un percorso fisso per il ponte, perché il percorso registrato cambia a ogni nuova
  cartella portable.

### 0.8.3: striature in ortografica, "Appoggia sulla base" (30/10/2026)
- Trovati da Valerio con l'assieme STEP Micromouse (SolidWorks, componenti in file separati:
  il convertitore segue da solo i rimandi del file principale; 120 pezzi, 31 colori).
- **Striature in isometrica**: auto-ombre sui triangoli sottili attorno ai fori. Punto colpito
  ricalcolato dal triangolo (`RenderGeometry::surface_point`, `surface_point` nello shader) al
  posto di `o + d·t`, che con l'origine arretrata dell'ortografica sbagliava più dello
  scostamento Wächter-Binder. Test `forge_render_cpu/tests/ortho_acne.rs` (prima 7627 auto-ombre
  su 16244 punti, ora 0).
- **Appoggia**: `presets::product_bounds` sui vertici veri (prima scatole ruotate: 12,9 mm di
  troppo sul Micromouse, fondale messo sotto il prodotto); `has_product` leggero per
  l'interfaccia; `product_bounds_approx` per l'ancoraggio delle luci (pannello Luci, ogni frame).
  "Appoggia sulla base" scende fino al contatto con la scenografia sotto (`contact.rs`, lo stesso
  calcolo dello snap a contatto del gizmo), altrimenti quota 0. Test
  `rotated_part_touches_the_base_exactly`, `drop_stops_on_the_scenery_below`.
- **Da verificare da Valerio** su Windows: render isometrico del Micromouse senza striature (GPU
  e CPU), "Appoggia" con il fondale fotografico.
- Nota: su Windows con `core.autocrlf=true` il test `english_covers_every_interface_text`
  fallisce (le stringhe su più righe prendono `\r\n`); non dipende da queste modifiche.

### 0.9.0: `inspect_components`, Claude guarda i pezzi (30/10/2026)
- Richiesta di Valerio: che Claude capisca da solo che cosa sono i pezzi (forma e scopo) e
  applichi i materiali, guardando la viewport invece di fare render.
- `forge_app/src/mcp/inspect.rs`: raggruppamento dei pezzi (parte STEP con i gruppi di colore =
  un pezzo; pezzi con le stesse mesh = copie), rasterizzatore CPU con z-buffer e sovracampionamento
  2×, spigoli scuriti, griglia PNG con numeri. Strumento in `forge_mcp::tools` e `mcp/tools.rs`,
  citato nelle istruzioni del server. Guida `docs/MCP.md` / `MCP.en.md` aggiornata.
- Test: raggruppamento e copie, immagine (misure, pezzo disegnato, arancione nell'assieme, scena
  invariata), nomi delle viste; `real_model_sheets` (ignorato) per un `.frmesh` vero.
- Prova sul Micromouse: 25 pezzi diversi in 3 immagini da 0,03–0,06 s; riconoscibili sensore
  HC-SR04, Arduino UNO, L298N, ruote, dissipatore, pila AA, portabatterie, molla, viti.
- **Da verificare da Valerio**: una sessione Claude vera che usa lo strumento e applica i
  materiali della libreria.

### 0.10.0: materiali predefiniti con mappe, primi tre (30/10/2026)
- Richiesta di Valerio: predefiniti "veri" con mappe, prendendo alcune mappe dalla libreria
  Poly Haven scaricata (non la libreria in sé). Poly Haven copre legni, pelli, tessuti, pietre;
  non ha metalli satinati, plastiche, carbonio (da generare). Il renderer non ha anisotropia né
  vernice trasparente (fasi a parte, per spazzolati e laccati).
- Fatti Rovere (Oak Veneer 01), Pelle nera (Leather White resa grigia, tinta 0,06) e Marmo
  bianco lucido (generato). `MAPPED` in `forge_scene::material`, `builtin_textures_dir()`
  (`FORGE_MATERIALS_DIR`, `materiali\` accanto all'exe, `assets/materiali` risalendo);
  `forge_project::load` ritrova le mappe dei predefiniti spostati; pacchetti aggiornati.
- Esempi: `forge_render_core` `materiali_predefiniti` (mappe 1K JPEG, marmo generato),
  `forge_render_cpu` `confronto_materiali` (prima/dopo), `forge_scene` `elenco_predefiniti`
  (elenco e schede `impostazioni.json`).
- Cartella di lavoro di Valerio: `dist\MCP\nuova libreria` (11 categorie numerate, 123
  cartelle con scheda; le tre fatte hanno anche le mappe). Mappe da altre fonti solo CC0.
- Prossimi: altri legni, pietre, tessuti da Poly Haven; generatore per spazzolati, sabbiati,
  goffrati, layer di stampa 3D, carbonio.

### 0.11.0: libreria unificata, metalli BlenderKit (30/10/2026)
- Valerio ha scaricato 43 materiali BlenderKit (cartella `dist\MCP\blender material\metal`):
  tutti **Royalty Free** (licenza letta da `blenderkit_data\config`), 5 solo procedurali (tolti,
  nel Cestino). Repository **privato**: va bene per uso personale; prima di distribuire,
  togliere `assets/materiali-blenderkit` (LEGGIMI nella cartella).
- `forge_library::blend` (lettore .blend: zstd a più frame, intestazione classica e Blender 5,
  SDNA, campi per nome) e `forge_library::blenderkit` (dal Principled BSDF alle immagini;
  valori degli slider dai nodi). CLI `libreria importa-blenderkit`. 37 convertiti in
  predefiniti a dati con l'esempio `blenderkit_predefiniti` (1 senza Principled diretto).
- Predefiniti a dati: `DATA_PRESET_DIRS`, `data_presets()`; relocate dei progetti e pacchetti.
- Finestra Libreria: scheda "Materiali" con "Libreria ufficiale" (predefiniti, miniature in
  `<libreria>\.cache\anteprime-predefiniti`) e "Scaricati". Verificata con uno screenshot.
- Test traduzioni: normalizzati i CRLF (prima falliva su Windows).
- Prossimi: rivedere scale e nomi dei metalli BK; i predefiniti ufficiali da fonti CC0
  (ambientCG) per una versione distribuibile; migliorare gli altri predefiniti con mappe.

### Importatore .blend fase 1 e 0.12.0: anteprime sullo shader ball (30–31/10/2026)
- Importatore `.blend` nel programma, piano in 5 fasi approvato da Valerio. Fase 1 fatta
  (`forge_library::blenderkit`): tutti i materiali, gruppi, immagini esterne, Mapping, normal
  DirectX, Displacement, rapporto per parametro. Regola: interamente procedurale = non si
  importa; parte procedurale = si importa con avviso. 38/38 file di prova importabili.
- Anteprime dei materiali sullo shader ball scelto da Valerio (`assets/anteprima`, CC-BY 4.0),
  impostazioni di qualità/campioni/risoluzione e "Svuota cache anteprime"; pannello Materiali
  con i soli materiali della scena.
- Valerio: i materiali BlenderKit attuali sono un'idea valida ma alcuni non gli piacciono; si
  farà prima una libreria buona con l'importatore, poi si deciderà come integrarla.
- Prossimo: fase 2 (anteprima nella finestra dell'importatore), poi fasi 3–5.

### 0.13.0: importatore .blend nel programma (31/10/2026, fasi 2–4)
- Libreria → "Importa materiale da Blender…" (e pulsante nella finestra Libreria): sorgente
  (scelta o trascinamento del file, materiali, mappe), parametri (slider dell'editor + tabella
  Blender → ForgeRender con lo stato), anteprima sullo shader ball, import nella libreria.
  `crates/forge_app/src/ui/blend_import.rs`. La scena aperta non cambia.
- Resta la fase 5 ("Esporta come predefinito" con controllo della licenza) e la decisione su
  come integrare nel programma la libreria che Valerio costruirà con l'importatore.

### 0.14.0: dimensioni locali, GPU prima scelta, vista Mesh (31/10/2026)
- Dimensioni e scala lungo gli assi dell'oggetto (`Scene::local_size`, `frame_of`,
  `user_matrix`; `UserTransform::matrix(pivot, frame)` e `from_matrix`): un cubo ruotato
  resta un cubo, le dimensioni non cambiano con la rotazione.
- GPU di partenza in ogni modalità; anteprime dei materiali sulla GPU (dispositivo dedicato,
  CPU di riserva); anteprima dell'importatore che segue la finestra.
- Viewport Standard/Mesh, pulsante "Materiale" che apre la libreria, primitive più fitte.
- Restano in attesa: proiezione Box nello spazio dell'oggetto (la texture non scivola sugli
  oggetti ruotati) e dimensione dell'oggetto dell'anteprima (10 cm / 30 cm / 1 m).

### 0.15.0: proiezione Box sugli assi dell'oggetto e sfumata, Appoggia a contatto (31/10/2026)
- Texture Box nello spazio dell'oggetto (`Scene::texture_frame`, `RenderGeometry::node_frames`,
  `displace::BoxFrame`) e sfumate tra le tre proiezioni (`box_weights`, esponente 8) su CPU
  e GPU; displacement con coordinate dell'oggetto.
- Anteprime: dimensione dell'oggetto (Reale 9 cm / 30 cm / 1 m), miniature v6.
- "Appoggia" sul pezzo subito sotto (`contact::drop_onto_below`).

### 0.16.0: Fase 1 della revisione (31/10/2026)
- Displacement con proiezione Box sfumata (`Params::height_box`, `FLAG_DISPLACED_BOX`, altezza e
  direzione per vertice negli UV con `pack_dir`), soglia dei pesi continua, conversione del nodo
  Displacement di Blender (Scale, Midlevel, metodo) e riconversione dei 38 predefiniti BlenderKit.
- Fase 2 (ancoraggio Component / Assembly / World) e Fase 3 (Appoggia) in attesa di approvazione:
  piano in `reports/chatgpt/CLAUDE_HANDOFF.md`.

### 0.17.0: Fase 2 (ancoraggio texture) e Fase 3 (Appoggia) della revisione (31/10/2026)
- `TextureMapping::anchor` (Component / Assembly / World), `Scene::texture_frame_for`,
  `render_scene::box_frames` → `RenderScene::node_frames` (shading CPU e GPU); migrazione dei
  progetti schema 1 con `app_version` < 0.15 su World (`forge_project::migrate_box_anchor`).
- `contact::drop_onto_below`: caduta indipendente per pezzo, `Contact::edge_hit` (spigolo
  contro spigolo, griglia 2D), pezzi nascosti scelti.

### 0.18.0: integrazione AI con un clic (31/10/2026)
- `forge_app/src/mcp/setup.rs` (rilevamento, Collega/Scollega per Claude Desktop, Claude Code,
  Codex; ponte in cartella fissa), `ui/ai_setup.rs` (finestra), saluto del ponte con il nome
  dell'assistente (`rpc::client_name`, `link::Client::connect(assistant)`).
- Prossimo: luci spot volumetriche con fascio regolabile.

### 0.19.0: luci spot volumetriche (31/10/2026)
- `Light::beam_density` / `beam_length`; `render_core::fog::beam_fogs` (griglia a cono con
  `BakedFog::from_fn`, flag `camera_only`); integratori CPU/GPU: i fasci contano solo sui percorsi
  dalla camera (rimbalzi speculari) con diffusione singola.

### 0.20.0: scala dei materiali adattata all'oggetto (31/10/2026)
- `TextureMapping::fit_object`; `BoxFrame::scale` (1 / lato del pezzo) calcolato in
  `box_frames`; CPU e GPU (parola 7 del riferimento) e displacement (`Params::box_unit_m`);
  predefiniti BlenderKit e importatore in modalità adattata.


### 0.21.0: editor del materiale, esportazione zip, correzioni (31/10/2026)
- `ui/material_editor.rs` (finestra Modifica materiale, anteprima girevole con
  `central::render_preview_view` / `orbit_camera`), `inspector::material_form` condiviso con il
  pannello Materiali, `forge_library::export::material_zip` (zip = formato dei predefiniti a dati).
- `Quality::Draft` a 128 campioni; `presets::sizing_bounds` per gli oggetti del menu Aggiungi
  (esclude la scenografia selezionata: i fondali rifatti non crescono più).
- Prossimo: quando Valerio consegna la cartella di zip, inserirli come predefiniti.


### 0.22.0: viste multiple, turntable con video, passaggi di render (31/10/2026)
- `forge_app/src/sequence.rs`:
  - piano dei fotogrammi;
  - rotazione delle radici del prodotto tramite `UserTransform::from_matrix`;
  - `fit_camera` / `plan_cameras` (inquadratura che contiene i punti del prodotto ruotati);
  - `top_camera`;
  - macchina a stati in `tick_sequence` (identificata dal numero di job della sessione,
    `JobInfo::session_job`);
  - ripristino finale.
- `forge_app/src/video.rs`: MP4 con Media Foundation (`windows` 0.62), ffmpeg come riserva.
- `forge_render_cpu::passes`: profondità, maschera, ID.
- `ui/sequence.rs`: finestre e modale di avanzamento.
- Opzioni di prova: `--sequenza <cartella>` (con `--sequenza-viste`, `-passo`, `-campioni`,
  `-passaggi`) e `--sequenze`.


### 0.23.0: denoiser al 100% e soglia di rumore 0,03 di base (31/10/2026)
- `presets::apply_simple_base`: `noise_threshold` 0,03 e `denoise.strength` 1,0;
  `Preferences::new_project_settings`: `noise_threshold` 0,03. `RenderSettings::default`
  (motore, CLI, test) non cambia.
- Su richiesta di Valerio solo sul branch, non su main.

## Prossimi passi
Vedi il piano qui sopra e la tabella di sintesi in docs/ROADMAP.it.md.
