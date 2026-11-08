---
id: export.viste
titolo: Viste e turntable
parole: viste, viste multiple, turntable, giro, video rotazione, 360, frontale, laterale, tre quarti, dall'alto, in serie
ui: render.sequences
correlate: export.immagine, anim.video, cam.obiettivo
---
# Viste e turntable

**Dove si trova:** **File → Viste e turntable…** (anche nel menu Rendering).

## Viste multiple

Fa il render del prodotto da più punti di vista automatici, un'immagine ciascuno: **Frontale**,
**Tre quarti destra**, **Destra**, **Posteriore**, **Sinistra**, **Tre quarti sinistra**,
**Dall'alto**.

| Controllo | Cosa fa |
|---|---|
| Viste | Spunta quelle che vuoi (almeno una) |
| Inquadra il prodotto (sempre tutto dentro) | Ogni vista inquadra tutto il prodotto |
| Campioni per immagine | Qualità di ogni immagine |
| Passaggi di render per ogni immagine | Anche albedo, normali, profondità, maschera del prodotto e ID dei pezzi per ogni vista |
| Nome dei file, Cartella… | Dove e con che nome si salvano le immagini |
| Avvia / Interrompi | Interrompendo, le immagini già fatte restano; la scena torna com'era |

## Turntable (video)

Il prodotto gira su se stesso mentre camera, luci e fondale restano fermi.

| Controllo | Cosa fa |
|---|---|
| Rotazione tra due immagini | Più è piccola, più il movimento è fluido (e più immagini da calcolare) |
| Fotogrammi al secondo, Durata di un giro | Cambiando la durata si ricalcola la rotazione tra due immagini |
| Giri nel video | Il giro si ripete senza rifare i render: il video gira in loop senza scatti |
| Senso di rotazione | Antiorario o orario |
| Campioni per immagine | Sotto i 256 campioni il video può sfarfallare |
| Camera | La vista attuale, oppure **Crea una camera da questa vista** per tenere l'inquadratura nel progetto |
| Crea il video, Impostazioni video… | MP4 e altri formati: vedi [Salvare l'animazione](anim.video) |
