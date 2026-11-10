# ForgeRender — studio e piano di integrazione delle tecniche Cycles

**Data:** 10/11/2026  
**Destinatario:** Claude Code  
**Repository:** https://github.com/tarabaz/ForgeRender  
**Branch di lavoro richiesto:** `sviluppo` (o un branch dedicato creato da `sviluppo`; mai lavorare direttamente su `main`)  
**Riferimento iniziale:** ForgeRender 0.53.0, handoff aggiornato al 09/11/2026.

## Obiettivo e mandato

Valerio vuole portare in ForgeRender tutte queste tecniche, ispirandosi all'implementazione di Blender Cycles:

1. **Light Tree**
2. **Path Guiding**
3. **Null Scattering per i volumi**
4. **Wavefront Path Tracing GPU**
5. **Adaptive Subdivision e displacement geometrico reale**

Il mandato è **studiare e implementare tutte e cinque**, non limitarsi a una relazione o a una lista di TODO. Tuttavia, non fare una riscrittura monolitica: procedere per milestone indipendenti, compilabili e testabili, mantenendo sempre il renderer precedente come fallback finché la nuova tecnica non è validata.

Non copiare interi moduli da Cycles. Ricostruire algoritmi e invarianti, quindi implementarli idiomaticamente in Rust/WGSL e nell'architettura esistente. Prima di ogni modifica, leggere il codice attuale e mappare i punti d'integrazione reali. I percorsi citati in questa specifica sono riferimenti Cycles, non percorsi da replicare letteralmente in ForgeRender.

## Regole obbligatorie di lavoro

- Leggere questa specifica, `reports/chatgpt/CLAUDE_HANDOFF.md`, gli ultimi report e il codice interessato.
- Verificare il branch e il commit di partenza; non presumere che la 0.53.0 sia ancora HEAD.
- Prima di implementare ogni tecnica, scrivere una breve analisi: stato attuale, moduli coinvolti, algoritmo scelto, rischi, test e benchmark.
- Implementare tutte le cinque funzionalità, ma in milestone separate. Non fermarsi dopo l'audit.
- Non introdurre una dipendenza pesante o una nuova libreria senza averne motivato necessità, licenza, manutenzione e compatibilità Windows/MSVC.
- Non compromettere i progetti `.frproject` esistenti: nuovi campi devono avere default e migrazione compatibile.
- Ogni funzionalità deve avere un'opzione di attivazione e un fallback al metodo precedente, almeno durante lo sviluppo e la validazione.
- Nessuna regressione silenziosa: se un caso non è supportato, segnalarlo e usare il fallback.
- Non dichiarare test, benchmark o verifiche visive che non siano stati eseguiti davvero.
- Aggiornare manuale, README, changelog e traduzioni quando la funzionalità diventa utilizzabile.
- Registrare i risultati in una nuova sezione di `reports/chatgpt/CLAUDE_HANDOFF.md` e in report tecnici dedicati sotto `reports/`.

## Materiale Cycles da studiare

Repository ufficiale:
- Cycles: https://projects.blender.org/blender/cycles
- Mirror GitHub utile per consultazione: https://github.com/blender/cycles
- Organizzazione dei moduli: https://developer.blender.org/docs/features/cycles/source_layout/
- Kernel scheduling e wavefront: https://developer.blender.org/docs/features/cycles/kernel_scheduling/
- Render scheduling: https://developer.blender.org/docs/features/cycles/render_scheduling/
- Ottimizzazione: https://developer.blender.org/docs/features/cycles/optimization/
- Release notes Cycles 5.0: https://developer.blender.org/docs/release_notes/5.0/cycles/
- Manuale volumi: https://docs.blender.org/manual/en/5.0/render/cycles/render_settings/volumes.html
- Manuale sampling e Path Guiding/Light Tree: https://docs.staging.blender.org/manual/en/latest/render/cycles/render_settings/sampling.html
- Manuale displacement: https://docs.blender.org/manual/en/latest/render/materials/components/displacement.html
- Panoramica delle capacità: https://www.blender.org/features/rendering/

### File sorgente Cycles da leggere

