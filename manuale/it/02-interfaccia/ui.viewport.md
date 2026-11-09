---
id: ui.viewport
titolo: Viewport, navigazione e gizmo
parole: viewport, navigare, orbitare, ruotare la vista, zoom, gizmo, sposta, ruota, scala, seleziona, snap, ortografica, isometrica, navigatore, griglia, griglia metrica, griglia del pavimento, misura
ui: view.viewport, view.ortho, view.isometric, view.frame_selection, view.frame_all, tool.select, tool.move, tool.rotate, tool.scale
correlate: scene.oggetto, keys.elenco, cam.obiettivo
---
# Viewport, navigazione e gizmo

**Dove si trova:** area centrale, vista **Viewport**.

La viewport è la vista di lavoro in tempo reale: mostra le forme e i colori dei materiali, non
la luce finale. La foto è nella vista **Render**.

## Muoversi

| Azione | Effetto |
|---|---|
| Trascina con il tasto sinistro | Gira attorno alla scena |
| Maiusc + trascina, o tasto destro / centrale | Sposta la vista di lato |
| Rotella | Avvicina o allontana |
| Home | Inquadra tutto |
| F | Inquadra la selezione |
| 5 | Prospettica / ortografica |

Il **navigatore degli assi** nell'angolo (come in Blender) mostra X, Y e Z: clicca un asse per
guardare lungo quell'asse. Accanto ci sono le icone della vista ortografica e **isometrica**
(ortografica, 45° di lato). **Vista → Viste standard** ha frontale, retro, destra, sinistra,
alto e basso.

## Griglia metrica

Sul pavimento (altezza 0) c'è una griglia in misure reali: la scena è in metri, quindi una cella
da 1 cm è 1 cm a qualsiasi zoom. Ogni tot celle una linea principale più marcata; la linea rossa è
l'asse X, quella blu l'asse Z. Quando le celle sullo schermo diventano troppo piccole le linee
minori sfumano e restano le principali, così la griglia si legge da vicino e da lontano. Sta dietro
agli oggetti e non intralcia selezione e gizmo. Unità, cella, linee principali e intensità sono in
**Modifica → Impostazioni… → Viewport**; la stessa griglia c'è nell'anteprima dell'importazione
STEP. Nel render non compare mai.

## Selezionare e trasformare

Scegli lo strumento nella barra o con i tasti:

| Tasto | Strumento |
|---|---|
| Q | Seleziona |
| W | Sposta |
| E | Ruota |
| R | Scala |

- **Clic** su un pezzo per selezionarlo, **Ctrl+clic** per aggiungerlo o toglierlo, **Esc** per
  deselezionare.
- Trascina frecce, anelli o quadrati del **gizmo**. Tieni premuto **Ctrl** per lo snap a passi
  fissi; i passi (spostamento, rotazione, scala) e gli assi **Globale / Locale** sono nella riga
  degli strumenti sopra la viewport.
- **Alt + trascina** sposta la selezione invece della camera.
- Anche luci, maniglie del cono degli spot, riquadri degli effetti ed emettitori di nebbia hanno
  il loro gizmo. Spot e luci direzionali si puntano trascinando il **rombo** del loro bersaglio.
- **Annulla / Ripeti:** Ctrl+Z, Ctrl+Y.

Per i valori esatti usa la scheda **Selezione**: vedi [Spostare, ruotare e
ridimensionare](scene.oggetto).

## Nascondere

**H** nasconde la selezione, **Maiusc+H** nasconde tutto il resto, **Alt+H** mostra di nuovo
tutto. Gli oggetti nascosti non vanno nel render.
