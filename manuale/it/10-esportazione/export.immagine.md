---
id: export.immagine
titolo: Salvare l'immagine e i passaggi di render
parole: esporta, salva immagine, png, jpg, jpeg, tiff, exr, 16 bit, passaggi, albedo, normali, profondità, maschera, alfa, scontorno, id dei pezzi, compositing
ui: file.export, file.export_passes
correlate: render.esposizione, export.viste, render.stili
---
# Salvare l'immagine e i passaggi di render

## Esporta immagine

**Dove si trova:** **File → Esporta immagine…** (Ctrl+E); **Salva immagine…** in modalità
Semplice.

Salva il render attuale (quello che vedi nella vista Render, con denoiser, esposizione e tone
mapping). Il formato segue l'estensione scelta nella finestra:

| Formato | Uso |
|---|---|
| PNG (senza perdita, 8 bit) | Predefinito: web, documenti, nessuna perdita di qualità |
| JPG (leggero, per web e social) | File piccoli (qualità 92) |
| TIFF 16 bit (per il fotoritocco) | Più sfumature per colore: niente scalini quando ritocchi molto l'immagine |
| EXR (valori lineari, per il compositing) | La luce come calcolata, con valori oltre il bianco, senza tone mapping |

I file vanno di base nella cartella `Rendering` accanto al programma (o in
`Documenti\ForgeRender` se la cartella del programma non è scrivibile).

## Passaggi di render

**Dove si trova:** **File → Esporta passaggi di render…**.

Immagini tecniche dell'ultimo render, per la post-produzione. Non sono render in più: descrivono
la scena pixel per pixel.

| Passaggio | Cosa contiene |
|---|---|
| Albedo | Il colore dei materiali senza luce |
| Normali | L'orientamento delle superfici |
| Profondità | Distanza dalla camera: PNG 16 bit (bianco = vicino) ed EXR in metri |
| Maschera del prodotto | Bianco dove c'è il prodotto: per scontornarlo dallo sfondo |
| ID dei pezzi | Un colore per pezzo, per selezionare i pezzi in un programma di fotoritocco |

## Consigli

- Per un prodotto sul **bianco pieno**, usa uno sfondo a tinta unita bianco (vedi
  [Stili del render](render.stili)) oppure scontornalo con la maschera del prodotto.
- Per la stampa e i ritocchi pesanti meglio il **TIFF 16 bit**.
