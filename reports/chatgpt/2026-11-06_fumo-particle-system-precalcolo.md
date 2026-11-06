# ForgeRender — Precalcolo completo del fumo e interazione del Particle System

Data: 06/11/2026  
Tipo: richiesta tecnica di Valerio a Claude Code  
Stato: da analizzare e implementare  
Branch di riferimento: sviluppo

## Obiettivo

Rendere persistenti e coerenti i calcoli temporali del fumo e del Particle System durante le animazioni. Il precalcolo deve includere tutti i dati necessari al rendering e le particelle devono interagire realmente con gli oggetti animati.

## 1. Fumo: salvare anche la griglia fine

Il sistema esistente salva già dati temporanei del fumo e consente la ripresa dal primo fotogramma modificato. Estendere il precalcolo affinché salvi su disco anche la griglia fine utilizzata per il risultato.

- Calcolare e salvare tutti i dati necessari per ogni fotogramma, inclusa la griglia fine.
- Il render dell'animazione deve leggere e riutilizzare i dati precalcolati, senza ricostruire o ricampionare la griglia fine per ogni frame.
- Conservare riuso dei frame invariati e ripresa dal primo frame realmente modificato.
- Invalidare la cache quando cambiano parametri, sorgenti, geometrie, trasformazioni o altri input che influenzano la simulazione.
- Gestire versione del formato, integrità, cache incompleta/corrotta e pulizia dei temporanei.
- Esaminare e riutilizzare, dove sensato, l'infrastruttura esistente in crates/forge_render_core/src/smoke_bake.rs.

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
- Valutare il riuso dei meccanismi comuni di smoke_bake.rs (impronte, cache, ripresa, invalidazione), mantenendo distinti i dati e le logiche specifiche delle particelle.

## 4. Analisi architetturale richiesta

Prima dell'implementazione, descrivere brevemente:
- dove vengono eseguite oggi le simulazioni di fumo e particelle;
- come vengono valutate le trasformazioni degli oggetti a ogni frame;
- come sono gestite collisioni e interazioni;
- quali dati sono già salvati e quali ricostruiti;
- come viewport e render recuperano lo stato simulato.

Evitare due sistemi di cache duplicati se è possibile condividere il meccanismo comune. Non compromettere la reattività dell'anteprima durante la normale modifica della scena.

## 5. Criteri di accettazione

1. Il precalcolo del fumo salva anche la griglia fine e il render la riutilizza senza ricostruirla per ogni frame.
2. Una modifica ai parametri del fumo invalida dal primo frame interessato, non prima.
3. Il Particle System reagisce visibilmente agli oggetti animati lungo la timeline.
4. Viewport e render mostrano la stessa evoluzione temporale delle particelle.
5. Una modifica alla traiettoria o geometria di un oggetto interagente ricalcola soltanto la parte dipendente dalla modifica.
6. I dati precalcolati restano riutilizzabili dopo la riapertura del progetto, se validi.
7. Dimensioni e pulizia della cache sono controllate.
8. Nessuna regressione alla simulazione normale del fumo o all'anteprima delle particelle.

## 6. Test richiesti

Creare una scena riproducibile con emettitore, oggetto animato che attraversa la traiettoria delle particelle e prova di controllo con oggetto fermo. Includere anche una simulazione di fumo su più frame con griglia fine.

Verificare: primo precalcolo; riproduzione da cache; modifica parametri e invalidazione; modifica della traiettoria e aggiornamento delle interazioni; confronto viewport/render; chiusura e riapertura progetto; cache parziale o non valida.

I test unitari non bastano per dichiarare risolto il problema: effettuare anche una verifica visiva reale dell'animazione in viewport e del render, se l'ambiente lo consente.

## 7. Risposta richiesta a ChatGPT

Aggiungere al normale handoff una risposta datata con:
- causa effettiva del problema del Particle System;
- comportamento precedente e comportamento ottenuto;
- file modificati e motivazione;
- architettura e formato della cache per fumo e particelle;
- strategia di invalidazione e ripresa;
- test e comandi realmente eseguiti e relativi risultati;
- verifica visiva svolta oppure esplicitamente non eseguita;
- dimensioni dei temporanei e tempi di precalcolo/lettura, se misurati;
- limiti residui e attività aperte.

Questa nota è una richiesta di analisi e implementazione, non una dichiarazione che le funzionalità siano già state realizzate o verificate. Distinguere fatti osservati, ipotesi, test eseguiti e attività ancora da fare.
