# Novità di ForgeRender

La versione è in un solo punto: `version` in `[workspace.package]` del `Cargo.toml` alla radice.
Tutti i componenti la ereditano; il titolo della finestra mostra `ForgeRender v<versione> (<commit>)`.

## Regola per le versioni (0.x)

- **0.X.0** (minore): un gruppo di funzioni nuove consegnato. Si incrementa nel commit
  che chiude il gruppo, insieme a questa pagina.
- **0.X.Y** (correzione): solo correzioni di difetti, senza funzioni nuove.
- Ogni incremento aggiunge qui una sezione con data e commit. Il numero cambia nel `Cargo.toml`
  (e nell'installer `packaging/windows/forgerender.iss`, solo come valore di riserva).

## 0.23.0 — 31/10/2026

**Valori di partenza del render**
- Denoiser attivo al 100% (prima 90%) nella modalità Semplice.
- Soglia di rumore 0,03 (prima 0,05) nella modalità Semplice e nei progetti nuovi in modalità
  Avanzata (prima l'arresto a soglia lì era spento). Immagini più pulite, render un po' più
  lunghi.

## 0.22.0 — 31/10/2026

**Viste multiple e turntable con video MP4** (Rendering → Viste e turntable…)
- Viste multiple automatiche: frontale, tre quarti destra e sinistra, destra, sinistra,
  posteriore, dall'alto (si scelgono con le caselle). Il prodotto gira su se stesso, mentre
  camera, luci e fondale restano fermi come in uno studio. Ogni vista viene inquadrata da sola,
  con il prodotto sempre tutto dentro.
- Turntable: si sceglie la rotazione tra due immagini (es. 2°) oppure la durata di un giro in
  secondi; la finestra mostra quante immagini servono e quanto dura il video. Si regolano anche
  i fotogrammi al secondo (24–60), i giri ripetuti nel video e il senso orario o antiorario.
  Il giro si chiude senza ripetere la prima immagine, così il video gira in loop senza scatti.
  L'inquadratura è una sola e contiene il prodotto in ogni posizione del giro.
- Video MP4 H.264 creato con l'encoder incluso in Windows (anche quello hardware della scheda
  video): niente da installare. Se manca, si usa ffmpeg quando è installato.
- Contro lo sfarfallio, ogni immagine usa:
  - gli stessi campioni per pixel, quindi il rumore residuo resta fermo;
  - un numero di campioni fisso, senza campionamento adattivo;
  - il denoiser solo sull'immagine finale.
  Misurato sul fondale fermo di due immagini consecutive: 61 dB di PSNR, meno di un livello
  su 255 di differenza media.
- Camera: si usa quella di render attiva; se non c'è, la vista attuale, con il pulsante "Crea
  una camera da questa vista". Alla fine, o se si interrompe, scena, camera e impostazioni
  tornano come prima.

**Passaggi di render** (File → Esporta passaggi di render…, oppure per ogni immagine delle
sequenze)
- Albedo, normali, profondità (PNG 16 bit con bianco = vicino, più EXR in metri), maschera del
  prodotto per scontornare e ID dei pezzi (un colore per pezzo). Sono calcolati sulla CPU
  dall'ultimo render, con la stessa inquadratura.

## 0.21.0 — 31/10/2026

**Editor del materiale ed esportazione in zip**
- Libreria materiali: un clic non applica più il materiale, apre la finestra "Modifica
  materiale" (doppio clic: applica come prima). Si modificano tutti i valori e le mappe, poi:
  "Applica alla selezione", "Salva nella libreria" (un predefinito diventa un materiale della
  libreria), "Esporta zip…", "Annulla modifiche".
- "Esporta zip…" crea uno zip con `materiale.json` (tutte le impostazioni) e le mappe, con i nomi
  dei file resi relativi: scompattato in una cartella è un materiale predefinito pronto.
- L'anteprima della finestra si gira trascinandola con il mouse (doppio clic: vista iniziale);
  mentre si trascina è più leggera, poi torna alla qualità piena.

**Correzioni**
- Qualità "Bozza" a 128 campioni (prima 64): nessuna partenza sotto i 128.
- Fondale da studio e fondale fotografico: aggiunti più volte con il precedente selezionato,
  ognuno si dimensionava sul precedente e cresceva ogni volta. Ora si dimensionano sempre sul
  prodotto (la scenografia nella selezione non conta).

## 0.20.0 — 31/10/2026

**Scala dei materiali adattata all'oggetto**
- Editor materiali, proiezione Box: "Scala: In metri / Adatta all'oggetto". Con "Adatta
  all'oggetto" la ripetizione segue la dimensione di ogni pezzo (lato più lungo, o dell'assieme
  con l'ancoraggio Assieme): "Ripetizioni sull'oggetto", 1 di partenza. La stessa texture sta
  bene su una sfera da 5 cm e su un tavolo, come nelle anteprime di BlenderKit.
- I 38 materiali BlenderKit partono in "Adatta all'oggetto" con 1 ripetizione (prima 1 m per
  ripetizione: su oggetti piccoli la texture era enorme e sfocata). L'importatore .blend fa lo
  stesso: le ripetizioni del nodo Mapping diventano ripetizioni sull'oggetto.
- Rilievo e displacement seguono la stessa scala; CPU e GPU coincidono. Rovere, marmo, pelle e i
  materiali scaricati restano in metri.

## 0.19.0 — 31/10/2026

**Luci spot volumetriche**
- Pannello Luci, luce Spot: "Fascio visibile → Volumetrico" mostra il cono di luce nell'aria,
  come in una sala con un po' di foschia. Si regolano l'intensità del fascio (densità della
  foschia) e la lunghezza; forma e bordo seguono l'apertura e la sfumatura del cono, colore e
  ombre (gli oggetti nel fascio proiettano coni d'ombra) vengono dalla luce.
- Il fascio lo vede la camera (anche attraverso vetri e specchi) con una diffusione sola: non
  illumina la scena con rimbalzi deboli e rumorosi, quindi le superfici restano pulite.
  Le nebbie simulate (ghiaccio secco) restano fisiche come prima.
- CPU e GPU danno lo stesso risultato. Anche dal controllo IA: edit_light con beam_density e
  beam_length_m.

## 0.18.0 — 31/10/2026

**Integrazione AI con un clic**
- Strumenti → Integrazione AI…: stato del server MCP di ForgeRender e di ogni assistente
  (Non installato, Non configurato, Da aggiornare, Configurato, Connesso) con i pulsanti
  Collega, Aggiorna e Scollega. Niente terminale né percorsi da copiare.
- Claude Desktop (anche versione Microsoft Store e cartella Claude-Data), Claude Code
  (terminale e app, con il comando claude o il file utente), Codex di OpenAI. ChatGPT risulta
  non supportato: collega solo server remoti su internet.
- Il ponte forgerender-mcp.exe si copia da solo in %LOCALAPPDATA%\ForgeRender\mcp: i
  collegamenti restano validi quando aggiorni o sposti il programma.
- Prima di modificare una configurazione se ne fa una copia (.backup-forgerender); si scrive
  solo la voce forgerender; un file rovinato non si tocca.
- Al primo clic su "Controllo IA" senza assistenti collegati si apre la finestra; al primo
  collegamento riuscito il controllo si accende da solo. Dopo il collegamento di Claude Desktop,
  pulsante "Riavvia Claude Desktop" (serve una volta sola, con conferma).
- Nella barra: "Controllato da Claude Code" (o Claude Desktop, Codex): il ponte dice al
  programma quale assistente si è collegato.

## 0.17.1 — 31/10/2026

**Correzioni**
- Riavvio automatico solo nella vista Render: nella viewport puoi spostare pezzi, luci e camera
  senza che il render riparta né che la vista passi al Render. Un render in corso diventato
  vecchio si ferma e riparte quando torni nella vista Render (con il riavvio spento continua
  come prima).
- Guarda camera: la viewport mostra l'inquadratura intera della camera di render (prima con
  immagini larghe veniva tagliata ai lati), con il resto della scena più scuro tutto attorno.
  La rotella ingrandisce o rimpicciolisce la vista attorno all'inquadratura senza muovere la
  camera (con "Pilota" la rotella muove la camera come prima).

## 0.17.0 — 31/10/2026

**Fase 2: ancoraggio delle texture Box**
- Nell'editor dei materiali (proiezione Box) si sceglie l'ancoraggio: **Pezzo** (la texture
  segue il singolo pezzo, come dalla 0.15), **Assieme** (una sola proiezione per tutto
  l'assieme, continua anche tra pezzi ruotati diversamente; segue l'assieme) o **Mondo** (texture
  ferma nella scena, come fino alla 0.14). CPU e GPU coincidono nelle tre modalità.
- Progetti salvati prima della 0.15: all'apertura i materiali Box passano su Mondo, così le
  venature restano dove erano (avviso nella finestra di apertura). Schema del progetto 2.

**Fase 3: Appoggia**
- Con più pezzi scelti ognuno scende per conto suo, dal più basso al più alto: si fermano sul
  proprio appoggio e si impilano.
- Contatto spigolo contro spigolo: due spigoli incrociati non si compenetrano più.
- Un pezzo nascosto scelto si appoggia comunque su ciò che ha sotto.
- Un solo Ctrl+Z annulla tutto (l'operazione cambia solo i pezzi scelti).

## 0.16.0 — 31/10/2026

**Fase 1 della revisione: correttezza di materiali e texture**
- Displacement con proiezione Box sfumata: l'altezza di ogni punto è la media delle tre
  proiezioni secondo la direzione di spostamento, come per le altre mappe. Spariscono i tagli
  netti sulle superfici curve con il displacement acceso; la superficie resta chiusa (niente
  crepe) e CPU e GPU coincidono. Ogni vertice spostato porta l'altezza applicata e la
  direzione: lo shading ricostruisce la superficie originale e legge colore, rilievo e altezza
  negli stessi punti.
- Soglia dei pesi Box continua: le proiezioni deboli spariscono senza salti (prima un gradino
  dell'1 % del peso).
- Importatore Blender: il nodo Displacement si converte con i suoi valori (Scale × 1000 = mm,
  Midlevel = livello medio) e il metodo del materiale (Bump only → rilievo, Displacement only →
  displacement, entrambi → tutti e due). Il nodo Bump usa Strength × Distance; la Normal Map
  collegata al Normal del Bump non si perde più.
- Materiali BlenderKit predefiniti riconvertiti: 30 su 38 cambiano solo nei valori di rilievo e
  displacement (mappe identiche); copia degli originali in `reports/revisione-0.16/`.

## 0.15.0 — 31/10/2026

**Texture a proiezione Box**
- Proiezione sugli assi dell'oggetto: la texture segue il pezzo quando lo si sposta o ruota
  (prima restava ferma nel mondo e scivolava), e su un cubo ruotato la venatura è
  allineata alle facce. Pezzi accostati dello stesso assieme restano continui.
- Le tre proiezioni si sfumano secondo la normale: sulle superfici curve (sfera, toro)
  niente più tasselli che cambiano da un triangolo all'altro. Vale anche per normal map,
  rilievo e ritaglio, su CPU e GPU (stesse formule), e per il displacement.

**Anteprime dei materiali**
- Impostazioni → Anteprime dei materiali → Dimensione dell'oggetto: Reale (9 cm), 30 cm
  (predefinita) o 1 m. Texture, rilievo e displacement si scalano sulla sfera come su un
  oggetto di quella dimensione, così l'anteprima somiglia al pezzo nella scena.
- Miniature rifatte (versione v6) con la nuova proiezione.

**Appoggia**
- Il pulsante "Appoggia" del pannello Oggetto (e l'opzione dello strumento MCP) fa scendere
  i pezzi scelti finché toccano l'oggetto subito sotto (altro pezzo, fondale, pavimento),
  come lo snap a contatto verso il basso; con niente sotto vanno a quota 0. Ctrl+Z annulla.

## 0.14.0 — 31/10/2026

**Dimensioni e scala sugli assi dell'oggetto**
- Le dimensioni del pannello Oggetto sono quelle dell'oggetto lungo i suoi assi, non il
  riquadro nel mondo: un cubo da 800 mm ruotato resta 800 × 800 × 800. Cambiando una
  dimensione si scala l'oggetto lungo il suo asse, senza deformarlo.
- La scala (pannello e gizmo) agisce sempre lungo gli assi propri del componente (la sua
  rotazione nel file sorgente più quella utente). I progetti con scala uniforme non cambiano;
  quelli con scala diversa sui tre assi di un pezzo ruotato nel file ora lo allungano lungo i
  suoi assi.

**GPU come prima scelta**
- Il motore di render parte sulla GPU in ogni modalità appena si sa che funziona (la CPU solo
  se la GPU manca o se la sceglie l'utente); le Impostazioni propongono GPU per i nuovi
  progetti. Funziona con schede NVIDIA, AMD e Intel (Vulkan o DirectX 12).
- Anteprime dei materiali (pannello, miniature della libreria, importatore) sulla GPU, con un
  dispositivo dedicato per non disturbare il render; la CPU solo se la GPU non c'è
  (`FORGE_PREVIEW_CPU=1` la forza, modalità sicura sempre CPU).

**Interfaccia**
- Importa materiale da Blender: l'anteprima segue la colonna e la finestra (si rifà alla
  nuova dimensione, a gradini di 64 pixel).
- Viewport: vista **Standard** o **Mesh** (triangoli delle mesh sopra l'ombreggiatura) nella
  barra in alto della viewport; `--mesh` all'avvio.
- Barra in alto: "Materiale" è un pulsante che apre direttamente la libreria.
- Oggetti generati (sfera, cilindro, tazza, fondali) e scena di test con mesh tre volte più
  fitte: niente sfaccettature visibili sulla sfera e sul toro.

## 0.13.0 — 31/10/2026

**Libreria → Importa materiale da Blender…**
- Finestra nel programma (anche dal pulsante "Importa da Blender…" della Libreria): si sceglie
  o si **trascina** un file `.blend` (con la finestra aperta il `.blend` non viene importato
  come modello). Lettura in background, senza Blender.
- **Sorgente**: materiali del file (si sceglie quale), shader, mappe trovate e mancanti,
  immagini del file non usate, avvisi; i materiali interamente procedurali non si mostrano e
  non si importano, quelli in parte procedurali sì con l'avviso.
- **Parametri**: nome, categoria, colore base, metallic, roughness/glossiness, IOR, scala e
  posizionamento texture, rilievo (gli stessi controlli dell'editor dei materiali), più la
  tabella "Da Blender a ForgeRender" con lo stato (esatto, approssimato, non supportato) e la
  nota della conversione.
- **Anteprima** sullo shader ball delle anteprime, renderizzata dal vero motore su un thread a
  parte; si rifà quando cambia un valore.
- **Importa nella libreria**: mappe e valori salvati con le funzioni della libreria; la scena
  aperta non cambia. Avvio con `--importa-blend FILE`.

## 0.12.0 — 31/10/2026

**Anteprime dei materiali sullo shader ball, materiali della scena**
- Le miniature (Libreria, pannello Materiali) e l'anteprima del materiale si renderizzano sulla
  scena scelta da Valerio: **USD Shader Ball** (Khronos glTF Sample Assets, CC-BY 4.0) con le
  sue luci, camera e ambiente (`assets/anteprima`). Il materiale va sul pezzo
  `material_surface`; senza la cartella si torna alla sfera.
- **Impostazioni → Libreria e import → Anteprime dei materiali**: qualità Bassa / Media / Alta,
  risoluzione minima (128, 256, 512 px), numero di campioni e **"Svuota cache anteprime"**. Le
  miniature dei predefiniti si rifanno da sole cambiando i campioni; quelle salvate nella
  libreria si rifanno svuotando la cache. Le miniature vecchie (sfera) si rifanno da sole.
- **Pannello Materiali**: al posto dell'elenco completo (doppione della Libreria) solo i
  materiali della scena (assegnati, predefinito, selezionato) con la miniatura; "Libreria
  materiali…" apre la finestra con tutti.
- **Importatore .blend, fase 1** (già nel commit precedente): tutti i materiali di un file,
  gruppi di nodi, immagini esterne, Mapping, normal DirectX, rapporto dei parametri.

## 0.11.0 — 30/10/2026

**Libreria unificata e metalli BlenderKit**
- **Finestra Libreria, scheda "Materiali"**: in un solo elenco la **Libreria ufficiale** (tutti i
  materiali predefiniti, per categoria, con la sfera di anteprima salvata in cache) e gli
  **Scaricati** (ambientCG, Poly Haven, cartelle e zip importati). La ricerca vale per tutti;
  un clic applica il materiale alla selezione.
- **37 materiali BlenderKit predefiniti** (36 metalli: acciai satinati, spazzolati, inox,
  zincati, ossidati, verniciati, dorati, ramati, titanio…; un rovere), convertiti dai `.blend`
  senza Blender: mappe 1024 px e valori degli slider letti dai nodi del materiale. Cartella
  `assets/materiali-blenderkit` (≈25 MB), categorie "Metalli BlenderKit" e "Legni BlenderKit".
  **Licenza Royalty Free**: solo uso personale; prima di distribuire il programma la cartella va
  tolta (vedi il suo LEGGIMI).
- Predefiniti "a dati": `materiale.json` + mappe in una cartella, letti all'avvio
  (`forge_scene::material::data_presets`); i progetti ne ritrovano le mappe anche spostati.
- Strumenti: `forgerender-cli libreria importa-blenderkit CARTELLA [--solo-elenco]` (nella
  libreria locale) ed esempio `blenderkit_predefiniti` (per i predefiniti). I materiali solo
  procedurali (nodi di rumore e rampe) non hanno immagini e si saltano.
- Il test delle traduzioni ora passa anche su Windows (fine riga CRLF dei sorgenti).
- Lettore dei file `.blend` senza Blender (`forge_library::blend`: file compressi zstd, anche
  Blender 5, catalogo SDNA) e lettura dei nodi del materiale (`forge_library::blenderkit`).
  Nuova dipendenza `ruzstd` (Rust puro, MIT).
- Avviso delle licenze (`licenze\AVVISO_TERZE_PARTI.txt`) completato dopo la revisione del
  31/10: attribuzione CC-BY dello USD Shader Ball (0.12.0), mappe Poly Haven CC0 (0.10.0),
  limiti della licenza BlenderKit, `ruzstd` e `pulldown-cmark`.

## 0.10.0 — 30/10/2026

**Materiali predefiniti con mappe (primi tre)**
- **Rovere** (venatura vera, Poly Haven Oak Veneer 01, CC0), **Pelle nera** (grana di Poly
  Haven Leather White resa neutra, tinta nera dal materiale) e **Marmo bianco lucido**
  (venature generate, ripetibili senza giunture) hanno colore, rugosità e normal a 1024 px.
  Gli altri predefiniti non cambiano.
- Mappe in `assets/materiali/<id>/` (circa 2 MB in tutto), copiate in `materiali\` accanto al
  programma nella portable e nell'installer; si rifanno con l'esempio `materiali_predefiniti`.
  Se la cartella manca, i predefiniti restano senza mappe.
- I progetti salvati con un'altra installazione ritrovano le mappe nella cartella del programma
  in uso.
- Per preparare gli altri: `elenco_predefiniti --cartelle <radice>` crea una cartella per
  categoria e materiale con `impostazioni.json` (valori degli slider dell'editor, stessi nomi
  e ordine) da regolare; `confronto_materiali` fa i render prima/dopo.

## 0.9.0 — 30/10/2026

**Controllo IA: Claude guarda i pezzi e capisce che cosa sono**
- Nuovo strumento MCP **`inspect_components`**: immagini veloci dei pezzi, in stile viewport
  (non un render), per far capire all'assistente forma e funzione di ogni pezzo prima di
  scegliere i materiali, anche quando i nomi non dicono niente ("NAUO32", "Part1").
- Ogni pezzo diverso ha un riquadro numerato: a sinistra il pezzo da solo e inquadrato, a destra
  l'assieme con il pezzo in arancione (in trasparenza dove è coperto). 12 riquadri per immagine,
  poi le pagine successive; vista a scelta (tre quarti, alto, basso, fronte, retro, lato).
- I pezzi identici (viti, ruote ripetute) compaiono una volta sola con l'elenco delle copie; una
  parte STEP divisa per colore conta come un pezzo solo. Per ogni riquadro: nome, id da usare in
  `apply_material`, copie, misure in mm, materiali e colori attuali.
- Disegnato sulla CPU (z-buffer, spigoli scuriti): non tocca la scena, non sposta la camera e non
  salva file. Sull'assieme Micromouse (120 pezzi con mesh, 25 diversi) 0,03–0,06 s per immagine.

## 0.8.3 — 30/10/2026

**Correzioni**
- **Striature in vista isometrica / ortografica**: nel render comparivano righe scure che
  partivano dai fori dei modelli CAD (i triangoli lunghi e sottili si facevano ombra da soli).
  In ortografica i raggi partono metri indietro e il punto colpito, calcolato come
  origine + direzione × distanza, aveva un errore più grande dello scostamento
  anti-autointersezione. Ora il punto si ricalcola dal triangolo colpito (coordinate
  baricentriche), su CPU e GPU. Test `ortho_acne`: con il calcolo di prima 7627 auto-ombre su
  16244 punti, ora 0.
- **"Appoggia sulla base" appoggia davvero**: il punto più basso si calcolava dalle scatole dei
  pezzi ruotate insieme ai pezzi, che sporgono oltre la forma (assieme STEP Micromouse: 12,9 mm),
  e il prodotto restava sollevato; lo stesso valore metteva il fondale sotto il prodotto. Ora
  l'ingombro del prodotto usa i vertici veri (fondale, luci, inquadratura compresi).
- **"Appoggia sulla base" a contatto**: il prodotto scende finché una qualsiasi delle sue parti
  tocca la scenografia che ha sotto (fondale, pavimento, parete aggiunti), con lo stesso calcolo
  dello snap a contatto del gizmo (ora in `contact.rs`, usato da tutti e due); se sotto non c'è
  niente, va a quota 0. Vale anche per il pannello Semplice e per il Controllo IA.

## 0.8.2 — 30/10/2026

**Correzioni**
- **Riavvio automatico con una regola sola**: acceso, il render riparte **sempre** quando cambiano
  vista o scena (in corso, in pausa, finito o fermato con Stop, in vista Render o Viewport);
  spento, non riparte **mai** e resta l'avviso "Immagine non aggiornata". Sostituisce le
  eccezioni della 0.8.1. La camera di render attiva (senza "Pilota") resta ferma come prima.

## 0.8.1 — 30/10/2026

**Correzioni**
- **Riavvio automatico anche a render finito**: se il render è completato e lo stai guardando
  (vista Render), muovendo la vista o cambiando la scena riparte da solo; prima ripartiva solo
  mentre era ancora in corso e a render finito restava "Immagine non aggiornata". Nella Viewport
  non riparte in sottofondo (riparte tornando sulla vista Render); dopo Stop non riparte.
  Con una camera di render attiva (senza "Pilota") la vista si muove liberamente e il render
  resta sull'inquadratura della camera, come prima. Regola in `should_auto_restart` (test).

## 0.8.0 — 30/10/2026

**Manuale interno** (fase 3 di `docs/PIANO_INTERFACCIA_E_MANUALE.md`)
- **Guida → Manuale** (F1): finestra con indice per capitoli, pagina al centro, Home,
  Indietro/Avanti, "Vedi anche" e lingua del manuale (segue quella dell'interfaccia).
- **Ricerca offline**: titoli, parole chiave, sinonimi (`manuale/sinonimi.<lingua>.txt`), id e
  testo; ignora maiuscole e accenti, accetta prefissi ("rugos") e un errore di battitura
  ("rougness"). Tutte le parole cercate devono comparire nella pagina.
- **Aiuto contestuale**: F1 col mouse sopra un controllo apre la sua pagina (altrimenti quella
  della scheda attiva); icona del manuale accanto a Roughness, IOR e denoiser.
- Contenuti fuori dal programma, in `manuale/` (Markdown, una pagina per funzione, cartelle per
  lingua, immagini in `manuale/img/`): si aggiornano senza ricompilare. Compresa nel portable e
  nell'installer.
- Nuovo crate `forge_manual` (pagine, Markdown con `pulldown-cmark`, ricerca, controlli).
- **Controlli automatici** (test): pagine mancanti, collegamenti e immagini rotti, id ripetuti,
  pagine fuori indice, lingue incomplete, casi di ricerca del piano ("IOR", "rugosità", "grana",
  "denoiser"…), e ogni collegamento dell'interfaccia al manuale deve avere la sua pagina in
  tutte le lingue.
- Prime pagine in italiano e inglese: Benvenuto, Roughness, IOR, Denoiser, Scorciatoie (valori
  presi dal codice). Guida → Scorciatoie da tastiera apre la pagina del manuale.

**Compilazione**
- `Build-ForgeRender.ps1` compila e include anche il **convertitore STEP** (`forgerender-step.exe`,
  Open CASCADE): la prima volta richiede decine di minuti, poi resta nella cache. Serve CMake: se
  non è nel PATH lo script lo cerca (anche quello di Visual Studio); se manca o la compilazione
  fallisce, il resto prosegue e il riepilogo dice cosa fare. `-SkipStep` lo salta.

**Correzioni**
- Il simbolo "→" non è nel carattere dell'interfaccia e si vedeva un quadratino: tolto dai
  testi (un test lo controlla); nel manuale diventa la freccia delle icone.

## 0.7.0 — 30/10/2026

**Nuova interfaccia: barra dei menu e toolbar separate** (fase 2 di
`docs/PIANO_INTERFACCIA_E_MANUALE.md`)
- **Barra dei menu**: File, Modifica, Scena, Oggetti, Vista, Rendering, Libreria, Strumenti, Guida,
  con icone e scorciatoie scritte a destra. Modalità Semplice/Avanzata e lingua restano sempre a
  sinistra.
- **Toolbar delle azioni rapide** sotto i menu: Nuovo, Apri, Salva, Importa · Libreria,
  Aggiungi ▾, Luci ▾, Camera ▾, Ambiente ▾, Materiale ▾ · **Avvia render** (arancione), Stop,
  Riavvia, Esporta · a destra Controllo IA, Viewport | Render, Riavvio automatico.
- Icone dal carattere **Phosphor** (MIT, compreso nel programma, nessuna dipendenza nuova).
- **Elenco unico dei comandi** (`commands.rs`): menu, toolbar e tastiera usano gli stessi
  comandi, testi e scorciatoie; un test controlla che le scorciatoie scritte siano quelle vere.
- Le voci che aprono un pannello dell'Avanzata (Ambiente, Effetti, Camera…) dalla Semplice
  passano da sole all'Avanzata, con un avviso.

**Funzioni nuove**
- **Esporta immagine…** (Ctrl+E): un solo comando, il formato si sceglie nella finestra di
  salvataggio: PNG, JPG (qualità 92), TIFF 16 bit (per il fotoritocco), EXR (valori lineari, per
  il compositing). Anche "Salva immagine…" della Semplice e `save_image` del Controllo IA.
- **Progetti recenti** (File), gli ultimi 10, salvati nelle impostazioni; un progetto sparito
  viene tolto dall'elenco con un messaggio.
- **Esci** (File) e **conferma delle modifiche non salvate** (Salva / Non salvare / Annulla)
  prima di Nuova scena, Apri, Progetti recenti, Esci e chiusura della finestra.
- **Viste standard** (Vista e Camera ▾): frontale, retro, destra, sinistra, alto, basso;
  Isometrica e Prospettica/ortografica anche da menu.
- **Luci da menu** (Oggetti → Luce, Luci ▾): area, punto, spot, direzionale, portale.
- Scorciatoie nuove: Ctrl+N nuova scena vuota, Ctrl+Maiusc+S salva con nome, Ctrl+E esporta.
  Guida → Scorciatoie da tastiera mostra l'elenco vero.

**Correzioni**
- Maiusc+H (nascondi il resto) e Alt+H (mostra tutto) facevano la stessa cosa di H: egui ignora
  Maiusc e Alt in più, e H veniva controllato per primo. Ora le combinazioni più specifiche
  vengono lette prima (test).
- F5 ora avvia, mette in pausa e riprende come il pulsante (prima ripartiva da zero).

**Manuale (fase 1)**
- `tools/manuale/inventario.py` → `manuale/inventario.json` e `manuale/INVENTARIO.md`: i 366
  controlli dell'interfaccia trovati nei sorgenti, per zona, con un id proposto.

## 0.6.0 — 30/10/2026

**Modalità Semplice**
- Valori di base sempre attivi (nascosti nel pannello, visibili in Avanzata): arresto a soglia di
  rumore 0,050 con almeno 32 campioni; denoiser "Anteprima veloce + finale" (ripulisce anche
  durante il render), CUDA acceso (se la scheda non lo supporta il denoiser usa da solo la CPU),
  qualità finale alta e anteprima veloce, prima anteprima a 8 campioni, intensità 90 %, dettagli
  preservati. Le qualità Bozza/Bilanciata/Alta usano anche loro il denoiser in anteprima.
- **Dispositivo di calcolo** con due pulsanti, "Scheda video (GPU)" e "Processore (CPU)", al posto
  della casella. La GPU diventa predefinita appena risulta funzionante (una volta sola: poi vale la
  scelta dell'utente o di Claude).

**Correzioni**
- Fondali (curvo degli aspetti chiaro, scuro e scenografico, tela dello studio e Aggiungi →
  Fondale) larghi 12 volte il prodotto e alti 3,5 volte: prima i bordi entravano nelle
  inquadrature 16:9 e nelle viste tre quarti e isometrica.

**Statistiche**
- **Utilizzo GPU e VRAM** (usata, totale e libera) letti ogni secondo dal driver NVIDIA (NVML,
  `nvml.dll` / `libnvidia-ml.so.1`, caricata solo se c'è), per la scheda che fa il render. Con
  schede AMD o Intel la riga dice "lettura non disponibile" e il motivo. Non provato su una
  scheda NVIDIA vera in questa versione (nessuna nel container di sviluppo).

**Schermata di benvenuto**
- Nuova grafica dal mockup: render del cocktail, logo ForgeRender (`assets/logo.png`) con
  "v. 0.6.0" sotto; in piccolo, sotto le informazioni, le licenze dei componenti open source
  (Open Image Denoise Apache 2.0, librerie Rust MIT/Apache 2.0, Open CASCADE LGPL). Tolte le righe Formati e
  Interfaccia; "Mostra all'avvio" spostato in Impostazioni → Informazioni.

**Controllo IA**
- `set_render_settings` accetta `denoise_preview` (denoiser anche durante il render) e le risposte
  dicono se il denoiser è installato (`denoiser_installato`).

**Guide**
- `docs/MCP.md` e `docs/MCP.en.md` con screenshot del programma (barra nei tre stati, finestra
  comandata con legenda, registro) ed esempi renderizzati con il denoiser: i quattro aspetti, le
  viste della camera, una luce prima e dopo, materiali creati. Script per rifarle in
  `tools/guida-mcp/`.

## 0.5.0 — 30/10/2026

**Controllo IA separato dal programma**
- ForgeRender lo apre l'utente e resta aperto: Claude non lo avvia e non lo chiude più.
- Pulsante **"Controllo IA"** nella barra in alto (spento a ogni avvio): solo quando è acceso un
  assistente può comandare la finestra. **"Controllato da IA"** compare mentre è collegato.
- Nuovo eseguibile **`forgerender-mcp.exe`** (crate `forge_mcp_bridge`): è il server MCP che
  Claude avvia; inoltra i comandi alla finestra aperta su `127.0.0.1` con una chiave segreta
  nuova a ogni accensione. Con la finestra chiusa o il controllo spento risponde con un errore
  che spiega cosa fare. Compilato da `Build-ForgeRender.ps1` e incluso nei pacchetti.
- Render lunghi: `render` aspetta al massimo ~45 s, poi risponde "in corso"; la finestra finisce
  e salva da sola. Nuovo comando `render_status` (25 comandi).
- `forgerender --mcp` ora accende solo il controllo all'avvio (come `--controllo-ia`): la
  configurazione di Claude va aggiornata con il percorso di `forgerender-mcp.exe`
  (`docs/MCP.md`).
- Provato con Claude Code vero: due sessioni di seguito sulla stessa finestra (la seconda trova
  le modifiche della prima), finestra aperta dopo lo scollegamento, render di 77 s completato.

## 0.4.0 — 30/10/2026

**Controllo da Claude (MCP)**
- `forgerender --mcp`: la finestra si apre normalmente e un assistente IA (Claude Code, Claude
  Desktop) la comanda con il Model Context Protocol. Guida e configurazione: `docs/MCP.md`.
- 24 comandi: stato della scena, nuova scena, apri/salva progetto, importa modello, prepara
  studio (4 aspetti, 3 qualità), adatta la scena, annulla preset, aggiungi oggetti, sposta/ruota/
  scala, mostra/nascondi, aggiungi/modifica/elimina luci (per lato rispetto all'inquadratura,
  Kelvin, intensità relativa, raggio d'azione), colore di tutte le luci, cerca/applica/crea
  materiali, camera (viste, focale, ortografica, zoom, profondità di campo), ambiente,
  impostazioni del render, render con anteprima restituita all'assistente, stop, salva immagine.
- Sicurezza: niente codice eseguibile, argomenti controllati, nessuna sovrascrittura di file
  senza `overwrite`, materiali della libreria non modificati.
- Indicatore "IA collegata (MCP)" in alto; la verifica di `claude mcp list` apre e richiude la
  finestra da sola.

**Altro**
- I log della console vanno su stderr (stdout resta per i risultati e per MCP).
- `Build-ForgeRender.ps1` copia le DLL del denoiser anche in `target\release`, il percorso fisso
  da dare a Claude Code.

## 0.3.0 — 29/10/2026

**Foto prodotto**
- Aspetto **Studio fotografico**, scena di partenza del programma: fondale in tela inclinato
  (anche in Aggiungi → "Fondale fotografico (tela inclinata)") e quattro luci da studio fuori
  dall'inquadratura.
- **Colore delle luci** nel pannello Semplice: calda 3200 K, neutra 5500 K, fredda 7500 K o
  colore per singola luce; cambiarlo non fa diventare l'aspetto "Personalizzato".
- **Adatta la scena al prodotto**: dopo aver spostato o scalato il prodotto ridimensiona fondale e
  luci sulle sue misure (tiene materiale del fondale e colori delle luci) e inquadra; si annulla
  con "Annulla preset".
- Luci e oggetti nuovi nascono vicino alla selezione o al prodotto, non al centro della scena.

**Viewport e camera**
- **Vista ortografica e isometrica**: icona del cubo e pulsante "Iso" sotto il navigatore, tasto 5
  del tastierino; "Proiezione" Prospettica/Ortografica e "Altezza inquadratura" nella camera di
  render. Stessa immagine su CPU e GPU (test).

**Luci**
- Puntiformi e spot: **raggio d'azione** (distanza massima) e **sfumatura del bordo**,
  indipendenti dall'intensità, disegnati come una bolla nella viewport (piena = dove finisce la
  luce, tratteggiata = dove comincia la sfumatura). Spento = luce fisica come prima. CPU e GPU
  (test).

**Interfaccia**
- **Lingua italiano / inglese** con cambio immediato di tutta l'interfaccia: selettore in alto e
  Impostazioni → Lingua. Le traduzioni sono file separati `lang/Lan_ita.json` e
  `lang/Lan_eng.json` (compresi nel programma e copiati nei pacchetti); un file `Lan_*.json` nella
  cartella `lang` accanto al programma aggiunge una lingua.
- **Schermata di benvenuto** all'avvio: immagine `assets/splash.png` (render di ForgeRender, da
  sostituire), versione e commit. Resta davanti finché non si preme "Chiudi"; "Mostra all'avvio"
  la disattiva; si riapre da File → "Benvenuto e informazioni…".

## 0.2.0 — 29/10/2026

**Interfaccia**
- Modalità **Semplice / Avanzata** con selettore in alto (predefinita Semplice): "Foto prodotto in
  5 passi" con pulsanti grandi.
  - Appoggia sulla base, Inquadra il prodotto.
  - Aspetti Sfondo chiaro / Sfondo scuro / Studio scenografico.
  - **Prepara scena prodotto** e Annulla preset.
  - Qualità Bozza / Bilanciata / Alta.
  - Formati 16:9, 1:1, 4:5, 9:16.
  - Valori: `docs/INTERFACCIA_SEMPLICE.md`.
- Menu **Impostazioni**: libreria e import, cartella della libreria, valori dei nuovi progetti,
  informazioni; salvate in `impostazioni.json`.
- **Canc** elimina dal progetto anche i componenti importati, senza toccare il file;
  ripristino con Ctrl+Z o da "Eliminati dal progetto".
- Menu File: "Importa modello…" con i formati veri.

**Libreria dei materiali**
- Riconoscimento delle mappe dal suffisso del nome. Prima `metal_plate_02` perdeva colore e
  displacement e usava l'AO come metallicità.
- Materiali importati male segnalati ("⚠ da riscaricare") e riscaricati da `scarica-tutto`.
- Mappe Poly Haven solo in EXR scaricate; mappe mancanti annotate; `bump` e `rough_ao` gestite.
- EXR a un canale (AO, rugosità, height di Poly Haven) letti; colore EXR lineare corretto; height
  del displacement a 16 bit.
- Displacement spento di default all'import; "Valore proposto" nel pannello.
- File MaterialX conservati, con i loro valori visibili nel pannello; "× mappa" accanto ai cursori.
- **Salva / Salva con nome / Ripristina** dei materiali della libreria.
- Categorie e tag (Poly Haven, ambientCG) con filtro; anteprime regolabili 64–512 px.
- `forgerender-cli libreria importa-polyhaven`: libreria dai download locali di Poly Haven
  (copia o spostamento).

**Render**
- Majorant 3D della nebbia (circa 1,7–2× più veloce sulla CPU nella scena del cocktail).
- Campionamento adattivo (spento di default) e diagnostica (`--adaptive-map`).

**Pacchetto e strumenti**
- Benchmark pronti nello zip portable (`samples\…\bench_*.frproject`); comandi nel README e nel
  LEGGIMI.
- Cartella `reports/` con le misure; quaderno condiviso con ChatGPT.
- `Build-ForgeRender.ps1`: aggiornamento da GitHub e compilazione locale con un comando. La build
  di GitHub Actions parte solo a mano.

## 0.1.0

Prima versione: path tracer CPU e GPU, import glTF/GLB, OBJ, STL, STEP, SolidWorks e Blender;
libreria materiali, denoiser OIDN, camera professionale, luci, nebbia volumetrica, progetti
`.frproject`, pacchetti Windows portable e installer.
