# Piano: nuova interfaccia e manuale interno (da approvare)

Stato: **proposta**. Nessuna riga di codice è stata cambiata per questo piano.
Base dell'analisi: commit `770234c` (versione 0.6.0), branch `claude/festive-heisenberg-d2ow6z` = `main`.

Legenda usata in tutto il documento:

- **[C]** esiste già nel programma (verificato nel codice);
- **[R]** esiste, va solo spostato o riorganizzato;
- **[N]** non esiste: servirebbe una nuova implementazione (da approvare a parte);
- **[D]** da documentare nel manuale.

---

## 0. Cosa c'è oggi (analisi del codice)

### 0.1 Struttura dell'interfaccia

| Zona | File | Contenuto |
|---|---|---|
| Barra in alto (una sola riga) | `ui/toolbar.rs` (225 righe) | modalità, lingua, Controllo IA, menu File/Vista/Aggiungi, Importa, Salva, Libreria, Impostazioni, Avvia/Pausa/Riprendi, Stop, Riavvia, Esporta PNG, Viewport/Render, Modalità sicura, Riavvio automatico |
| Pannello sinistro | `ui/tree.rs` | albero della scena (componenti, visibilità), elenco luci |
| Centro | `ui/central.rs`, `gizmo.rs`, `navigator.rs` | viewport, barra strumenti Seleziona/Sposta/Ruota/Scala, Globale/Locale, Snap, Annulla/Ripeti, navigatore assi, ortografica/Iso |
| Pannello destro, Semplice | `ui/simple.rs` | "Foto prodotto in 5 passi" |
| Pannello destro, Avanzata | `ui/inspector.rs` (2468 righe) | 7 schede: Oggetto, Materiali, Luci, Ambiente, Effetti, Camera, Render |
| Barra in basso | `ui/status.rs` | avanzamento, statistiche, CPU/RAM/GPU/VRAM, denoiser, log |
| Finestre | `settings.rs`, `library.rs`, `splash.rs`, `mod.rs` (opzioni di import) | Impostazioni (5 sezioni), Libreria, Benvenuto |

Numeri reali, utili per stimare il lavoro:

- **789 testi** dell'interfaccia tradotti (`lang/Lan_eng.json`);
- **circa 230 controlli interattivi** (pulsanti, caselle, cursori, menu a tendina, campi):
  128 nel solo `inspector.rs`.

### 0.2 Scorciatoie da tastiera che esistono davvero [C]

| Tasto | Azione |
|---|---|
| Ctrl+S | Salva progetto |
| Ctrl+O | Apri progetto |
| Ctrl+I | Importa modello |
| Ctrl+Z | Annulla |
| Ctrl+Y, Ctrl+Shift+Z | Ripeti |
| Ctrl+D | Duplica selezione |
| Canc | Elimina selezione (anche componenti importati, ripristinabili) |
| Home | Inquadra tutto |
| F | Inquadra selezione |
| F5 | Avvia / pausa render |
| 5 (tastierino) | Prospettica / ortografica |
| H, Shift+H, Alt+H | Nascondi selezione, nascondi le altre, mostra tutto |
| Q, W, E, R | Seleziona, Sposta, Ruota, Scala |
| Ctrl (trascinando) | Snap |
| Alt+trascina nella viewport | Sposta la selezione invece della camera |
| Shift+trascina, tasto destro o centrale | Panoramica della camera |
| Esc | Annulla il trascinamento in corso |

Mancano oggi [N]: F1 (aiuto), Ctrl+N (nuovo), Ctrl+Shift+S (salva con nome), F12 (esporta).

### 0.3 Sistemi già pronti e riutilizzabili

- **Traduzioni** (`i18n.rs`): `tr()`, `tf()`, `tk()`, file esterni `lang/Lan_*.json`, cambio lingua
  immediato e un **test che scandisce il sorgente** e fallisce se un testo non è tradotto o se
  una traduzione non è più usata. Lo stesso principio serve per la copertura del manuale.
- **Impostazioni** (`prefs.rs`): `impostazioni.json`, applicate con `apply_prefs`.
- **Screenshot automatici**: `tools/guida-mcp/scatta.py` + `annota.py` (numeri, riquadri,
  ritagli, per lingua) e `galleria.py` (render puliti con il denoiser, comandati via MCP).
  Sono la base per le centinaia di immagini del manuale.