**Light Tree**
- https://github.com/blender/blender/blob/main/intern/cycles/scene/light_tree.cpp
- https://github.com/blender/blender/blob/main/intern/cycles/scene/light_tree.h
- https://github.com/blender/blender/blob/main/intern/cycles/kernel/light/tree.h
- Studio teorico collegato all'implementazione: *Importance Sampling of Many Lights with Adaptive Tree Splitting*, Conty Estevez e Kulla: https://fpsunflower.github.io/ckulla/data/many-lights-hpg2018.pdf

**Path Guiding**
- https://github.com/blender/cycles/tree/main/src/integrator
- https://github.com/blender/blender/tree/main/intern/cycles/integrator
- https://github.com/blender/blender/blob/main/intern/cycles/blender/addon/properties.py (opzioni e parametri esposti: training, superficie, volume, probabilità, MIS)
- https://developer.blender.org/docs/features/cycles/kernel_scheduling/

**Null Scattering e volumi**
- https://github.com/blender/blender/tree/main/intern/cycles/kernel/integrator
- https://github.com/blender/blender/tree/main/intern/cycles/kernel/closure
- https://github.com/blender/blender/blob/main/intern/cycles/scene/volume.cpp
- https://github.com/blender/blender/blob/main/intern/cycles/kernel/closure/volume.h
- Release note che descrive l'introduzione del metodo unbiased: https://developer.blender.org/docs/release_notes/5.0/cycles/
- Manuale con distinzione fra null scattering unbiased e ray marching biased: https://docs.blender.org/manual/en/5.0/render/cycles/render_settings/volumes.html

**Wavefront Path Tracing**
- https://developer.blender.org/docs/features/cycles/kernel_scheduling/
- https://github.com/blender/cycles/tree/main/src/integrator
- https://github.com/blender/cycles/tree/main/src/kernel
- https://github.com/blender/cycles/tree/main/src/device
- Paper citati dalla documentazione Cycles: *Megakernels Considered Harmful: Wavefront Path Tracing on GPUs* e *The Iray Light Transport Simulation and Rendering System*. Recuperare e leggere i lavori originali, non basarsi solo sui riassunti.

**Adaptive Subdivision e displacement**
- https://github.com/blender/blender/tree/main/intern/cycles/subd
- https://github.com/blender/blender/tree/main/intern/cycles/bvh
- https://docs.blender.org/manual/en/latest/render/materials/components/displacement.html
- https://developer.blender.org/docs/release_notes/5.0/cycles/

Nota: le URL GitHub sono punti d'ingresso alla sorgente. Seguire include, chiamanti, strutture dati e kernel collegati; non limitarsi ai file elencati. La documentazione ufficiale indica che Cycles separa integrator, kernel, device, scene, BVH e subdivision: questa separazione architetturale è importante per capire dove collocare ciascuna responsabilità in ForgeRender.

---

# 1. Light Tree

## Perché implementarlo

Quando una scena contiene molte luci puntiformi, spot, area light, mesh emissive o sorgenti generate da effetti, campionare le luci con una strategia semplice può sprecare molti shadow ray e produrre rumore. Il Light Tree organizza gli emettitori gerarchicamente e sceglie una sorgente con probabilità legata alla sua probabile contribuzione dal punto di shading.

Cycles documenta che il metodo può ridurre sensibilmente il rumore nelle scene con molte luci, pagando un costo leggermente maggiore per campione. Le euristiche sono meno affidabili con falloff personalizzati, visibilità non fisica o shader emissivi complessi: questi casi vanno trattati esplicitamente.

## Cosa studiare nel codice Cycles

- Costruzione degli emitter: luci regolari e triangoli emissivi.
- Misure geometriche aggregate nei nodi: bounding box, centroide e cono di orientamento.
- Criterio di suddivisione gerarchica e costo dei nodi.
- Traversal probabilistico dal punto di shading.
- Calcolo della probabilità di selezione e PDF finale.
- Gestione delle luci distanti, mesh emissive, luci disattivate e light linking.
- Interazione con Next Event Estimation (NEE) e Multiple Importance Sampling (MIS).

## Cosa implementare in ForgeRender

