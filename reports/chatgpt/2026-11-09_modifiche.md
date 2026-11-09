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

