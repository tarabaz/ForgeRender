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

_In attesa di implementazione e riscontro._

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

### Richiesta di Valerio

La funzione Environment carica una scenografia composta da fondale e luci. La gestione delle luci deve evitare sia la perdita delle luci originali della scena sia l'accumulo di luci quando si cambia scenografia.

### Comportamento richiesto

**Primo caricamento di una scenografia**
1. Se nella scena sono già presenti luci dell'utente, disattivarle (non eliminarle e non modificarne i parametri).
2. Caricare e attivare tutte le luci appartenenti alla scenografia selezionata.
3. Le luci originali devono restare nella scena, conservate e riattivabili dall'utente.

**Cambio da una scenografia a un'altra**
1. ForgeRender deve distinguere in modo affidabile le luci create/caricate da Environment dalle luci preesistenti dell'utente.
2. Le luci della scenografia precedente vanno riutilizzate e aggiornate con quelle della nuova scenografia, senza crearne copie a ogni cambio.
3. Aggiornare tutti i parametri di ciascuna luce: tipo, posizione, orientamento/rotazione, colore, intensità/potenza, dimensioni e ogni altra proprietà supportata dal modello luce.
4. Se la nuova scenografia contiene più luci della precedente, aggiungere soltanto quelle mancanti.
5. Se contiene meno luci, disattivare le luci Environment in eccesso della scenografia precedente. Non eliminarle, così da mantenere una gestione reversibile e coerente.
6. Non modificare né disattivare le luci dell'utente durante i cambi successivi di Environment: la sostituzione deve riguardare esclusivamente le luci riconosciute come appartenenti alla scenografia.

### Aspetti tecnici da verificare

- Ispezionare il codice reale del caricamento Environment, la struttura delle luci e il sistema di identificazione/serializzazione.
- Introdurre, se necessario, un'identificazione persistente e non ambigua delle luci gestite da Environment (ad esempio un gruppo/ruolo o metadato dedicato). Non basarsi soltanto sul nome visibile della luce.
- Il caricamento ripetuto della stessa scenografia deve essere idempotente: non deve generare duplicati.
- Verificare cosa accade quando si carica un Environment dopo aver modificato manualmente una luce della scenografia: il cambio deve applicare integralmente i parametri della nuova scenografia.
- Verificare anche il caso di una scenografia senza luci, se ammesso dal formato.
- Conservare il comportamento del fondale e non alterare le altre luci/oggetti della scena.

### Test richiesti

Preparare test che coprano almeno:
1. Prima applicazione: le luci preesistenti vengono disattivate ma conservate; le luci Environment vengono caricate e attivate.
2. Cambio con meno luci: le luci Environment in eccesso vengono disattivate, senza eliminazione.
3. Cambio con più luci: vengono aggiunte soltanto le nuove luci necessarie.
4. Aggiornamento completo dei parametri, inclusi posizione e orientamento.
5. Le luci originali dell'utente restano invariate nei cambi successivi.
6. Ricaricare lo stesso Environment non crea duplicati.

Lavorare sul branch di sviluppo. Nel riscontro finale indicare file modificati, strategia usata per identificare le luci Environment, test realmente eseguiti e commit. Non dichiarare come verificato ciò che non è stato provato.

## Chiarimento e revisione della strategia luci Environment — 09/11/2026

### Decisione preferita da Valerio e raccomandazione di ChatGPT

Valutata l'alternativa, si preferisce gestire le luci della scenografia come elementi di proprietà dell'Environment, con un ciclo di vita legato alla scenografia attualmente caricata. Questa strategia **sostituisce, per i cambi di Environment, la precedente indicazione di riutilizzare le luci e disattivare quelle in eccesso**.

Comportamento aggiornato:

1. Al primo caricamento di un Environment, disattivare le luci già presenti nella scena, senza eliminarle né modificarne i parametri: sono luci originali dell'utente.
2. Creare le luci appartenenti al nuovo Environment e attivarle.
3. Nell'albero della scena mostrare un suffisso chiaro, ad esempio `[ENV]`, accanto al nome di ogni luce gestita dalla scenografia, così che l'utente possa riconoscerle immediatamente.
4. Quando si cambia Environment, eliminare **soltanto** le luci appartenenti all'Environment precedente e creare quelle della nuova scenografia con tutti i parametri previsti (tipo, posizione, orientamento, colore, potenza, dimensioni e ogni altra proprietà supportata).
5. Le luci originali dell'utente, prive del contrassegno Environment, non devono essere eliminate o modificate nei cambi successivi.
6. Se il nuovo Environment contiene più o meno luci, il numero di luci Environment presenti dopo il caricamento deve corrispondere esattamente a quello previsto dalla nuova scenografia: nessun residuo e nessun duplicato.
7. Ricaricare lo stesso Environment deve produrre lo stesso risultato, senza accumulare luci.

### Identificazione e sicurezza

Il suffisso `[ENV]` nell'albero è un'indicazione visiva per l'utente, **non deve essere l'unico criterio tecnico per decidere quali luci eliminare**. Usare un metadato/ruolo interno dedicato e persistente per identificare in modo affidabile le luci create dal caricamento Environment. Il nome visualizzato può includere il suffisso, ma non deve essere la fonte di verità.

Verificare anche salvataggio e riapertura del progetto: l'appartenenza delle luci all'Environment deve restare riconoscibile. Se il formato/progetto non conserva un metadato adatto, individuare una soluzione compatibile prima di implementare la cancellazione.

### Test aggiornati

- Primo caricamento: luci preesistenti disattivate e conservate; luci Environment create, identificate e attive.
- Cambio Environment: vengono eliminate solo le vecchie luci Environment e create quelle nuove.
- La scena utente non perde né modifica le proprie luci durante i cambi.
- Cambio da Environment con molte luci a uno con poche e viceversa: nessun residuo.
- Tutti i parametri delle nuove luci corrispondono alla nuova scenografia, inclusi posizione e orientamento.
- Suffisso `[ENV]` visibile nell'albero e metadato interno coerente.
- Salvataggio e riapertura del progetto preservano l'identificazione.
- Ricaricamento dello stesso Environment senza duplicati.

**Nota per Claude:** questa è la strategia preferita e aggiorna le indicazioni precedenti sul cambio di scenografia. Prima di implementare, controlla come il modello scena gestisce identificatori, eliminazione delle luci, persistenza e nomi nell'albero; poi scegli la soluzione più sicura senza affidarti al solo suffisso.
