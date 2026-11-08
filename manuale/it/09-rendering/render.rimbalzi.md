---
id: render.rimbalzi
titolo: Rimbalzi, Russian roulette, clamp e seed
parole: rimbalzi, rimbalzi massimi, percorsi di luce, profondità, russian roulette, clamp, clamp indiretto, firefly, puntini bianchi, seed, path tracing
ui: panel.render.path_tracing
correlate: render.campioni, mat.transmission, render.caustiche
---
# Rimbalzi, Russian roulette, clamp e seed

**Dove si trova:** scheda Render → **Path tracing**.

## Rimbalzi massimi

**A cosa serve:** quante volte un percorso di luce può rimbalzare prima di fermarsi. Ogni
rimbalzo aggiunge la luce che arriva da altre superfici: la luce indiretta che riempie le ombre, i
riflessi dei riflessi e soprattutto la luce che attraversa i **vetri**, che vuole almeno due
rimbalzi per ogni superficie di vetro che attraversa.

![1 rimbalzo: solo luce diretta, vetri e specchi neri](img/comune/render_bounces_01.png) ![3 rimbalzi: vetro in parte trasparente, specchi ancora scuri dentro](img/comune/render_bounces_03.png) ![12 rimbalzi: luce completa](img/comune/render_bounces_12.png)

*Rimbalzi massimi 1, 3 e 12.*

- **Intervallo:** da 1 a 64.
- **Predefinito:** 12 (modalità Semplice: Bozza 8, Bilanciata 12, Alta 16).
- **Tempo:** più rimbalzi costano un po' di tempo in più per campione, ma solo dove la luce
  rimbalza davvero tante volte; oltre 12 la differenza si vede solo con molti vetri uno dietro
  l'altro (bicchieri con liquido, lampadari, lenti).

## Russian roulette da

Da questo rimbalzo in poi, i percorsi scuri che portano poca luce si fermano a caso (e quelli che
restano contano di più, così il risultato resta giusto). Fa risparmiare tempo senza cambiare
l'immagine in media.

- **Intervallo:** da 1 a 16. **Predefinito:** 3.
- Valori più bassi = passate più veloci ma un po' più di rumore nella luce indiretta.

## Clamp indiretto

Limita la luminosità della luce indiretta. Riduce i **firefly** (pixel isolati molto luminosi,
tipici di luci piccole viste attraverso un vetro o nei riflessi lucidi), ma toglie un po' di
energia, quindi l'immagine non è più fisicamente esatta.

- **Intervallo:** da 0 a 1000. **Predefinito:** 0 = disattivato (fisicamente corretto).
- Se ti serve, parti da valori alti (10 – 20) e abbassali solo quanto basta.

## Seed

Il numero di partenza della sequenza casuale. La stessa scena con lo stesso seed dà esattamente
la stessa immagine; cambiandolo cambia il disegno del rumore, a parità di qualità. Serve per i
confronti e per rifare lo stesso fotogramma.
