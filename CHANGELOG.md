# Novità di ForgeRender

La versione è in un solo punto: `version` in `[workspace.package]` del `Cargo.toml` alla radice.
Tutti i componenti la ereditano; il titolo della finestra mostra `ForgeRender v<versione> (<commit>)`.

## Regola per le versioni (0.x)

- **0.X.0** (minore): un gruppo di funzioni nuove consegnato a Valerio. Si incrementa nel commit
  che chiude il gruppo, insieme a questa pagina.
- **0.X.Y** (correzione): solo correzioni di difetti, senza funzioni nuove.
- Ogni incremento aggiunge qui una sezione con data e commit. Il numero cambia nel `Cargo.toml`
  (e nell'installer `packaging/windows/forgerender.iss`, solo come valore di riserva).

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
