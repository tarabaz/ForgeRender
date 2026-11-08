---
id: render.pannello
titolo: Pannello Render, dispositivo e risoluzione
parole: impostazioni di render, dispositivo, gpu, cpu, scheda video, processore, risoluzione, dimensioni, pixel, thread, rt core, avvia, stop, riavvia, riavvio automatico
ui: panel.render, render.stop, render.restart, render.auto_restart
correlate: render.campioni, render.denoiser, render.rimbalzi, render.esposizione
---
# Pannello Render, dispositivo e risoluzione

**Dove si trova:** modalità Avanzata → scheda **Render**; **Rendering → Impostazioni di render e
denoiser…**.

Dall'alto in basso la scheda contiene: [stile del render e contorni](render.stili), la camera di
rendering, il dispositivo, il colore (RGB o spettrale) e le [caustiche](render.caustiche), la
risoluzione, i [campioni per pixel](render.campioni), il [path tracing](render.rimbalzi),
la [visualizzazione](render.esposizione) e la [riduzione del rumore](render.denoiser).

## Dispositivo

| Dispositivo | Note |
|---|---|
| GPU (compute shader) | Predefinito. La scheda video tramite Vulkan o DirectX 12. Quando la scheda ha le unità di ray tracing (**RT core**) il pannello lo dice: i raggi si tracciano in hardware, stesso risultato, molte volte più veloce |
| CPU (tutti i core) | Funziona su ogni PC. **Thread CPU** (in **Path tracing → Avanzate**): 0 = automatico (thread logici − 1, per lasciare reattiva l'interfaccia) |

Se la GPU non si può usare, il pannello dice perché e il render va sulla CPU. CPU e GPU danno la
stessa immagine.

## Risoluzione

Pulsanti **HD** (1280 × 720), **Full HD** (1920 × 1080), **QHD** (2560 × 1440), **4K**
(3840 × 2160), **Quadrato** (1080 × 1080), **Anteprima** (640 × 360), oppure qualunque misura da
16 a 16384 pixel per lato. Larghezza e altezza doppie fanno quattro volte i pixel e circa quattro
volte il tempo, a parità di qualità.

## Avvio, pausa, stop

| Comando | Tasti | Cosa fa |
|---|---|---|
| Avvia render / Pausa / Riprendi | F5 | Avvia, mette in pausa o riprende il path tracing |
| Stop | | Ferma e tiene l'immagine attuale |
| Riavvia | | Riparte da zero campioni |
| Riavvio automatico | | Quando cambiano materiali, luci o camera il render riparte da solo (solo nella vista Render). La casella è sopra l'area centrale, a destra |

Alcune impostazioni non riavviano il render: il numero di campioni, l'esposizione, il tone
mapping e il denoiser valgono sull'immagine in calcolo. Le altre (risoluzione, rimbalzi, clamp,
seed, colore, caustiche, stile) richiedono di ripartire: il pannello lo dice.
