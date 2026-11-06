# ForgeRender — finestra dedicata per importazione STEP

**Richiesta di Valerio — 06/11/2026**

## Riferimento visivo

Screenshot di riferimento fornito in questa conversazione: **“demo impostazione step.jpg”** (CAD Converter Pro, schermata STEP File Import / Mesh Conversion / 3D Preview).

> Nota operativa: il file immagine originale è presente come allegato nella conversazione ChatGPT, ma non è stato possibile inserirne il binario nel repository GitHub con gli strumenti disponibili in questa sessione. Non considerare quindi questo documento come prova che l'immagine sia già stata caricata nel repository. La schermata va allegata/cop iata nella cartella di scambio quando si trasferisce il task a Claude Code.

## Obiettivo

Valerio vuole che l'importazione dei file **STEP** in ForgeRender avvenga tramite una finestra/pannello dedicato, ispirato al layout dello screenshot, invece di affidarsi a una conversione poco configurabile o a impostazioni nascoste.

La finestra deve permettere di controllare i parametri di tessellazione e conversione prima di creare la mesh finale, mostrando contemporaneamente una preview del risultato.

## Struttura UX desiderata

Prendere come riferimento la suddivisione in tre colonne dello screenshot:

1. **Geometria sorgente**
   - Nome del file STEP selezionato e dimensione.
   - Anteprima della geometria CAD/B-rep originale, se disponibile.
   - Informazioni utili sul modello: unità, dimensioni/bounding box, numero di solidi/componenti e stato dei controlli geometrici, se ricavabili.
   - Impostazioni di importazione: unità (mm, cm, m, pollici) e asse verticale/up-axis, quando pertinenti.
2. **Tessellazione e conversione**
   - Parametri di qualità della conversione B-rep → mesh.
   - Tolleranza di deviazione (scostamento massimo dalla superficie CAD).
   - Tolleranza angolare/superficiale, se supportata dalla libreria STEP.
   - Lunghezza massima degli spigoli, se implementabile in modo corretto.
   - Opzione per preservare gli spigoli vivi, se supportata.
   - Pulsanti per annullare, ripristinare i valori predefiniti e avviare la conversione.
   - Durante la conversione: indicatore di avanzamento e stato leggibile; evitare percentuali inventate se il backend non espone progressi reali.
3. **Anteprima 3D**
   - Preview interattiva della mesh tessellata, preferibilmente con modalità shaded e wireframe.
   - Controlli di illuminazione, zoom e rotazione.
   - Riepilogo della mesh convertita: triangoli, vertici e, se disponibili, warning/errori.

## Comportamento richiesto

- La conversione non deve partire definitivamente prima che l'utente abbia confermato i parametri.
- L'utente deve poter modificare i parametri e rigenerare la preview senza dover reimportare manualmente il file.
- I valori devono avere unità esplicite e tooltip che spieghino l'effetto pratico dei parametri.
- Prevedere preset di qualità (ad esempio Bozza, Bilanciata, Alta qualità) solo se mappabili su parametri tecnici reali e documentati.
- Gestire file non validi, geometrie vuote, superfici problematiche e conversioni fallite con messaggi comprensibili.
- Per modelli STEP grandi, valutare conversione asincrona e possibilità di annullamento senza bloccare l'interfaccia.
- Conservare il flusso attuale per gli altri formati: questa richiesta riguarda specificamente STEP.
- Verificare come i parametri disponibili nel convertitore STEP attuale corrispondono ai controlli UI: non mostrare controlli che il backend ignora.

## Indicazioni per Claude Code

Prima di implementare:
1. Ispezionare il flusso STEP esistente, il processo/helper `forgerender-step.exe`, i parametri OCCT realmente esposti e il modo in cui la mesh viene importata nell'app.
2. Elencare i parametri realmente configurabili e distinguere quelli supportati da quelli che richiederebbero modifiche al convertitore.
3. Proporre una struttura UI coerente con egui e con lo stile attuale di ForgeRender, mantenendo la finestra utilizzabile anche con risoluzioni ridotte.
4. Non replicare alla lettera il branding o gli elementi grafici di CAD Converter Pro: usare lo screenshot solo come riferimento per layout, gerarchia e flusso.
5. Implementare preview e progress reporting solo nei limiti dei dati effettivamente forniti dal backend.
6. Aggiungere test per mapping dei parametri, conversione e gestione degli errori; riportare i comandi eseguiti e ciò che non è stato testato.

**Risultato atteso:** una finestra professionale di importazione STEP, con impostazioni di tessellazione accessibili e preview 3D del risultato prima dell'importazione definitiva.
