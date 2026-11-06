# ForgeRender — Modifiche ad animazione, simulazioni e salvataggio video

Data: 06/11/2026  
Tipo: richiesta tecnica di Valerio a Claude Code  
Stato: da analizzare e implementare  
Branch di riferimento: sviluppo

## Obiettivo

Raccogliere in questa nota le modifiche richieste da Valerio relative ad animazioni, simulazioni, Particle System e configurazione dell'esportazione video. La nota è volutamente più ampia del solo Particle System: le richieste future sul sistema di animazione potranno essere aggiunte qui.

## 1. Fumo: salvare anche la griglia fine

Il sistema esistente salva già dati temporanei del fumo e consente la ripresa dal primo fotogramma modificato. Estendere il precalcolo affinché salvi su disco anche la griglia fine utilizzata per il risultato.

- Calcolare e salvare tutti i dati necessari per ogni fotogramma, inclusa la griglia fine.
- Il render dell'animazione deve leggere e riutilizzare i dati precalcolati, senza ricostruire o ricampionare la griglia fine per ogni frame.
- Conservare riuso dei frame invariati e ripresa dal primo frame realmente modificato.
- Invalidare la cache quando cambiano parametri, sorgenti, geometrie, trasformazioni o altri input che influenzano la simulazione.
- Gestire versione del formato, integrità, cache incompleta/corrotta e pulizia dei temporanei.
- Esaminare e riutilizzare, dove sensato, l'infrastruttura esistente in `crates/forge_render_core/src/smoke_bake.rs`.

## 2. Particle System: interazione con oggetti animati

Problema osservato da Valerio: il Particle System si anima nella viewport, ma durante un'animazione con oggetti in movimento le particelle sembrano non interagire con essi. Non si osservano collisioni, spinte o deviazioni coerenti con il movimento degli oggetti. Il fumo nel rendering, invece, sembra reagire agli oggetti animati.

Prima di modificare:
- Riprodurre il problema con una scena minima e deterministica.
- Seguire il percorso della simulazione: aggiornamento temporale, trasformazioni degli oggetti, collisioni/interazioni, viewport e render.
- Individuare la causa effettiva; non presumere che sia un semplice problema di visualizzazione.
- Verificare se viewport e render utilizzano lo stesso stato simulato o percorsi distinti.