1. Inventariare tutti gli emitter oggi campionabili dal renderer: point, spot, directional/sun, area, environment e geometrie emissive, incluse luci prodotte da effetti/lucciole.
2. Definire una rappresentazione interna di emitter e nodo del tree, indipendente dalla GUI e serializzazione della scena.
3. Costruire il tree lato CPU al momento opportuno (build/update della scena), evitando ricostruzioni a ogni bounce.
4. Iniziare con una gerarchia robusta basata su bounding box, orientamento e potenza/energia stimata. Valutare un criterio SAH/bucket simile a Cycles.
5. Implementare il sampling dal punto di shading con probabilità condizionali lungo il tree.
6. Restituire sempre una PDF coerente con la probabilità effettiva di selezione. La contribuzione deve essere divisa per la PDF corretta.
7. Integrare la PDF con MIS e NEE: non aggiungere il tree come selezione scollegata dall'estimatore esistente.
8. Appiattire e caricare la struttura in un buffer GPU compatibile con WGSL; se non è ancora possibile, mantenere fallback GPU corretto e documentare il limite.
9. Aggiornare/invalidation del tree quando cambiano posizione, potenza, forma, orientamento, emissione o stato delle luci/mesh emissive.
10. Conservare un sampler uniforme di fallback e una modalità di confronto A/B.

## Test e benchmark richiesti

- Scene con 1, 4, 16, 64, 256, 1.000+ emitter.
- Point/spot distribuite nello spazio, area light, mesh emissive e combinazioni.
- Test statistico: la media del risultato con Light Tree deve convergere allo stesso valore del sampler di riferimento entro l'incertezza Monte Carlo.
- Test PDF: probabilità finite, positive, somma normalizzata e nessun NaN/Inf.
- Test di invalidazione dopo modifica delle luci.
- Misurare build time del tree, memoria, tempo per sample, tempo totale e varianza/MSE a tempo fisso.
- Includere una scena con luci non fisiche/custom per verificare fallback o avviso.

**Criterio di accettazione:** nessuna variazione sistematica dell'energia stimata; riduzione misurabile della varianza o del tempo per raggiungere una qualità target in scene con molte luci; nessuna regressione importante nelle scene con poche luci.

---

# 2. Path Guiding

## Perché implementarlo

Il path tracer standard può impiegare molto tempo a trovare direzioni utili quando l'illuminazione indiretta arriva da aperture piccole, corridoi, ambienti complessi o riflessioni diffuse. Path Guiding apprende durante il render quali direzioni, in una determinata regione e situazione di shading, tendono a trasportare più energia e le usa per proporre campioni migliori.

Non è un risolutore universale di caustiche: non promettere che eliminerà il rumore da rifrazione o caustiche complesse. Cycles espone separatamente guiding di superficie e volume, probabilità di guida e numero massimo di campioni di training. La documentazione del manuale consultata segnala Path Guiding come CPU-only in quella versione/documentazione: verificare lo stato effettivo della versione sorgente studiata prima di attribuirgli supporto GPU.

## Cosa studiare

- Ciclo di vita del training: raccolta dati, aggiornamento distribuzione, uso durante il render.
- Rappresentazione spaziale e direzionale della distribuzione appresa.
- Campionamento misto fra BSDF e guida.
- PDF della distribuzione combinata e pesi MIS.
- Training deterministico/non deterministico, seed e riproducibilità.
- Strategie per superfici diffuse/rough e per volumi.
- Gestione di memoria, reset quando cambia la scena e aggiornamenti progressivi.

## Cosa implementare

**Prima milestone CPU.** Non iniziare dal GPU guiding: realizzare una prima implementazione CPU corretta, misurabile e statisticamente valida.

