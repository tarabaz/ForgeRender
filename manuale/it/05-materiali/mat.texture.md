---
id: mat.texture
titolo: Texture, posizionamento e rilievo
parole: texture, mappa, mappe, pbr, uv, proiezione box, normal map, bump, height, displacement, occlusione, ao, opacità, ritaglio, scala, adesivo
ui: panel.materials.textures
correlate: mat.pannello, mat.roughness, fx.effetti
---
# Texture, posizionamento e rilievo

**Dove si trova:** modalità Avanzata → scheda **Selezione** (materiale del pezzo scelto) o **Scena → Materiali** → sezione **Texture**; con le texture,
anche **Scala e posizionamento texture** e **Rilievo (normal, bump, displacement)**.

## Slot delle texture

| Slot | Cosa fa |
|---|---|
| Colore base | L'immagine della superficie; viene moltiplicata per il colore base (messo a bianco quando carichi una mappa) |
| Roughness | Mappa di rugosità (canale rosso); il cursore la moltiplica. **È una mappa di lucidità (gloss)** la inverte |
| Metallicità | Mappa di metallicità; il cursore la moltiplica |
| Normal map | Rilievo fine senza cambiare la forma. **Formato DirectX** per le mappe con il verde invertito (Unreal, Quixel, *_NormalDX) |
| Rilievo (height) | Mappa di altezza: bump, o vero spostamento della geometria (displacement) |
| Occlusione (AO) | Scurisce le cavità; **Intensità AO** 0 – 1. Il path tracer calcola già l'occlusione reale: valori bassi sono più fedeli |
| Opacità (ritaglio) | Le parti nere diventano buchi (foglie, lamiera forata, etichette) |

## Posizionamento

Per i modelli CAD, che di solito non hanno coordinate UV, la proiezione **Box** stende l'immagine
da tre lati in misure reali: scegli quanto è grande una ripetizione della texture (in mm, cm o m),
la rotazione e lo spostamento. L'ancoraggio può essere il **Pezzo**, l'**Assieme** o il
**Mondo**: con Pezzo la texture si muove insieme al pezzo.

## Rilievo

- **Normal map** e **bump** cambiano solo la luce, non il contorno: leggeri e buoni per il
  dettaglio fine.
- Il **displacement** sposta davvero la superficie (il contorno cambia). Va bene per pietre,
  mattoni e cortecce; sui metalli lisci crea bozzi. In **Impostazioni → Libreria e import** scegli
  se i materiali nuovi lo ricevono all'import (spento di base).

## Etichette e loghi

Per mettere un'immagine (logo, etichetta) solo su un pezzo, usa un **adesivo** (decal) nella
**Scena → Effetti**: vedi [Effetti](fx.effetti).
