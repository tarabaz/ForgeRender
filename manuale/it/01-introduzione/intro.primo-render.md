---
id: intro.primo-render
titolo: Il primo render
parole: per iniziare, primi passi, guida rapida, principiante, tutorial, come si comincia
ui: render.start
correlate: intro.semplice, scene.importa, export.immagine
---
# Il primo render

Cinque passi da un file 3D a una foto finita. Valgono allo stesso modo in modalità Semplice e
Avanzata.

1. **Importa il modello:** **File → Importa modello…** (Ctrl+I). Si aprono glTF/GLB, OBJ, STL,
   STEP e file di Blender; vedi [Importare un modello](scene.importa).
2. **Prepara la scena:** **Scena → Prepara scena prodotto** appoggia il prodotto sulla base,
   aggiunge un fondale se manca, applica aspetto e qualità scelti e inquadra il prodotto. Si
   annulla con **Annulla preset**.
3. **Assegna i materiali:** clicca un pezzo nella viewport, apri la **libreria materiali** e
   clicca un materiale. Vedi [Il pannello Materiali](mat.pannello).
4. **Avvia il render:** **Rendering → Avvia render** (F5). L'immagine parte sgranata e si
   pulisce passata dopo passata; alla fine il denoiser toglie la grana che resta.
5. **Salva l'immagine:** **File → Esporta immagine…** (Ctrl+E): PNG, JPG, TIFF 16 bit o EXR. Vedi
   [Salvare l'immagine](export.immagine).

## Viewport e vista Render

L'area centrale ha due viste:

- **Viewport:** la vista di lavoro in tempo reale. Qui sposti oggetti, luci e camera.
- **Render:** l'immagine del path tracer, cioè la foto vera.

Con **Riavvio automatico** acceso, ogni modifica a materiali, luci o camera fa ripartire il
render da solo, ma solo nella vista Render: nella Viewport puoi lavorare senza che parta.
**Stop** tiene l'immagine attuale, **Riavvia** riparte da zero campioni.

## Quanto ci vuole?

Dipende dal numero di **campioni per pixel** (vedi [Campioni](render.campioni)): 128–256
campioni con il denoiser bastano per la maggior parte delle foto; vetri, liquidi e scene scure
ne vogliono di più. La barra di stato in basso mostra campioni, tempo e tempo rimanente.