1. Definire una struttura di training per regioni spaziali e distribuzioni direzionali. Scegliere la rappresentazione dopo aver studiato il codice Cycles e valutato memoria/complessità in Rust; documentare la scelta.
2. Raccogliere statistiche dai cammini completati senza introdurre dipendenza circolare o contaminare la stima.
3. Addestrare/aggiornare la distribuzione progressivamente, con budget di training configurabile.
4. Campionare una miscela fra BSDF e distribuzione guidata. La PDF usata nel peso deve essere la PDF completa della miscela, non solo quella del ramo scelto.
5. Integrare correttamente MIS con NEE e Light Tree.
6. Applicare inizialmente il guiding alle componenti diffuse e sufficientemente rough. Escludere o gestire separatamente delta/specular, vetro e casi in cui il modello non è adatto.
7. Garantire che il metodo resti unbiased: il guiding modifica la distribuzione di campionamento, non la funzione di trasporto né la corretta ponderazione.
8. Esporre impostazioni chiare: attivo/disattivo, training samples/budget, probabilità di guida e superficie/volume se supportati.
9. Resettare o invalidare i dati quando cambiano geometria, materiali, luci o ambiente in modo incompatibile con il training precedente.
10. Solo dopo la validazione CPU, progettare un backend GPU. Non trasferire automaticamente la struttura CPU in WGSL senza misurarne layout, aggiornamenti e costo.

**Volume guiding:** implementarlo come estensione separata dopo il surface guiding, in modo che un problema nel volume non renda inutilizzabile il guiding di superficie.

## Test e benchmark

- Cornell box, stanza illuminata da una piccola apertura, corridoio, interni con luce indiretta, scena con materiali rough.
- Guiding disattivato contro attivato, stessi seed e budget temporale.
- Confrontare MSE/varianza rispetto a un riferimento ad alto spp e misurare il tempo per raggiungere la stessa qualità.
- Test di normalizzazione PDF, miscela BSDF/guide, MIS e riproducibilità con seed fissato.
- Test che disabilitino il guiding per materiali delta e casi non supportati.
- Verificare che il training non peggiori la convergenza su scene semplici.

**Criterio di accettazione:** stima non distorta entro l'incertezza statistica e miglioramento ripetibile in scene con illuminazione indiretta difficile. Se non migliora una scena, deve poter essere disattivato senza penalità significativa.

---

# 3. Null Scattering per i volumi

## Perché implementarlo

Il ray marching tradizionale valuta il volume a intervalli/step. È intuitivo e può essere veloce, ma introduce un compromesso fra costo, dettaglio e bias: step troppo grandi perdono dettaglio; step piccoli costano molto. Inoltre, volumi sovrapposti possono mostrare artefatti a blocchi o discontinuità.

Il Null Scattering è un approccio di tracking stocastico: usa eventi di scattering reali e null-collision per campionare il trasporto nel volume senza dipendere da un passo fisso di integrazione. Il metodo unbiased può essere più rumoroso o più lento in alcune scene; il vantaggio è evitare il bias da discretizzazione del ray marching. Cycles 5.0 lo usa come algoritmo predefinito, mantenendo il ray marching biased come alternativa.

## Cosa studiare

- Delta tracking / null-collision tracking e ratio tracking.
- Coefficiente di estinzione σ_t, scattering σ_s, assorbimento σ_a e albedo.
- Majorant valido per la densità/estinzione in una regione.
- Campionamento della distanza libera e probabilità di collisione nulla/reale.
- Volumi eterogenei, sovrapposizione e somma dei coefficienti.
- Phase function (almeno Henyey–Greenstein se già presente o necessaria).
- Emissione volumetrica e stima della transmittance.
- Strategie per griglie sparse e gerarchie di majorant.

## Cosa implementare

1. Mappare l'attuale percorso volumetrico CPU/GPU, compresi `BakedFog`, `GridSpec`, majorant per blocchi 8³, nebbia, fumo, ghiaccio secco e foschia.
2. Conservare l'attuale ray marching come metodo legacy/fallback e per eventuali anteprime veloci.
3. Definire una rappresentazione dei coefficienti fisici e chiarire come densità interna, unità e parametri dei materiali si convertono in σ_a/σ_s/σ_t.
4. Implementare prima un tracking unbiased CPU su volume omogeneo, poi su densità eterogenea.
5. Usare majorant rigorosi: il majorant deve essere sempre >= al massimo coefficiente effettivo nella regione. Se viene violato, il render deve segnalarlo/testarlo; non mascherare il problema con clamp arbitrari.
6. Integrare le collisioni nulle senza alterare il peso del cammino. Verificare matematicamente il peso/probabilità per ogni evento.
7. Gestire volumi sovrapposti sommando correttamente i coefficienti di estinzione e scattering; non sommare semplicemente le opacità.
8. Integrare emissione e phase function; verificare separatamente assorbimento puro, scattering puro e combinato.
9. Trasferire la logica in GPU/WGSL solo dopo la correttezza CPU, mantenendo identiche convenzioni fisiche e test di riferimento.
10. Valutare gerarchia di majorant e skipping di regioni vuote per non fare tracking in celle prive di densità.
11. Rendere selezionabile il metodo: Unbiased (Null Scattering) e Legacy (Ray Marching), con etichetta chiara sul possibile bias del secondo.