- **Controllo IA (MCP)**: permette di preparare scene e render in modo ripetibile per le
  immagini comparative.

### 0.4 Cosa nel mockup non corrisponde al programma reale

Il mockup è una buona direzione, ma va corretto in alcuni punti:

1. Nella scheda **Materiali** compare "Lente ottica": appartiene alla scheda **Camera**.
2. Manca la scheda **Render** tra le schede del pannello destro: va tenuta.
3. **Manca il pulsante "Controllo IA"**: va tenuto, ben visibile (scelta di Valerio).
4. La barra del titolo personalizzata (logo e pulsanti _ □ ✕ disegnati dal programma) richiede
   una finestra senza bordi di Windows. Rischi: si perdono l'aggancio ai lati dello schermo
   (Snap di Windows), il doppio clic per massimizzare e il comportamento con più monitor.
   **Proposta:** barra del titolo normale di Windows con l'icona FR (`assets/logo`), logo
   piccolo a sinistra della barra dei menu.
5. Le **icone** della toolbar: il carattere attuale di egui non le ha (già successo con ★ ↶ ▶).
   Serve un set di icone (vedi 1.5).
6. La riga "GPU: utilizzo non rilevato" del mockup è superata: dalla 0.6.0 c'è la lettura NVML.

---

## PARTE A — Interfaccia

### A.1 Nuova barra dei menu (comandi verificati)

Solo comandi esistenti, salvo quelli marcati [N] che propongo come piccole aggiunte
(ognuna da approvare).

**File**
- Nuova scena vuota [C] · Nuova scena di test [C]
- Apri progetto… Ctrl+O [C] · Progetti recenti ▸ [N, piccolo: elenco in `impostazioni.json`]
- Salva Ctrl+S [C] · Salva con nome… [C] (+ Ctrl+Shift+S [N])
- Importa modello… Ctrl+I [C]
- Esporta immagine PNG… [C] (altri formati: [N], vedi A.6)
- Esci [N, banale: chiude la finestra; con progetto non salvato chiede conferma → [N] avviso]

"Chiudi progetto" non lo propongo: senza documenti multipli equivale a "Nuova scena vuota".

**Modifica**
- Annulla Ctrl+Z [C] · Ripeti Ctrl+Y [C]
- Duplica Ctrl+D [C] · Elimina Canc [C]
- Nascondi / Nascondi le altre / Mostra tutto (H) [C]
- Impostazioni… [R da File] · Ripristina impostazioni predefinite [R da Impostazioni]

Copia/Incolla non esiste: "Duplica" copre lo stesso uso. Non lo propongo.

**Scena**
- Prepara scena prodotto [R da Semplice] · Adatta la scena al prodotto [R] · Annulla preset [R]
- Aspetto ▸ Studio fotografico / Sfondo chiaro / Sfondo scuro / Studio scenografico [R]
- Appoggia sulla base [R] · Inquadra il prodotto [R]
- Ambiente… (apre la scheda Ambiente) [R] · Effetti / Nebbia… [R]

**Oggetti** (ex "Aggiungi")
- Pavimento, Fondale da studio, Fondale fotografico (tela inclinata), Parete, Cubo, Sfera,
  Cilindro, Tazza [C] — sono tutte le primitive esistenti.
- Luce ▸ Puntiforme, Spot, Direzionale, Area, Portale [C, oggi dal pannello Luci]

**Vista**
- Inquadra tutto Home [C] · Inquadra selezione F [C]
- Prospettica / Ortografica (5) [C] · Isometrica [C]
- Viste standard ▸ Frontale, Retro, Destra, Sinistra, Alto, Basso [C, oggi solo cliccando il navigatore]
- Viewport / Render [R dalla barra]

**Rendering**
- Avvia / Pausa F5 [C] · Stop [C] · Riavvia [C]
- Dispositivo ▸ GPU / CPU [R] · Qualità ▸ Bozza / Bilanciata / Alta [R] · Formato ▸ 16:9, 1:1, 4:5, 9:16 [R]
- Impostazioni di render… (scheda Render) [R] · Denoiser… [R]
- Riavvio automatico ✓ [R dalla barra]
- Esporta immagine PNG… [C]

