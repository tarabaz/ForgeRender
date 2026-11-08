---
id: render.esposizione
titolo: Esposizione e tone mapping
parole: esposizione, ev, stop, luminosità, troppo scuro, troppo chiaro, tone mapping, aces, filmico, standard, clamp, alte luci, contrasto
ui: panel.render.display
correlate: env.ambiente, light.tipi, export.immagine
---
# Esposizione e tone mapping

**Dove si trova:** scheda Render → **Visualizzazione (non riavvia il render)**.

Tutte e due cambiano solo il modo in cui la luce calcolata diventa i colori dell'immagine: valgono
subito, senza riavviare il render, e si applicano ai PNG, JPG e TIFF esportati.

## Esposizione (EV)

**A cosa serve:** schiarisce o scurisce tutta l'immagine, come l'esposizione di una fotocamera.
Ogni passo (EV, *stop*) raddoppia o dimezza la luce.

![Esposizione −1 EV: metà della luce](img/comune/render_exposure_minus1.png) ![Esposizione 0 EV: come calcolata](img/comune/render_exposure_0.png) ![Esposizione +1 EV: il doppio della luce](img/comune/render_exposure_plus1.png)

- **Intervallo:** da −6 a +6 EV.
- **Predefinito:** 0.
- **Usala** per correggere tutta l'immagine. Se è sbagliata solo una zona, cambia le luci.

## Tone mapping

**A cosa serve:** la luce calcolata dal renderer non ha un limite in alto (un softbox è migliaia
di volte più luminoso di un'ombra), mentre un'immagine va dal nero al bianco. Il tone mapping
decide come comprimerla.

![Standard (clamp): colori saturi, alte luci tagliate al bianco](img/comune/render_tonemapping_standard.png) ![ACES filmico: alte luci che sfumano morbide, più naturale](img/comune/render_tonemapping_aces.png)

*Tutte e due a +0,5 EV.*

| Scelta | Effetto |
|---|---|
| ACES filmico | Predefinito. Una curva da pellicola: le alte luci sfumano piano verso il bianco, i colori delle zone molto luminose si desaturano come sulla pellicola |
| Standard (clamp) | Nessuna compressione: quello che supera il bianco viene tagliato. Colori più saturi e contrastati, alte luci bruciate prima |

## Consigli

- Tieni **ACES** per le foto con riflessi luminosi, vetri e luci in vista.
- Usa **Standard** quando conta di più il colore esatto di una superficie che le alte luci
  (campioni di colore, illustrazioni piatte).
- L'esportazione **EXR** salva la luce lineare (con l'esposizione, senza tone mapping): la curva
  si decide dopo, in un programma di compositing.