## Test e benchmark

- Volume omogeneo con soluzione analitica Beer–Lambert.
- Volume eterogeneo con densità graduale, fumo sottile, plume denso, bordi netti.
- Due o più volumi sovrapposti e volumi che si intersecano.
- Assorbimento puro, scattering puro, emissione, anisotropia.
- Confronto statistico con soluzione analitica o riferimento ad alta precisione.
- Test che il majorant non venga mai superato dai valori campionati.
- CPU/GPU: stesso seed se possibile e confronto delle medie su molte repliche, non pixel-identità.
- Benchmark di tempo, memoria, numero di null collision, collisioni reali e varianza.

**Criterio di accettazione:** nessun bias da step size nel metodo unbiased; corretta transmittance e corretta energia media; prestazioni documentate su volumi realistici. Non eliminare il ray marching finché il nuovo percorso non è validato.

---

# 4. Wavefront Path Tracing GPU

## Perché implementarlo

Un megakernel GPU prova a gestire l'intero cammino in un kernel lungo. I thread, però, divergono: alcuni colpiscono superfici diffuse, altri vetri, altri volumi, altri terminano presto. La GPU finisce per eseguire percorsi molto diversi nello stesso gruppo di thread.

Il wavefront divide il path tracing in kernel specializzati e mette i cammini in code. Ogni kernel lavora su cammini che devono eseguire lo stesso tipo di operazione; si possono compattare e ordinare le code per aumentare la coerenza. Cycles usa questo schema su GPU, con stato del cammino compatto, storage SoA e code separate anche per shadow path. Il CPU path tracing di Cycles, nella documentazione consultata, continua invece a usare un percorso megakernel: non è necessario forzare wavefront anche sulla CPU.

## Cosa studiare

- Kernel graph del path tracer e transizioni fra stati.
- IntegratorState e campi minimi necessari a riprendere un cammino.
- Code di ray generation, intersection, surface shading, volume shading, shadow rays, continuation e termination.
- Compaction degli indici attivi e scheduling dei kernel.
- SoA vs AoS, coalescing e budget di memoria.
- Shadow path branching e gestione del capacity/overflow delle code.
- Ordinamento per materiale/shader e coerenza.
- Scheduling di tile e batch per mantenere la GPU occupata.

## Cosa implementare

1. Fare prima un audit e un benchmark del backend GPU attuale: identificare se è megakernel, quali dati vengono mantenuti nei registri/buffer e dove si verifica la divergenza.
2. Stabilire baseline su RTX 5070 e su backend software/altre GPU disponibili, con scene semplici e complesse.
3. Progettare un `PathState` compatto, con ray, throughput, pixel/sample ID, bounce, RNG state, stato del cammino e dati di shading strettamente necessari.
4. Preferire SoA o layout ibrido solo dopo misure di accesso/coalescing; non copiare la struttura dati Cycles senza verificare le esigenze WGSL.
5. Introdurre code e kernel a stadi: almeno generate camera rays, intersect, shade surface, shadow visibility, continue/terminate. Separare volume shading quando il Null Scattering sarà pronto.
6. Implementare compaction robusta degli indici attivi, contatori atomici o scansione efficiente e protezione da queue overflow.
7. Gestire esplicitamente i cammini secondari generati da NEE/shadow rays; non perdere contributi quando una coda si riempie.
8. Mantenere lo stesso estimatore, RNG riproducibile e comportamento di MIS del backend attuale.
9. Implementare prima un percorso wavefront limitato alle superfici, confrontarlo con il megakernel, poi aggiungere volumi, trasparenza e casi complessi.
10. Conservare il megakernel come fallback selezionabile e per dispositivi dove wavefront non conviene.
11. Integrare Light Tree e Path Guiding in modo che possano produrre richieste di campionamento compatibili con le code, senza introdurre dipendenze dall'ordine di esecuzione.
12. Considerare il costo di dispatch multipli in WGPU: per scene piccole il wavefront può essere più lento. Il renderer deve poter scegliere il percorso più conveniente, se i benchmark giustificano un criterio automatico.

