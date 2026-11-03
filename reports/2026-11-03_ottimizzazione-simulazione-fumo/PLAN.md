# ForgeRender — piano di accelerazione della simulazione fumo/nebbia

**Data:** 2026-11-03  
**Destinatario:** Claude Code  
**Priorità:** alta — prossimo step tecnico principale  
**Branch di riferimento:** `main`  
**Commit di riferimento verificato:** `13f5bf3d9c2d69f3b0219a675ed01c2206782168`

## Obiettivo richiesto da Valerio

La generazione della nebbia da ghiaccio secco è troppo lenta, soprattutto alle qualità alte. Non basta ottenere un miglioramento marginale o un semplice raddoppio: l'obiettivo è una riduzione **di parecchie volte** del tempo di calcolo, mantenendo un risultato visivo comparabile e senza trasformare la qualità alta in una simulazione palesemente più povera.

**Target di progetto (da misurare, non da dichiarare raggiunto in anticipo):**
- obiettivo minimo per considerare riuscito il lavoro: **almeno 5×** più veloce sulla scena di riferimento a parità di qualità visiva;
- obiettivo ambizioso: **8–10× o oltre**;
- il solo riuso della cache non conta come accelerazione della prima generazione;
- se il target non è raggiunto, riportare il miglior rapporto misurato e spiegare il collo di bottiglia residuo. Non degradare silenziosamente il risultato per inseguire il numero.

Il confronto deve usare la stessa scena, gli stessi parametri, seed, hardware e criteri di qualità. Nessun numero di speedup va scritto come risultato senza benchmark ripetibile.

## Stato del codice verificato su `main`

File principali letti:
- `crates/forge_volume/src/sim.rs`
- `crates/forge_volume/src/lib.rs`
- `crates/forge_render_core/src/fog.rs`

Fatti osservati:
1. La simulazione usa una griglia MAC 3D con campi di velocità, densità del vapore e concentrazione del gas pesante.
2. Il passo comprende forze, trasporto semi-lagrangiano della velocità, proiezione della pressione e trasporto MacCormack limitato di densità/concentrazione.
3. La pressione usa Gauss–Seidel rosso/nero con **16 iterazioni** per passo (`PRESSURE_ITERS`).
4. Esiste già un riquadro di lavoro intorno alla nebbia (`WORK_MARGIN = 6`) e diverse funzioni limitano alcune operazioni al bounding box. Esiste anche `near_fog`, che calcola la vicinanza alle celle con densità.
5. Nonostante ciò, vari campi e buffer sono ancora allocati sulla griglia intera; occorre verificare, fase per fase, quante celle vengono realmente visitate e quanto costa il lavoro fuori dalla regione attiva.
6. La griglia di simulazione è circa il 70% della risoluzione impostata, limitata da `sim_cells` e `SIM_MAX`; il dettaglio viene poi cotto su una griglia più fine.
7. Il renderer ha cache in memoria per ostacoli, simulazione e volume finale. La cache non persiste tra riavvii e non accelera il primo calcolo.
8. La voxelizzazione delle collisioni è un costo separato e va misurato separatamente dalla fluid simulation e dal bake del dettaglio.

Questi punti descrivono il codice letto, non sono un nuovo benchmark. Prima di cambiare l'architettura, ricontrollare i file sul branch/commit effettivamente usato per l'implementazione.

## Direzione tecnica: intervenire sul costo reale, non solo sui preset

### 1. Profilazione e benchmark di riferimento — obbligatori prima delle modifiche

Aggiungere o migliorare una diagnostica che riporti separatamente:
- costruzione/voxelizzazione degli ostacoli;
- inizializzazione e sorgente;
- applicazione delle forze;
- trasporto della velocità;
- divergenza e soluzione della pressione;
- trasporto MacCormack dei campi scalari;
- calcolo del dettaglio e majorant;
- tempo totale, passi simulati, dimensioni griglia e numero di celle attive visitate.

Registrare anche allocazioni/byte principali, numero di celle attive per passo e percentuale della griglia totale. Evitare log per-cella che falsino i tempi.

