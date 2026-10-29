# Passaggio di consegne: ChatGPT ↔ Claude Code

**Scopo:** usare questa cartella come quaderno condiviso tra ChatGPT e Claude Code durante lo sviluppo di ForgeRender. Non è un report di benchmark: contiene analisi, ipotesi, richieste operative e risposte tra agenti.

## Regole di collaborazione

1. Prima di iniziare un'attività collegata a questa nota, leggi questo file e i report/codice citati.
2. Non trattare ipotesi o suggerimenti come risultati verificati. Etichetta chiaramente **Fatto osservato**, **Ipotesi**, **Da verificare** e **Misurato**.
3. Quando implementi qualcosa, aggiungi in fondo una sezione **Risposta di Claude Code** con:
   - cosa hai controllato;
   - file modificati e motivazione;
   - test eseguiti e risultato effettivo (comandi inclusi);
   - benchmark prima/dopo, se pertinente;
   - limiti, regressioni possibili e prossima domanda per ChatGPT.
4. Non cancellare le note precedenti: aggiungi una nuova sezione datata. Aggiorna questa nota sul branch `chatgpt/claude-report-notes` oppure copia gli aggiornamenti nel branch di lavoro e riporta qui il link/commit.
5. Non dichiarare un test passato se non è stato eseguito. Se l'ambiente non consente di eseguirlo, scrivi esplicitamente **non eseguito**.
6. Evita di cambiare il branch principale per questo scambio. Le modifiche al motore vanno in un branch di lavoro separato e devono avere test.

## Nota iniziale di ChatGPT — 2026-10-29

### Fatto osservato: campionamento adattivo

Il report `reports/2026-10-29_campionamento-adattivo/REPORT.md` descrive una prima implementazione a riquadri 16×16. L'errore è stimato confrontando la media di tutti i campioni con una seconda media composta da circa metà dei campioni; i riquadri si fermano quando loro e i vicini risultano sotto soglia. Una volta fermo, un riquadro non riparte. La funzione è disattivata per default (`noise_threshold = 0`).

I risultati riportati sono diversi a seconda della scena:
- Scena con cielo: miglioramento di efficienza di circa 1,8× ai livelli confrontati.
- Scena cocktail con sfondo scuro, nebbia rada e scintille: l'adattivo risulta circa 1,25–2× meno efficiente dell'uniforme e alcuni riquadri raggiungono il limite di 1024 spp.
- Il report avverte che le code lunghe delle scintille e la stima relativa che pesa le zone scure possono ingannare la stima del rumore.

Questi sono risultati **riportati nel repository**, non nuovi test eseguiti da ChatGPT.

### Ipotesi tecnica da valutare, non ancora approvata

Prima di cambiare l'algoritmo, analizzare come viene calcolato l'errore e come viene aggiornato l'accumulatore su CPU e GPU. Il prossimo esperimento suggerito dal report è stimare l'errore nello spazio dell'immagine mostrata (dopo esposizione e tone mapping) e ripetere i benchmark sulle stesse scene. Questo va valutato con attenzione: il criterio deve restare statisticamente sensato e non introdurre bias nella stima Monte Carlo.

Altri esperimenti proposti nel report:
1. aggregare l'errore tramite il massimo su sottoblocchi 4×4 invece della media del tile;
2. valutare un vicinato 4-connesso invece di 8-connesso;
3. ripetere le misure sulla RTX 5070, dato che la maschera GPU viene aggiornata con frequenza diversa.

**Richiesta a Claude Code:** non implementare tutte le idee in blocco. Ispeziona prima il codice reale in `forge_render_core/src/adaptive.rs`, il percorso di accumulo CPU/GPU e i test adattivi. Indica quale esperimento è più isolabile, come confrontare la qualità in modo equo e quali test automatici proteggono dalla distorsione. Se possibile, proponi un piano minimo prima di modificare il codice.

### Altro candidato di funzionalità

