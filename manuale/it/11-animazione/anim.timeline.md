---
id: anim.timeline
titolo: Timeline e chiavi
parole: timeline, animazione, animare, chiave, chiavi, fotogramma chiave, interpolazione, ease, modificatore, tremolio, ripeti, avanti e indietro, cursore del tempo, area di lavoro, fps, durata
ui: panel.timeline
correlate: anim.video, fx.effetti, export.viste
---
# Timeline e chiavi

**Dove si trova:** in basso nella finestra, scheda **Timeline** accanto a **Console**. **Stacca dal
programma** la apre in una finestra a parte (anche su un altro schermo), **Riporta nel programma**
la rimette al suo posto.

## Elementi e proprietà

**+ Elemento** aggiunge quello che vuoi animare: gli oggetti selezionati, la camera, una luce, un
fumo, un ghiaccio secco, la foschia d'ambiente, la polvere in sospensione o l'ambiente. Ogni
elemento mostra le sue proprietà di base (per gli oggetti posizione, rotazione, scala e opacità;
per la camera posizione e bersaglio; per le luci posizione e intensità); **+** aggiunge le altre
una alla volta (scala per asse, visibilità, colore, bersaglio, apertura del cono, campo visivo,
diaframma, distanza di messa a fuoco, densità, rotazione dell'HDRI, luminosità). Posizione,
rotazione, bersaglio e colore sono una riga sola con i tre valori.

## Chiavi

- Il **rombo** di una riga mette o toglie una chiave al cursore del tempo, con il valore attuale.
- Quando una proprietà ha già delle chiavi, spostare l'oggetto con il gizmo o cambiare il valore
  nei pannelli **registra una chiave** al cursore del tempo.
- Clic su una chiave per selezionarla; **Ctrl** o **Maiusc** aggiungono o tolgono; trascina
  nell'area vuota per selezionare con un rettangolo; trascina una chiave per spostare tutte quelle
  selezionate (si agganciano ai fotogrammi).
- **Canc** le elimina, **Ctrl+C** / **Ctrl+V** le copiano al cursore del tempo, **F9** applica
  rallentamento in entrata e in uscita.
- Il **tasto destro** sceglie l'interpolazione verso la chiave dopo: tieni, lineare, rallenta in
  entrata, in uscita, in entrata e in uscita, curva morbida.

## Modificatori

**ƒ+** aggiunge un modificatore a una proprietà, applicati in ordine: tremolio (ampiezza,
frequenza, seme), oscillazione, rampa continua (per esempio 90°/s per un turntable), ripeti, avanti
e indietro, sposta, moltiplica, limiti e a scatti (stop motion). Su una riga con tre assi le
lettere X Y Z scelgono su quali agisce.

## Riproduzione

Pulsanti: all'inizio, chiave prima, riproduci / pausa, chiave dopo, alla fine, ripeti. **Durata**
(anche fino a quando si calcolano fumi e nebbie) e **FPS**; zoom e scorrimento del tempo. La barra
dell'**area di lavoro** sopra il righello (si trascinano gli estremi o il centro) decide la parte
che si riproduce e si salva.

Nella **Viewport** la riproduzione è in tempo reale; nella vista **Render** Play calcola ogni
fotogramma con pochi campioni (20 di base) e va avanti: più lento del tempo reale, ma vedi
l'animazione renderizzata.

## Effetti nel tempo

Fumo simulato e ghiaccio secco hanno una riga loro: trascina il suo inizio per scegliere quando
parte l'effetto e imposta quando si ferma l'emissione. Quello che è uscito continua a muoversi e a
dissolversi dopo lo stop.

Ogni modifica a chiavi, modificatori, area di lavoro, durata e tempi degli effetti si annulla con
Ctrl+Z.
