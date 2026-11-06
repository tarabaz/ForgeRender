# Decisione approvata — interazione unidirezionale della polvere sospesa

Data: 06/11/2026  
Progetto: ForgeRender  
Stato: **approvato da Valerio**

## Decisione

Per l'effetto **Polvere sospesa**, adottare come architettura iniziale un'interazione **unidirezionale**:

- gli oggetti in movimento possono influenzare il moto della polvere;
- le correnti d'aria e il campo di velocità del fumo, quando disponibili e convenienti da campionare, possono trascinare o deviare la polvere;
- la polvere **non modifica la simulazione del fumo** e non genera feedback sulle correnti.

## Motivazione

Questa soluzione riduce sensibilmente complessità e costo computazionale, evitando un accoppiamento bidirezionale che difficilmente darebbe un vantaggio visivo proporzionato. L'obiettivo è ottenere un effetto convincente: pulviscolo leggero che reagisce all'ambiente, senza trasformare la simulazione in un sistema fluidodinamico accoppiato.

## Indicazioni per l'implementazione

1. Riutilizzare, ove possibile, il campo di velocità o le informazioni di spinta dell'aria già calcolate per il fumo e per gli oggetti animati.
2. Applicare l'influenza alla polvere con intensità controllabile e attenuata in base a inerzia/risposta dei granelli.
3. Se il campo del fumo non è disponibile nel punto in cui viene aggiornata la polvere, non introdurre dipendenze invasive: proporre un'interazione semplificata e documentarne il limite.
4. Non implementare feedback dalla polvere al fumo.
5. Mantenere comportamento deterministico e coerente tra viewport, salto diretto nella timeline e render.
6. Misurare il costo con quantità rappresentative di particelle e verificare che l'interazione non comprometta le prestazioni.

## Ambito della decisione

Questa approvazione riguarda **la polvere sospesa** descritta nella sezione dedicata del documento:

`reports/chatgpt/2026-11-06_modifiche-animazione-simulazioni-export-video.md`

Non estendere automaticamente questa decisione al nuovo effetto **Lucciole / Bosco incantato**, che ha requisiti distinti di illuminazione reale e deve essere progettato separatamente.

