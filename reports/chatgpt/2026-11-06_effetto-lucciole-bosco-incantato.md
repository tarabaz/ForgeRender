# ForgeRender — Nuovo effetto: Lucciole / Bosco incantato

Data: 06/11/2026
Tipo: nuova funzionalità richiesta da Valerio
Stato: da analizzare, progettare e implementare
Ambito: Particle System, illuminazione, animazione e rendering

## Obiettivo

Creare un nuovo effetto selezionabile, denominato **Lucciole** (preset estetico: **Bosco incantato**). Deve simulare uno stormo di lucciole che vola nello spazio tridimensionale con traiettorie irregolari, prevalentemente orizzontali, e si illumina in modo intermittente e asincrono.

Non deve essere un semplice sistema di particelle emissive con alone aggiunto in post-produzione. Le lucciole devono contribuire all'illuminazione della scena: quando si accendono, la loro luce deve influenzare realmente superfici e oggetti circostanti, nel percorso di illuminazione del renderer.

Il progetto possiede già particelle luminose/polvere luminosa e un sistema di alone/glow (Dust::glow, nodi effetto:polvere:luce:<n>, forge_render_core/src/glow.rs). Questi elementi vanno studiati e riutilizzati dove appropriato, ma il nuovo effetto deve risolvere esplicitamente la differenza tra emissione visibile/post-effetto e illuminazione reale della scena.

## 1. Comportamento delle lucciole

Ogni lucciola ha un ciclo indipendente:
1. vola nello spazio anche quando è spenta;
2. rimane invisibile durante la fase spenta;
3. inizia ad accendersi con una rampa graduale (glow-in);
4. raggiunge un'intensità luminosa variabile;
5. si spegne con una rampa graduale (glow-out);
6. continua a volare e attende un intervallo variabile prima di una nuova accensione.

Le accensioni devono essere asincrone: evitare che tutte le lucciole si accendano o spengano insieme. Il risultato deve ricordare un gruppo naturale in un bosco, con punti luminosi che appaiono e scompaiono in posizioni diverse.

### Parametri temporali

L'utente deve poter controllare:
- durata minima e massima della fase spenta;
- durata minima e massima dell'accensione (glow-in);
- durata minima e massima della fase accesa;
- durata minima e massima dello spegnimento (glow-out);
- eventuale pausa/intervallo tra cicli;
- intensità minima e massima della luce;
- variazione casuale dei tempi e dell'intensità;
- distribuzione casuale delle fasi iniziali, per evitare che tutte le lucciole partano nello stesso stato.

Usare unità comprensibili (secondi o frame, dichiarando quale) e validare i limiti. Le transizioni devono essere morbide e prive di flicker.

## 2. Movimento: stormo caotico ma naturale

Le lucciole devono muoversi continuamente anche quando sono spente.

Caratteristiche:
- movimento caotico e irregolare, ma non nervoso;
- componente **orizzontale predominante**;
- componente verticale presente ma sensibilmente inferiore e regolabile in percentuale;
- traiettorie curve, cambi di direzione graduali, derive e piccole oscillazioni;
- velocità variabile tra individui;
- movimento indipendente, evitando traiettorie identiche o sincronizzate;
- permanenza entro un volume/area di emissione configurabile, con rientro graduale quando una lucciola si allontana troppo, senza teletrasporti ai bordi.

Evitare moto balistico uniforme, traiettorie rettilinee troppo lunghe, rumore indipendente per frame, cambi di direzione istantanei e sciami troppo ordinati.

### Parametri di movimento

Esporre controlli chiari per:
- quantità/numero di lucciole;
- estensione del volume di movimento;
- velocità media e variazione casuale;
- intensità del movimento orizzontale;
- percentuale/intensità del movimento verticale rispetto a quello orizzontale;
- fluidità/inerzia dei cambi di direzione;
- dispersione/densità dello stormo;
- attrazione leggera verso una zona centrale o confini morbidi del volume.

Documentare gli assi rispetto al mondo o al volume dell'effetto e consentire di orientare/ruotare il volume.

## 3. Colori: palette probabilistica e variazioni controllate

Implementare una palette con più voci. Per ogni voce l'utente può scegliere:
- colore;
- percentuale di assegnazione sul totale.