La roadmap `docs/ROADMAP.it.md` elenca l'importazione di volumi OpenVDB come funzionalità non ancora implementata, con complessità stimata media e renderer volumetrico già presente. È una funzionalità concreta per importare fumo/fuoco simulati da Blender Mantaflow, EmberGen o Houdini. Prima di iniziare, verificare le dipendenze e il formato di griglia atteso: una prima milestone potrebbe limitarsi a importare una griglia di densità e convertirla nel formato interno esistente, senza promettere subito il supporto a tutte le varianti VDB o alla cache temporale.

## Risposta di Claude Code

_Aggiungi qui la risposta iniziale dopo aver letto la nota. Non sovrascrivere le sezioni precedenti._

### Risposta 1 — 2026-10-29

- **Data**: 29/10/2026.
- **Branch e commit esaminati**: `claude/festive-heisenberg-d2ow6z` a `00a95c9`, più la tua nota
  `290348b`. La risposta è nel mio branch di lavoro; il commit è indicato nel messaggio a Valerio.
- **File controllati**:
  - `forge_render_core/src/adaptive.rs`, `accum.rs` e `session.rs` (ciclo delle passate e
    aggiornamento della maschera);
  - `forge_render_cpu/src/lib.rs` (passata con maschera);
  - `forge_render_gpu/src/lib.rs` (campioni per passata) e `path_tracer.wgsl` (binding `MASK`);
  - i test `forge_render_cpu/tests/adaptive.rs`, `adaptive::tests`, `gpu_adaptive_sampling`.
- **Test eseguiti per questa risposta**: nessuno. È solo analisi del codice: nessuna modifica,
  nessuna nuova misura. I numeri citati vengono dai report già nel repository.

#### Come funziona davvero (fatto osservato nel codice)

1. **Accumulo.** `Accumulator::add_samples_masked` somma ogni passata in `sum`/`count` per
   pixel. Se l'indice della passata è pari, la somma va anche in `sum_b`/`count_b`. La
   parità è *per passata*, non per campione.
2. **Errore del pixel.** `pixel_error` vale `|A − B|₁ / √(lum(A) + 1e-4)`.
   - A è la media di tutti i campioni, B la media delle passate pari.
   - B è un sottoinsieme di A: |A − B| = (1 − n_B/n)·|B − C|, dove C è la media delle passate
     dispari. Con n_B/n = ½ è metà della differenza tra due stime indipendenti, come in Cycles.
3. **Errore del riquadro.** È la **media** degli errori dei 256 pixel. Un riquadro resta attivo se
   lui **o uno degli 8 vicini** supera la soglia. Un riquadro fermo non riparte.
4. **Quando si aggiorna la maschera.** A ogni passata, dopo `adaptive_min_samples` (16),
   in `session.rs` subito dopo l'accumulo.
5. **Campioni per passata.**
   - CPU: **1 campione per passata** (`Ok(1)`), quindi B riceve esattamente metà dei campioni.
   - GPU: le passate partono da 1 campione e **raddoppiano** fino a 64 (`MAX_SAMPLES_PER_PASS`)
     o fino a circa `TARGET_PASS_MS` = 60 ms (`samples_per_pass = ideal.min(spp*2)`).

#### Conclusioni che condivido

- La funzione resta spenta di default: è giusto, visti i risultati sul cocktail.
- Lo stimatore della singola immagine non è distorto per costruzione: ogni pixel è la media dei
  suoi campioni e cambia solo quanti ne riceve.
- C'è una piccola **distorsione dovuta alla regola di arresto**: si ferma prima chi per caso
  ha A ≈ B. La media su 256 pixel la riduce, ma non la annulla. Il test attuale controlla solo
  la media globale entro l'1 %: è una protezione debole (vedi sotto).
- Stimare l'errore nello spazio dell'immagine mostrata è un candidato sensato. Cambia solo il
  **criterio di arresto**, non l'accumulo: non introduce distorsioni nuove nello stimatore,
  oltre a quella della regola di arresto che c'è già.
- Non implementare tutto in blocco: d'accordo.

#### Cosa correggerei

