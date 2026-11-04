# Richiesta Valerio → Claude Code — 04/11/2026
## Tilt-shift indipendente dalla DOF e camera manipolabile nella viewport

Questa richiesta integra il backlog già presente in `reports/chatgpt/CLAUDE_HANDOFF.md`. Non eliminare né sostituire le richieste precedenti.

## 1. Modalità Tilt-Shift indipendente dal diaframma

- Aggiungere una modalità **Tilt-Shift** separata dalla profondità di campo (DOF) fisica. Attivarla/disattivarla non deve modificare apertura/diaframma, focale, focus distance o altri parametri fisici.
- Definire una fascia nitida nell'immagine con posizione, larghezza e orientamento (orizzontale, verticale, angolo libero).
- Esporre un controllo manuale della transizione verso le zone sfocate, indipendente dalla larghezza della fascia nitida. L'utente deve poter scegliere una transizione molto stretta o molto ampia senza che la fascia nitida si allarghi automaticamente.
- Fornire valori numerici e slider con limiti minimi/massimi espliciti, clamp e gestione robusta dei casi limite: transizione zero/dura, transizione massima, fascia molto stretta o al bordo dell'immagine. Se utile, controllare separatamente le transizioni sui due lati della fascia.
- Preferire un effetto screen-space/post-process guidato dalla depth buffer, con blur progressivo e depth-aware per limitare il bleeding fra oggetti a profondità diverse. Non cambiare la scala della scena o il diaframma per ottenere l'effetto.
- Definire e documentare cosa accade quando DOF fisica e Tilt-Shift sono attivi insieme: composizione prevedibile, parametri fisici invariati.
- Salvare le impostazioni nel progetto e mantenere i default retrocompatibili. Verificare CPU/GPU e l'eventuale preview.

## 2. Camera come componente vero e proprio

- La camera deve comparire nell'albero/componenti immediatamente **sopra Luci**, essere selezionabile e mostrare posizione, direzione e target nella viewport.
- Aggiungere un bersaglio come per la luce spot: un punto o un oggetto da mantenere al centro della mira.
- Consentire di spostare e orientare la camera direttamente nella viewport con gizmo/manipolatori, non soltanto tramite valori numerici.
- Distinguere chiaramente:
  - traslazione della camera;
  - orbitazione attorno al target, mantenendo fermo il bersaglio;
  - spostamento del target, mantenendo la posizione della camera e aggiornando la direzione.
- Il target deve avere un marker/manipolatore visibile e selezionabile. Evitare conflitti con la normale navigazione della viewport.
- Il componente deve controllare la camera realmente usata dal renderer, non essere un oggetto visuale scollegato.
- Verificare serializzazione, apertura dei vecchi progetti, undo/redo e coerenza con gli strumenti MCP esistenti (in particolare set_camera e scene_info).

## 3. Altre richieste già nel backlog: non perderle

Implementare anche le richieste già annotate nel documento principale, senza cancellarle o considerarle implicitamente completate:

1. **Stili e contorni:** mantenere gli stili a colori esistenti; aggiungere Tecnico bianco e nero e Fumetto bianco e nero; sfondo disattivabile/trasparente o con colore configurabile distinguendo pixel di miss e piano geometrico; diagnosticare e correggere i puntini isolati dei contorni senza perdere dettagli validi; valutare un controllo di sensibilità o un filtro dedicato ai componenti isolati.
2. **Foschia:** aggiungere la quota iniziale/inferiore oltre all'altezza massima/superiore, così da poter creare una fascia sospesa, ad esempio 2,2–2,8 m. Verificare la semantica reale di haze_floor e top_height, il riferimento verticale, le transizioni morbide, CPU/GPU e la retrocompatibilità.
3. **Preset foschia:** la raccolta dei nuovi valori predefiniti è ancora aperta. È stata ricevuta solo la schermata per Fumo tossico a terra. **Non implementare i nuovi default** finché Valerio non invia le altre schermate e conferma che la raccolta è terminata. Non modificare gli effetti già salvati.

## 4. Consegna e verifiche

- Ispezionare il codice aggiornato prima di decidere l'architettura; non assumere che le descrizioni del backlog siano ancora identiche al codice.
- Implementare in milestone, senza perdere nessuna richiesta. Dare priorità a camera/target/viewport e Tilt-Shift.
- Aggiornare UI italiana/inglese, documentazione e changelog.
- Testare serializzazione, limiti, trasformazioni camera-target, orbitazione, undo/redo, coerenza CPU/GPU e casi limite del blur.
- Eseguire render di confronto per verificare visivamente fascia nitida, transizioni e artefatti. Dichiarare chiaramente test non eseguiti e limiti.
- Mantenere i default e la compatibilità dei progetti esistenti. Non dichiarare completata una funzione solo perché analizzata o compilata.

**Stato:** richiesta di implementazione. ChatGPT non ha modificato codice applicativo né eseguito build, test o render.
