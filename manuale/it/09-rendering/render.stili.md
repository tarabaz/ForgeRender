---
id: render.stili
titolo: Stili del render, contorni e sfondo
parole: stile, fumetto, toon, cel shading, illustrazione tecnica, gooch, contorni, inchiostro, linee, bianco e nero, sfondo, sfondo a tinta unita, sfondo bianco
ui: panel.render.style
correlate: render.pannello, env.ambiente, export.immagine
---
# Stili del render, contorni e sfondo

**Dove si trova:** in cima alla scheda Render. Passando sopra ogni stile si vede un piccolo
esempio.

![Fotorealistico](img/comune/render_style_photoreal.png) ![Fumetto](img/comune/render_style_toon.png) ![Illustrazione tecnica](img/comune/render_style_technical.png) ![Fotorealistico con contorni a inchiostro](img/comune/render_outlines_on.png)

| Stile | Cosa fa |
|---|---|
| Fotorealistico | Path tracing fisico: riflessi, rifrazioni, ombre morbide e luce di rimbalzo |
| Fumetto | Colori pieni dei materiali in tre fasce nette (luce, mezza ombra, ombra) dalle luci della scena, con le loro ombre nette e un riflesso netto sui materiali lucidi |
| Illustrazione tecnica | Ombreggiatura morbida da caldo in luce a freddo in ombra, senza ombre nere, così le forme si leggono bene |

**Bianco e nero:** il fumetto in grigi, l'illustrazione tecnica come un disegno a matita.

Gli stili agiscono sulla prima superficie vista dalla camera, quindi vetri e specchi riflettono e
rifrangono come sempre. Funzionano su CPU e GPU, anche con il render Spettrale.

## Sfondo a tinta unita

Dove la camera non vede niente, un colore (**Bianco**, **Grigio chiaro**, **Nero** o qualunque)
al posto dell'ambiente, che continua a illuminare la scena. Va bene per le foto da catalogo sul
bianco pieno.

## Contorni a inchiostro

Linee scure su sagome, confini tra oggetti e salti di profondità, disegnate sopra l'immagine
finale (anche dopo il denoiser), con qualsiasi stile.

| Controllo | Cosa fa |
|---|---|
| Spessore | Larghezza della linea in pixel (predefinito 1,5) |
| Colore | Colore della linea |
| Anche gli spigoli vivi | Linee anche dove due facce dello stesso oggetto formano un angolo netto; **Angolo degli spigoli** dice da quale angolo (predefinito 35°, più basso = più linee) |
| Salto di profondità | Sensibilità ai salti di profondità dentro lo stesso oggetto (predefinito 3 %, più basso = più linee) |
| Pulizia | Toglie i pezzi di linea più corti di così (pixel) |
| Niente linea sul bordo del pavimento | Lascia senza linea il bordo di pavimento e fondale contro lo sfondo |