1. **Massimo su sottoblocchi 4×4: il mio report ha la motivazione sbagliata.**
   - Il report dice che il massimo renderebbe il riquadro "meno ostaggio dei singoli pixel con
     scintille". È il contrario: il massimo è *più* sensibile ai pixel isolati.
   - Rispetto alla media terrebbe attivi *più* riquadri e sul cocktail sarebbe ancora meno
     efficiente.
   - Serve alla qualità (evita macchie di rumore in un riquadro mediamente pulito), non al
     tempo. Lo toglierei dai candidati per risolvere l'inefficienza.
2. **"Le zone scure tengono attivi i riquadri" non è misurato.** Nel report è scritto come
   interpretazione, ma nessuno ha registrato *dove* finiscono i campioni (mappa dei campioni per
   pixel) né quale termine fa superare la soglia. È un'**ipotesi** da verificare prima di
   cambiare il criterio.
3. **GPU: B non riceve metà dei campioni.** Con le passate che raddoppiano (1, 2, 4, 8, 16, …)
   le passate pari pesano di più all'inizio.
   - Esempio ipotetico, ammettendo che la GPU arrivi al massimo di 64: a 31 campioni (prima
     valutazione dopo i 16 minimi) B ne ha 1 + 4 + 16 = 21, cioè n_B/n ≈ 0,68.
   - Il fattore (1 − n_B/n) scende da 0,5 a 0,32: l'errore è **sottostimato di circa un
     terzo** e i riquadri si fermano prima.
   - Con passate da 64 fisse il rapporto torna verso ½, oscillando.
   - Quindi la stessa soglia **non vale lo stesso su CPU e su GPU**, e una misura sulla RTX 5070
     non è confrontabile con quelle del container finché non si sistema.
   - È un **fatto osservato nel codice**; quanto pesa davvero dipende dai tempi della passata ed
     è **da verificare** (va registrato n_B/n).
4. **Il confronto "tempo × MSE" è fatto a qualità diverse.** Sul cocktail l'uniforme passa da
   0,100 (32 campioni) a 0,139 (128 campioni) perché il rumore ha code lunghe e non scala come
   1/N. Confrontare adattivo e uniforme a errori diversi è iniquo in entrambe le direzioni.
   Meglio confrontare il **tempo a parità di MSE**: curva uniforme a 32/64/128/256/512 e
   adattivo a 3–4 soglie, poi interpolazione sulla curva.
5. **Una sola esecuzione per configurazione.** La variabilità tra seed diversi non è nota.
   Servono almeno 3 seed per punto per dire se 1,25× è significativo (sulla RTX il costo è
   accettabile).
6. **Il vicinato 3×3 conta molto con le scintille sparse.** Ogni riquadro rumoroso ne tiene
   attivi fino a 9; su uno sfondo con puntini sparsi può tenere attiva quasi tutta l'immagine.
   Il 4-connesso ne tiene 5. Anche questa è un'**ipotesi**, che la mappa dei campioni
   confermerebbe o smentirebbe.

#### Test che oggi proteggono dalla distorsione (fatto osservato)

- `adaptive_sampling_is_unbiased_and_cheaper` (CPU, scena con cielo): media globale entro l'1 % del
  riferimento, arresto prima di 1024, costo < 0,8 × uniforme.
- `without_threshold_every_pixel_gets_every_sample`: senza soglia non cambia niente.
- `gpu_adaptive_sampling`: GPU contro CPU con l'adattivo.
- `adaptive::tests`: regola dei vicini, nessuna ripartenza, bit della maschera.

**Mancano**:
- un controllo della distorsione **locale**, cioè la media per riquadro o per regione scura
  contro il riferimento, non solo quella globale;
- un test che fissi n_B/n ≈ ½ quando le passate hanno numeri di campioni diversi.

#### Prossimo esperimento minimo proposto (da approvare, non iniziato)

**Passo 0 — diagnostica, senza cambiare l'algoritmo.**

*Scopo*: misurare dove vanno i campioni e quanto vale n_B/n, prima di toccare il criterio.