**Libreria**
- Apri la libreria [C] · Materiali / HDRI / Texture (apre la libreria sulla sezione) [R]
- Scarica da ambientCG / Poly Haven [C] · Importa cartella o zip [C]
- Cartella della libreria… (Impostazioni → Percorsi) [R]

**Strumenti**
- Controllo IA (on/off) [R] · Registro (log) [C, oggi nella barra in basso]
- Vista del denoiser (risultato, albedo, normali) [C, oggi nella scheda Render] · Diagnostica del campionamento adattivo [C solo nella CLI `--adaptive-map`: portarla nella GUI è [N]]

**Guida**
- Manuale F1 [N] · Guida rapida (primo render) [N] · Tutorial ▸ [N]
- Scorciatoie da tastiera [N, pagina del manuale]
- Novità della versione (CHANGELOG) [N, pagina del manuale]
- Licenze e attribuzioni [N, pagina] · Benvenuto e informazioni [R]

### A.2 Toolbar delle azioni rapide (riga sotto i menu)

Gruppi separati, icona + testo breve + suggerimento al passaggio del mouse:

| Gruppo | Pulsanti |
|---|---|
| Progetto | Nuovo · Apri · Salva · Importa |
| Contenuti | Libreria · Aggiungi ▾ · Luci ▾ · Ambiente ▾ · Materiale ▾ |
| Camera | Camera ▾ (viste standard, inquadra, ortografica) |
| Render | **Avvia render** (grande, arancione) · Stop · Riavvia · Esporta PNG |
| A destra | Viewport \| Render · Controllo IA · Riavvio automatico |

Modalità (Semplice/Avanzata) e lingua restano **sempre visibili** a sinistra, nella barra dei
menu, come nel mockup.

### A.3 Semplice e Avanzata

- I **menu sono gli stessi** nelle due modalità (un solo posto da imparare).
- Una voce che apre un controllo solo dell'Avanzata (es. Effetti/Nebbia, Portale) **passa da sola
  all'Avanzata** sulla scheda giusta e mostra un avviso breve "Passato alla modalità Avanzata".
  Alternativa: voce grigia con suggerimento. Consiglio la prima.
- Il pannello Semplice resta com'è (5 passi, valori di base fissi della 0.6.0).

### A.4 Componenti UI da toccare

- `toolbar.rs`: diviso in `menubar.rs` + `quickbar.rs`.
- Un **elenco unico dei comandi** (`commands.rs`): id stabile, testo, scorciatoia, icona, azione.
  Menu, toolbar, scorciatoie e manuale leggono tutti da lì: niente doppioni, e la scorciatoia
  scritta nel menu è sempre quella vera.
- `style.rs`: spaziature e dimensioni dei pulsanti della toolbar.
- Pannelli, viewport, statistiche: **non toccati**, salvo l'icona "?" (Parte B).

### A.5 Icone

Proposta: carattere di icone **Phosphor** tramite il crate `egui-phosphor` (licenza MIT, circa
1 MB, nessuna dipendenza pesante), se ne esiste una versione per egui 0.37 (da verificare
prima di approvare; altrimenti si incorpora direttamente il file del carattere Phosphor, MIT). È una **nuova dipendenza: da approvare**. Alternativa
senza dipendenze: icone SVG disegnate a mano e compilate nel programma (più lavoro, stesso
risultato).

### A.6 Lacune del programma emerse (nuove implementazioni, da decidere a parte)

| Lacuna | Peso | Nota |
|---|---|---|
| Progetti recenti | piccolo | elenco degli ultimi 10 in `impostazioni.json` |
| Esci con conferma se non salvato | piccolo | serve tracciare "modificato dopo il salvataggio" |
| Esporta JPG / TIFF 16 bit / EXR | medio | il crate `image` li supporta; EXR serve al fotoritocco |
| Viste standard da menu | piccolo | la logica esiste nel navigatore |
| Diagnostica adattivo nella GUI | piccolo | oggi solo `--adaptive-map` della CLI |

### A.7 Rischi di regressione

- **Scorciatoie**: spostandole nell'elenco comandi si possono rompere. → test che preme ogni
  scorciatoia su una scena di prova e controlla l'effetto.
- **Traduzioni**: tutti i testi nuovi passano da `tr()`; il test i18n esistente li controlla.
- **Semplice**: i valori di base vengono riapplicati a ogni frame; le voci di menu che li
  toccano devono passare dalle stesse funzioni (`presets.rs`).