Preparare una scena benchmark riproducibile (preferibilmente la demo cocktail/ghiaccio secco già presente), con seed e parametri fissi, a qualità Media, Alta, Molto alta e Ultra. Fare almeno 3 esecuzioni per configurazione e riportare mediana e dispersione. Salvare immagini di riferimento e confronti.

### 2. Sparse grid / active tiles — priorità massima

Valutare e implementare un dominio sparso a blocchi (tile 8³ o 16³, da scegliere con benchmark), attivando i blocchi che contengono nebbia o sono entro un margine di trasporto/forze.

Requisiti:
- non iterare né azzerare ogni passo l'intero volume se la nebbia occupa solo una piccola porzione;
- mantenere un halo sufficiente per interpolazione, advezione, pressione e collisioni;
- attivare nuovi tile quando la nebbia si espande e disattivare quelli vuoti solo dopo un margine temporale/spaziale, evitando popping;
- preservare le condizioni al contorno e la continuità tra tile;
- non usare una soglia di densità che tagli filamenti o code visibili;
- misurare il rapporto celle attive/totali e il costo di gestione della struttura sparsa.

Non limitarsi a creare una maschera sparsa ma continuare a scorrere tutti gli array densi: il vantaggio deve arrivare dal numero effettivo di operazioni e dalla memoria toccata.

### 3. Pressure solver più efficiente — priorità alta

La proiezione della pressione può dominare il costo perché viene ripetuta a ogni passo. Valutare:
- multigrid geometrico o metodo equivalente con convergenza più rapida;
- warm-start della pressione dal passo precedente;
- criterio di arresto basato sul residuo/divergenza, con limite massimo e minimo di iterazioni;
- soluzione sui soli tile attivi con halo coerente.

Non ridurre semplicemente le 16 iterazioni a un numero basso senza misurare residuo, divergenza e artefatti. Il risultato deve restare incomprimibile entro una tolleranza dichiarata. Confrontare tempo per passo e qualità del campo di velocità.

### 4. Advezione e aggiornamento dei campi

Profilare e ottimizzare i kernel di trasporto:
- evitare calcoli ripetuti di coordinate, indici e campionamenti trilineari;
- riusare valori e stencil quando matematicamente corretto;
- ridurre allocazioni temporanee e copie di buffer;
- valutare layout dei campi e parallelizzazione Rayon, verificando saturazione e overhead;
- conservare il limitatore MacCormack e i controlli anti-attraversamento pareti, salvo alternativa dimostrabilmente equivalente.

Un eventuale schema semi-Lagrangiano più economico deve essere confrontato visivamente e numericamente: non sostituire MacCormack con un trasporto molto più dissipativo senza una misura del dettaglio perso.

### 5. Risoluzione adattiva / livelli di dettaglio

Valutare una simulazione coarse-to-fine o una griglia adattiva:
- risoluzione più alta vicino a sorgente, fronti e vortici;
- risoluzione inferiore nelle zone uniformi o lontane;
- transizioni senza discontinuità, perdita di massa o cambi improvvisi della forma;
- mantenere un livello di qualità equivalente a quello attuale per le impostazioni High/Very High/Ultra.

La risoluzione adattiva deve ridurre davvero le celle aggiornate; una semplice variazione cosmetica della risoluzione non basta.

### 6. GPU compute — valutare come seconda grande leva

Il PC di riferimento dell'utente ha Ryzen 9 9900X e RTX 5070. Verificare se conviene portare i kernel più costosi su GPU tramite l'infrastruttura wgpu/Vulkan già presente, senza introdurre CUDA/OptiX obbligatori.

Prima di scegliere:
- stimare trasferimenti CPU↔GPU e sincronizzazioni;
- verificare supporto e limiti WGSL/wgpu per i solver necessari;
- confrontare GPU compute con sparse CPU: non assumere che GPU sia automaticamente più veloce;
- mantenere un fallback CPU funzionante e risultati deterministici/tolleranze definite.

