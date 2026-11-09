# Modifiche richieste — 09/11/2026

## Sfondo personalizzabile della viewport

### Richiesta di Valerio

Impostare come colore predefinito dello sfondo della viewport:

- RGB: **40, 44, 52**
- HEX: **#282C34**
- Nome suggerito: **Antracite**

È un grigio antracite leggermente freddo, scelto per mantenere un buon contrasto anche con oggetti neri o molto scuri.

Nelle **Impostazioni** prevedere la scelta tra questi preset:

| Nome | RGB | HEX |
|---|---:|---|
| Antracite (predefinito) | 40, 44, 52 | #282C34 |
| Grigio Blender-like | 48, 52, 59 | #30343B |
| Scuro tecnico | 34, 40, 49 | #222831 |
| Grigio chiaro | 53, 58, 66 | #353A42 |
| Personalizzato | A scelta dell'utente | A scelta dell'utente |

Per **Personalizzato**, riutilizzare il selettore colore già presente in ForgeRender, consentendo l'inserimento RGB e HEX se supportato dall'implementazione esistente.

### Requisiti di implementazione

1. Prima ispezionare il sistema esistente di preferenze e colori della viewport: evitare sistemi paralleli o duplicazioni del selettore.
2. Il valore iniziale per nuove installazioni e preferenze non ancora definite deve essere RGB 40, 44, 52.
3. Conservare la scelta dell'utente nelle preferenze e ripristinarla dopo il riavvio.
4. Applicare il cambiamento immediatamente alla viewport, senza riavvio.
5. Il colore deve riguardare esclusivamente la viewport e non alterare il rendering finale.
6. Verificare il contrasto di griglia, assi, selezione e gizmo con tutti i preset.
7. Aggiornare le traduzioni italiana e inglese e il manuale, se necessario.
8. Aggiungere test per default, persistenza/caricamento delle preferenze e selezione dei colori.

### Vincoli e verifica

- Lavorare sul branch di sviluppo, senza modificare direttamente `main`.
- Prima di modificare, riferire brevemente quali parti del codice gestiscono oggi sfondo viewport e preferenze.
- Al termine indicare file modificati, test realmente eseguiti (con comandi e risultati) e commit.
- Non dichiarare verifiche visive o test non eseguiti.

## Risposta di Claude Code

Riscontro di Claude Code — 09/11/2026, versione 0.51.0. Valerio ha chiesto di portare il lavoro anche su `main`: il commit è su `main` e su `sviluppo`. Vale per tutte e tre le richieste di questo file.

Parti del codice coinvolte (lette prima di modificare):
- Sfondo della viewport: era fisso, `Vec3::splat(0.045)` lineare (circa grigio 60 sRGB), passato a `ViewportRenderer::render`/`capture` da `ForgeApp::viewport_texture` e `capture_viewport` (`app.rs`) e una costante uguale in `ui/step_import.rs`. La texture della viewport è `Rgba8UnormSrgb`, quindi il colore di pulizia va dato lineare.
- Preferenze: `prefs.rs` (`Preferences`, `#[serde(default)]`, salvate in `impostazioni.json` da `ForgeApp::apply_prefs` appena cambiano nella finestra `ui/settings.rs`). Selettore colore esistente: `ui/color_picker.rs` (`rgb_edit`, RGB/HEX/CMYK/palette).
- Avvio del render: tutti i pulsanti passano da `ForgeApp::toggle_pause` (Avvia render della barra, F5, menu Rendering, pannello Semplice, scheda demo) o `ForgeApp::start_render` (Riavvia, MCP, timeline, viste). `start_render` metteva `View::Render` solo a fine avvio o in attesa di geometria/nebbie; la ripresa da pausa o da fermato (`session.resume()`) non cambiava vista, e nemmeno gli avvii falliti.
- Ambiente: il menu Ambiente è `presets::Look` (`apply_look`, `prepare_scene`, `fit_scene`). Le luci avevano già il segno salvato `Light::from_look`; `set_look_lights` riprendeva il numero (`id`) delle luci con lo stesso nome, e `mark_legacy_look_lights` segnava come d'ambiente tutte le luci dei progetti vecchi se i nomi coincidevano con un aspetto. I numeri delle luci li assegna `Scene::assign_element_ids` (mai uno ancora usato dalla timeline).

