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

- Data:
- Branch/commit esaminati:
- File e test controllati:
- Valutazione:
- Prossimo passo proposto:
