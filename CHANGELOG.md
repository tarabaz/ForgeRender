# Novità di ForgeRender

🇬🇧 [Read the changelog in English](CHANGELOG.en.md) (versione di riferimento)

La versione è in un solo punto: `version` in `[workspace.package]` del `Cargo.toml` alla radice.
Tutti i componenti la ereditano; il titolo della finestra mostra `ForgeRender v<versione> (<commit>)`.

## Regola per le versioni (0.x)

- **0.X.0** (minore): un gruppo di funzioni nuove consegnato. Si incrementa nel commit
  che chiude il gruppo, insieme a questa pagina.
- **0.X.Y** (correzione): solo correzioni di difetti, senza funzioni nuove.
- Ogni incremento aggiunge qui una sezione con data e commit. Il numero cambia nel `Cargo.toml`
  (e nell'installer `packaging/windows/forgerender.iss`, solo come valore di riserva).

## 0.52.0 — 9 nov 2026

- **L'IA gestisce gli effetti:** quattro comandi MCP nuovi. `list_effects` restituisce ogni effetto della scena con tutti i suoi valori (i nomi dei campi salvati nel progetto) e cosa si può aggiungere; `add_effect` aggiunge foschia d'ambiente (5 tipi), fumo volumetrico (9 preset), ghiaccio secco (5 preset), polvere in sospensione, lucciole, oppure un adesivo da un'immagine sui pezzi scelti, esattamente come i pulsanti della scheda Effetti; `edit_effect` cambia solo i valori indicati (campi sconosciuti e tipi sbagliati si rifiutano, i valori fuori intervallo si riportano nei limiti come nel pannello), accende, spegne o rinomina l'effetto; `remove_effect` lo elimina. Anche `scene_info` elenca gli effetti.
- **Laser dall'IA:** `add_light` e `edit_light` gestiscono il laser (rosso, verde, blu, viola o bianco, diametro, riflessi parziali, fascio visibile).
- **Guida e manuale:** la guida MCP (comandi ed esempi) e la pagina del controllo con l'IA descrivono gli effetti, in inglese e in italiano.

## 0.51.0 — 9 nov 2026

- **Sfondo della viewport a scelta:** la viewport parte ora su **Antracite** (RGB 40, 44, 52, #282C34), un grigio scuro leggermente freddo che lascia ben leggibili anche gli oggetti neri o molto scuri. **Impostazioni → Viewport** propone Antracite (predefinito), Grigio Blender-like (#30343B), Scuro tecnico (#222831), Grigio chiaro (#353A42) e **Personalizzato** con il selettore colore del programma (RGB, HEX, CMYK, palette). Il colore cambia subito la viewport e l'anteprima STEP, si salva con le impostazioni e resta dopo il riavvio; il render finale non cambia.
- **Render apre la scheda Render:** avviando o riprendendo un render da qualunque punto (pulsante Avvia render, F5, menu Rendering, pannello della modalità Semplice, scheda della demo) l'area centrale passa prima di tutto alla scheda **Render**, anche mentre il render aspetta la geometria o le nebbie, così si vede subito che è partito. Mettere in pausa un render in corso lascia la vista dov'è.
- **Luci dell'ambiente:** le luci caricate da un ambiente (Studio fotografico, Sfondo chiaro, Sfondo scuro, Studio scenografico) appartengono all'ambiente e nell'albero della scena hanno **[ENV]** dopo il nome. Il primo ambiente spegne le luci già presenti senza eliminarle né cambiarle. Cambiando ambiente si eliminano solo le luci dell'ambiente precedente e si creano da zero quelle nuove, con tutti i valori del nuovo ambiente: nessun residuo e nessun doppione, anche passando da molte luci a poche e ricaricando lo stesso ambiente. Le tue luci non si eliminano né si modificano mai, anche se hanno lo stesso nome: si riconoscono da un segno interno salvato nel progetto, mai dal nome o dal suffisso.

## 0.50.1 — 9 nov 2026

- **Anteprima STEP come la viewport:** la finestra di importazione STEP non disegna più un'anteprima sua approssimata (meno triangoli, facce mancanti, niente profondità). Inserisce il modello convertito in una scena vuota esattamente come fa Importa (stessi pezzi, colori, asse verticale e appoggio sul pavimento) e lo disegna con il motore della viewport: stessa ombreggiatura, antialiasing e profondità. **Ombreggiata** o **Fil di ferro** (gli spigoli dei triangoli della vista Mesh della viewport); il mouse gira, sposta e ingrandisce come nella viewport, il doppio clic o **Inquadra** inquadra di nuovo il modello. Cambiando l'asse verticale o l'appoggio sul pavimento l'anteprima si aggiorna senza convertire di nuovo.
- **Importa carica il modello dell'anteprima:** Importa mette nel progetto proprio il modello convertito che si vede nell'anteprima, senza rifare la conversione.

## 0.50.0 — 8 nov 2026

- **Due righe in alto:** il selettore Semplice / Avanzata e un ingranaggio per le Impostazioni sono a destra nella barra dei menu; il selettore della lingua non è più in alto (sta in Impostazioni → Lingua). Nella barra degli strumenti Avvia render, Stop, Riavvia ed Esporta stanno a destra accanto ad AI MCP; le linguette Viewport | Render e Riavvio automatico sono subito sopra l'area centrale.
- **Pannello di destra in tre schede:** **Selezione** mostra ciò che è scelto nella viewport o nell'albero (un oggetto con posizione, misure e materiale, oppure una luce, una camera o un effetto); **Scena** raccoglie Luci, Ambiente, Effetti, Camera e Materiali come sotto-schede; **Render** tiene le impostazioni del calcolo, con roulette russa, limite della luce indiretta, seme casuale e thread della CPU in una sezione **Avanzate** chiusa. Un clic su qualunque cosa nella scena apre Selezione.
- **Un solo albero della scena:** gli oggetti, poi i gruppi Luci, Camere ed Effetti (nebbie, fumi, polvere, foschia, lucciole, adesivi), ognuno apribile e chiudibile, con un + per aggiungerne uno.
- **Materiali:** le proprietà del materiale vengono subito dopo i materiali della scena e i pulsanti di assegnazione; sostituisci, assegna per nome, materiale predefinito, nuovo, duplica, elimina e salva nella libreria sono nella sezione **Gestione materiali**, chiusa. I cursori hanno nomi italiani con il termine tecnico tra parentesi.
- **Parte bassa:** timeline e console restano ridotte a una riga finché la timeline non ha qualcosa; **Riduci** le richiude. Backend e processore occupano una riga sola.
- **Leggibilità:** testo piccolo da 9 a 12 punti, testi e note più chiari; arancione solo per Avvia render, l'avanzamento del render e ciò che è selezionato nella scena; la guida ai tasti del gizmo è in una sezione chiusa. L'avviso del denoiser mancante ha un pulsante che apre la pagina del manuale con il rimedio.
- **Manuale:** le pagine su finestra, materiali, luci, ambiente, effetti, camera e rendering descrivono le schede nuove (inglese e italiano).

## 0.49.0 — 8 nov 2026

- **Il manuale interno è completo:** Guida → Manuale (F1) ha ora 33 pagine in 12 capitoli, in inglese e in italiano: introduzione e primo render, modalità Semplice, finestra, viewport e impostazioni, importazione, scheda Oggetto e progetti, luci, materiali, ambiente, effetti, camera, rendering, salvataggio delle immagini, viste e turntable, timeline e video, controllo con l'IA e scorciatoie da tastiera. Ogni pagina dice dove sta il controllo, a cosa serve, intervallo e valore predefinito (presi dal codice del programma) e gli errori comuni. F1 sopra una scheda apre la pagina di quella scheda; in modalità Semplice apre la pagina della modalità Semplice.
- **Immagini di confronto:** le impostazioni principali mostrano la stessa scena calcolata con il valore basso, medio e alto, una accanto all'altra: campioni (16, 64, 256, 1024), soglia di rumore, denoiser spento e acceso, rimbalzi massimi, esposizione, tone mapping, caustiche, stili del render e contorni, roughness, metallic, specular, IOR, transmission, dimensione della luce, temperatura colore, intensità dell'ambiente, focale, diaframma e densità della foschia. 56 immagini, 640 × 480, calcolate da ForgeRender sulla GPU con almeno 256 campioni (tranne i confronti su campioni e soglia di rumore), sulla scena di prova e sulla sfera delle anteprime dei materiali.
- **Immagini su una riga diventano una galleria:** nelle pagine del manuale più immagini nello stesso paragrafo si disegnano una accanto all'altra con le loro didascalie (su più righe se la finestra è stretta); passando col mouse sopra un'immagine rimpicciolita la si vede a grandezza naturale.
- **Immagini anche nel repository:** `docs/manual/renders/` ha gli stessi file, con il nome impostazione_cursore_valore (per esempio `render_samples_064.png`); `tools/manuale/render_confronti.js` li rifà con `forgerender-cli`.
- **Test nuovi:** ogni immagine di confronto è usata da una pagina e la sua copia in `docs/manual/renders/` è identica; la ricerca trova le pagine nuove con le parole di tutti i giorni (campioni, diaframma, kelvin, caustiche, focale, esposizione, rimbalzi, metallicità). Nuovi sinonimi per la ricerca nelle due lingue.

## 0.48.4 — 7 nov 2026

- **Fumo da una sorgente appoggiata sul pavimento:** quando tutte le celle nel raggio della sorgente erano occupate da un oggetto, o la sorgente cadeva tra i centri delle celle, non usciva niente. Capitava soprattutto nell'anteprima della viewport con riquadri grandi, dove le celle sono più grosse: l'anteprima restava vuota mentre il render il fumo lo faceva. Ora la sorgente emette dalle celle libere più vicine, al più due celle più in là. I fumi che già uscivano non cambiano.

## 0.48.3 — 6 nov 2026

- **Barra di stato ferma:** ogni contatore ha ora il suo spazio, largo quanto il suo valore più lungo, così i numeri cambiano senza spingere quelli dopo. Anche la scritta dello stato non sposta più la barra di avanzamento.
- **Anteprima del fumo al passo con gli oggetti animati:** premendo play poteva restare un'anteprima calcolata prima di animare un oggetto, e un fumo "solo anteprima" non selezionato non veniva più ricalcolato: il fumo sembrava non vedere l'oggetto che si muoveva. Ora in entrambi i casi si ricalcola.

## 0.48.2 — 6 nov 2026

- **Tolta la voce RAM processo dalla barra di stato:** era la lettura lenta dietro lo scatto corretto nella 0.48.1, e nient'altro la usava. RAM di sistema resta. Le impostazioni salvate con quella voce continuano a funzionare: viene saltata e tutto il resto si conserva.

## 0.48.1 — 6 nov 2026

- **Corretto lo scatto regolare della viewport:** una volta al secondo il programma leggeva la propria memoria per la barra di stato, e quella lettura fermava l'interfaccia per 200–300 ms, con qualsiasi scena. Le letture di CPU, memoria e GPU ora stanno su un thread separato e l'interfaccia non le aspetta mai.

## 0.48.0 — 6 nov 2026

Dalla sezione 9 di `reports/chatgpt/2026-11-06_modifiche.md`.

- **Le esportazioni non si fermano più con la finestra ridotta a icona:** il passaggio da un fotogramma all'altro stava dentro il disegno dell'interfaccia, che con la finestra ridotta non viene eseguito, quindi l'esportazione aspettava che la finestra tornasse. Ora sta nella logica del programma, che continua senza disegnare. Una finestra solo coperta non si era mai fermata.
- **Il PC resta sveglio durante un'esportazione:** la sospensione automatica è bloccata per la durata del lavoro; lo schermo può spegnersi e uno spegnimento chiesto da te non viene impedito.
- **Lascia il PC usabile:** una scelta nuova nelle impostazioni video abbassa la priorità del programma, ffmpeg compreso, e mette pause sulla GPU. Di partenza resta Massima velocità, e la scelta vale dal fotogramma dopo anche a esportazione in corso.
- **Riprendere un'esportazione interrotta:** dopo un crash, un video non riuscito o un'interruzione, riavviando lo stesso lavoro si può continuare dai fotogrammi fatti o ricominciare. Una scheda nella cartella dei fotogrammi confronta progetto, impostazioni di render, intervallo e numero di fotogrammi, così fotogrammi di lavori diversi non si mescolano mai.
- **Chiudere durante un'esportazione chiede prima:** continui a esportare oppure interrompi ed esci, anche da finestra ridotta, che torna davanti per mostrare la domanda. Anche le modifiche non salvate ora vengono chieste chiudendo da finestra ridotta.
- **Esiti onesti:** un'animazione interrotta o fallita non viene più annunciata come finita, e un video che non si è potuto creare viene segnalato come errore.
- **Log più chiaro:** ogni fotogramma, l'avvio della codifica, finestra nascosta e di nuovo visibile, pausa, ripresa e interruzione vengono scritti con il loro orario.
- **Corretto:** chiudere o salvare da ridotta a icona registrava una finestra da 64 × 64, e all'avvio dopo si apriva una finestra minuscola. Ora resta l'ultima dimensione buona, e una dimensione minuscola già salvata viene riparata all'avvio.

## 0.47.0 — 6 nov 2026

Il resto di `reports/chatgpt/2026-11-06_modifiche.md`.

- **Lucciole / Bosco incantato:** un effetto nuovo con uno stormo che vola soprattutto in orizzontale e si accende a intervalli, ogni lucciola per conto suo. Le lucciole accese sono luci vere della scena, fermate dagli oggetti, su CPU e GPU; oltre il massimo scelto quelle vicine si uniscono in una luce con la stessa potenza. Corpo e alone si regolano a parte dalla luce sulla scena. Palette con percentuali, variazione tonale nella stessa famiglia di colore, ciclo completo di accensione e spegnimento, volume con rotazione. Tutto si calcola da tempo e seme: Play, salto a un fotogramma, viewport e render coincidono.
- **La polvere in sospensione si muove:** i granelli fluttuano piano lungo la timeline anche senza niente di animato, come funzione continua del tempo: niente tremolii, e il salto a un fotogramma dà gli stessi granelli della riproduzione. Gli oggetti in movimento spingono la polvere vicina e la tengono fuori, le correnti del fumo la sollevano, e la polvere non agisce mai sul fumo. Comandi nuovi: Moto nel tempo e Influenza dell'aria.
- **L'anteprima del fumo sente gli oggetti animati:** le sue particelle li ignoravano perché l'anteprima simulava con gli oggetti fermi all'istante del cursore; ora usa gli stessi solidi in movimento del precalcolo.
- **Griglia finale del fumo su file:** una volta calcolata, la griglia alla risoluzione del render di ogni fotogramma viene salvata e riletta: rifare il render di un'animazione non la ricostruisce né la ricampiona.
- **Precalcolo del fumo più solido:** i file hanno una versione del formato e una lunghezza controllata, un fotogramma interrotto o rovinato si rifà invece di essere usato, la scrittura è atomica e la cartella resta sotto i 40 GB.
- **Finestra di importazione STEP:** tre colonne per il file, la tassellazione e un'anteprima 3D. Bozza, Bilanciata, Alta qualità o Personalizzata corrispondono alle deviazioni lineare e angolare vere del convertitore; si può riconvertire senza riaprire il file, fermare una conversione, ruotare e ingrandire il risultato ombreggiato o in fil di ferro, leggere triangoli, vertici, dimensioni e pezzi, e importare solo quando va bene. Più file insieme tengono la finestra di prima.

## 0.46.0 — 6 nov 2026

Da `reports/chatgpt/2026-11-06_modifiche.md`.

- **Impostazioni di esportazione video:** una finestra nuova sceglie contenitore (MP4, MOV, MKV, AVI), codec (H.264, H.265, ProRes, FFV1, RGB non compresso), CRF o bitrate, formato dei pixel, frequenza, risoluzione e file di destinazione, con sei preset e una stima della dimensione. Senza ffmpeg accanto al programma o nel PATH restano MP4 H.264 e AVI non compresso, e le altre scelte sono disattivate. L'ultima configurazione viene ricordata.
- **AVI non compresso senza strumenti esterni:** lo scrive ForgeRender, con gli indici OpenDML, così non si ferma al vecchio limite dei 2 GB.
- **File video affidabili:** il file prende il nome finale solo a lavoro finito, quindi un errore, il disco pieno o un'interruzione non lasciano niente a metà; la codifica si può interrompere; l'avviso riporta contenitore, codec, formato dei pixel, risoluzione, frequenza, dimensione e bitrate effettivi.
- **Contatori della barra di stato:** Impostazioni → Barra di stato sceglie fino a 9 contatori su 15; Backend e processore restano sempre all'estrema destra e la riga va a capo se serve. Nuovi: FPS viewport (misurati davvero), FPS video, fotogrammi fatti, RAM, GPU e VRAM separate.
- **Tempo rimanente:** mentre si crea un video la barra mostra la stima dei fotogrammi e poi della codifica, con "Calcolo stima…" finché non ci sono dati veri; per un'immagine sola viene dai campioni.
- **Puntare Spot e Direzionali dal viewport:** il bersaglio ha la sua maniglia (il rombo), la sorgente resta ferma e i campi numerici restano allineati.
- **Annulla per il gizmo delle luci:** ora si annulla anche il trascinamento di una luce, insieme all'eventuale chiave registrata.
- **Barra superiore uniforme:** tutti i controlli hanno la stessa altezza, gli stessi bordi e lo stesso allineamento, con icona e testo su una riga; il colore d'accento distingue l'azione principale, non la misura. I gruppi che entrano si misurano sui testi veri: vale per ogni lingua e scala.

## 0.45.0 — 6 nov 2026

Correzioni di quello che il beta test aveva lasciato aperto; dettagli in `reports/chatgpt/2026-11-06_betatest-0.44.md`, sezione 8.

- **Luci e ghiaccio secco hanno un numero stabile** nella timeline al posto del nome: due luci con lo stesso nome restano distinte e rinominarne una non perde più l'animazione. I progetti delle versioni precedenti si convertono all'apertura (con due luci omonime l'animazione resta sulla prima, quella che muoveva prima). Le luci duplicate prendono un numero loro e una luce nuova non eredita mai l'animazione di una eliminata. Cambiando aspetto, le luci con lo stesso nome tengono il numero.
- **Annulla e Ripeti coprono la timeline:** chiavi, modificatori, area di lavoro, durata, inizio e fine degli effetti. Trascinare una chiave è un passo solo, e spostare un oggetto animato con la sua chiave nuova è un passo solo anche quello.
- **Avvisi al posto delle finestre:** dopo il salvataggio di un'immagine, di un'animazione o dei passaggi di render compare un avviso in alto con **Apri cartella**, che non blocca più la barra. Resta finché ci si passa sopra con il mouse e sparisce da solo. L'avviso di un pezzo eliminato sparisce con Annulla. Le finestre restano per gli avvisi di importazione, gli errori e le domande.
- **Sorgente del fumo dentro un oggetto:** il controllo si fa ora sulla griglia vera appena il fumo cambia, anche con l'oggetto tagliato dal riquadro, e non serve più che sia partito un calcolo, quindi l'avviso compare anche dopo un riavvio. I punti dell'anteprima spariscono quando il render non mostrerebbe niente. Un fumo nuovo nato dentro un oggetto alza da solo la sorgente sopra di esso; su un fumo già sistemato lo fa con un clic il pulsante **Sposta la sorgente sopra l'oggetto**.
- **Pannelli:** la barra della timeline va a capo in una finestra stretta, così nessun comando esce. Le misure del render stanno in una riga con un + per vederle tutte, e alla timeline restano più righe. L'albero della scena tiene sempre titolo e qualche riga, anche con i pannelli Luci e Camere alti. Una riga lunga nel pannello laterale scorre invece di allargare il pannello.
- **Elenco materiali:** le copie fatte per l'opacità animata non si vedono se c'è il materiale originale, e non hanno più "(opacità animata)" nel nome.
- Controllato e non è un difetto: la memoria col fumo resta tra 920 e 1.004 MB in 5 minuti di Play continuo nel Render (la crescita vista nel beta test era la cache che si riempie, limitata a 17 nubi). Un pavimento eliminato nella scena di prova resta eliminato dopo salvataggio e riapertura: non riprodotto, ora lo controlla un test.

## 0.44.1 — 06/11/2026

Correzioni dal beta test; report completo in `reports/chatgpt/2026-11-06_betatest-0.44.md`.

- Annulla e Ripeti si svuotano quando si apre un altro progetto: le voci usano gli indici dei nodi e, su una scena con lo stesso numero di nodi, avrebbero toccato i pezzi sbagliati.
- Copia e incolla delle chiavi della timeline funzionano (egui trasforma Ctrl+C e Ctrl+V negli eventi Copy e Paste; Copy scrive anche un breve testo negli appunti di sistema, perché Paste arriva solo se negli appunti c'è qualcosa).
- Avviso nel pannello del fumo e in console quando la sorgente è dentro un oggetto con le collisioni accese: il fumo resta chiuso lì e nel render non si vede.
- Fumo simulato e ghiaccio secco hanno la loro riga nella timeline appena creati. Un effetto nuovo sposta il cursore solo se non c'è ancora un'animazione; togliendo un effetto si tolgono le sue righe.
- Il Play nella vista Render mostra solo fotogrammi finiti.
- Righello della timeline con due decimali sotto il mezzo secondo. Il menu Salva animazione resta aperto mentre si sceglie e Salva si distingue.
- Pannello in basso limitato al 60 % dell'altezza, anche quella salvata. Con la finestra stretta la barra toglie prima i comandi dei file, poi quelli per aggiungere, e tiene interi quelli del render.
- Testi: singolare per un componente eliminato, etichetta dell'anteprima del fumo non più sopra la riga del backend.

## 0.44.0 — 06/11/2026

**Fumo calcolato una volta sulla timeline**
- `forge_render_core::smoke_bake`: la simulazione gira una volta sulla durata e lascia nella cartella temporanea un file sparso a 8 bit per fotogramma e lo stato completo (`SimState`) una volta al secondo; ogni fotogramma ha un'impronta cumulativa (oggetti dentro il riquadro, emissione accesa o spenta) e una modifica riparte dall'ultimo stato prima del primo fotogramma diverso. Test `only_what_changed_is_computed_again`, `resuming_from_a_state_gives_the_same_smoke` (identico al bit, griglia e particelle). Su una scena di prova: 60 fotogrammi 6,3 s, un altro fotogramma 1 s, modifica dal secondo 1 ripartita dal fotogramma 25, 176 MB.
- `simulate_bake`: passi agganciati ai fotogrammi, indipendenti dalla durata.
- Anteprima del fumo nella viewport su tutta la durata, mostrata all'istante del cursore invece di ricalcolarla.

**Spostamento d'aria**
- `SmokeSim::air_push` (acceso di serie): gli oggetti animati si voxelizzano una volta nel loro spazio, si riportano nella griglia con la loro matrice a ogni passo e fanno da bordi in movimento (u·n = u_solido·n nella proiezione, l'aria nelle celle solide si muove con loro, il fumo viene spinto fuori dalle celle in cui entrano). Test `a_moving_solid_pushes_the_air`.

**Inizio e fine dell'emissione**
- `Timeline::stops`, `SmokeSim::emit_stop`, `FogVolume::emit_stop`: dopo lo stop la sorgente è spenta e quello che è uscito continua a vivere. Test `emission_stops_but_the_smoke_lives_on`.

**Timeline**
- Sempre attiva (menu Funzioni tolto), schede Timeline | Console; righe per posizione, rotazione, bersaglio, colore e scala per asse (`Target::rows`): una chiave va su tutti gli assi della riga, anche se registrata dal gizmo.
- Selezione delle chiavi: clic, Ctrl/Maiusc, rettangolo, trascinamento di gruppo, Canc, Ctrl+C/V, F9, Esc (`Timeline::shift_keys`, `settle_keys`, `delete_keys`, `set_keys_interp`, `copy_keys`, `paste_keys`). Lettere X Y Z sui modificatori.
- Barra dell'area di lavoro (`work_in`, `work_out`); Salva animazione: render di ogni fotogramma o anteprima della viewport (`ViewportRenderer::capture`), dalla viewport o da una camera, in una cartella nuova dentro Rendering, con MP4 facoltativo; Play nella vista Render con i campioni per fotogramma regolabili.
- Comandi del tempo nascosti nel pannello del fumo (`SHOW_TIME_CONTROLS`).

**Correzioni e prestazioni**
- Viewport: vertici nello spazio dell'oggetto e matrici in uno storage buffer letto dal vertex shader; `transforms_changed` aggiorna solo le matrici (circa 185 fps in riproduzione sulla scena di prova da 100.000 triangoli).
- "Costruzione BVH" senza fine: il Play nella vista Render inseguiva una revisione della geometria che cambiava a ogni fotogramma (ora il Play nel Render avanza quando il fotogramma è finito) e un valore limitato dalla scena (scala 0, opacità oltre il 100 %) si riscriveva a ogni fotogramma (test `a_clamped_value_settles`).
- `prefs::rendering_dir`: `Rendering` accanto al programma, o in Documenti\ForgeRender se non è scrivibile, di serie per salvare ed esportare.

## 0.43.0 — 05/11/2026

**Timeline (animazione)**
- `forge_scene::timeline`: `Timeline` (durata, fps, cursore, elementi, tracce, inizi degli effetti) salvata nel progetto; `Track` con chiavi (`Interp`: a scatto, lineare, morbida in entrata, in uscita, in entrata e uscita, curva continua Catmull-Rom), valore di base e modificatori (`Modifier`: tremolio, oscillazione, rampa, ripeti, avanti e indietro, spostamento, moltiplica, limiti, a scatti); `Target` (oggetto, luce, camera, fumo, nebbia, foschia, polvere, ambiente) e `Prop` con proprietà di base e compatibili; `Timeline::apply` scrive i valori nella scena e registra come chiave (o come valore di base) una proprietà cambiata dall'utente dall'ultima applicazione; fumo simulato e ghiaccio secco seguono il cursore (`effect_time` = cursore − inizio). Test `interpolations_pass_through_the_keys`, `modifiers_change_the_value`, `apply_moves_objects_and_records_user_changes`, `effects_follow_the_playhead`.
- App: menu Funzioni con Timeline (animazione), spenta di serie, accesa da sola quando arriva un effetto nel tempo; parte bassa con le schede Console | Timeline; finestra staccabile; riproduzione, modifica di chiavi e modificatori, render dell'animazione in fotogrammi PNG e MP4 (`ui/timeline.rs`); "Istante" del fumo che segue la timeline; opzioni di avvio `--timeline` e `--timeline-demo`.
- Opacità degli oggetti: `Material::opacity` (0–1), `MaterialParams::opacity`; `opaque_at` su CPU e GPU lascia passare un raggio con probabilità 1 − opacità (parola 36 del materiale, `MAT_WORDS` 37); la timeline dà all'oggetto una copia del suo materiale.

**Fumo a particelle**
- `SmokeSim::particles` e `patchiness`: la griglia calcola solo il moto dell'aria; circa 2,4 M particelle (trasporto al punto medio, dissolvenza e bordi come il fumo) si depositano alla fine su una griglia fino a 3 volte più fitta con una sfocatura 1-2-1 (`splat_particles`); il render usa il campo in proporzione (niente soglie né rumore procedurale), con il contrasto. Test `particle_smoke_stays_thin`.
- Preset: *Vapore da tazza* rifatto a particelle (sorgente a chiazze, poco calore, poca densità) e nuovo *Incenso o sigaretta* (sorgente di 2 mm, uscita calda e veloce, poca dissolvenza, contrasto 1,8).
- CLI `--patch F.json` (camera, ambiente, impostazioni, luci, fumi, polvere, materiali) per le prove senza ricompilare.

**Polvere luminosa**
- Alone delle lucciole: `Dust::halo_size` (in diametri del granello, 6 di serie) e `halo_strength`; `RenderScene::glow_points` (centro, diametro e luce di ogni granello luminoso, dai suoi vertici); `forge_render_core::glow`: all'avvio del render i granelli si proiettano con la camera del render (prospettica o ortografica, con il decentramento) e si scartano quelli coperti da un oggetto; `Frame::display` aggiunge una gaussiana del nucleo più una coda tre volte più larga, che va a zero a 9σ, prima del tilt-shift. Test `glow_adds_a_soft_disc_around_the_point`.
- `Dust::glow` (`DustGlow`: colore, intensità, parte): i granelli di ogni colore vanno in un nodo `effetto:polvere:luce:<n>` con un materiale emissivo; `is_any_dust_id`; test `glowing_dust_gets_its_own_emissive_nodes`. Pannello Polvere: "Granelli luminosi" con +.

**Interfaccia**
- Barra rapida: il gruppo di sinistra si taglia allo spazio lasciato da quello di destra (niente sovrapposizioni); AI MCP in fondo dopo un divisore, interruttore grande con icona e stato e ingranaggio piccolo sotto; Riavvio automatico su due righe.
- Libreria ed editor del materiale: riaprendoli la finestra staccata torna davanti (`ViewportCommand::Focus`) o quella interna va sopra le altre.

## 0.42.1 — 05/11/2026

**Selettore colore**
- Il quadrato saturazione/valore e la barra della tinta del selettore colore erano vuoti (si vedevano solo il cerchietto e il triangolo): egui 0.37 li disegna con una texture creata al volo, che nel programma non compariva. Ora sono disegnati in `color_picker.rs` con triangoli colorati ai vertici, senza texture.
- Ogni pulsante colore (luci, ambiente, foschia, fumo, polvere, stili di rendering, importazione da Blender) apre lo stesso selettore dei materiali (RGB, CMYK, palette RAL) al posto di quello di egui (`color_picker::rgb_edit`); lì il codice della palette non si conserva, resta solo il colore.

**La nebbia non riparte girando la camera**
- L'impronta che decide se rifare nebbia, fumo e foschia comprendeva la camera di render e le luci anche a foschia spenta, così girare la vista durante una simulazione di ghiaccio secco o di fumo la annullava e la faceva ripartire da zero. Ora camera e luci contano solo con la foschia accesa, e se cambia solo ciò che decide la foschia il calcolo in corso si ferma solo se sta calcolando la foschia: una simulazione di ghiaccio secco o di fumo finisce, poi si rifà solo la foschia.

## 0.42.0 — 05/11/2026

**Sostituisci materiale**
- `Scene::replace_material(da, a)`: ogni assegnazione di `da` (quindi anche i componenti che lo ereditano dal genitore) e il predefinito della scena passano ad `a`; i nodi di polvere e adesivi restano com'erano (test `replace_material_changes_every_user_and_the_default`, 100 oggetti). Pannello Materiali: "Sostituisci [elenco] con «selezionato» (N)" e clic destro su un materiale della scena → "Sostituisci con «selezionato»"; "Annulla sostituzione" rimette le assegnazioni di prima dell'ultima sostituzione.

**Polvere: due colori, opacità, variazione della dimensione, scaglie**
- `Dust::color2` e `color2_share`: i granelli del secondo colore vanno in un secondo nodo e materiale (`effetto:polvere:2`, `effects::is_dust_id`), scelti con un generatore a parte così i granelli dei progetti esistenti restano dov'erano (test `second_color_splits_the_grains`).
- `Dust::size_spread` (0–1, 0,5 = da metà al doppio come prima, 0 = tutti uguali, 1 = fino al triplo) (test `size_spread_controls_the_variation`).
- `Dust::opacity`: scala la luce diffusa dai granelli; un raggio d'ombra è fermato da un granello con probabilità pari all'opacità (per raggio sulla CPU con un hash, con `rnd()` sulla GPU), così in media l'ombra è parziale.
- `Dust::shape` = `Grains` (icosaedri, come prima) o `Flakes` (esagoni piatti con normale a caso) con `sparkle` e `flake_roughness`: `DustLook::response` dà ai granelli la diffusione in avanti di prima e alle scaglie una parte diffusa più un riflesso GGX a due facce tinto del loro colore, così brillano solo le scaglie girate giuste tra luce e camera (test `flakes_sparkle_unevenly`). GPU: `dust_response` nello shader, parametri 88–90 (`PARAM_WORDS` 92) e bit 3 di `P_HAS_DUST` per le scaglie.

**Esc deseleziona tutto**
- Nella viewport Esc toglie tutta la selezione (oggetti, luci, nebbie, fumi, polvere, camere) quando non sta annullando altro (trascinamento del gizmo o del cono dello spot, scelta dell'oggetto da inquadrare o del punto di fuoco); il selettore colore tiene il suo Esc. Aggiornati elenco delle scorciatoie e manuale.

**Fumo simulato solo anteprima**
- `SmokeCloud::preview_only`: il fumo simulato resta come anteprima a punti nella viewport (disegnata anche quando non è selezionato, con la nota "solo anteprima") e il render lo salta; la chiave della cache ignora l'opzione, così riaccendendo il calcolo vero si riusa quello già simulato (test `preview_only_smoke_is_skipped_by_the_render`). Pannello: "Calcolo vero per il render".

## 0.41.0 — 05/11/2026

**Fumo e nebbia non si ricalcolano per niente**
- La chiave delle cache che dipendono dalla geometria (`RenderGeometry::id`) era un contatore che cambiava a ogni ricostruzione della geometria, anche identica: al render si rifacevano il fumo simulato, il fumo calcolato su griglia, gli ostacoli della nebbia da ghiaccio secco e la foschia già pronti, anche senza modifiche. Ora è un'impronta del contenuto (triangoli, normali, UV, nodi): una scena identica dà la stessa chiave (test `same_scene_same_geometry_key`).

**Simulazione del fumo più veloce**
- Come la nebbia da ghiaccio secco, la simulazione lavora solo nella **zona attiva**: il riquadro di celle con fumo o calore, allargato di quanto il fluido può spostarsi in un passo più un margine; fuori l'aria è ferma e vuota, lì non si calcola niente e la pressione vale 0 (bordo aperto). Misurato (esempio `fumo_tempi` di `forge_volume`): fumo caldo 0,8 × 1,4 × 0,8 m 13,5 → 6,2 s, gas pesante 1,6 × 0,5 × 1,6 m 3,9 → 3,0 s, riquadro grande 3 × 2 × 3 m 25,5 → 6,1 s, con la stessa quantità di fumo.

**Contrasto del fumo**
- `SmokeCloud::contrast` (0,5–4, 1 = come prima): la densità relativa elevata a una potenza, così le parti diradate diventano trasparenti mentre i grumi restano densi e fanno ombra su sé stessi; un fumo leggero tiene il rilievo e i dettagli di uno denso invece di diventare un velo uniforme (test `contrast_thins_the_wisps_more_than_the_core`). Per un fumo trasparente ma contrastato: densità più bassa, contrasto a 2–3.

## 0.40.0 — 05/11/2026

**Fumo simulato che non finisce sul riquadro**
- **Bordi sfumati** (`SmokeSim::edge_fade`, 0–0,4 del lato, di serie 0,2): in una fascia vicino alle pareti e al soffitto del riquadro (non al fondo) fumo e calore si dissolvono nella simulazione, come se uscissero dal riquadro, invece di ammassarsi contro e di essere tagliati netti dal render (`smoke_sim`, test `soft_edges_keep_smoke_off_the_box`). La profondità della fascia cambia da punto a punto con un rumore frattale e nella fascia il render rinforza vortici, sbuffi e filamenti (`SmokeCloud::edge_keep`): il fumo si sfilaccia in lingue e buchi invece di svanire in modo uniforme. 0 = pieno fino al bordo, come prima.
- **Rotazione del riquadro** (`SmokeCloud::rotation_deg`, solo attorno alla verticale, perché il caldo sale comunque dritto): `to_local`, `to_world`, `corners`, `bounds` (riquadro del mondo che contiene quello ruotato), sorgente che gira con il riquadro; la griglia della simulazione copre il riquadro ruotato e il fumo che ne esce si perde (`SmokeSimParams::box_center`, `box_yaw`, `box_half`). Test `rotated_box_carries_the_source`.
- **Gizmo del fumo:** segue gli assi del riquadro; Ruota gira riquadro e sorgente insieme (anche il campo "Rotazione"); Scala ingrandisce insieme riquadro, posizione e raggio della sorgente. Con il gizmo e con il campo "Dimensioni" il fondo resta fermo: il riquadro cresce solo verso l'alto e non va più sotto il piano.
- **Preset "Vapore da tazza":** ora simulato (sorgente larga come la tazza, poco calore, vortici forti e turbolenza fine, dissolvenza veloce, dettaglio fine con molti filamenti), al posto del pennacchio procedurale che sembrava una ciminiera (test `cup_steam_rises_in_thin_wisps`).
- **Anteprima nella viewport:** non si butta più quando il fumo viene deselezionato (per esempio con un clic nella viewport mentre si gira la camera), così riselezionandolo non si ricalcola; un'etichetta in basso a sinistra mostra l'istante riprodotto (il giro che riparte da 0 non è un nuovo calcolo).

**Anteprime dei materiali a 512 px**
- Ogni anteprima si renderizza una volta a 512 px (`store::THUMBNAIL_RENDER`, file `anteprima-v6-512.png`: quelle a 512 già fatte restano valide) e si rimpicciolisce con un filtro Lanczos per le schede più piccole e l'inspector. Tolta l'impostazione "Risoluzione minima" (Bassa / Media / Alta cambiano solo i campioni).

**Correzioni**
- **Stili in bianco e nero:** con "Bianco e nero" acceso va in grigi l'intero campione di ogni pixel (`forge_render_cpu::style_gray`, lo stesso in WGSL), anche quello visto attraverso vetri e liquidi colorati o negli specchi: prima erano grigie solo le superfici disegnate e la palla di vetro arancione del render di Valerio restava arancione (test `style_mono.rs`, fumetto e tecnico, RGB e spettrale).

## 0.39.0 — 04/11/2026

**Fumo simulato da posizionare**
- La sorgente della simulazione si disegna come una sfera grande quanto la sorgente vera, selezionabile con un clic o con "Seleziona la sorgente" (`ForgeApp::smoke_source_handle`); il gizmo la sposta (`gizmo::DragStart::SmokeSource`, scostamento dal fondo del riquadro) e Scala ne cambia il raggio.
- **Anteprima in viewport** (`ui::smoke_preview`, `forge_render_core::smoke::smoke_preview`): la stessa simulazione su una griglia grossolana (circa 36.000 celle, meno di un secondo) in un thread a parte ogni volta che cambiano le impostazioni del fumo selezionato, con 48 fotogrammi (`forge_volume::smoke_sim::simulate_smoke_frames`) mostrati come nuvole di punti (fino a 5000), in loop o fermi su un istante scelto. Non si salva con il progetto.
- **Le celle seguono la sorgente** (`SmokeCloud::sim_cells`, `sim_cell`): le celle per lato della qualità, più fini quando una cella è più grande della sorgente (circa tre celle di diametro), entro il totale di celle della qualità; `sim_warning` e un avviso nel pannello se la sorgente resta più piccola di una cella. Causa del fumo invisibile segnalato da Valerio: un riquadro di 10 × 1,5 × 5 m a qualità 2 dava celle di 14 cm per una sorgente di 7 cm, e il gas pesante restava in uno strato più sottile di una cella, cancellato dal pavimento.

**Selettore colore (RGB / CMYK / Palette)**
- `ui::color_picker`: un solo pannello aperto dal campione dei colori del materiale (base, volume, emissione) con RGB (ruota di egui, R G B, esadecimale), CMYK (conversione matematica semplice, documentata come non colorimetrica) e Palette; anteprima immediata, Applica, Annulla/Esc che rimettono colore e codice.
- **RAL:** i valori digitali ufficiali RAL CLASSIC li vende RAL gGmbH come librerie da importare nei programmi di grafica. Con il via libera di Valerio la scheda palette comprende `assets/colori/RAL_Classic.json` (225 codici, nomi italiani e inglesi, valori sRGB **indicativi**: media delle tabelle di Wikipedia in inglese e in tedesco, quella tedesca dove differiscono di oltre 20/255; preparato da `tools/colori/ral_classic.js`), segnalati come non ufficiali. In più legge i file importati dall'utente in `<Libreria>/Colori`: Adobe Swatch Exchange `.ase` (RGB, Lab D50 → sRGB con adattamento di Bradford, CMYK, grigio), `.json`, `.csv`.
- `Material::color_codes` (campo → codice della palette, non scritto se vuoto): il codice si vede accanto al campione con il nome completo nel suggerimento; modificando il colore in RGB o CMYK si toglie.

**Correzioni**
- **Contorni a inchiostro e tilt-shift non ritardano più il render:** si calcolavano nel thread del render prima del primo campione (0,3–0,5 s a 1080p con 0,1–1,6 milioni di triangoli, a ogni ripartenza, e le ripartenze in coda si sommavano trascinando). Ora gli `Overlays` si calcolano in un thread a parte e si pubblicano appena pronti, si riusano finché geometria, camera, risoluzione e impostazioni delle linee restano uguali, e una serie di avvii in coda tiene solo l'ultimo (test `overlays.rs`).
- **Tilt-shift senza copie ripetute:** con 48 campioni su un raggio di oltre 20 pixel i riflessi luminosi si vedevano come copie separate (sul toro del render di Valerio). Ora la sfocatura si raccoglie da una piramide di immagini ridotte, con circa 3 pixel di raggio sul livello scelto, mescolando due livelli (test `large_blur_has_no_ghost_copies`).
- **Riflessi del fumetto (macchie bianche):** il riflesso era un bianco fisso +0,6 su ogni materiale non ruvido, messo dove il mezzo vettore della luce più forte superava 0,985 (circa 10°), con il centro delle luci ad area. Ora c'è solo dove la direzione speculare vede davvero la lampada (la forma della softbox per le luci ad area, un cerchietto per le altre), con la forza della riflettanza del materiale (circa 0,15 su una plastica scura, pieno sui metalli), su CPU e GPU (test `toon_highlight_follows_the_reflectance`). Corrisponde ai riflessi del render fotorealistico della scena di prova.
- **Puntini neri nei contorni:** non riprodotti dalla 0.37 in poi (scena di prova, 640×360 e 1600×900, con e senza pulizia): la correzione della 0.37 (campioni isolati nelle fessure tra triangoli) li copre.
- **Simboli mancanti:** il quadratino dopo "U8" nel selettore colore di egui era il suo pulsante di copia 📋 (U+1F4CB): dalla 0.37 di egui i caratteri dei simboli sono facoltativi (`monochrome_emoji_fonts`), ora attivati. Un test nuovo controlla che ogni carattere non ASCII dei testi dell'interfaccia e di `Lan_eng.json` abbia un glifo; ha trovato e sostituito ⊙ (camera agganciata a un oggetto), ● (modifiche non salvate), ✓ (in libreria) e ⧉ (stacca la finestra).
- **Manuale nella lingua dell'interfaccia:** segue la lingua attiva dell'interfaccia (anche dopo un cambio di lingua, dimenticando la scelta fatta nella finestra del manuale); una lingua senza manuale ripiega sull'inglese, poi sull'italiano.
- **Pulsante doppio nella barra:** "Libreria" e "Materiale" aprivano la stessa vista; "Libreria" è tolto (resta il menu Libreria con scarica, importa da Blender e cartella).
- **Anteprima del materiale:** trascinando a destra l'oggetto gira verso destra.

## 0.38.0 — 04/11/2026

**Nuova libreria dei materiali di serie**
- I materiali di serie sono dati, non codice: `assets/materiali/<NN Categoria>/<id>/materiale.json` più le mappe (il formato di "Esporta zip…"), letti una volta da `forge_scene::material::presets()` (`library_presets`, in cache). 125 materiali in 13 categorie (Metalli, Anodizzati, Vernici, Plastiche, Gomme, Vetri, Liquidi, Ceramiche e pietre, Legni, Tessuti e pelle, Asfalto, Coperture, Emissivi): tutti i materiali senza mappe e al massimo cinque con mappe per categoria, ridotte a 1024 px (108 MB in tutto). Senza la cartella restano pochi materiali di base (gli id della scena di prova).
- Ricavata dagli zip dei materiali preparati da Valerio seguendo `tools/libreria-serie/tabella.json` (id, nome italiano, categoria, dispersione, compreso o aggiuntivo) con l'esempio `libreria_serie` di `forge_render_core`. Corretti strada facendo: id doppi (anodizzati, plastiche lucide, opache e soft-touch, guarnizioni), categorie sbagliate (l'acqua era nei Vetri e non veniva trattata da liquido; intonaci nei Legni; i materiali Poly Haven tutti in "Libreria"; la categoria "Classici" con un solo materiale unita alle Plastiche), nomi in inglese.
- **Dispersione:** ogni trasparente ha il numero di Abbe (vetro 58, cristallo al piombo 32, vetro ottico 64, diamante 55, PMMA 57, policarbonato 30, resina trasparente 45, silicone 50, acqua 56, vini e liquori 55, olio d'oliva 47); la preparazione si ferma su un trasparente che non ce l'ha.
- **Mappe convertite senza cambiare il render:** le EXR e le PNG diventano JPEG (qualità 95), perché il motore legge rugosità, normal, metallicità e occlusione a 8 bit; le height diventano PNG a 16 bit, perché le legge a 16 bit; il colore lineare delle EXR diventa sRGB. Misurato sulle mappe convertite: PSNR 41–44 dB rispetto alle originali.
- **Niente BlenderKit:** i 29 materiali `bk-` e i 19 importati da BlenderKit (anodizzati, oro, tessuti) non sono nel programma; li sostituiscono materiali CC0 scaricati con l'esempio `sostituti_cc0` di `forge_library`: 21 da ambientCG (acciaio spazzolato e graffiato, alluminio tornito e graffiato, lamiera mandorlata, forata e ondulata, oro, rame, verderame, titanio, nichel, bronzo antico, ferro grezzo e arrugginito, verniciatura a polvere nera, metallo verniciato rosso, la base spazzolata dell'alluminio anodizzato) e 12 da Poly Haven (jersey, lino, velluto, velluto a coste, scamosciato, spugna, pile, vichy, piqué, pelle bianca e rossa, jacquard floreale). Fonti in `assets/materiali/LEGGIMI.md`.
- **Pacchetto aggiuntivo:** altri 109 materiali con mappe a 2K (circa 1,6 GB), non nel repository: si leggono da `materiali-aggiuntivi` accanto al programma, `assets/materiali-aggiuntivi` (compilazione locale) o `FORGE_EXTRA_MATERIALS_DIR` (`extra_materials_dir`); un materiale con lo stesso id sostituisce quello compreso.
- Progetti salvati: le vecchie cartelle delle mappe (`legno-rovere`, `marmo-bianco`, `pelle-nera`) restano in `assets/materiali/_vecchi`, e le mappe dei materiali di serie si ritrovano in ogni cartella di categoria (`builtin_map_dirs`). I progetti tengono i loro materiali come salvati.

## 0.37.0 — 04/11/2026

**Simulazione del fumo**
- Il fumo volumetrico ha il modo **Simulato** (`SmokeCloud::mode`, `SmokeSim`): fluido euleriano su
  griglia con temperatura e galleggiamento (`forge_volume::smoke_sim`: sorgente sferica con fumo,
  calore e velocità di uscita; forza (β·T − α·ρ)·ŷ; confinamento della vorticità; turbolenza curl
  noise; vento; trasporto semi-lagrangiano; pressione Gauss–Seidel rosso/nero; dissolvenza e
  raffreddamento; oggetti come celle solide; riquadro aperto). Deterministico. Congelato all'istante
  scelto; il campo (normalizzato sul 90° percentile, curva schiacciata) sostituisce la forma
  procedurale e sul suo bordo si aggiungono sbuffi e filamenti procedurali. Cache a parte rispetto al
  dettaglio. Qualità della simulazione 48–136 celle per lato. Preset: fumo caldo simulato, gas
  pesante simulato. La sorgente si vede nella viewport.

**Camera come componente**
- Pannello di sinistra: elenco **Camere** sopra **Luci** (selezione, camera attiva, aggiunta dalla
  vista). La camera di rendering selezionata mostra nella viewport una linea tratteggiata fino al
  bersaglio (un rombo); si seleziona cliccando l'icona della camera o il rombo. Gizmo: Sposta =
  camera e bersaglio insieme, Ruota = orbita attorno al bersaglio, il rombo = solo il bersaglio
  (`gizmo::Target::Camera`, `ForgeApp::selected_camera`, `camera_target_handle`).
  `RenderCamera::aim`: il bersaglio può seguire un oggetto (`follow_camera_aims`). Se la vista
  guarda attraverso la camera, la segue.

**Tilt-shift**
- `forge_render_core::tiltshift`: fascia nitida sull'immagine finale (posizione, larghezza, angolo,
  due transizioni indipendenti, sfocatura massima), sfocatura a disco che rispetta la profondità da
  un buffer per pixel calcolato all'avvio del render, applicata in `Frame::display` (CPU e GPU, dopo
  il denoiser e i contorni). Separata dalla profondità di campo fisica; con tutte e due accese si
  sommano. Accenderlo riavvia il render (buffer di profondità); gli altri valori si applicano subito
  (`Frame::with_tilt`), anche all'esportazione.

**Stili e contorni**
- **Bianco e nero** (`RenderSettings::style_mono`) per fumetto (fasce in grigi) e illustrazione
  tecnica (rampa di grigi, senza i colori dei materiali), CPU e GPU.
- **Sfondo a tinta unita** (`RenderSettings::background`): colore per i raggi della camera che non
  colpiscono niente, l'ambiente continua a illuminare (CPU, GPU, spettrale, guide del denoiser).
- Contorni: i puntini sparsi venivano da singoli raggi di controllo che passavano nella fessura tra
  due triangoli dello stesso oggetto e ne colpivano il retro; un campione isolato in disaccordo con i
  suoi quattro vicini concordi ora prende il loro valore (test `no_isolated_dots_on_a_smooth_sphere`).
  Nuovi: sensibilità ai salti di profondità (di serie 3 % come prima), pulizia dei pezzi di linea più
  corti di N pixel (di serie 2), niente linea sul bordo di pavimento e fondali contro lo sfondo.

**Polvere**
- Un suo riquadro (`Dust::region`, gizmo `Target::Dust`, riquadro tratteggiato nella viewport,
  "Adatta alla scena"): il riquadro automatico di prima era centrato tra la scena e la camera e
  tagliato a 6 m, così i granelli potevano finire a mezz'aria e il fascio sembrava interrotto. La
  polvere nuova riceve un riquadro attorno agli oggetti fino alle spot.
- Quantità da 1 per m³, dimensione fino a 18 mm; **Fanno ombra** (`Dust::shadows`): i granelli
  fermano i raggi d'ombra su CPU e GPU.

**Riga di comando**
- `render`: `--mono`, `--background R,G,B`, `--outline-clean PX`, `--tilt C,L,T`, `--tilt-angle`,
  `--tilt-blur`, `--dust-shadows`, `--dust-box`; i preset del fumo 6–7 sono simulati.

Non ancora fatto: annulla/ripeti per camera e riquadri di fumo e polvere; strumenti MCP per il fumo
e le opzioni nuove; sfondo trasparente.

## 0.36.0 — 04/11/2026

**Fumo volumetrico**
- Nuovo effetto **Fumo volumetrico** (scheda Effetti): nubi di fumo vere in un riquadro
  (`forge_scene::SmokeCloud`, `Scene::smokes`, salvate nel progetto). Il riquadro decide dove
  esiste il fumo: si seleziona dal punto al centro nella viewport e si sposta o ridimensiona con
  il gizmo (Sposta / Scala, `gizmo::Target::Smoke`), o si scrivono centro e dimensioni; il fumo
  renderizzato lo segue esattamente.
- Quattro forme (nube, pennacchio, filo d'incenso, banco) e sei preset che aggiungono una nube
  nuova sopra la selezione: nube sospesa, incenso, vapore da tazza, pennacchio da tubo, fumo denso
  industriale, banco di fumo.
- Campo di densità (`forge_render_core::smoke::smoke_density_at`) nelle coordinate del riquadro:
  contorno della forma attorcigliato da un rumore a dominio distorto, sbuffi a cavolfiore sul bordo
  (frattale di |rumore|), filamenti dalle creste del rumore, variazione lenta all'interno; pennacchi
  e incenso lisci in basso e turbolenti salendo. Altezza di partenza e massima nel riquadro, bordi
  sfumati sulle facce del riquadro.
- Comandi: densità (0,5–2000 1/m) con la scala "opaco oltre ~X", colore, controluce, grandezza dei
  riccioli, forza dei vortici (0–8), dettaglio frattale (1–10), ruvidità, sbuffi, filamenti,
  morbidezza dei bordi, variazione interna, si allarga / serpeggia / si dissolve, altezza di
  partenza e massima, niente fumo negli oggetti, qualità 1–4 (3–24 M celle), variante.
- Mezzo fisico (`BakedFog`, delta tracking con le ombre su CPU e GPU); l'interno degli oggetti si
  toglie con `voxelize` + `interior_cells`. Calcolato in anticipo con gli altri effetti sulla
  stessa barra (`Phase::Smoke`), con la cache.
- Prima versione procedurale, ancora senza simulazione fluida (vedi `docs/studio-fumo-fuoco.md`).

**Foschia**
- **Altezza di partenza** (`Haze::bottom_height`): la foschia comincia sopra il pavimento, con
  l'inizio sfumato e qualche sbuffo che pende sotto; con l'altezza massima fa una fascia sospesa
  (es. 2,2–2,8 m). Strati, diradamento e fumo a terra si misurano dall'altezza di partenza; la
  griglia comincia poco sotto (celle più fini).
- Slider più ampi: densità 0,001–50, addensamenti fino a 2 (pochi ammassi molto concentrati),
  grandezza dei vortici 0,01–20 m, forza fino a 8, dettaglio frattale fino a 10, ruvidità 0,2–0,85,
  diffusione in avanti −0,5–0,95, altezze fino a 20 m, spessore e distanza degli strati ×4.

**Riga di comando**
- `render`: `--haze-bottom`, `--smoke 0..5|NOME`, `--smoke-at X,Z`, `--smoke-y`,
  `--smoke-scale`, `--smoke-quality`, `--smoke-density`.

## 0.35.0 — 04/11/2026

**Stili di rendering**
- In cima alla scheda Render: **Fotorealistico**, **Fumetto** (cel shading: colore del materiale in
  tre fasce dalle luci della scena, con le loro ombre, e riflesso netto sui materiali lucidi) e
  **Illustrazione tecnica** (Gooch: da caldo a freddo con una luce da disegnatore, senza ombre
  nere), con un'immagine d'esempio al passaggio del mouse (`assets/stili/`). Applicati alla
  prima superficie non speculare vista dalla camera (vetri e specchi restano veri); CPU e GPU,
  RGB e spettrale (`forge_render_cpu::style`, `style_shade` nella shader).
- **Contorni a inchiostro** (con qualsiasi stile): sagome, confini tra oggetti, salti di
  profondità e spigoli vivi da 3×3 raggi per pixel, disegnati sopra l'immagine finale anche dopo
  il denoiser (`forge_render_core::outline`, `Frame::outline`). Spessore, colore, spigoli e angolo.
  Cambiare stile o contorni riavvia il render.

**Foschia**
- **Solo raggi delle luci** (di serie): la foschia aggiunge ai raggi dalla camera la luce di
  spot, sole e lampade diffusa una volta, senza velare né oscurare gli oggetti
  (`BakedFog::additive`); spento = foschia fisica come prima.
- **Addensamenti**, **Altezza massima** dal pavimento con **Sbuffi** che salgono a caso, e accanto
  alla densità la scala "si vede fino a ~X m".
- Pavimento trovato con 5×5 raggi verso il basso (prima si usava il punto più basso della scena:
  il fumo a terra finiva dentro lo spessore del pavimento).
- Griglia legata anche alla grandezza della scena (qualità 2: almeno 160 celle per lato; prima
  ~60 sulla scena di prova), vortici 2,5 volte più forti, più contrasto nei banchi, densità di
  partenza più basse. Tarato con render di prova (`forgerender-cli render --dark --spot --haze …`).

**Polvere**
- La illuminano solo spot e sole (i fasci); granelli di serie più grandi (2,5 mm) e meno fitti,
  **Brillantezza** regolabile; i granelli sono nelle guide del denoiser.

**Riga di comando**
- `render`: `--style`, `--outlines`, `--haze`, `--haze-density`, `--haze-quality`,
  `--haze-clumping`, `--haze-top`, `--dust`, `--dust-size`, `--dust-amount`, `--dark`, `--spot`.

## 0.34.0 — 04/11/2026

**Foschia in cinque tipi, con il frattale e la qualità**
- La foschia d'ambiente ha un **Tipo**: *Nebbia bassa* (a terra, si dirada salendo), *Nuvole di
  gas* (aria quasi limpida con ammassi densi), *Fumo tossico a terra* (strato di pochi cm sul
  pavimento, raccolto in filamenti), *Strati sospesi* (veli orizzontali a quote diverse, ondulati e
  bucati), *Riccioli di vapore* (vortici dentro i vortici, contrasto alto). Nella scheda Effetti
  il pulsante **Foschia** apre il menu dei tipi; il tipo si cambia anche dopo.
- Comandi del frattale: **Forza dei vortici** (distorsione del dominio, 0–4), **Dettaglio
  frattale** (1–8 ottave), **Ruvidità** (guadagno tra le ottave), oltre a quantità e grandezza
  dei vortici. Lo strato del fumo a terra ha il riquadro basso (celle fitte solo dove serve).
- **Qualità** 1–4: da 1 a 24 milioni di celle (48 MB), fino a 512 per lato.
- Tolti i preset Velo leggero, Raggi di luce, Soffitta polverosa e Foschia fitta.

**Polvere in sospensione come effetto a sé, solo nei fasci**
- Pulsante **Polvere in sospensione** nella scheda Effetti, senza bisogno della foschia.
- I granelli sono trasparenti per tutti i raggi; quelli dalla camera aggiungono solo la luce
  diretta delle luci della scena (spot, sole, punti, aree), con ombre e diffusione in avanti:
  nessuna luce dall'ambiente o di rimbalzo, quindi si vedono solo dentro un fascio. CPU (RGB e
  spettrale) e GPU (RGB e spettrale). Test `tests/dust.rs`.

**Interfaccia**
- I pulsanti ✕ e le frecce dell'adesivo mostravano un quadratino (simboli assenti dal carattere):
  ora sono icone (cestino, X, frecce). Un test vieta quei simboli nel codice dell'interfaccia.
- Messaggio d'errore dei video senza la freccia mancante.

## 0.33.0 — 03/11/2026

**Foschia d'ambiente e polvere in sospensione**
- Scheda Effetti, **Foschia d'ambiente**: aria velata in tutta la scena, con banchi, riccioli e
  vuoti (rumore frattale a dominio distorto), illuminata come la nebbia (diffusione multipla,
  ombre degli oggetti nei raggi), su CPU e GPU. Densità, colore, raggi in controluce, vortici,
  grandezza dei vortici, diradamento salendo, forma. Preset: Velo leggero, Raggi di luce,
  Soffitta polverosa, Foschia fitta, Nebbia bassa.
- **Niente foschia dentro gli oggetti**: i tratti percorsi dentro vetri e liquidi non attraversano
  nebbie (lenti e bicchieri restano limpidi), su CPU e GPU; i vetri "a foglio" senza retro non
  contano. La foschia si azzera anche nell'interno pieno dei pezzi, trovato con la
  voxelizzazione degli oggetti più un riempimento dell'interno (`fog::interior_cells`): la
  stessa griglia servirà a simulare la foschia. L'aria di una stanza chiusa resta aria.
- **Barra di avanzamento anche per la foschia**: si calcola in anticipo come la nebbia. Gli
  effetti si calcolano **uno alla volta** sulla stessa barra, che riparte da zero per ognuno
  ("Effetto 1 di 2: simulazione…", poi "Effetto 2 di 2: foschia d'ambiente…"); la ✕ li ferma
  tutti. Spostare camera o luci ricalcola la foschia (ne cambia il riquadro).
- **Polvere in sospensione**: granelli veri sotto il millimetro (fino a 150 000) che brillano
  dove li colpisce un raggio o la luce di rimbalzo. Quantità, dimensione, colore.

**Adesivi (decal)**
- Scheda Effetti, **Adesivo (decal)**: un'immagine sull'oggetto selezionato, nel punto che guarda
  la camera; segue le superfici curve e si sposta con l'oggetto. Un PNG trasparente lascia
  vedere l'oggetto (stessa immagine come colore e ritaglio: si usa il canale alfa). Larghezza,
  rotazione, spostamento, profondità, ruvidità, metallicità, "Riposiziona dalla vista".
- Salvati nel progetto e nei pacchetti `.frpack` (con l'immagine).

**STEP: conversione salvata**
- Lo STEP si converte una volta: la conversione resta in una cache dell'utente e si salva con il
  progetto (`modelli-convertiti/`) e nei pacchetti; all'apertura si rilegge invece di
  riconvertire. Banco ottico demo: da 27 s a 0,2 s. Si riconverte se cambia lo STEP, un pezzo
  dell'assieme o la qualità.
- I pacchetti includono anche i pezzi degli assiemi STEP "a più file". Corretta la demo **Lenti
  convergenti e divergenti**: il pacchetto aveva solo l'assieme vuoto (si vedevano solo i laser);
  ora ha lenti, supporti e laser.

**Nebbia da ghiaccio secco**
- Pulsante **✕** accanto alla barra della nebbia: annulla il calcolo in anticipo (avviso: verrà
  rifatto all'avvio del render).
- Cambiare la nebbia (qualità compresa) o la scena durante il calcolo lo ferma subito e lo fa
  ripartire da capo con i valori nuovi.

**Altro**
- Il render parte di serie sulla **GPU**; senza scheda video adatta si passa alla CPU (anche da
  riga di comando).
- Integrazione AI: il ponte MCP si aggiorna anche se un assistente lo sta usando (il file in uso
  viene rinominato), niente più "Accesso negato".

## 0.32.0 — 03/11/2026

**Nebbia da ghiaccio secco: simulazione molto più veloce**
- Ogni passo della simulazione lavora solo dove c'è la nebbia: campi grandi quanto il suo
  riquadro invece che tutta la griglia, forze, trasporti e pressione sulle celle entro 6 dalla
  nebbia, tabella dei vicini da 2 byte per la pressione, buffer riusati e velocità massima
  misurata nel passaggio che aggiorna già le velocità. Fisica invariata (stesse forze, trasporto
  MacCormack, pressione e collisioni); la pressione ora si risolve sulla forma della nebbia, con
  aria aperta oltre le 6 celle.
- Misurato sulla demo del cocktail (Ryzen 9 9900X, mediana di 3 corse, primo calcolo senza
  cache): Media 20,9 → 8,0 s (2,6×), Alta 11,4 → 3,7 s (3,1×), Molto alta 60,9 → 14,5 s (4,2×),
  Ultra 182 → 37 s (4,9×). Stessa massa di nebbia; forma complessiva entro il 3,5–6% (medie a
  blocchi). Report completo: `reports/2026-11-03_ottimizzazione-simulazione-fumo/REPORT.md`;
  misura: `cargo run --release -p forge_project --example fog_bench`.
- **Calcolo in anticipo**: poco dopo l'ultima modifica della nebbia o della scena la nebbia si
  calcola in sottofondo, così di solito è pronta quando premi Render (il pulsante Render resta
  libero nel frattempo).
- **Barra di avanzamento** nella barra in basso, accanto ai campioni: «Nebbia: simulazione 45%»
  (ostacoli, simulazione, dettaglio). «costruzione BVH» compare ora solo mentre si costruisce
  davvero la BVH.

**Interruttore AI MCP**
- Il pulsante nella barra in alto ora è **AI MCP: ON / OFF**, con accanto l'**ingranaggio** che
  apre le impostazioni dell'integrazione AI (collegare o scollegare Claude Desktop, Claude Code,
  Codex).

**Laser: riflessi parziali**
- Nuova opzione **Riflessi parziali** nella scheda del laser (spenta di base): come un laser
  vero, a ogni faccia di vetro una parte della luce (la frazione di Fresnel) si riflette e diventa
  un fascio a sé con la sua macchia, così attorno a un cristallo o a un diamante compaiono tanti
  puntini. I rami sotto l'1% della potenza del laser si lasciano perdere; fino a 64 tratti e 16
  macchie per laser. CPU e GPU, RGB e Spettrale (verificato dai test: stessa immagine entro il 4%).
  Spenta, il laser segue un solo percorso come prima.

**Scene demo**
- Nuova **Lenti convergenti e divergenti** (Luce e dispersione): il banco di Valerio con tre laser
  (due bianchi da laboratorio e uno viola UV) che attraversano una fila di lenti, si incrociano nel
  fuoco e si riaprono verso lo schermo.
- Nuova **Banco ottico: laser e prisma** (Luce e dispersione): il banco ottico da laboratorio di
  Valerio, con un laser bianco da laboratorio che si apre in uno spettro e un laser blu che non si
  scompone, lenti e prisma in cristallo dispersivo, render Spettrale con le caustiche. È un
  pacchetto `.frpack` in `samples/`: si estrae nella cartella dell'utente e Salva chiede dove
  salvarlo (mai dentro i file del programma).

## 0.31.0 — 02/11/2026

**Progetti con tutti i loro file (pacchetto)**
- **File → Salva pacchetto completo (.frpack)…**: un solo file con dentro tutto: il progetto, i
  modelli importati (con i file a cui rimandano: `.mtl` e texture degli OBJ, `.bin` e immagini dei
  glTF), le texture dei materiali usati e l'HDRI. Si apre con File → Apri su qualunque computer
  (anche dai progetti recenti, trascinandolo sulla finestra o con il doppio clic, una volta
  installato); **Salva** poi aggiorna il pacchetto stesso.
- **File → Salva in una cartella con tutti i file…**: il progetto con accanto `modelli/`,
  `textures/` e `hdri/`; basta copiare la cartella.
- **File → Salva come ZIP con tutti i file…**: la stessa cartella compressa in uno ZIP.

**Libreria dei materiali**
- La libreria e la scheda del materiale si aprono in una **finestra a parte**, che si sposta fuori
  dal programma o su un altro schermo; "⧉ Riporta nel programma" le rimette sopra la scena.
- **Applica alla selezione** è un pulsante grande e colorato subito sotto l'anteprima, con il
  numero dei pezzi selezionati; compare anche in alto nella libreria appena si sceglie un
  materiale.

**Laser**
- Nuovo laser **Viola UV (405 nm)**.
- Il laser **blu** si vede blu (prima sembrava viola: della lunghezza d'onda pura si tagliava la
  parte che lo schermo non può mostrare, e la tinta cambiava). I laser hanno ora i colori che hanno
  nelle foto.

**Fascio laser largo**: ogni tratto del fascio visibile finisce esattamente sulle facce del vetro
che attraversa. Prima finiva dritto sull'asse e poteva sporgere dal prisma fino a tre raggi: con un
fascio largo si vedeva un pezzo di fascio impossibile.

**Modalità Semplice**: scegliere un aspetto (Chiaro, Scuro, Drammatico…) non cancella più le luci
che hai aggiunto: si sostituiscono solo le luci dell'aspetto di prima.

**Schermata di benvenuto**: immagine nuova (banco ottico renderizzato con ForgeRender) con il logo
grande, la versione sotto il logo e il titolo "A new dimension of light".

## 0.30.0 — 02/11/2026

**Laser: luce a sé, rosso, verde, blu o bianco**
- Il **Laser** è nell'elenco delle luci da aggiungere (pannello Luci, Aggiungi), non più solo
  cambiando il tipo di un'altra luce.
- **Tipo di laser**: **Rosso (650 nm)**, **Verde (532 nm)**, **Blu (450 nm)** o **Bianco da
  laboratorio**. Rosso, verde e blu hanno una sola lunghezza d'onda come i laser veri: il loro
  percorso usa l'indice di quella lunghezza d'onda e non si scompongono mai, nemmeno in un prisma
  o in un diamante. Il bianco da laboratorio contiene tutti i colori e, nel render Spettrale, i
  vetri con dispersione lo aprono in un arcobaleno. I progetti salvati prima tengono il laser
  bianco; un laser nuovo parte rosso.

**I preset delle luci aggiungono, non sostituiscono**
- I preset del pannello Luci aggiungono le loro luci a quelle che ci sono già invece di
  cancellarle.

**Correzioni**
- Il fascio laser visibile non si vedeva dentro un liquido modellato un filo più grande della
  cavità del bicchiere (il modo giusto): il raggio della camera saltava la faccia interna del vetro
  che sta dentro il liquido e cercava il fascio solo dopo quella faccia, perdendo tutto il tratto
  nel drink. Ora si controlla anche il tratto saltato (CPU e GPU, RGB e Spettrale).

## 0.29.0 — 01/11/2026

**Luce nei bicchieri con il liquido**
- Il fascio laser e i fotoni delle caustiche ora trattano il liquido dentro un bicchiere come fa
  il path tracer: all'interfaccia vetro-liquido piegano con la coppia di indici vera (per esempio
  1,52 → 1,35) invece di comportarsi come se dall'altra parte ci fosse aria, e le facce del
  liquido che stanno dentro lo spessore del vetro non contano più come superfici. Prima, in un
  drink, il fascio piegava troppo e i colori si aprivano molto più che nella realtà.
- Il fascio visibile si affievolisce gradualmente dentro un liquido colorato (potenza a metà di
  ogni tratto) invece di essere disegnato con il colore che ha all'uscita.

**Categoria Liquidi**
- Nuova categoria **Liquidi** nella libreria dei materiali: Acqua (spostata dai Vetri), Vino
  rosso, Vino bianco, Whisky, Cocktail rosso, Olio d'oliva, con indice e dispersione veri (Abbe 55)
  e il colore del volume.
- Un materiale della categoria Liquidi vince sul contenitore dove si sovrappongono, senza altre
  impostazioni: basta modellare il liquido un filo più grande della cavità. Una Priorità più
  alta impostata a mano vince comunque.

**Nota sulla luce Laser**: è un fascio sottile di luce **bianca** (come il raggio di sole della
demo del prisma). Un laser vero ha un solo colore e non si scompone mai in un arcobaleno;
l'arcobaleno forte nel render del cocktail veniva da un vetro con Abbe 10,5 (dispersione
estrema: un vetro comune è circa 58) insieme alla piega sbagliata vetro-liquido corretta qui.

## 0.28.1 — 01/11/2026

**Correzioni**
- Le scene demo salvate come progetto e riaperte perdevano gli oggetti (diamante, fondale,
  pavimento, prisma…) e compariva "componenti con materiale assegnato non sono più presenti": le
  forme costruite dalle demo non finivano nel progetto. Ora si creano come gli oggetti del menu
  Aggiungi, quindi il progetto le salva e le ricrea all'apertura. I progetti salvati da una demo
  con la 0.26–0.28.0 vanno salvati di nuovo partendo dalla demo.

## 0.28.0 — 01/11/2026

**Caustiche (photon mapping progressivo)**
- Scheda Render → **Caustiche**: la luce che passa da vetri, cristalli e gemme lisce si concentra
  sulle superfici attorno (punti di fuoco luminosi, gli arcobaleni dei cristalli appesi). Funziona
  con **tutte le luci**: puntiforme, spot, direzionale (sole) e ad area. Con puntiformi, spot e
  sole il solo path tracing questi percorsi non li trova proprio.
- Come funziona: a ogni passata si emettono fotoni dalle luci verso i vetri lisci della scena, li
  attraversano (scelta di Fresnel tra riflessione e rifrazione, legge di Snell, riflessione
  totale, tinta del vetro, assorbimento di Beer-Lambert) e si posano sulla prima superficie non
  speculare. Il primo punto non speculare visto dalla camera li raccoglie entro un raggio che **si
  restringe a ogni passata** (Knaus-Zwicker): le caustiche diventano sempre più nitide e
  convergono al valore giusto; i percorsi "vetro → luce" non li conta più il path tracer, così
  niente viene contato due volte.
- Nel render **Spettrale** ogni fotone ha la sua lunghezza d'onda (l'indice del vetro dipende da
  quella): le caustiche si aprono in arcobaleni.
- Impostazioni: fotoni per passata (200 k di partenza) e raggio iniziale (0 = automatico, 1%
  della scena).
- CPU e GPU: sulla GPU i fotoni li segue la CPU e li raccoglie lo shader (RGB e spettrale, RT core
  e BVH dello shader). Verificato dai test: con una luce ad area l'immagine con i fotoni coincide
  con il solo path tracing (0,4% di differenza); CPU e GPU concordano entro il 2%.

**Scene demo**
- **File → Scene demo**, divise per che cosa mostrano: *Luce e dispersione* (Prisma e luce,
  **Laser nel diamante**), *Caustiche* (**Ciondoli di cristallo al sole**, **Vetri e caustiche
  sul tavolo**), *Effetti volumetrici* (Cocktail con ghiaccio secco), *Studio e materiali* (Scena
  di test). La scena di test e la demo del prisma si spostano qui dal menu File.
- Aprendo una demo compare una **scheda** che spiega che cosa mostra la scena, quali funzioni del
  motore usa e come rifarla passo per passo (materiali, luci, impostazioni), con il pulsante
  "Avvia il render".
- Il progetto del cocktail ora è anche nell'installer; alla prima apertura viene copiato nella
  cartella dell'utente.

**Forme nuove**
- Menu **Aggiungi**: **Prisma**, **Diamante (taglio brillante)** (proporzioni di Tolkowsky, 57
  facce) e **Pendente di cristallo** (goccia sfaccettata).

**Correzioni**
- Aggiungendo pavimento, fondale o parete con oggetti già in scena, va **sotto tutti gli
  oggetti**, non solo sotto quello selezionato (prima poteva finire sopra o dentro alcuni).

## 0.27.0 — 01/11/2026

**Luci puntate su un oggetto**
- Pannello Luci → **Punta su → Aggancia alla selezione** (spot, area, laser, direzionale): la luce
  resta puntata sull'oggetto. Il bersaglio segue l'oggetto quando si sposta; spostando la luce
  continua a puntarlo, ruotandola con il gizmo Ruota **gira attorno all'oggetto**. "Sgancia"
  rende di nuovo libero il bersaglio.

**Laser**
- Il fascio ora si vede anche **dentro il vetro** (un vetro vero non è mai perfettamente puro):
  entra bianco, attraversa il prisma o la gemma ed esce come arcobaleno.

**Correzioni dell'interfaccia**
- Pannello di sinistra: l'elenco **Luci** ha un'area sua con la barra di scorrimento e un **bordo
  trascinabile** per regolarne l'altezza; "Eliminati dal progetto" si sposta sopra l'albero della
  scena. Prima, con più luci, queste finivano sotto il pannello e non si potevano selezionare, e il
  bordo del pannello scendeva nella barra di stato.

## 0.26.0 — 01/11/2026

**Luce laser**
- Nuovo tipo di luce **Laser** (pannello Luci): fascio sottile (diametro in mm, potenza in W)
  dalla luce verso il bersaglio. Il percorso si calcola partendo dalla luce attraverso i vetri
  lisci (rifrazione, riflessione totale, perdite di Fresnel, assorbimento nel volume) e gli
  specchi fino alla prima superficie opaca, dove illumina una macchia gaussiana: esatta e senza
  rumore, anche dietro un vetro, dove un path tracer che parte dalla camera non la troverebbe mai.
- **Fascio visibile** sì/no ("Nell'aria"): diffusione singola in una foschia uniforme dentro il
  fascio, integrata analiticamente.
- Nel render **Spettrale** il percorso si calcola per 64 lunghezze d'onda (interpolate fra loro):
  dopo un prisma o una gemma dispersivi il fascio si apre a ventaglio arcobaleno e disegna uno
  spettro continuo sul pavimento. CPU e GPU (RT core e BVH dello shader) danno lo stesso risultato.
- **File → Nuova scena demo: prisma e luce**: un fascio di luce bianca entra in un prisma di vetro
  su fondo nero ed esce come arcobaleno (render spettrale già impostato). Nuova primitiva: prisma
  triangolare.

## 0.25.0 — 01/11/2026

**Ray tracing hardware (RT core)**
- Sulle schede con unità di ray tracing (NVIDIA RTX, AMD RX 6000 e successive, con Vulkan) il
  render GPU trova i triangoli colpiti dai raggi con l'hardware (ray query di wgpu) invece della
  BVH dello shader. Stessa immagine: 99,9% dei pixel identici alla BVH sulla stessa GPU, e tutti
  i test CPU/GPU invariati.
- Misurato su una RTX 5070, 256 campioni a 1280×720: scena di prova da 2,62 s a 0,25 s
  (**10,6×**), 1,6 milioni di triangoli da 7,69 s a 0,59 s (**13,0×**), 3,8 milioni da 8,86 s a
  0,59 s (**14,9×**); spettrale da 7,7 a 12,6×.
- Automatico: la scheda Render mostra "RT core attivi". Senza supporto (o con
  `FORGE_GPU_RT=0`) si usa la BVH dello shader come prima. Le texture di ritaglio (opacità)
  funzionano: il raggio riparte subito oltre i punti trasparenti.
- Benchmark: `cargo run --release -p forge_render_gpu --example rt_bench`.

## 0.24.0 — 01/11/2026

**Rendering spettrale (opzionale)**
- Render → Colore: **RGB** (predefinito, invariato) o **Spettrale**: quattro lunghezze d'onda per
  percorso (hero wavelength, campionamento visibile di PBRT v4), colori RGB convertiti in spettri
  lisci (sigmoide di un polinomio, tabella dei coefficienti nostra), ricostruzione CIE 1931 con il
  bianco della scena che resta bianco. CPU e GPU, stesso risultato. Circa 2 volte più lento
  dell'RGB, con più rumore di colore.
- Materiali trasparenti: **Dispersione** (numero di Abbe, equazione di Cauchy) con i predefiniti
  crown, flint, cristallo al piombo, policarbonato e diamante. Si vede solo in modalità spettrale.
- Controllo IA: `set_render_settings` accetta `color` (rgb / spettrale), `create_material`
  accetta `abbe`.
- Dettagli e verifiche: [docs/SPECTRAL.it.md](docs/SPECTRAL.it.md).

**Scelta della modalità all'avvio**
- All'avvio, dopo la schermata di benvenuto, la finestra "Come vuoi lavorare?" propone la
  modalità Semplice o Avanzata, con una breve descrizione di ognuna e l'indicazione che si
  cambia poi dal selettore "Modalità: Semplice | Avanzata" in alto a sinistra. Compare a ogni
  avvio finché non si spunta "Non mostrare più all'avvio"; si riattiva in Impostazioni,
  Informazioni ("Chiedi la modalità all'avvio").

**Materiali BlenderKit tolti**
- I 38 metalli BlenderKit (Royalty Free, solo uso personale) non sono più inclusi né elencati
  tra i predefiniti. L'importatore di materiali da file `.blend` / BlenderKit resta disponibile.

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