## Test e benchmark

- Scene diffuse semplici, molti materiali diversi, vetro/trasmissione, molte luci, volumi, fumo e scene con cammini di durata molto diversa.
- Confrontare megakernel e wavefront a parità di spp, seed e qualità.
- Misurare frame time, samples/s, occupancy se disponibile, numero di elementi per coda, overflow, compaction cost, memoria GPU e tempi per stadio.
- Test di queue overflow, zero elementi, un elemento, massimo capacity e molti cammini terminati.
- Test di correttezza: nessun pixel/sample perso o duplicato; stessa media statistica del megakernel.
- Profilare separatamente scene piccole e grandi: il costo dei dispatch può annullare i vantaggi su scene leggere.

**Criterio di accettazione:** miglioramento misurabile in almeno una classe significativa di scene sulla RTX 5070, correttezza statistica rispetto al megakernel, nessun overflow silenzioso e fallback affidabile.

---

# 5. Adaptive Subdivision e displacement geometrico reale

## Perché implementarli

Il bump mapping modifica le normali apparenti ma non la silhouette né la geometria reale. Il displacement sposta davvero la superficie e può creare dettagli, rilievi e silhouette più credibili. Per farlo servono abbastanza micro-poligoni: suddividere uniformemente una mesh ad alta risoluzione può però consumare troppa memoria e aumentare enormemente il costo BVH.

Adaptive Subdivision crea dettaglio dove serve, in funzione della dimensione proiettata sullo schermo/distanza dalla camera o di una soglia in object space. Cycles 5.0 documenta l'adaptive subdivision come funzionalità non più sperimentale. Per mappe displacement, 16/32-bit float sono preferibili a 8-bit quando serve dettaglio fine.

## Cosa studiare

- Moduli `subd/` e `bvh/` di Cycles.
- Catmull–Clark e suddivisione lineare.
- Tessellazione adattiva basata su lunghezza proiettata degli edge.
- Criterio di suddivisione in pixel e object space.
- Crack-free subdivision: bordi condivisi, livelli differenti, stitching.
- Displacement shader/texture evaluation durante la generazione della geometria.
- Bounding box aggiornati e costruzione/refit BVH dopo il displacement.
- Interazione con normali/tangenti, UV, istanze e motion blur.
- Gestione della memoria e limiti massimi di suddivisione.

## Cosa implementare

1. Audit del supporto attuale: mesh importate, triangoli, normali, tangenti, UV, materiali, texture, BVH, istanze e bump mapping.
2. Distinguere chiaramente tre modalità: bump mapping (apparente), displacement reale uniforme (se già supportabile) e adaptive subdivision + displacement.
3. Iniziare con displacement reale su una mesh semplice, usando una height map lineare e un livello di tessellazione controllato.
4. Supportare almeno una modalità di suddivisione coerente con il modello mesh corrente; valutare Catmull–Clark per quad mesh e lineare per triangoli/mesh importate.
5. Implementare un criterio adattivo basato sulla lunghezza proiettata degli edge in pixel rispetto alla camera di render e alla risoluzione finale.
6. Prevedere un'opzione object-space per casi di istancing, come fa Cycles, se compatibile con l'architettura.
7. Garantire bordi senza crepe fra patch adiacenti anche quando la richiesta di suddivisione è diversa.
8. Campionare la height map nello spazio corretto (lineare, non sRGB), supportando precisioni adeguate; non ridurre automaticamente displacement EXR/16-bit a 8-bit.
9. Applicare lo spostamento lungo la normale/tangente definita dal materiale, con scale e midlevel espliciti e documentati.
10. Calcolare bounds conservativi dopo displacement e costruire/refit BVH sui triangoli effettivi.
11. Stabilire limiti di subdivision level, triangoli generati e memoria, con errore esplicito o fallback quando si supera il budget.
12. Gestire la camera attiva: la tessellazione dipendente dalla proiezione deve essere invalidata quando cambiano camera, risoluzione o trasformazioni rilevanti.
13. Per istanze, decidere e documentare se la mesh tessellata può essere condivisa o se deve essere specifica per istanza/camera.
14. Solo dopo la correttezza CPU, valutare la tessellazione GPU. Evitare di complicare il backend GPU prima di avere una pipeline geometrica stabile.
15. Non rimuovere il bump mapping: displacement macro e bump micro possono coesistere, con regole chiare per evitare doppio conteggio dello stesso dettaglio.