La somma delle percentuali deve essere gestita chiaramente: validazione o normalizzazione automatica, con indicazione della distribuzione effettiva. Il colore assegnato a ciascuna lucciola deve restare stabile nel tempo.

Oltre ai colori statici, aggiungere una variazione casuale controllabile:
- percentuale di lucciole che ricevono una variazione;
- ampiezza della variazione cromatica;
- variazione limitata alla stessa famiglia/tonalità del colore assegnato, senza trasformare per esempio un giallo in un blu;
- seed deterministico.

Distinguere la palette multi-colore dalla variazione tonale. Il comportamento predefinito dovrebbe mantenere il colore stabile per individuo; se si offre una variazione durante i cicli di accensione, deve essere lieve e controllabile.

## 4. Illuminazione reale della scena — requisito fondamentale

Questa è la caratteristica che distingue il nuovo effetto dai granelli luminosi esistenti.

Quando sono accese, le lucciole devono essere sorgenti luminose effettive:
- la loro luce contribuisce all'illuminazione delle superfici vicine;
- produce variazioni di luce e colore su oggetti, terreno, foglie e tronchi;
- segue l'intensità del ciclo glow-in/accensione/glow-out;
- da spente, il contributo luminoso è nullo o trascurabile e il corpo è invisibile;
- il colore della luce corrisponde al colore assegnato.

Il glow visibile non deve sostituire l'illuminazione fisica.

### Analisi tecnica obbligatoria

Ispezionare i percorsi CPU e GPU e valutare:
- sorgenti puntiformi dinamiche o area lights;
- campionamento esplicito delle luci del sistema;
- integrazione con illuminazione diretta e MIS, se applicabile;
- contributi locali o sistema di illuminazione dedicato.

Non assegnare automaticamente una luce completa e costosa a ogni lucciola senza misurare il costo. Con centinaia o migliaia di sorgenti il rendering potrebbe diventare troppo lento.

Proporre una soluzione scalabile che mantenga il contributo luminoso reale, valutando clustering delle lucciole vicine, selezione delle sorgenti più influenti, rappresentazione gerarchica del campo luminoso o campionamento stocastico con pesi corretti.

Le approssimazioni sono ammesse se documentate e se producono illuminazione coerente. Non chiamare illuminazione reale un alone aggiunto in post-produzione o un colore disegnato direttamente sui pixel.

### Ombre e occlusione

Valutare se e a quale costo le lucciole devono produrre ombre:
- la luce deve essere occlusa dagli oggetti interposti, almeno nel percorso di illuminazione del renderer;
- se ombre complete per ogni lucciola sono troppo costose, progettare un compromesso controllabile e dichiararne i limiti;
- evitare che la luce attraversi pareti o oggetti opachi in modo palesemente scorretto.

## 5. Glow visibile e corpo della lucciola

Quando la lucciola è accesa, il corpo/punto luminoso diventa visibile e il glow aumenta/diminuisce in sincronia con la luce. Dimensione apparente e intensità del glow devono essere configurabili e il glow dovrebbe essere visibile anche in viewport, se supportato.

Quando è spenta, il corpo è invisibile e non resta alone residuo, ma la particella continua la traiettoria.

Distinguere emissione visibile e contributo luminoso alla scena, mantenendoli coerenti ma controllabili separatamente.

## 6. Determinismo, timeline e precalcolo

- Seed deterministico per ogni lucciola.
- Stato iniziale e fase del ciclo distribuiti deterministicamente.
- Il salto diretto a un frame deve produrre lo stesso risultato della riproduzione dall'inizio.
- Viewport, render singolo e render animazione devono usare lo stesso stato temporale.
- Il risultato non deve dipendere dalla velocità di riproduzione o dal frame rate effettivo della UI.

Preferire, se possibile, traiettorie e cicli ricavabili da tempo assoluto + seed, senza integrare tutta la simulazione dall'inizio. Se interazioni con oggetti o campi richiedono stato simulato, valutare precalcolo/cache e invalidazione dal primo frame interessato.

## 7. Interazione con oggetti e ambiente

Valutare:
- gli oggetti in movimento possono perturbare leggermente il moto delle lucciole vicine;
- le lucciole restano nel volume definito, con confini morbidi;
- evitare attraversamenti evidenti di superfici solide, se risolvibile con collisioni semplificate;
- non serve un modello biologico completo: l'obiettivo è un effetto credibile, controllabile e riproducibile.