### Sfondo della viewport

- Predefinito RGB 40, 44, 52 (#282C34) per le installazioni nuove e per i file di impostazioni senza il campo (`VIEWPORT_BACKGROUND_DEFAULT` in `crates/forge_app/src/prefs.rs`).
- **Impostazioni → Viewport**: Antracite (predefinito), Grigio Blender-like (48, 52, 59), Scuro tecnico (34, 40, 49), Grigio chiaro (53, 58, 66), ognuno con campione, RGB e HEX; **Personalizzato** usa il selettore colore già esistente (`rgb_edit`: RGB, HEX, CMYK, palette). Nessun sistema parallelo.
- Salvato in `impostazioni.json` appena cambia, ripristinato al riavvio, applicato subito alla viewport, alle registrazioni "Anteprima (viewport)" e all'anteprima STEP. Il render finale non lo usa.
- Contrasto con griglia, assi, selezione e gizmo: **non verificato a schermo**. La viewport non disegna una griglia; selezione e gizmo non li ho guardati con i quattro sfondi.

### Passaggio alla scheda Render

- Tutti i percorsi che avviano un render convergono su `ForgeApp::toggle_pause` o `ForgeApp::start_render` (Avvia render della barra, F5, menu Rendering, pulsante della modalità Semplice, scheda della demo, Riavvia, MCP, viste, timeline). `start_render` ora mette la scheda Render come prima cosa (anche se aspetta geometria o nebbie, o se l'avvio fallisce); `toggle_pause` la mette quando avvia o riprende, non quando mette in pausa un render in corso (`render_command_shows_render`). Il render in sé non è cambiato.
- **Non provato a mano** nelle due modalità: verificato dalla regola e dal codice.

### Luci dell'ambiente

- Strategia di identificazione: il campo `Light::from_look` (già esistente, salvato nel progetto). Il nome e il suffisso non contano mai; tolto il vecchio riconoscimento per nome dei progetti di prima (`mark_legacy_look_lights`).
- Primo caricamento (nessuna luce d'ambiente nella scena): le luci presenti si spengono, senza eliminarle né cambiare altro. Cambio: si eliminano solo le luci con `from_look`, le nuove si creano da zero (accese, numero assegnato dalla scena, tutti i valori del preset). Le luci dell'utente non si toccano ai cambi successivi.
- Albero della scena: "[ENV]" dopo il nome (`presets::ENV_LIGHT_SUFFIX`), con spiegazione nel suggerimento.
- Fondale e altri oggetti: comportamento invariato.
- Conseguenze da sapere: nei progetti salvati prima del segno `from_look` le luci contano come dell'utente (il primo ambiente le spegne e aggiunge le sue). Cambiando ambiente l'animazione delle vecchie luci d'ambiente non passa alle nuove. Corretto anche `matching_look`, che ora ignora il numero della luce.

### File modificati

`crates/forge_app/src/prefs.rs`, `crates/forge_app/src/ui/settings.rs`, `crates/forge_app/src/app.rs`, `crates/forge_app/src/ui/step_import.rs`, `crates/forge_app/src/presets.rs`, `crates/forge_app/src/ui/tree.rs`, `lang/Lan_eng.json`, manuale EN+IT (`ui.impostazioni`, `render.pannello`, `intro.semplice`), `README.md`, `README.it.md`, `CHANGELOG.en.md`, `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`, `packaging/windows/forgerender.iss`, note di lavoro.

### Test eseguiti davvero

- `cargo fmt --all`
- `cargo clippy --workspace --exclude forge_step --all-targets --release -- -D warnings`: nessun avviso.
- `cargo test --workspace --exclude forge_step --release --no-fail-fast`: 412 passati, 0 falliti, 10 ignorati.
- Test nuovi: `prefs::tests::viewport_background_default_presets_and_persistence` (predefinito, file vecchio, valori ed HEX dei preset, salvataggio e caricamento di ogni preset e di un personalizzato); `app::auto_restart_tests::render_command_switches_to_the_render_tab`; `presets::tests::environment_lights_are_replaced_and_user_lights_kept` (i test 1–8 richiesti: primo caricamento, cambio, luci dell'utente intatte, molte↔poche, tutti i valori uguali al nuovo ambiente compresi posizione e bersaglio, segno interno, salvataggio e riapertura con `forge_project::save/load`, stesso ambiente due volte); `presets::tests::user_light_named_like_environment_is_never_deleted`. Il suffisso "[ENV]" nell'albero non ha un test automatico (è interfaccia).
- **Non eseguite:** prove a schermo nel programma aperto.

## Comportamento automatico: passaggio alla scheda Render

### Richiesta aggiuntiva di Valerio

Quando l'utente preme **Render** in qualsiasi punto/modalità dell'interfaccia (modalità Semplice o altre modalità), ForgeRender deve portare automaticamente la vista centrale sulla scheda **Render**.

L'obiettivo è evitare che il rendering parta mentre l'utente continua a vedere la scheda **Viewport**, creando confusione o facendo pensare che il comando non abbia funzionato. Un utente poco esperto potrebbe inoltre non accorgersi del selettore per passare manualmente da Viewport a Render.

### Requisiti

1. Il comportamento deve essere uniforme in tutte le modalità dell'interfaccia, inclusa **Semplice**.
2. Al comando Render, selezionare automaticamente la scheda **Render** prima o contestualmente all'avvio del rendering, così che il cambio sia immediatamente evidente.
3. Non limitare il comportamento al pulsante presente in una sola barra o pannello: verificare tutti i percorsi UI che avviano un render e farli convergere su un comportamento coerente.
4. Non modificare il funzionamento del rendering né interrompere eventuali render già in corso; valutare il caso di un nuovo comando durante un render attivo secondo la logica già prevista dall'applicazione.
5. Verificare il comportamento nelle diverse modalità e nei diversi punti dell'interfaccia da cui è possibile avviare un render.


## Gestione delle luci quando si carica o si cambia Environment

### Richiesta definitiva di Valerio

La funzione Environment carica una scenografia composta da fondale e luci. Le luci della scenografia devono essere trattate come elementi appartenenti all'Environment e avere un ciclo di vita legato alla scenografia caricata.

**Questa è la strategia definitiva: quando si cambia Environment, eliminare le sole luci della scenografia precedente e crearne di nuove. Non riutilizzare le vecchie luci e non limitarsi a disattivare quelle in eccesso.**

### Comportamento richiesto

**Primo caricamento di una scenografia**
1. Se nella scena sono già presenti luci dell'utente, disattivarle, ma non eliminarle né modificarne i parametri.
2. Creare e attivare tutte le luci appartenenti alla nuova scenografia.
3. Nell'albero della scena mostrare un suffisso riconoscibile, ad esempio `[ENV]`, accanto al nome di ciascuna luce Environment.

**Cambio da una scenografia a un'altra**
1. Individuare le luci appartenenti all'Environment attualmente caricato.
2. Eliminare esclusivamente quelle luci Environment.
3. Creare e attivare le luci della nuova scenografia, con tutti i parametri previsti: tipo, posizione, orientamento/rotazione, colore, intensità/potenza, dimensioni e ogni altra proprietà supportata.
4. Le luci originali dell'utente non devono essere eliminate né modificate durante i cambi successivi.
5. Dopo il caricamento, devono essere presenti esattamente le luci previste dalla nuova scenografia: nessun residuo e nessun duplicato, sia quando la nuova scenografia ha più luci sia quando ne ha meno.
6. Ricaricare la stessa scenografia deve produrre lo stesso risultato, senza accumulare luci.

### Identificazione e sicurezza

- Il suffisso `[ENV]` nell'albero serve a riconoscere visivamente le luci della scenografia da parte dell'utente.
- Il suffisso **non deve essere l'unico criterio tecnico** per decidere quali luci eliminare. Usare un metadato/ruolo interno dedicato e persistente per identificare in modo affidabile le luci create dal caricamento Environment.
- Verificare che l'identificazione sopravviva al salvataggio e alla riapertura del progetto.
- Non eliminare mai una luce dell'utente sulla base del solo nome o di una somiglianza con una luce Environment.
- Conservare il comportamento del fondale e non alterare gli altri oggetti della scena.

### Test richiesti

1. Primo caricamento: luci preesistenti disattivate ma conservate; luci Environment create, identificate e attive.
2. Cambio Environment: eliminate solo le vecchie luci Environment; create tutte quelle della nuova scenografia.
3. La scena utente non perde né modifica le proprie luci durante i cambi.
4. Cambio da Environment con molte luci a uno con poche e viceversa: nessun residuo.
5. Tutti i parametri delle nuove luci corrispondono alla nuova scenografia, inclusi posizione e orientamento.
6. Suffisso `[ENV]` visibile nell'albero e metadato interno coerente.
7. Salvataggio e riapertura del progetto preservano l'identificazione.
8. Ricaricamento dello stesso Environment senza duplicati.

Lavorare sul branch `sviluppo`. Prima di implementare, controllare come il modello scena gestisce identificatori, eliminazione delle luci, persistenza e nomi nell'albero. Nel riscontro finale indicare file modificati, strategia di identificazione, test realmente eseguiti e commit. Non dichiarare come verificato ciò che non è stato provato.


## Icona Windows ForgeRender — 09/11/2026

### Richiesta di Valerio

Integrare nel repository l'icona ufficiale fornita da Valerio e includerla nel prossimo build di ForgeRender.

- File previsto: `assets/forgerender.ico` (da aggiungere).
- Uso: icona dell'eseguibile Windows e, dove applicabile, icona della finestra/taskbar.
- Il trasferimento binario tramite l'integrazione GitHub non ha preservato integralmente il file ICO: un tentativo incompleto è stato rimosso. **Il file ICO valido non è attualmente presente nel repository** e va caricato tramite Git sul PC di sviluppo prima dell'integrazione.
- Nel prossimo build, integrare l'icona nelle risorse Windows dell'eseguibile (non limitarsi a copiarla accanto all'EXE).
- Verificare separatamente l'icona mostrata da Explorer, collegamento/installer e finestra/taskbar: per quest'ultima controllare l'integrazione eframe/winit/Windows, perché la risorsa dell'EXE da sola potrebbe non impostare l'icona della finestra.
- Aggiornare gli script di build/package e l'installer se necessario, mantenendo un'unica sorgente dell'icona.
- Non cambiare il logo né ridisegnarlo. Non dichiarare verificata la resa visiva senza averla controllata nel programma e in Windows Explorer.

**Nota tecnica per Claude:** quando `assets/forgerender.ico` sarà presente nel branch, controlla che sia un ICO valido e che contenga davvero le risoluzioni multi-size attese; quindi integra l'icona nel prossimo build. Non creare un'icona sostitutiva né usare un file incompleto.

## Bug: colore sfondo viewport diverso dal colore selezionato — 09/11/2026

### Riscontro di Valerio (screenshot 09/11/2026, ForgeRender 0.51.0 portable)

In Impostazioni → Viewport è selezionato **Grigio chiaro**, RGB 53, 58, 66, HEX `#353A42`. La viewport reale resta invece quasi nera. Campionando lo screenshot nelle zone libere della viewport si misurano circa RGB (9, 11, 14), non RGB (53, 58, 66). La discrepanza è netta e visibile.

Il percorso impostazioni indica `E:\\ForgeRender-0.51.0-portable-x64\\impostazioni.json`.

### Verifiche iniziali

- Il commit 0.51.0 dichiara che il colore scelto viene applicato subito alla viewport e all'anteprima STEP.
- Nel codice attuale `ForgeApp::viewport_background()` converte `prefs.viewport_background` da sRGB a lineare e `viewport_texture()` passa il valore a `ViewportRenderer::render(..., bg, ...)`.
- Non è ancora stata individuata la causa. Non assumere che sia un semplice problema di conversione gamma: il valore atteso e quello visualizzato sono molto distanti.
- Il fatto che il file sia nella cartella portable 0.51.0 va tenuto presente: verificare che la build effettivamente in esecuzione contenga il commit 0.51.0 e che non ci siano percorsi di rendering/texture alternativi che ignorano il parametro.

### Richiesta a Claude Code

Riprodurre e correggere il bug, non limitarsi a cambiare i valori dei preset o il testo dell'interfaccia.

1. Tracciare il valore dalla selezione del preset in `ui/settings.rs`, al salvataggio/applicazione delle preferenze, fino al clear effettivo della texture GPU della viewport.
2. Verificare se `ViewportRenderer::render` usa davvero `bg` per il colore di clear e se il clear avviene su una texture sRGB con la corretta conversione. Controllare anche alpha/blending, eventuali passaggi successivi che sovrascrivono il fondo e il percorso della viewport mostrata a schermo (non solo STEP preview/capture).
3. Aggiungere un test che controlli il colore passato/usato dal renderer e, se possibile, un test di rendering/readback per i quattro preset e un colore personalizzato.
4. Verificare visivamente in esecuzione almeno `#282C34` e `#353A42`: lo sfondo libero della viewport deve risultare coerente con il campione/valore RGB/HEX, senza essere quasi nero. Verificare anche che il render finale non cambi.
5. Aggiornare manuale/riscontro solo dopo la verifica reale. Riportare causa precisa, file modificati, test eseguiti e commit.

**Non considerare risolto** con i soli test di persistenza delle preferenze: il bug segnalato riguarda la resa GPU reale.


## Correzione sfondo viewport: colore effettivo non corrisponde alla preferenza — 09/11/2026

### Problema osservato da Valerio

Nella schermata Impostazioni → Viewport è selezionato **Grigio chiaro**, RGB 53, 58, 66 (#353A42), ma lo sfondo effettivamente visibile nella viewport rimane quasi nero. Dallo screenshot fornito, una zona libera della viewport misura circa RGB (9, 11, 14), quindi il colore mostrato non è quello selezionato.

Non è accettabile che il selettore mostri un colore e che la viewport ne disegni un altro. Non aggiungere workaround, overlay o correzioni approssimative: individuare la causa reale e correggere il punto responsabile.

### Indagine richiesta prima della modifica

Tracciare l'intero percorso del colore dalla preferenza UI fino al pixel finale della viewport:
1. valore selezionato nel pannello Impostazioni e relativa persistenza;
2. stato/configurazione passato alla viewport e aggiornamento immediato quando si cambia preset;
3. codice che disegna il fondale della viewport (render pass GPU, clear color, renderer/texture e compositing);
4. eventuali sfondi separati usati da viewport, anteprima STEP, modalità Semplice o altre modalità;
5. eventuali valori hardcoded, fallback o passaggi successivi che sovrascrivono il colore scelto.

Controllare in particolare il percorso di rendering effettivo della viewport e il suo render pass/clear color: il fatto che il selettore salvi e mostri correttamente il valore non dimostra che il renderer lo stia usando.

### Correzione richiesta

- Stabilire **un'unica sorgente di verità** per il colore di sfondo selezionato.
- Il renderer che produce lo sfondo visibile della viewport deve ricevere e usare esattamente quel colore, convertendolo correttamente nel formato/spazio colore richiesto dal renderer.
- Eliminare o collegare alla stessa sorgente eventuali colori hardcoded, fallback o sfondi paralleli che causano divergenze.
- Il cambio preset/colore personalizzato deve aggiornare immediatamente il fondale effettivamente disegnato, senza riavvio.
- La viewport deve mostrare il colore selezionato in modo uniforme in tutte le modalità e dimensioni della finestra.
- Se l'anteprima STEP usa un percorso grafico distinto, verificare che riceva la stessa preferenza e produca lo stesso colore di sfondo.
- Non cambiare il colore del render finale: la modifica riguarda esclusivamente lo sfondo della viewport e le sue anteprime.

### Verifiche obbligatorie

Per ciascuno dei quattro preset e per un colore personalizzato:
1. selezionare il colore nelle Impostazioni;
2. verificare il colore effettivo dei pixel in una zona libera della viewport, lontano da oggetti, gizmo e overlay;
3. confrontare il valore effettivo con RGB/HEX selezionato, ammettendo solo la tolleranza dovuta alla conversione colore documentata;
4. ripetere in viewport normale, modalità Semplice e anteprima STEP se applicabile;
5. cambiare preset più volte e verificare che non restino valori precedenti o cache non aggiornate;
6. salvare, riaprire il programma e verificare che il colore effettivo corrisponda alla preferenza persistita.

Aggiungere test automatici per la propagazione del valore dalla preferenza alla configurazione del renderer, ma non considerarli sostitutivi della verifica visiva/pixel della viewport reale.

### Riscontro richiesto a Claude

Prima descrivere la causa individuata indicando **file, funzioni e passaggio preciso** in cui il colore viene perso/sovrascritto o non applicato. Poi descrivere la correzione, i file modificati e i test effettivamente eseguiti. Non dichiarare risolto il problema senza aver verificato il colore effettivamente disegnato.


## Griglia metrica configurabile della viewport — 09/11/2026

### Richiesta di Valerio

Aggiungere alla viewport una griglia metrica 3D ispirata a Blender, con quadrati in scala reale e impostazioni configurabili dall'utente. La griglia deve aiutare a valutare dimensioni, distanze e proporzioni degli oggetti.

### Funzioni richieste

- Visibilità attivabile/disattivabile dalle impostazioni.
- Selezione dell'unità di misura: millimetri (mm), centimetri (cm), metri (m), coerente con il sistema di unità della scena se già presente.
- Selezione della dimensione reale della cella: preset utili (ad esempio 1, 5, 10, 20, 50, 100 mm; equivalenti in cm/m) e valore personalizzato.
- Selezione dell'intervallo delle linee principali (ad esempio ogni 5, 10 o 20 celle).
- Distinzione visiva tra linee secondarie e principali, con assi e origine riconoscibili.
- Eventuale regolazione dell'intensità/contrasto, mantenendo la griglia discreta rispetto agli oggetti.
- Preferenze persistenti e aggiornamento immediato della viewport.

### Comportamento grafico e tecnico

1. La griglia deve essere ancorata alle coordinate del mondo 3D e al piano di riferimento/ground plane già usato dalla viewport, non alla telecamera.
2. La spaziatura rappresenta distanze reali della scena: zoom e movimento camera non devono modificarne la dimensione nel mondo.
3. Adeguare la leggibilità allo zoom: quando le celle diventano troppo fitte o troppo piccole sullo schermo, attenuare/nascondere le suddivisioni minori e mantenere leggibili le linee principali, senza cambiare la scala reale impostata.
4. Gestire correttamente la prospettiva e le viste ortografiche, se supportate.
5. La griglia deve restare dietro agli oggetti e non interferire con selezione, gizmo, assi, manipolazione o picking.
6. La griglia è un ausilio della viewport: non deve comparire nel rendering finale, né alterare illuminazione, materiali o calcoli del render.
7. Usare il sistema di unità e il piano di riferimento esistenti, evitando di introdurre una seconda convenzione incoerente. Se ForgeRender non ha una scala fisica della scena definita, chiarire nelle impostazioni che le unità sono unità di scena e non millimetri assoluti garantiti.

### Impostazioni suggerite

Sezione Impostazioni → Viewport, accanto alle preferenze dello sfondo:
- Mostra griglia (toggle)
- Unità (mm/cm/m o unità scena)
- Dimensione cella (preset + personalizzata)
- Linea principale ogni N celle
- Intensità/contrasto (facoltativo)

### Verifiche richieste

- Controllare che la distanza tra linee corrisponda alla scala impostata in coordinate mondo.
- Verificare zoom avanti/indietro, rotazione camera, prospettiva e ortografica.
- Verificare griglia attiva/disattiva e persistenza delle preferenze.
- Verificare che la griglia non compaia nel render finale e non interferisca con picking, selezione e gizmo.
- Provare celle piccole e grandi (ad esempio 1 mm, 10 mm, 10 cm, 1 m) e controllare leggibilità e adattamento delle linee.

Prima di implementare, ispezionare il sistema coordinate/unità e il disegno della griglia già esistente. Riutilizzare o estendere quello attuale se possibile; evitare di sovrapporre due griglie. Nel riscontro indicare file/funzioni interessati, scelte tecniche, test effettivamente eseguiti e commit. Lavorare sul branch sviluppo.


## Estendere sfondo e griglia anche alla finestra di importazione STEP — 09/11/2026

### Richiesta aggiuntiva di Valerio

La griglia metrica configurabile richiesta per la viewport deve essere visualizzata anche nella finestra di importazione/anteprima dei file STEP. Anche lo sfondo della finestra di anteprima STEP deve essere **identico** a quello scelto per la viewport.

### Comportamento richiesto

- Usare nella preview STEP lo stesso colore di sfondo effettivo della viewport, letto dalla medesima preferenza/sorgente di verità. Non introdurre un colore STEP separato, un valore hardcoded o una variante approssimativa.
- Mostrare nella preview STEP la stessa griglia metrica della viewport: stessa visibilità, unità, dimensione reale delle celle, intervallo delle linee principali, colori/contrasto e impostazioni di intensità.
- Le impostazioni devono essere condivise: modificando sfondo o griglia in Impostazioni → Viewport, sia la viewport principale sia la preview STEP devono aggiornarsi coerentemente, senza riavvio.
- La griglia nella preview STEP deve essere riferita alle coordinate e al piano di riferimento della preview, con scala reale coerente con quella della viewport; non deve essere un'immagine sovrapposta o una griglia fissa rispetto alla finestra.
- Mantenere la griglia dietro al modello STEP e non interferire con rotazione, zoom, selezione o controlli della preview.
- Sfondo e griglia restano esclusivamente elementi di visualizzazione: non devono essere importati nel modello STEP né comparire nel rendering finale.

### Verifiche richieste

1. Confrontare viewport e preview STEP usando ciascun preset di sfondo: il colore delle aree libere deve corrispondere.
2. Confrontare griglia attiva/disattiva, dimensione cella e linee principali: impostazioni e scala devono coincidere.
3. Cambiare impostazioni mentre la finestra STEP è aperta e verificare l'aggiornamento immediato, se la finestra è progettata per restare aperta.
4. Verificare che la griglia resti ancorata al mondo durante zoom e rotazione del modello STEP.
5. Verificare che né griglia né sfondo entrino nel render finale o nei dati importati.

Prima di implementare, controllare il renderer dedicato alla preview STEP (già descritto nell'handoff come basato su ViewportRenderer con texture propria) e riutilizzare lo stesso percorso/configurazione di sfondo e griglia della viewport, evitando due implementazioni divergenti. Questa richiesta si aggiunge alla correzione già annotata per il disallineamento tra colore selezionato e colore realmente disegnato: prima risolvere la causa di quel disallineamento, poi assicurarsi che viewport e preview STEP condividano lo stesso risultato. Riportare file/funzioni modificati e test effettivi. Lavorare su sviluppo.


## Correzione selezione del preset Personalizzato per lo sfondo viewport — 09/11/2026

### Problema segnalato da Valerio

In Impostazioni → Viewport, cliccando su **Personalizzato**, la modalità non viene selezionata immediatamente: diventa selezionabile/attiva soltanto dopo che l'utente modifica manualmente il colore. Dopo aver scelto un preset predefinito, inoltre, può non essere più possibile tornare a Personalizzato con un semplice clic: sembra necessario modificare di nuovo il colore manualmente.

### Comportamento richiesto

- Il clic su **Personalizzato** deve selezionare immediatamente la modalità Personalizzato, anche se il colore non viene modificato.
- Quando si entra in Personalizzato, mantenere e mostrare il colore personalizzato già memorizzato; se non esiste ancora un valore personalizzato, inizializzarlo in modo deterministico (ad esempio dal colore attualmente visualizzato), senza obbligare l'utente a intervenire sul selettore.
- La selezione della modalità e la modifica del valore RGB/HEX sono due azioni distinte: il cambio di modalità non deve dipendere da un evento di modifica del colore.
- Dopo aver selezionato un preset predefinito, l'utente deve poter tornare a Personalizzato con un solo clic, senza dover prima cambiare manualmente RGB/HEX.
- Passare da Personalizzato a un preset e poi nuovamente a Personalizzato deve conservare il valore personalizzato precedente, salvo che l'utente lo modifichi esplicitamente.
- La selezione deve aggiornare subito il colore effettivo della viewport e, una volta condivisa la configurazione, anche quello della preview STEP; non deve essere necessario riaprire le impostazioni o riavviare il programma.
- La selezione visiva del radio/preset deve sempre corrispondere alla modalità realmente applicata.

### Indagine e test

Controllare la gestione dello stato UI: verificare che il radio button Personalizzato non sia legato impropriamente al confronto tra colore corrente e colore custom, né che la callback venga invocata soltanto quando cambia RGB/HEX. Separare esplicitamente `background_mode` (preset/custom) dal valore `custom_background_color`.

Testare la sequenza completa: preset A → Personalizzato senza toccare il colore → preset B → Personalizzato con un solo clic → modifica colore custom → preset A → Personalizzato. Verificare stato selezionato, valore conservato, colore effettivo nella viewport e nella preview STEP. Aggiungere test automatici per le transizioni di stato e non dichiarare risolto il problema senza aver provato la sequenza nell'interfaccia.