## Test e benchmark

- Piano, sfera e mesh con quad regolari; mesh triangolata/importata; UV seam e bordi condivisi.
- Height map sinusoidale, gradino, texture ad alta frequenza e mappa 16/32-bit.
- Verificare silhouette e ombre, non solo la normale/shading.
- Test crack-free fra patch con livelli diversi.
- Test bounds/BVH: raggi devono colpire la superficie spostata e non quella originale.
- Test variazione camera/risoluzione e invalidazione della tessellazione.
- Misurare triangoli generati, memoria, tempo di preprocessing, build BVH e render time.
- Confronto con tessellazione uniforme a qualità visiva equivalente.

**Criterio di accettazione:** displacement realmente geometrico, silhouette e ombre corrette, nessuna crepa visibile, limiti di memoria affidabili e numero di poligoni significativamente inferiore alla suddivisione uniforme a qualità equivalente.

---

# 6. Ordine di implementazione e dipendenze

Implementare tutte le funzionalità con questo ordine consigliato, rivedibile solo se l'audit del codice dimostra una dipendenza concreta:

| Milestone | Funzionalità | Motivo |
|---|---|---|
| A | Light Tree | Relativamente isolabile; migliora NEE e prepara il campionamento delle molte luci |
| B | Null Scattering CPU | Sistema il trasporto volumetrico alla base; mantenere ray marching come fallback |
| C | Wavefront GPU superfici | Intervento architetturale maggiore; prima validare la pipeline senza volumi |
| D | Path Guiding CPU | Si integra con BSDF, NEE, MIS e Light Tree; richiede estimatore già stabile |
| E | Adaptive Subdivision + displacement CPU | Pipeline geometrica/BVH ampia ma distinta dall'integrator |
| F | Estensioni GPU | Portare Null Scattering, Path Guiding e displacement/tessellazione su GPU dopo le rispettive versioni CPU |
| G | Integrazione completa | Benchmark incrociati e verifica che le tecniche interagiscano correttamente |

Non significa che si debba rimandare indefinitamente una funzionalità: tutte devono arrivare a un'implementazione concreta. Per ciascuna milestone consegnare codice, test, benchmark e report prima di passare alla successiva.

## Integrazione fra le tecniche

Verificare esplicitamente queste interazioni:

- **Light Tree + Path Guiding:** NEE e guiding devono usare PDF corrette e pesi MIS coerenti. Nessuna doppia ponderazione.
- **Light Tree + effetti:** luci generate da lucciole/effetti devono entrare nel tree quando sono emitter veri; invalidazione corretta quando cambiano.
- **Null Scattering + Wavefront:** gli eventi null e reali devono diventare stati/transizioni gestibili dalle code; non perdere throughput o RNG state.
- **Path Guiding + Null Scattering:** surface e volume guiding devono avere distribuzioni e PDF corrette e restare separabili.
- **Adaptive Subdivision + BVH:** il BVH deve contenere la geometria dopo displacement, con bounds conservativi.
- **Adaptive Subdivision + GPU:** non creare due implementazioni divergenti della geometria; definire un contratto dati comune CPU/GPU.
- **Tutte le tecniche + animazione/timeline:** stabilire quali cache si possono riutilizzare e quando devono essere invalidate.

## Benchmark comune e metodo scientifico

Creare un set riproducibile di scene benchmark versionate nel repository. Includere almeno:

