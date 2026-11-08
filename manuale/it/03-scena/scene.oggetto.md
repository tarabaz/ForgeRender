---
id: scene.oggetto
titolo: Spostare, ruotare e ridimensionare (scheda Oggetto)
parole: oggetto, posizione, dimensioni, misure, sposta, ruota, scala, appoggia, pavimento, duplica, elimina, snap, contatto
ui: panel.object, edit.duplicate, edit.delete, edit.hide, edit.isolate, edit.show_all
correlate: ui.viewport, scene.importa, mat.pannello
---
# Spostare, ruotare e ridimensionare (scheda Oggetto)

**Dove si trova:** modalità Avanzata → scheda **Oggetto**, con uno o più componenti selezionati.

| Controllo | Cosa fa |
|---|---|
| Duplica / Elimina | Copia o toglie la selezione (Elimina la toglie dal progetto, non dal file importato; Ctrl+Z la riporta) |
| Appoggia | Scende finché tocca l'oggetto che ha subito sotto (un altro pezzo, il fondale o il pavimento); con niente sotto va a quota 0 |
| Centra nell'origine | Porta il centro su X = 0, Z = 0 |
| Reimposta | Torna alla posizione del file originale |
| Snap a contatto | Spostandolo con il gizmo si ferma quando tocca un altro oggetto e non lo attraversa |
| Ruota 90° | Rotazioni rapide attorno a X, Y e Z |
| Dimensioni (mm) | Misure reali della selezione; con **Mantieni proporzioni** gli altri lati seguono |
| Spostamento (mm) | Posizione |
| Rotazione | Angoli in gradi |
| Scala | Fattore; pulsanti rapidi ×0.1, ×10 e ×25.4 (da pollici a millimetri) |

Ogni valore si annulla con Ctrl+Z. Gli stessi movimenti si fanno nella viewport con il gizmo:
vedi [Viewport](ui.viewport).

## Nascondere, duplicare, eliminare

| Tasti | Comando |
|---|---|
| Ctrl+D | Duplica la selezione |
| Canc | Elimina la selezione |
| H | Nascondi la selezione |
| Maiusc+H | Nascondi tutto il resto |
| Alt+H | Mostra tutto |

## Consiglio

**Scena → Appoggia sulla base** e **Scena → Adatta la scena al prodotto** lavorano su tutto il
prodotto: sono il modo più rapido per poggiarlo sul fondale e ridimensionare luci e fondale
attorno a lui.