Comportamento richiesto:
- Le particelle devono interagire con gli oggetti animati usando la trasformazione effettiva degli oggetti a ogni istante della timeline.
- Gestire le interazioni già supportate dal sistema (collisione, spinta, deviazione o trascinamento dell'aria), senza introdurre proprietà superflue.
- Garantire coerenza tra anteprima viewport e rendering dell'animazione.
- Non limitarsi a riprodurre particelle in movimento indipendenti dalla scena.

## 3. Precalcolo e cache del Particle System

Valutare e implementare un precalcolo temporale persistente anche per il Particle System, se non esiste già.

- Salvare gli stati necessari a riprodurre le particelle nei vari frame, incluse le interazioni con gli oggetti animati.
- Fare in modo che viewport e render leggano gli stessi risultati precalcolati.
- Se cambia un oggetto che influenza la simulazione, invalidare dal primo frame interessato in avanti, preservando i precedenti quando possibile.
- Non invalidare inutilmente la simulazione per modifiche che non ne influenzano la fisica.
- Gestire riapertura del progetto, cache mancante/incompleta, versionamento e pulizia.
- Valutare il riuso dei meccanismi comuni di `smoke_bake.rs` (impronte, cache, ripresa, invalidazione), mantenendo distinti i dati e le logiche specifiche delle particelle.

## 4. Impostazioni complete di esportazione video

Attualmente l'esportazione dell'animazione produce un MP4 con impostazioni scelte automaticamente dal programma. Valerio vuole poter decidere formato e qualità del file, senza essere vincolato a una compressione aggressiva o a un unico MP4 predefinito.

Analizzare prima il percorso di esportazione attuale, le librerie/encoder disponibili e le dipendenze del progetto. Poi aggiungere un pannello o una finestra di impostazioni video accessibile prima di avviare il salvataggio dell'animazione.

### Impostazioni che l'utente deve poter controllare

- **Contenitore/formato:** almeno MP4 e AVI; valutare MOV e MKV se supportati in modo affidabile dall'encoder disponibile.
- **Codec:** mostrare soltanto codec realmente disponibili per il formato scelto. Non confondere contenitore e codec: AVI, per esempio, può contenere video non compresso oppure codec lossless.
- **Qualità/compressione:** offrire preset chiari, ad esempio:
  - dimensione ridotta / compressione alta;
  - qualità alta;
  - qualità molto alta / compressione leggera;
  - lossless, quando supportato;
  - non compresso, quando supportato.
- **Bitrate:** possibilità di impostare il bitrate (almeno per codec che lo supportano), con indicazione o stima della dimensione finale.
- **Qualità del codec:** esporre il parametro CRF o equivalente quando pertinente, spiegandone il significato e senza mostrare controlli non applicabili al codec selezionato.
- **Pixel format e profondità colore:** esporre le opzioni realmente supportate, ad esempio 8/10 bit quando disponibili; evitare conversioni di qualità non dichiarate.
- **Frame rate:** usare il frame rate della timeline come valore iniziale, ma permettere di controllarlo se l'architettura lo consente senza alterare in modo ambiguo la temporizzazione.
- **Risoluzione:** mantenere la risoluzione del render come valore iniziale e consentire di scegliere una risoluzione di esportazione solo se esiste un ridimensionamento corretto e documentato.
- **Percorso e nome file:** scelta esplicita della destinazione e dell'estensione coerente con il contenitore selezionato.
- **Preset salvabili:** almeno preset predefiniti sensati e possibilità di ricordare l'ultima configurazione usata.

### Caso d'uso prioritario: AVI non compresso e compressione meno aggressiva

Valerio vuole poter esportare, per esempio:
- un AVI non compresso, se supportato;
- un AVI con codec lossless (valutare FFV1 o alternativa effettivamente disponibile);
- un MP4 con qualità elevata e compressione meno aggressiva rispetto al preset attuale;
- eventualmente un formato intermedio ad alta qualità per ulteriori lavorazioni in software di montaggio.

Non dare per scontato che ogni codec sia disponibile. Verificare l'encoder effettivo e le combinazioni contenitore/codec supportate. Se AVI non compresso o FFV1 non sono disponibili nell'attuale backend, spiegare il motivo e valutare l'integrazione minima necessaria, senza introdurre dipendenze pesanti o binari esterni senza motivazione.

### UX e protezione da errori

- Mostrare una breve descrizione comprensibile di ogni preset, indicando il compromesso qualità/dimensione.
- Avvisare quando un formato non supporta una particolare opzione.
- Non applicare silenziosamente un codec o una qualità diversi da quelli scelti.
- Gestire correttamente errori di encoder, spazio disco insufficiente, interruzione e file parziali: non lasciare un output apparentemente valido se l'esportazione è fallita.
- Se possibile, mostrare una stima della dimensione prima dell'esportazione, distinguendo stima da valore garantito.
- Conservare invariati i frame renderizzati: la scelta del codec deve riguardare la codifica del video e non modificare il rendering della scena.

### Criteri di accettazione video

1. Prima dell'esportazione è possibile scegliere formato/contenitore e codec compatibile.
2. È possibile selezionare un MP4 ad alta qualità con compressione meno aggressiva del preset attuale.
3. È possibile esportare AVI non compresso, se supportato, oppure un'alternativa lossless chiaramente identificata.
4. Le impostazioni non applicabili al codec selezionato sono disabilitate o nascoste, non ignorate silenziosamente.
5. Il file risultante è riproducibile e la sua estensione corrisponde al contenitore reale.
6. Vengono riportati codec, pixel format, frame rate, risoluzione e parametri di qualità effettivamente usati.
7. Un errore o un'interruzione non lascia un file incompleto presentato come esportazione riuscita.
8. Il percorso di esportazione standard continua a funzionare e mantiene un preset predefinito ragionevole.

## 5. Analisi architetturale richiesta

Prima dell'implementazione, descrivere brevemente:
- dove vengono eseguite oggi le simulazioni di fumo e particelle;
- come vengono valutate le trasformazioni degli oggetti a ogni frame;
- come sono gestite collisioni e interazioni;
- quali dati sono già salvati e quali ricostruiti;
- come viewport e render recuperano lo stato simulato;
- quale backend/encoder produce attualmente il video, quali contenitori e codec supporta e quali parametri espone.

Evitare sistemi di cache duplicati se è possibile condividere il meccanismo comune. Non compromettere la reattività dell'anteprima durante la normale modifica della scena e non cambiare il risultato del rendering per effetto delle sole impostazioni di codifica video.

## 6. Criteri di accettazione generali

1. Il precalcolo del fumo salva anche la griglia fine e il render la riutilizza senza ricostruirla per ogni frame.
2. Una modifica ai parametri del fumo invalida dal primo frame interessato, non prima.
3. Il Particle System reagisce visibilmente agli oggetti animati lungo la timeline.
4. Viewport e render mostrano la stessa evoluzione temporale delle particelle.
5. Una modifica alla traiettoria o geometria di un oggetto interagente ricalcola soltanto la parte dipendente dalla modifica.
6. I dati precalcolati restano riutilizzabili dopo la riapertura del progetto, se validi.
7. Dimensioni e pulizia della cache sono controllate.
8. Nessuna regressione alla simulazione normale del fumo o all'anteprima delle particelle.
9. L'esportazione video consente di scegliere consapevolmente formato e qualità, inclusi percorsi ad alta qualità/lossless quando supportati.

## 7. Test richiesti

Creare una scena riproducibile con emettitore, oggetto animato che attraversa la traiettoria delle particelle e prova di controllo con oggetto fermo. Includere anche una simulazione di fumo su più frame con griglia fine.

Verificare per le simulazioni: primo precalcolo; riproduzione da cache; modifica parametri e invalidazione; modifica della traiettoria e aggiornamento delle interazioni; confronto viewport/render; chiusura e riapertura progetto; cache parziale o non valida.

Per il video, esportare almeno brevi sequenze di prova con:
- MP4 a preset standard;
- MP4 a qualità elevata/bitrate più alto o CRF più basso, secondo l'encoder;
- AVI non compresso, se disponibile;
- AVI lossless con FFV1 o codec equivalente, se disponibile.

Verificare con uno strumento di ispezione dei media i parametri effettivi del file prodotto (contenitore, codec, pixel format, frame rate, risoluzione e bitrate), oltre alla riproduzione del file. Confrontare dimensioni e qualità, senza dichiarare equivalenti impostazioni diverse.

I test unitari non bastano per dichiarare risolti i problemi: effettuare anche una verifica visiva reale dell'animazione in viewport e del render, e aprire i video esportati in un player o software di montaggio, se l'ambiente lo consente.

## 8. Risposta richiesta a ChatGPT

Aggiungere al normale handoff una risposta datata con:
- causa effettiva del problema del Particle System;
- comportamento precedente e comportamento ottenuto;
- file modificati e motivazione;
- architettura e formato della cache per fumo e particelle;
- strategia di invalidazione e ripresa;
- backend video, contenitori, codec e opzioni effettivamente supportati;
- impostazioni predefinite e comportamento dei preset video;
- test e comandi realmente eseguiti e relativi risultati;
- verifica visiva svolta oppure esplicitamente non eseguita;
- dimensioni dei temporanei, dimensioni dei video e tempi di precalcolo/esportazione, se misurati;
- limiti residui e attività aperte.

Questa nota è una richiesta di analisi e implementazione, non una dichiarazione che le funzionalità siano già state realizzate o verificate. Distinguere fatti osservati, ipotesi, test eseguiti e attività ancora da fare.
