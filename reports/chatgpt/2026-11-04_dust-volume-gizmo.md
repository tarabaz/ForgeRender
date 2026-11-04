# Richiesta Valerio → Claude Code — 04/11/2026
## Pulviscolo: gizmo a box per spostare e posizionare il volume/area d'interazione nella viewport

### Problema osservato
Attualmente il pulviscolo sembra essere generato/posizionato prendendo come riferimento la posizione iniziale nella viewport. Se successivamente sposto l'effetto o la sua posizione, il pulviscolo resta legato alla zona in cui era stato generato e non segue correttamente lo spostamento.

### Funzionalità richiesta
Integrare un **box gizmo (volume delimitatore 3D)** visibile e manipolabile direttamente nella viewport, che permetta di posizionare e spostare l'area/volume in cui il pulviscolo interagisce con la scena.

L'utente deve poter:
- selezionare il volume del pulviscolo nella viewport;
- trascinare il box per spostarlo nello spazio 3D;
- usare, se coerente con il sistema di gizmo già esistente, le maniglie per ridimensionarlo e modificarne l'estensione;
- eventualmente ruotarlo, se il volume supporta un orientamento e la rotazione ha un significato reale;
- vedere chiaramente i limiti del volume mentre lo modifica, con una rappresentazione wireframe/trasparente che non copra la scena;
- spostare il volume anche dopo la sua creazione, senza dover cancellare e ricreare l'effetto.

### Comportamento atteso
- La posizione del box deve determinare la regione spaziale effettiva in cui il pulviscolo viene generato/considerato dal renderer, non essere soltanto un elemento grafico della viewport.
- Quando il box viene spostato, il volume e il relativo campo di densità/particelle devono seguire la nuova posizione. Non devono rimanere particelle fantasma nella posizione precedente, né deve essere necessario ricreare manualmente l'effetto.
- Definire chiaramente se il box rappresenta un volume di generazione, un volume di influenza o entrambi. Claude deve prima ispezionare il modello attuale del pulviscolo e mantenere una semantica coerente con il modo in cui il renderer lo calcola.
- Se il pulviscolo è attualmente legato a una mesh/materiale o a una posizione campionata/baked, verificare come rendere la trasformazione dinamica senza introdurre incoerenze o costi eccessivi. Non aggiungere un gizmo che sposta solo il wireframe lasciando invariato il risultato renderizzato.
- Gestire correttamente aggiornamento/invalidation della cache, preview e render finale dopo ogni modifica del box.
- Salvare posizione, dimensioni ed eventuale rotazione nel progetto; garantire undo/redo, serializzazione e retrocompatibilità.
- Integrare selezione, manipolazione e visualizzazione con il sistema gizmo/viewport già presente, evitando un sistema parallelo se quello esistente può essere esteso.
- Aggiornare UI e localizzazione IT/EN, documentazione e test.

### Verifiche richieste
1. Creare un effetto pulviscolo, spostare il box di una distanza evidente e verificare che anche il risultato renderizzato si trasferisca nella nuova zona.
2. Modificare le dimensioni e verificare che la regione di generazione/influenza si aggiorni.
3. Spostare il box più volte: non devono restare residui nella posizione precedente.
4. Salvare e riaprire il progetto: posizione e dimensioni devono essere conservate.
5. Verificare preview e render finale, CPU e GPU se entrambi i percorsi gestiscono l'effetto.
6. Verificare che il box non alteri involontariamente geometria, luci o altri effetti volumetrici.

### Prima di implementare
Ispezionare il codice e descrivere come viene generato e posizionato oggi il pulviscolo, perché la sua distribuzione sembra restare ancorata alla posizione iniziale e quale trasformazione/volume è tecnicamente corretto esporre. Se un box oriented non è supportato dal modello attuale, proporre una prima versione con AABB allineato agli assi, ma deve essere realmente collegato al rendering.

**Stato:** richiesta funzionale da analizzare e implementare; nessuna modifica al codice, build, test o render è stata eseguita da ChatGPT.
