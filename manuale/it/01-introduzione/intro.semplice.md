---
id: intro.semplice
titolo: Modalità Semplice (foto prodotto in 5 passi)
parole: semplice, facile, guidata, procedura, passi, foto prodotto, aspetto, formato
ui: mode.simple, scene.prepare, scene.fit, scene.undo_preset, scene.drop_to_base, scene.frame_product
correlate: intro.primo-render, light.colore, render.campioni
---
# Modalità Semplice (foto prodotto in 5 passi)

**Dove si trova:** interruttore **Semplice** in alto a sinistra. È la modalità predefinita.

**A cosa serve:** una sola colonna con pulsanti grandi che porta dal modello alla foto. Ogni
scelta scrive gli stessi valori che trovi in modalità Avanzata, quindi puoi passare dall'una
all'altra in qualsiasi momento.

## 1. Prodotto

- **Importa modello…** apre il file 3D.
- **Appoggia sulla base** mette il prodotto esattamente sul pavimento o sul fondale.
- **Inquadra il prodotto** lo inquadra senza pavimento e fondale.
- **Adatta la scena al prodotto** rifà fondale e luci sulle misure attuali del prodotto.

## 2. Aspetto

| Aspetto | Risultato |
|---|---|
| Sfondo chiaro | Fondale bianco e luce morbida: foto da catalogo o e-commerce |
| Sfondo scuro | Fondale scuro lucido e tre punti luce: il prodotto stacca dal nero |
| Studio scenografico | Luce calda di taglio e controluce fredda: foto d'effetto |
| Studio fotografico | Fondale in tela inclinato e quattro luci da studio, come in sala posa |

**Colore delle luci** cambia tutte le luci insieme: **Calda** (3200 K, gialla come una
lampadina), **Neutra** (5500 K, bianco da studio), **Fredda** (7500 K, azzurrina). Vedi
[Colore della luce](light.colore).

**Prepara scena prodotto** applica l'aspetto: appoggia il prodotto, aggiunge il fondale se manca,
applica aspetto e qualità e inquadra. **Annulla preset** riporta luci, ambiente, qualità, fondale
e posizione del prodotto a com'erano prima dell'ultimo preset. Se cambi luci o ambiente a mano,
l'aspetto risulta *Personalizzato*.

## 3. Materiali

Clicca un pezzo nella vista, poi **Apri la libreria materiali** e clicca un materiale: viene
applicato ai pezzi selezionati.

## 4. Formato e qualità

| Formato | Dimensioni |
|---|---|
| Orizzontale | 16:9, 1920 × 1080 |
| Quadrato | 1:1, 1080 × 1080 (Instagram) |
| Verticale | 4:5, 1080 × 1350 (Instagram) |
| Storie | 9:16, 1080 × 1920 |

| Qualità | Campioni | Rimbalzi | Quando |
|---|---|---|---|
| Bozza | 128 | 8 | Per provare inquadratura e materiali |
| Bilanciata | 256 | 12 | Buona per la maggior parte delle foto |
| Alta | 1024 | 16 | Foto finale: vetri e riflessi più puliti |

**Dispositivo di calcolo:** **Scheda video (GPU)**, molto più veloce con una scheda recente, o
**Processore (CPU)**, che funziona su ogni PC. Se non c'è una GPU utilizzabile il pulsante GPU è
disattivato.

## 5. Render

**Avvia render** (poi **Pausa** / **Riprendi**) e **Salva immagine…** (PNG, JPG, TIFF 16 bit o
EXR). In modalità Semplice il denoiser è sempre acceso con i valori migliori.
