# Novità di ForgeRender

La versione è in un solo punto: `version` in `[workspace.package]` del `Cargo.toml` alla radice.
Tutti i componenti la ereditano; il titolo della finestra mostra `ForgeRender v<versione> (<commit>)`.

## Regola per le versioni (0.x)

- **0.X.0** (minore): un gruppo di funzioni nuove consegnato. Si incrementa nel commit
  che chiude il gruppo, insieme a questa pagina.
- **0.X.Y** (correzione): solo correzioni di difetti, senza funzioni nuove.
- Ogni incremento aggiunge qui una sezione con data e commit. Il numero cambia nel `Cargo.toml`
  (e nell'installer `packaging/windows/forgerender.iss`, solo come valore di riserva).

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
