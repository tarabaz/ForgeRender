---
id: render.caustiche
titolo: Caustiche e render spettrale
parole: caustiche, photon mapping, fotoni, luce concentrata, spettrale, lunghezza d'onda, dispersione, arcobaleno, prisma, abbe, rgb
ui: panel.render.caustics, panel.render.color
correlate: mat.transmission, render.rimbalzi, light.tipi
---
# Caustiche e render spettrale

**Dove si trova:** scheda Render → **Colore** (RGB / Spettrale) e **Caustiche**.

## Caustiche

**A cosa serve:** la luce che passa da vetri lisci, cristalli e gemme si concentra sulle superfici
attorno: le macchie luminose sotto un bicchiere, la luce messa a fuoco da una lente, gli arcobaleni
di un pendente. Il path tracing da solo trova raramente questi percorsi, soprattutto con luci
piccole; l'opzione **Caustiche** li calcola con il photon mapping progressivo e funziona con tutte
le luci, anche puntiformi, spot e sole.

![Caustiche spente: l'ombra della sfera di vetro è uniforme](img/comune/render_caustics_off.png) ![Caustiche accese: la sfera concentra la luce in una macchia luminosa](img/comune/render_caustics_on.png)

*Luce puntiforme dietro la sfera di vetro, 512 campioni.*

| Controllo | Intervallo | Predefinito | Cosa fa |
|---|---|---|---|
| Caustiche | sì / no | no | Accende il photon mapping |
| Fotoni per passata | 20k – 2000k | 200k | Più fotoni = caustiche meno rumorose a ogni passata, passate più lente |
| Raggio iniziale | 0 – 100 mm | 0 = automatico (1 % della scena) | Raggio entro cui un punto raccoglie i fotoni alla prima passata; si restringe da solo a ogni passata, così le caustiche diventano sempre più nitide |

Le caustiche nascono dai vetri lisci (trasmissione, ruvidità bassa). I rimbalzi dei fotoni sono i
**Rimbalzi massimi** del render. Sulla GPU i percorsi dei fotoni si costruiscono sulla CPU e la GPU
li raccoglie.

## RGB o Spettrale

| Scelta | Cosa fa |
|---|---|
| RGB | Predefinito. Tre canali, rosso, verde e blu: veloce |
| Spettrale | La luce viaggia per lunghezze d'onda (4 per raggio): **dispersione** nei vetri e nei gioielli (arcobaleni), colori più fedeli con luci sature. Più lento e con più rumore di colore: servono più campioni |

La dispersione si regola nel materiale (**Dispersione**, numero di Abbe): vedi
[Transmission](mat.transmission). Si vede solo con il render Spettrale; caustiche e spettrale si
possono usare insieme per gli arcobaleni di prismi e gemme.