- **MCP**: i comandi di Claude chiamano le stesse funzioni dell'app; il test del ponte esistente
  deve restare verde.
- **Spazio verticale**: due righe (menu + toolbar) al posto di una tolgono circa 40 px alla
  viewport. Toolbar compatta (icone 20 px) per contenerlo.

---

## PARTE B — Manuale interno

### B.1 Architettura (contenuti separati dal codice)

```
manuale/
  indice.toml            capitoli, ordine, id delle pagine (uguale per tutte le lingue)
  sinonimi.it.toml       "sposta" → move, "rugosità" → roughness, …
  sinonimi.en.toml
  it/
    05-materiali/
      mat.roughness.md   una pagina per funzione, nome = id stabile
      …
  en/
    05-materiali/
      mat.roughness.md
  img/
    comune/              render e confronti senza testo (uguali per le lingue)
    it/  en/             screenshot e annotazioni con testo
```

Ogni pagina è **Markdown** con un'intestazione di dati:

```
+++
id = "mat.roughness"
titolo = "Roughness (rugosità)"
parole = ["rugosità", "lucido", "opaco", "satinato"]
ui = ["inspector.materials.roughness"]   # controlli dell'interfaccia descritti
correlate = ["mat.metallic", "mat.texture.roughness"]
+++
```

Struttura della scheda (le 16 voci richieste, quando applicabili): Descrizione · Dove si trova
· A cosa serve · Come si usa (passi) · Valori ammessi · Predefinito · Effetto sulla scena e sul
render · Quando usarlo · Esempio · Limiti · Errori comuni · Vedi anche.

**I valori (intervalli, predefiniti) vengono presi dal codice**: uno script li estrae dai
`Slider::new(.., a..=b)`, `DragValue::range`, `Default` delle strutture e li confronta con
quelli scritti nelle pagine (test, vedi B.6). Così il manuale non può dire 0–2 se il cursore va
da 0 a 1.

### B.2 Dove sta il manuale nel pacchetto

- Cartella `manuale/` **accanto all'eseguibile** (portable e installer), non dentro l'exe:
  si aggiorna senza ricompilare e l'exe non cresce.
- Se la cartella manca, la finestra lo dice e indica dove metterla (niente crash).
- Dimensione stimata: 300–400 screenshot PNG ottimizzati (palette, ~40–70 KB) + ~150 render
  JPG → **circa 25–40 MB** per entrambe le lingue. Da verificare dopo il primo capitolo.

### B.3 Visualizzatore (finestra "Manuale")

- Lettura Markdown con `pulldown-cmark` (crate piccolo, puro Rust, MIT: **nuova dipendenza da
  approvare**) e disegno con egui scritto da noi: titoli, paragrafi, elenchi, tabelle, grassetto,
  codice, immagini, collegamenti interni `[IOR](mat.ior)`.
  Alternativa già pronta: `egui_commonmark` (più dipendenze, meno controllo sulle immagini).
  Consiglio la prima.
- Layout da Help Center: ricerca in alto, indice a sinistra con capitoli espandibili, pagina al
  centro, "Vedi anche" in fondo, Home, Indietro/Avanti, cronologia della sessione.
- Lingua: segue l'interfaccia; selettore nella finestra per cambiarla solo nel manuale.
- Immagini caricate solo quando la pagina si apre (avvio rapido) e tenute in cache.

### B.4 Ricerca (offline, locale)

- Indice costruito all'apertura della finestra (pochi millisecondi per ~400 pagine).
- Cerca in: titolo, id, parole chiave, sinonimi, nomi dei controlli, testo.
- Normalizzazione: minuscole, senza accenti ("opacità" = "opacita").
- Tolleranza: prefissi ("rugos" → rugosità) e un errore di battitura per parole lunghe.
- Peso: titolo > parole chiave/sinonimi > nome controllo > testo.
- Test con i casi richiesti: "IOR", "sposta", "HDRI", "denoiser" devono dare la pagina giusta
  al primo posto.

Nessun motore di ricerca esterno (tantivy & simili): non serve a questa scala.

### B.5 Aiuto contestuale

- **F1**: apre la pagina del controllo sotto il mouse; se non ce l'ha, quella della scheda o
  del pannello attivo; altrimenti l'indice.
