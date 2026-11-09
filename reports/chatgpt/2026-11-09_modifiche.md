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

