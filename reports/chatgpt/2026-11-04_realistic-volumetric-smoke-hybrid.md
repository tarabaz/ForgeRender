# Richiesta Valerio → Claude Code — 04/11/2026
## Nuvole di fumo volumetriche realistiche — approccio ibrido procedurale + simulazione

Valerio ha scelto esplicitamente:
- **Modalità di creazione: BOTH** — sia simulazione fisica sia generazione procedurale.
- **Aspetto visivo: REALISTIC** — priorità a fumo morbido, naturale, cinematografico e credibile fisicamente. Non prioritizzare nuvole stilizzate/cartoon.

Questa richiesta approva lo sviluppo di questa direzione e chiede di trasformare lo studio esistente `docs/studio-fumo-fuoco.md` in un piano operativo concreto, verificato sul codice attuale. Non significa approvare automaticamente ogni dettaglio o stima contenuta nello studio: prima verificare lo stato reale del branch.

## 1. Obiettivo prodotto

Aggiungere a ForgeRender un sistema per creare **vere nuvole volumetriche di fumo**, posizionabili nella scena e utilizzabili in render statici di prodotto/ambientazioni. Il risultato deve avere struttura interna tridimensionale, bordi irregolari, riccioli, filamenti, variazioni di densità e scattering credibile, con interazione con le luci e ombre volumetriche.

Casi d'uso:
- nube di fumo che esce da un tubo, ugello, camino o contenitore;
- fumo che sale, si espande, forma vortici e si dissolve;
- nube sospesa in una posizione scelta, congelata a un istante per il rendering;
- fumo attraversato da fasci luminosi, con scattering e ombre;
- vapore realistico da tazza, pentola o superficie calda;
- fumo d'incenso/sigaretta e fumo industriale.

## 2. Due workflow che devono coesistere

### A. Simulazione fisica
L'utente definisce un contenitore/volume, una o più sorgenti e i parametri di simulazione. Il solver evolve il fumo nel tempo. L'utente può scegliere il tempo/istante da congelare e usare il risultato come volume statico nel path tracer.

Parametri artistici principali, con nomi comprensibili:
- sorgente: punto, sfera, disco, box e/o mesh di scena;
- posizione, orientamento e dimensioni sorgente;
- intensità/portata di emissione;
- densità del fumo;
- temperatura e galleggiamento (fumo caldo che sale);
- gravità/peso (gas pesante che scende);
- turbolenza, vortici e scala dei vortici;
- espansione;
- dissipazione/decadimento;
- dettaglio e qualità simulazione;
- durata e tempo finale/istante da visualizzare;
- seed deterministico.

La simulazione deve essere riproducibile: stessi parametri e seed producono lo stesso risultato. Cache e invalidazione devono reagire alle modifiche corrette.

### B. Generazione procedurale
Workflow rapido per chi non vuole simulare. L'utente crea una nube direttamente con forma/volume e parametri artistici, oppure parte da un preset e modifica la nube. La generazione deve creare un campo di densità 3D credibile, non una semplice texture 2D o una sfera uniforme.

Controlli:
- forma/ingombro iniziale (sfera, ellissoide, pennacchio, box o forma libera se fattibile);
- densità e opacità;
- scala e intensità dei dettagli/frattali;
- filamenti e riccioli;
- deformazione/turbolenza procedurale;
- morbidezza dei bordi;
- anisotropia/scattering;
- colore/albedo del fumo;
- seed;
- eventuale variazione verticale o direzione preferenziale.

Il risultato deve essere modificabile in modo non distruttivo. Il workflow procedurale deve poter essere usato da solo e non richiedere la simulazione.

### C. Interoperabilità
I due workflow devono condividere lo stesso formato volumetrico di uscita e lo stesso percorso di rendering. Se tecnicamente fattibile, consentire:
- iniziare con una nube procedurale e usarla come stato iniziale della simulazione;
- aggiungere dettaglio procedurale a una simulazione senza alterarne la struttura macroscopica;
- congelare/bake la simulazione in un volume modificabile/renderizzabile;
- conservare parametri sorgente, seed e impostazioni per riaprire e rigenerare.

Non duplicare due implementazioni del volume nel renderer: preferire una rappresentazione comune.

## 3. Aspetto e resa: priorità realistica

La nube deve evitare l'aspetto di una nebbia uniforme o di un blob procedurale. Priorità:
- struttura volumetrica a più scale;
- masse principali morbide e disomogenee;
- riccioli e filamenti secondari;
- bordi che si sfilacciano;
- variazioni di densità interne;
- scattering con luci e attenuazione coerenti;
- ombre volumetriche credibili;
- assenza di banding, cubi/voxel evidenti e tagli netti del volume.

Fornire preset realistici iniziali: incenso, vapore da tazza, pennacchio da tubo, fumo denso industriale, nube sospesa. I preset sono punti di partenza modificabili, non risultati bloccati.

## 4. Volume e manipolazione nella viewport