- **Icona "?"** solo accanto ai controlli difficili (stima ~25): Displacement, Normal,
  Roughness, Metallic, IOR, Transmission, Profondità di campo, Focale, Shift, Denoiser, Soglia
  di rumore, Campionamento adattivo, GPU/CPU, HDRI, Portali, Nebbia, Raggio d'azione luci,
  Kelvin, Snap, Controllo IA…
- Suggerimenti: testo breve + riga finale "F1: guida completa".
- Tecnica: un aiuto `ui.help("mat.ior", response)` attaccato al controllo; registra l'id e
  aggiunge il suggerimento. Gli id sono quelli delle pagine.

### B.6 Verifica automatica della copertura

Test di `cargo test`, come quello delle traduzioni:

1. Ogni id `help("…")` nel codice ha una pagina **in ogni lingua**.
2. Ogni pagina con `ui = [...]` punta a controlli che esistono ancora (documentazione orfana).
3. Ogni controllo interattivo dei file `ui/*.rs` ha un `help("…")` **oppure** è in una lista
   esplicita di eccezioni motivate (es. pulsante "Chiudi"). Il controllo è un'analisi del
   sorgente: trova `button`, `checkbox`, `Slider`, `DragValue`, `ComboBox`… senza id.
4. Stessi id e stesse sezioni in tutte le lingue; nessuna pagina vuota o con segnaposto.
5. Collegamenti interni e immagini esistono; nessun id duplicato.
6. Valori e intervalli scritti nelle pagine = quelli del codice (B.1).
7. Rapporto `manuale/copertura.md` generato: controlli totali, documentati, mancanti.