Non tentare una riscrittura GPU completa come primo commit se la profilazione indica che sparse tiles + pressure solver possono già dare un guadagno molto maggiore con rischio inferiore. Ma non escludere la GPU senza una valutazione concreta.

### 7. Cache persistente e precalcolo — utile, ma non sostituisce l'ottimizzazione

La cache attuale è in memoria. Valutare una cache su disco versionata e invalidata tramite hash robusto di:
- parametri della simulazione;
- seed e versione dell'algoritmo;
- griglia e dimensioni;
- impronta delle collisioni/ostacoli nella zona.

Aggiungere eventualmente precalcolo asincrono e riuso del risultato. Scrittura atomica, gestione file corrotti, dimensione massima/cache cleanup e compatibilità tra versioni sono obbligatori.

Questa funzione migliora le riaperture e i render successivi, ma **non deve essere conteggiata** nel target di speedup della prima simulazione.

## Piano di implementazione richiesto

Non fermarti a una relazione teorica: dopo una breve verifica e un benchmark baseline, implementa in milestone piccole, testabili e misurabili, puntando al massimo guadagno reale.

1. **Baseline e profiling:** benchmark e report per fase; nessuna modifica visiva.
2. **Sparse active tiles:** implementazione concreta, test di espansione/halo/continuità e benchmark.
3. **Pressure solver:** warm start e/o multigrid o alternativa misurata; test di divergenza e confronto.
4. **Ottimizzazione advezione/allocazioni:** solo interventi supportati dal profiler.
5. **GPU/adaptive resolution:** implementare la strada che, dopo misure, promette il maggiore guadagno residuo con rischio ragionevole.
6. **Cache persistente/precalcolo:** separata dalle misure della prima generazione.

Claude può cambiare l'ordine se il profiling dimostra chiaramente che un'altra fase è il collo di bottiglia, ma deve spiegare la scelta e non saltare la misurazione.

## Criteri di accettazione

- Speedup della **prima generazione** misurato su hardware e scena di riferimento, a parità di parametri e qualità: target minimo 5×, obiettivo 8–10× o oltre.
- Mediana di almeno 3 run per configurazione; riportare tutti i tempi, non soltanto il migliore.
- Confronto immagini con seed identico e metriche numeriche (RMSE/MAE o metriche appropriate) più ispezione visiva di filamenti, dissipazione, contatto con superfici, vortici e conservazione della massa.
- Nessuna regressione nei test esistenti; test nuovi per sparse tiles, halo, confini, pressione e cache.
- Nessun crash, deadlock o uso memoria incontrollato su Ultra.
- CPU fallback mantenuto se si introduce GPU compute.
- Le qualità esistenti non devono diventare più lente; riportare separatamente ogni preset.
- Cache hit e cache miss misurati separatamente.
- Report finale con commit, file modificati, comandi eseguiti, test reali, benchmark prima/dopo, limiti e problemi aperti.

## Vincoli

- Non cambiare il comportamento o la scena demo dell'utente senza necessità.
- Non rimuovere funzionalità fisiche (collisioni, vortici, MacCormack) solo per ottenere un numero più alto.
- Non ridurre automaticamente la qualità richiesta dall'utente dietro le quinte.
- Non dichiarare raggiunto il target senza misure sul PC di riferimento; se i benchmark sono eseguiti solo in un container o su hardware diverso, specificarlo.
- Non fare merge su main o pubblicare una nuova release senza richiesta esplicita di Valerio.
- Documentare tutto in un report dedicato e aggiungere la risposta di Claude Code in fondo a `reports/chatgpt/CLAUDE_HANDOFF.md`.

## Domanda finale a Claude Code

Qual è il collo di bottiglia dominante misurato? Quale combinazione di sparse tiles, pressure solver, advezione, risoluzione adattiva e GPU offre il miglior rapporto tra speedup e rischio? Implementa il percorso scelto nel prossimo step importante e dimostra il guadagno con benchmark a qualità equivalente: **non fermarti a un 2× se c'è una strada concreta per ottenere un'accelerazione molto più grande**.