Integrare un box gizmo 3D per il contenitore/area di simulazione o generazione:
- spostamento, ridimensionamento e rotazione ove supportata;
- rappresentazione wireframe/trasparente;
- gizmo separato per sorgenti, se presenti;
- il box deve controllare realmente il volume renderizzato e la regione simulata, non essere solo decorativo;
- spostare o ridimensionare deve aggiornare correttamente distribuzione, cache e preview senza lasciare residui nella posizione precedente;
- salvare trasformazione, dimensioni e impostazioni nel progetto;
- undo/redo e riapertura progetto devono mantenere lo stato.

Definire con chiarezza la differenza tra contenitore di simulazione, volume di generazione procedurale e sorgente. Non confonderli nell'interfaccia.

## 5. Rendering e compatibilità

Ispezionare e verificare il percorso effettivo `BakedFog`/volume e il rendering CPU/GPU. La stessa nube deve avere aspetto coerente nei due backend, entro tolleranze documentate. Preservare delta tracking, ratio tracking per ombre, Henyey–Greenstein e majorant corretti.

Verificare:
- scattering e assorbimento del fumo;
- ombre volumetriche;
- interazione con superfici e luci;
- comportamento con raggi volumetrici e luci in controluce;
- gestione dei volumi sovrapposti;
- majorant e limiti numerici;
- memoria e tempi di bake/render;
- serializzazione e retrocompatibilità dei progetti esistenti.

Non introdurre emissione del fumo come default: il fumo realistico è principalmente scattering/assorbimento. L'emissione volumetrica appartiene a una futura fase fuoco/brace, se necessaria.

## 6. Piano di sviluppo richiesto a Claude

Prima di modificare codice:
1. Rileggere questo documento e `docs/studio-fumo-fuoco.md`.
2. Ispezionare il codice e confermare quali parti dello studio sono ancora valide, quali sono già implementate e quali sono cambiate.
3. Individuare il modello volumetrico comune, i punti d'ingresso UI/viewport, serializzazione, cache e percorsi CPU/GPU.
4. Proporre una roadmap incrementale con dipendenze, rischi e test, distinguendo MVP da funzionalità successive.
5. Evidenziare quali funzionalità possono riutilizzare il solver attuale e quali richiedono nuovi canali/campi o modifiche al renderer.

Direzione consigliata per le fasi:
- **Fase 1 — Procedurale realistico MVP:** generazione di nubi statiche 3D con seed, forme, densità multi-scala, filamenti e scattering, box gizmo e preset. Deve già produrre render convincenti senza simulazione.
- **Fase 2 — Simulazione fumo realistica:** riuso/evoluzione del solver esistente, temperatura e galleggiamento, sorgenti configurabili, turbolenza, dissipazione e selezione del tempo congelato.
- **Fase 3 — Pipeline ibrida:** bake comune, dettaglio procedurale guidato dalla simulazione e possibilità di usare una nube procedurale come stato iniziale.
- **Fase 4 — Qualità e ottimizzazione:** risoluzione adattiva/sparsa, preview rapida, cache, benchmark, CPU/GPU parity.
- Fuoco/combustione resta un progetto separato e successivo: non bloccare il fumo realistico in attesa della simulazione delle fiamme.

Questa sequenza è una proposta: Claude deve motivare eventuali cambi d'ordine in base all'architettura effettiva.

## 7. Test e criteri di accettazione

Preparare scene di test riproducibili:
- pennacchio sottile di incenso con vortici;
- vapore bianco che sale da una tazza;
- nube densa industriale che si espande;
- nube procedurale sospesa in un box;
- nube illuminata lateralmente e in controluce;
- nube che interagisce con oggetti e produce ombre volumetriche.

Criteri:
- il volume appare tridimensionale da più angolazioni, non come billboarding;
- densità e bordi hanno variazioni a più scale;
- il fumo riceve luce e produce attenuazione/ombre credibili;
- spostando il box, il risultato segue davvero;
- seed e parametri riproducono il risultato;
- CPU e GPU sono confrontati;
- vecchi progetti e attuale effetto nebbia non cambiano inaspettatamente.

Produrre render comparativi e benchmark. Riportare i test effettivamente eseguiti, non solo quelli previsti.

## 8. Stato e consegna

Questa è una **approvazione della direzione prodotto e richiesta di piano operativo**, non autorizzazione a fare un refactor massivo senza prima verificare il codice. Claude deve rispondere con:
- stato attuale verificato;
- roadmap tecnica e stima per fasi;
- primo MVP consigliato;
- rischi e compromessi;
- file/moduli che intende toccare;
- test di accettazione.

Dopo approvazione del piano, procedere per fasi piccole, con commit separati e handoff aggiornato in `reports/chatgpt/CLAUDE_HANDOFF.md`.

**Decisione di Valerio:** workflow ibrido (simulazione fisica + procedurale), stile realistico/cinematografico. Nessuna build, modifica applicativa o test è stato eseguito da ChatGPT.