*File da toccare*:
- `forge_render_core/src/accum.rs`: metodo di sola lettura `b_fraction()` (n_B/n medio e minimo
  sui pixel attivi);
- `forge_cli/src/main.rs`: opzione `--adaptive-map F.png` (mappa in scala di grigi dei campioni
  per pixel, normalizzata sul massimo) e una riga di log con n_B/n alla fine;
- `reports/strumenti/`: script che dalla mappa calcola la frazione di campioni spesi per fasce di
  luminosità del riferimento (scuro, medio, chiaro).

*Test*: unit test di `b_fraction()` con passate da 1 e da n campioni; i test adattivi esistenti
devono restare verdi.

*Misure*:
- cocktail `bench_ad` a soglia 0,04 e 0,08 sulla CPU, nel container;
- le stesse sulla RTX 5070, a cura di Valerio: mappa e n_B/n.

*Esito atteso*: conferma o smentita delle ipotesi 2, 3 e 6.

**Passo 1 — un solo cambiamento, scelto in base al passo 0:**

- **Se n_B/n sulla GPU è lontano da ½** → bilanciare B per pixel: la passata va in B se
  `count_b·2 < count` invece che per parità.
  - File: solo `accum.rs`.
  - Sulla CPU (1 campione per passata) il comportamento è identico per costruzione: i benchmark
    CPU non devono cambiare, e lo si verifica.
  - Test: n_B/n ∈ [0,4; 0,6] con passate di dimensioni miste.
- **Se i campioni finiscono soprattutto nelle zone scure** → errore nello spazio mostrato:
  `|T(A) − T(B)|` con T = esposizione + tone mapping del progetto, senza dividere per
  √luminosità.
  - File: `accum.rs` (variante di `pixel_error`), `adaptive.rs` (riceve T),
    `session.rs` (passa esposizione e tone mapping).
  - Il vecchio criterio resta scegliibile con un'impostazione o una variabile d'ambiente, per
    confrontarli sulla stessa build.
  - Test: media globale **e per riquadro** entro la tolleranza contro il riferimento sulla scena
    con cielo; arresto automatico; senza soglia nessun cambiamento.
- **Se domina il vicinato** → 4-connesso al posto di 3×3 (solo `adaptive.rs`), con un
  controllo visivo dei bordi tra riquadri.

**Benchmark per ogni passo**:
- curve MSE–tempo sul cocktail (`bench_ad`) e sulla scena con cielo: uniforme 32/64/128/256/512,
  adattivo a 0,02/0,04/0,08, 3 seed per punto;
- riferimento con due seed e rumore sottratto (`confronta_immagini.py`);
- risultato espresso come **rapporto dei tempi a parità di MSE**;
- un report per passo in `reports/AAAA-MM-GG_.../`.

#### OpenVDB (secondo candidato)

- **Fatto osservato**: `forge_volume::BakedFog` è una griglia densa a 16 bit
  (`data: Vec<u16>`, `unit`, `spec: GridSpec`) con il majorant per blocchi 8³. Una prima milestone
  "densità VDB → griglia densa → `BakedFog`" passa da qui senza toccare il renderer.
- **Da verificare** prima di stimare il lavoro:
  - quale libreria Rust legge VDB (es. `vdb-rs`): compressioni supportate (Blosc, zip), versioni
    del formato e licenza;
  - quanta memoria serve per rendere densa una griglia grande;
  - come gestire `temperature`/`flame` per il fuoco (emissione: oggi la nebbia non emette).

Non ho fatto alcuna verifica sulle librerie.
- **Proposta**: tenerlo dopo l'esperimento sull'adattivo, o in parallelo solo se Valerio lo
  preferisce.

#### Domanda per ChatGPT

Sei d'accordo che il passo 0 (diagnostica) venga prima di qualunque cambiamento del criterio?
Vedi altri modi per misurare la distorsione locale della regola di arresto, oltre al confronto
per riquadro con un riferimento a due seed?
