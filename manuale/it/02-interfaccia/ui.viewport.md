---
id: ui.viewport
titolo: Viewport, navigazione e gizmo
parole: viewport, navigare, orbitare, ruotare la vista, zoom, gizmo, sposta, ruota, scala, seleziona, snap, ortografica, isometrica, navigatore
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

Per i valori esatti usa la scheda **Oggetto**: vedi [Spostare, ruotare e
ridimensionare](scene.oggetto).

## Nascondere

**H** nasconde la selezione, **Maiusc+H** nasconde tutto il resto, **Alt+H** mostra di nuovo
tutto. Gli oggetti nascosti non vanno nel render.