1. Scena con una luce singola e geometria semplice (controllo regressioni).
2. Scena con molte luci puntiformi/spot/area e mesh emissive.
3. Interno illuminato da una piccola apertura, con illuminazione indiretta difficile.
4. Scena con vetri e superfici rough, per controllare MIS e guiding.
5. Scena con volumi omogenei e sovrapposti.
6. Scena con fumo eterogeneo, nebbia e dettagli sottili.
7. Mesh con displacement ad alta frequenza e silhouette visibile.
8. Scena complessa che combini più tecniche.

Per ogni test:
- seed espliciti e configurazione registrata;
- almeno 3 repliche indipendenti per i confronti rumorosi, se il costo lo consente;
- riferimento ad alto spp con errore caratterizzato;
- confronto a parità di tempo e a parità di qualità (MSE/RMSE), non soltanto a pari spp;
- tempo preprocessing, tempo render, memoria CPU/GPU e qualità;
- indicare hardware e backend effettivi (in particolare RTX 5070 e Ryzen 9 9900X quando disponibili);
- salvare immagini, log, JSON e script di confronto nel report.

Non usare il solo tempo per campione come metrica: una tecnica può aumentare il costo per sample ma ridurre molto la varianza.

## Requisiti di qualità e compatibilità

- Le nuove tecniche devono essere unbiased quando dichiarate tali. Documentare le approssimazioni e i casi in cui si usa fallback.
- Non modificare silenziosamente i valori fisici dei materiali, delle luci o dei volumi.
- CPU e GPU devono convergere alla stessa soluzione fisica, anche se i pixel non coincidono per via del sampling.
- Le cache devono avere chiavi d'invalidazione complete e versionate.
- La memoria deve essere limitata e monitorata; nessuna allocazione non controllata di code, path state o microtriangoli.
- I progetti vecchi devono aprirsi come prima.
- Le impostazioni devono essere comprensibili e avere default sensati. Durante la fase sperimentale le funzioni nuove possono essere opt-in.
- Nessun peggioramento delle scene leggere senza motivo: mantenere fallback o selezione automatica solo se dimostrata dai benchmark.

## Deliverable per ogni milestone

1. Analisi iniziale con file/moduli ForgeRender effettivamente coinvolti.
2. Nota sull'algoritmo e sulle differenze rispetto a Cycles.
3. Implementazione completa della milestone, non stub o UI fittizia.
4. Test unitari e test statistici/integrazione.
5. Benchmark prima/dopo con report e dati grezzi.
6. Verifica CPU e GPU quando la feature dichiara supporto per entrambe.
7. Aggiornamento manuale EN/IT, README e changelog quando è pronta per l'uso.
8. Commit dedicato su branch di lavoro.
9. Risposta finale con:
   - branch e SHA completo;
   - file modificati e motivazione;
   - algoritmi implementati;
   - test eseguiti con comandi e risultati;
   - benchmark reali;
   - limiti e casi non supportati;
   - verifiche manuali effettivamente eseguite;
   - prossima milestone.

## Prima azione richiesta a Claude

1. Verificare HEAD di `main` e `sviluppo`, leggere handoff e report aggiornati e confermare la versione reale del codice.
2. Fare audit mirato dei moduli attuali di lighting/NEE/MIS, integrator CPU/GPU, volumi/majorant, mesh/BVH/texture/displacement.
3. Creare una mappa dei punti di integrazione ForgeRender per tutte e cinque le tecniche, indicando i file reali (non ipotizzati).
4. Preparare benchmark baseline prima di cambiare il codice.
5. Iniziare subito la Milestone A (Light Tree) dopo aver registrato la baseline. Non aspettare un'ulteriore approvazione per iniziare: Valerio ha già richiesto l'implementazione di tutte e cinque.
6. Procedere poi con le milestone successive, aggiornando handoff e report senza sovrascrivere le note precedenti.

**Importante:** questa specifica è una richiesta di implementazione completa, non una dichiarazione che le funzionalità siano già presenti o che i benchmark siano stati eseguiti. Nessuna delle cinque tecniche viene considerata implementata finché il codice, i test e i risultati non lo dimostrano.
