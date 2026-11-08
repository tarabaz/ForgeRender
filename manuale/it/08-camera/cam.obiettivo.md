---
id: cam.obiettivo
titolo: Camera, obiettivo e focale
parole: camera, fotocamera, obiettivo, focale, mm, zoom, campo visivo, grandangolo, teleobiettivo, prospettiva, ortografica, camera di rendering, decentramento
ui: panel.camera
correlate: cam.dof, cam.tiltshift, ui.viewport, export.viste
---
# Camera, obiettivo e focale

**Dove si trova:** modalità Avanzata → scheda **Camera**; **Vista → Camera e obiettivo…**.

## Camera della viewport e camere di rendering

L'immagine si prende dalla **vista della viewport** o da una **camera di rendering**. **+ Nuova
camera dalla vista** salva il punto di vista attuale come camera; le camere sono elencate nel
pannello Scena, sopra le luci. Una camera selezionata mostra una linea verso il suo bersaglio (un
rombo): il gizmo sposta insieme camera e bersaglio, **Ruota** fa girare la camera attorno al
bersaglio, e dopo un clic sul rombo si sposta solo il bersaglio. **Punta su oggetto** fa
inquadrare un oggetto alla camera, lo mette a fuoco e lo segue quando si sposta.

Viste rapide: **Frontale**, **Destra**, **Alto**, **Isometrica**; **Inquadra tutto**, **Inquadra
selezione**.

## Obiettivo (focale)

**Obiettivo** imposta la focale in millimetri, equivalente full frame (sensore 36 × 24 mm), da 4
a 2000 mm; i pulsanti danno 14, 24, 35, 50, 85, 135 e 200 mm. Accanto vedi l'angolo di campo
verticale.

![24 mm: grandangolo, prospettiva forte](img/comune/cam_focal_024.png) ![50 mm: prospettiva naturale](img/comune/cam_focal_050.png) ![135 mm: teleobiettivo, prospettiva schiacciata](img/comune/cam_focal_135.png)

*La camera arretra man mano che la focale cresce, così gli oggetti restano grandi più o meno
uguali: cambia la prospettiva.*

| Focale | Effetto | Uso |
|---|---|---|
| 14 – 24 mm | Grandangolo: oggetti vicini grandi, lontani piccoli, linee che convergono | Interni, inquadrature d'effetto |
| 35 – 50 mm | Simile all'occhio umano | Uso generale |
| 85 – 135 mm | Teleobiettivo: prospettiva piatta, sfondo vicino | Foto di prodotto senza deformazioni |
| 200 mm e oltre | Molto piatta, quasi ortografica | Foto di prodotto dall'aspetto tecnico |

## Proiezione

- **Prospettica:** come una fotocamera, gli oggetti lontani sono più piccoli.
- **Ortografica:** niente prospettiva, come in un disegno tecnico; **Altezza inquadratura**
  automatica (segue lo zoom) o in metri.

## Decentramento

**Decentramento** (orizzontale e verticale, da −100 % a 100 %) sposta l'immagine senza girare la
camera, come gli obiettivi decentrabili dei fotografi di architettura: le linee verticali restano
verticali quando inquadri dal basso un oggetto alto.
