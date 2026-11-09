---
id: fx.effetti
titolo: Scheda Effetti (fumo, nebbia, polvere, lucciole, adesivi)
parole: effetti, fumo, nebbia, ghiaccio secco, foschia, polvere, lucciole, adesivo, decal, etichetta, logo, volumetrico, pennacchio, incenso, vapore
ui: panel.effects
correlate: fx.foschia, light.tipi, anim.timeline
---
# Scheda Effetti (fumo, nebbia, polvere, lucciole, adesivi)

**Dove si trova:** modalità Avanzata → **Scena → Effetti** → **Aggiungi effetto**, oppure il **+** del gruppo **Effetti** nel pannello Scena; l'effetto scelto compare nella scheda **Selezione**; **Scena →
Effetti e nebbia…**.

Gli effetti volumetrici si calcolano **in anticipo**, uno alla volta, con l'avanzamento nella
barra di stato; poi il render li usa su CPU e GPU. Il calcolo parte da solo 2,5 secondi dopo
l'ultima modifica (mai mentre trascini un oggetto) e non cambia scheda: resti nella viewport. Il
comando **Render** calcola subito ciò che manca. Se annulli il calcolo (la **X** nella barra di
stato) resta annullato finché non cambi la scena o non avvii il render. Ogni effetto ha un riquadro (o un emettitore)
che selezioni nella viewport e sposti o ridimensioni con il gizmo. **Attivo nel render** lo spegne
senza eliminarlo.

| Effetto | Cos'è |
|---|---|
| Foschia | Aria con nebbia in tutta la scena o in una fascia, dove i fasci di luce diventano visibili. Vedi [Foschia d'ambiente](fx.foschia) |
| Fumo | Nubi volumetriche in un riquadro: *Nube*, *Pennacchio*, *Filo d'incenso*, *Banco*, oppure una simulazione fisica con calore e galleggiamento; preset come incenso, vapore da una tazza, pennacchio da un tubo, fumo industriale |
| Ghiaccio secco | Nebbia fredda e pesante che scende da un emettitore, si posa, si allarga e scorre attorno agli oggetti (simulazione di fluido) |
| Polvere in sospensione | Granelli che brillano solo dentro i fasci di spot e sole, come la polvere in un raggio che entra da una finestra. Con **Scaglie** ogni scaglia, mentre fluttua, si inclina anche piano per conto suo, così con camera e luci ferme i riflessi si accendono e si spengono |
| Lucciole | Uno sciame che vola e si accende a intervalli; le lucciole accese sono luci vere |
| Adesivo (decal) | Un'immagine (logo, etichetta, PNG con trasparenza) proiettata sull'oggetto selezionato dove guarda la camera; avvolge le superfici curve e si muove con l'oggetto |

## Fumo e ghiaccio secco: i controlli principali

| Controllo | Cosa fa |
|---|---|
| Densità | Quanto è opaco il fumo pieno; il pannello mostra la distanza oltre la quale diventa opaco |
| Colore / Scattering | Vapore bianco, fumo grigio, fuliggine scura |
| Scala dettagli, vortici, turbolenza | Grandezza e forza di sbuffi e riccioli |
| Risalita, dissolvenza | Quanto in fretta sale e si dissolve |
| Qualità | Dimensione della griglia: più celle = più dettaglio, più memoria e tempo |
| Seed / Nuovo | Un'altra forma con gli stessi parametri |

Il fumo simulato e il ghiaccio secco seguono la timeline: la loro riga nella
[Timeline](anim.timeline) decide quando l'emissione parte e quando si ferma.

## Consigli

- Gli effetti si vedono solo dove c'è **luce** da diffondere: uno spot o il sole che attraversa il
  fumo o la foschia lo fa risaltare; con il solo ambiente sembra piatto.
- Gli effetti volumetrici vogliono più [campioni](render.campioni) di una scena pulita.
