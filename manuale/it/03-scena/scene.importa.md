---
id: scene.importa
titolo: Importare un modello
parole: importa, apri modello, gltf, glb, obj, stl, step, stp, blender, blend, solidworks, cad, unità, tassellazione
ui: file.import
correlate: scene.oggetto, scene.progetti, intro.primo-render
---
# Importare un modello

**Dove si trova:** **File → Importa modello…** (Ctrl+I), oppure **Importa modello…** in modalità
Semplice.

| Formato | Note |
|---|---|
| glTF / GLB | Consigliato (anche come esportazione da SolidWorks): arrivano materiali, texture, luci e camere |
| OBJ (con .mtl) | Si leggono i materiali del file .mtl |
| STL | Solo geometria; va scelta l'unità |
| Blender `.blend` | Tramite una copia di Blender installata |
| STEP (.step, .stp) | Convertito una volta in triangoli, con una finestra sua (vedi sotto) |
| SolidWorks SLDPRT / SLDASM | Sperimentale |

## Opzioni di importazione

| Opzione | Significato |
|---|---|
| Unità (STL / OBJ) | Millimetri, centimetri, metri o pollici: questi file non dicono in che unità sono |
| Asse verticale | Automatico: STL con Z in alto; OBJ, STEP e SolidWorks con Y in alto |
| Qualità triangolazione (STEP) | Bassa, Media, Alta |
| Posizione: Appoggia sul pavimento dopo l'importazione | Il modello arriva esattamente sul pavimento |
| Angolo di piega normali | Oltre questo angolo tra due facce lo spigolo resta vivo, sotto la superficie viene ammorbidita |

Ogni modello tiene la struttura dei suoi pezzi: l'albero dei componenti compare nel pannello
Scena a sinistra, e ogni pezzo può avere il suo materiale.

## File STEP

Un singolo file STEP apre una finestra sua: il file a sinistra, le impostazioni di
tassellazione al centro e a destra il modello **come verrà importato**: gli stessi pezzi, colori,
asse verticale e appoggio sul pavimento, disegnati esattamente come nella viewport. Scegli
**Ombreggiata** o **Fil di ferro** (gli spigoli dei triangoli, come la vista Mesh della viewport);
trascina per girare, Maiusc o tasto destro per spostare, rotella per avvicinare, doppio clic o
**Inquadra** per inquadrarlo di nuovo. Sotto ci sono triangoli, vertici, dimensioni e pezzi.
Cambiando l'asse verticale o **Appoggia sul pavimento dopo l'importazione** l'anteprima si
aggiorna subito, senza convertire di nuovo.

| Tassellazione | Deviazione lineare / angolare | Quando |
|---|---|---|
| Bozza | 1 % e 28,6° | Per provare in fretta |
| Bilanciata | 0,4 % e 17,2° | Il valore di sempre: curve pulite a un costo ragionevole |
| Alta qualità | 0,15 % e 8,6° | Primi piani e superfici lucide |
| Personalizzata | i due valori sotto | **Deviazione lineare**: quanto la mesh può scostarsi dalla superficie CAD. **Deviazione angolare**: angolo massimo tra due triangoli vicini su una curva |

Puoi premere **Converti** quante volte vuoi; il modello entra nella scena solo con **Importa**, ed
è proprio il modello dell'anteprima (non si converte di nuovo). Il risultato si conserva: riaprendo
il progetto lo STEP non si converte di nuovo.

## Errori comuni

- **Modello enorme o minuscolo:** unità sbagliata per STL/OBJ. Reimporta con l'unità giusta, o usa
  i pulsanti di scala rapida della scheda [Oggetto](scene.oggetto) (×25.4 converte da pollici a
  millimetri).
- **Curve a faccette su uno STEP:** convertilo di nuovo con una tassellazione di qualità più alta.