Separare interazioni indispensabili da miglioramenti opzionali, così il costo non cresce senza controllo.

## 8. Interfaccia e controlli

Creare un pannello dedicato chiamato **Lucciole / Bosco incantato**, organizzato in gruppi:
1. Generale e quantità;
2. Volume e movimento;
3. Ciclo di accensione/spegnimento;
4. Palette colori e percentuali;
5. Variazione tonale;
6. Intensità luminosa e glow;
7. Qualità/prestazioni e interazione con la scena.

Usare controlli comprensibili, unità esplicite, tooltip e valori predefiniti gradevoli. Evitare di esporre decine di parametri tecnici se possono essere raggruppati in controlli intuitivi.

Fornire un preset **Bosco incantato**: molte lucciole, movimento lento e irregolare, accensioni asincrone, palette calda e luce morbida. Tutti i parametri restano modificabili.

## 9. Prestazioni e qualità

- Misurare il costo su CPU e GPU e non assumere che una pipeline rappresenti l'altra.
- Evitare una sorgente completa e costosa per ogni particella se non scala.
- Misurare il costo al crescere del numero di lucciole e delle sorgenti effettivamente campionate.
- Verificare rumore Monte Carlo, fireflies, convergenza e comportamento con campionamento adattivo.
- Evitare che l'effetto faccia esplodere il rumore o i tempi di render.
- Se si usa selezione/cluster, assicurarsi che la distribuzione non favorisca arbitrariamente alcuni colori o zone.
- Glow e illuminazione della scena devono poter essere regolati separatamente.

## 10. Criteri di accettazione

1. Esiste un nuovo effetto selezionabile chiamato Lucciole, con preset Bosco incantato.
2. Le lucciole si muovono continuamente anche quando sono spente.
3. Da spente sono invisibili; si rendono visibili gradualmente durante l'accensione e scompaiono gradualmente allo spegnimento.
4. Ogni individuo ha tempi e fasi asincroni, con intervalli configurabili.
5. Il moto è caotico ma fluido, prevalentemente orizzontale e con componente verticale inferiore regolabile.
6. È possibile configurare più colori con percentuali di assegnazione.
7. È possibile configurare la percentuale di particelle che ricevono variazioni tonali e l'ampiezza della variazione nella stessa tonalità.
8. Le lucciole accese illuminano realmente la scena nel renderer: superfici e oggetti ricevono colore e intensità della luce.
9. La luce rispetta l'occlusione degli oggetti, nei limiti della soluzione scelta e documentata.
10. Glow visibile e illuminazione reale seguono lo stesso ciclo, ma sono controllabili separatamente.
11. Play, salto diretto al frame, viewport, render singolo e render animazione producono stati coerenti e deterministici.
12. Il costo è misurato con quantità crescenti di lucciole e non compromette eccessivamente CPU/GPU.
13. Il normale sistema di polvere luminosa e gli altri effetti esistenti non subiscono regressioni.

## 11. Piano di lavoro e risposta richiesta

Prima di implementare, Claude deve:
1. Ispezionare il sistema esistente di particelle luminose, il glow in post-produzione e i percorsi di illuminazione CPU/GPU.
2. Spiegare quali parti possono essere riutilizzate e quali devono essere nuove.
3. Proporre una strategia concreta per l'illuminazione reale scalabile, con compromessi e rischi.
4. Indicare come garantire determinismo temporale e sincronizzazione tra viewport e render.
5. Implementare il nuovo effetto senza alterare il comportamento degli effetti esistenti.

Al termine, aggiungere al normale handoff una risposta datata con architettura scelta per movimento, ciclo luminoso, palette e illuminazione; file modificati; test CPU/GPU realmente eseguiti; verifica visiva con scena dimostrativa; tempi di render e misure al variare del numero di lucciole; limiti/approssimazioni di illuminazione e ombre; aspetti non verificati e prossimi interventi.

Non dichiarare completato l'effetto se le particelle sono soltanto visibili con glow in post-produzione: il requisito centrale è che le lucciole illuminate contribuiscano all'illuminazione della scena.