Limite onesto: il test 3 vede i controlli scritti nel codice, non quelli creati in cicli con
testi variabili (es. l'elenco delle primitive): quelli si coprono con una pagina per gruppo e un
id per voce, elencati a mano.

### B.7 Capitoli e stima delle pagine

| # | Capitolo | Pagine (per lingua) |
|---|---|---|
| 1 | Introduzione (cos'è, requisiti, Semplice/Avanzata, concetti di rendering) | 8 |
| 2 | Interfaccia (ogni menu e voce, toolbar e pulsanti, viewport, pannelli, barra di stato, statistiche) | 45 |
| 3 | Progetti e import (nuovo, apri, salva, recenti, formati, unità, orientamento, opzioni di import) | 16 |
| 4 | Oggetti e trasformazioni (primitive, selezione, gizmo, globale/locale, snap, visibilità, eliminati) | 22 |
| 5 | Materiali (PBR, ogni parametro, ogni slot texture, mapping, rilievo, MaterialX, libreria) | 38 |
| 6 | Luci (5 tipi, ogni parametro, Kelvin, raggio d'azione, preset, colore di tutte le luci) | 24 |
| 7 | Camera (focale, ortografica, DOF, lamelle, shift, punta su oggetto, camere di render) | 18 |
| 8 | Ambiente ed effetti (uniforme, gradiente, HDRI, portali, nebbia) | 16 |
| 9 | Rendering (path tracing, CPU/GPU, campioni, adattivo, denoiser, tone mapping, esportazione, statistiche) | 26 |
| 10 | Libreria (cartelle, materiali, HDRI, texture, download, import, Poly Haven, ambientCG) | 16 |
| 11 | Controllo IA / MCP (dalla guida attuale, divisa in pagine) | 12 |
| 12 | Risoluzione problemi e domande frequenti | 14 |
| 13 | Scorciatoie | 1 |
| 14 | Licenze e attribuzioni | 2 |
| T | Tutorial (9, vedi B.8) | 9 |
| | **Totale** | **circa 290 per lingua, ~580 in tutto** |

### B.8 Tutorial

Primo render · Importare un modello CAD · Materiale metallico · Materiale plastico (e
differenze dal metallo) · Usare Poly Haven · Creare uno studio fotografico · Configurare la
camera · Ottimizzare il render · Usare il Controllo IA.
Ognuno con: obiettivo, cosa serve, passi numerati con screenshot, risultato atteso (render
pulito con denoiser), errori comuni, consigli.

### B.9 Immagini

- **Screenshot** (Xvfb + `scatta.py`, due lingue): stima 300–350 in totale, di cui ~60 annotati
  con numeri e legenda (numero n nell'immagine = voce n del testo, controllato dallo script).
- **Confronti** (render puliti, denoiser sempre attivo, stessa scena e stessa luce): stima 25
  serie da 3 immagini, per esempio Roughness 0/0,5/1, Metallic 0/0,5/1, IOR 1,0/1,45/2,4,
  normal map sì/no, displacement sì/no, focale 24/50/135 mm, DOF sì/no, Kelvin 3200/5500/7500,
  raggio d'azione sì/no, denoiser sì/no, i 4 aspetti, HDRI diversi, nebbia, campioni
  32/256/1024.
- Tempo di render nel container (CPU): ~3 min per immagine a 512 campioni → circa 4 ore in
  tutto, in background.
- Le immagini senza testo (render) sono **comuni alle lingue**; quelle con testo si rifanno per
  ogni lingua dallo stesso script.

---

## PARTE C — Lingue

- Stessa struttura di cartelle e stessi id per ogni lingua; `indice.toml` unico.
- Una lingua nuova = una cartella `manuale/<codice>/` + `sinonimi.<codice>.toml` + i suoi
  screenshot (lo script li rifà con l'interfaccia in quella lingua).
- Il test B.6 punto 4 blocca pagine mancanti, vuote o con lingua mista (controllo di parole
  tipiche italiane nelle pagine inglesi e viceversa).
- Regola di lavoro: **ogni capitolo si consegna nelle due lingue insieme**, mai uno solo.

---

## PARTE D — Fasi, dipendenze, verifiche

| Fase | Contenuto | Dipende da | Versione | Verifica |
|---|---|---|---|---|
| 1 | Inventario automatico dei controlli (`manuale/inventario.json`: finestra, pannello, scheda, testo, tipo, id proposto) | — | — | inventario rivisto da Valerio |
| 2 | Elenco comandi + barra dei menu + toolbar con icone | approvazione di A, icone | 0.7.0 | test scorciatoie, test i18n, screenshot Semplice/Avanzata, prova MCP |
| 3 | Visualizzatore del manuale, ricerca, F1, "?", test di copertura (con 3 pagine di prova) | fase 2 | 0.8.0 | test ricerca, test collegamenti, test copertura |
| 4 | Contenuti capitolo per capitolo, IT+EN, screenshot per capitolo | fase 3 (e interfaccia stabile) | 0.8.x | test copertura in crescita, controllo visivo delle immagini |
| 5 | Tutorial e immagini comparative | fase 4 | 0.9.0 | ogni tutorial rifatto da zero seguendo i passi |
| 6 | Pacchetto portable/installer con `manuale/`, copertura 100 % | tutte | 1.0.0? | prova del pacchetto offline |

Perché quest'ordine: gli screenshot vanno fatti **dopo** il cambio dell'interfaccia, altrimenti
si rifanno tutti.

**Stima realistica del lavoro**: la fase 2 è una o due sessioni; la 3 una o due; la 4 è la più
grande (circa 580 pagine e 300 immagini): da fare a blocchi di 2–3 capitoli per sessione.

### Criteri di accettazione (verificabili)

1. Menu e toolbar separati; ogni comando di oggi raggiungibile (lista di controllo dall'inventario).
2. Tutte le scorciatoie della tabella 0.2 funzionano (test).
3. Semplice e Avanzata: stessi menu, nessuna funzione persa (screenshot + test).
4. Manuale da Guida e da F1, offline, IT ed EN completi (test 1–6 verdi).
5. Ricerca: i casi IOR, sposta, HDRI, denoiser trovano la pagina giusta al primo posto (test).
6. Copertura: rapporto al 100 % o eccezioni motivate in elenco.
7. Immagini: ogni immagine referenziata esiste; i render sono denoisati; le annotazioni
   corrispondono alla legenda.

## Decisioni chieste a Valerio

1. Barra del titolo: normale di Windows (consigliata) o personalizzata come nel mockup?
2. Icone: crate `egui-phosphor` (consigliato) o SVG fatte a mano?
3. Manuale in Markdown con `pulldown-cmark` (consigliato) o `egui_commonmark`?
4. Voci Avanzata dalla Semplice: passaggio automatico (consigliato) o voci grigie?
5. Lacune A.6: quali implementare (consiglio: recenti, Esci con conferma, viste standard,
   esportazione JPG/TIFF/EXR)?
6. Si parte dalla fase 1 (inventario) appena approvato?
