# Palette di colori comprese

`RAL_Classic.json`: i codici RAL Classic con nome italiano, nome inglese e un valore sRGB
**indicativo**, mostrati nella scheda Palette del selettore colore.

- I valori **non sono quelli ufficiali RAL**: i valori digitali ufficiali li vende RAL gGmbH come
  libreria digitale (RAL CLASSIC, file `.ase`), che si può importare nel selettore con «Importa…».
- I valori sono la media delle tabelle pubbliche di Wikipedia in inglese ("List of RAL colours") e
  in tedesco ("RAL-Farbe"); dove le due differiscono di più di 20 su 255 (colori fluorescenti e blu
  molto saturi, fuori dallo spazio sRGB) si tiene quella tedesca. I nomi italiani sono scritti a
  mano. Lo script è `tools/colori/ral_classic.js`.
- RAL è un marchio di RAL gGmbH; i codici servono solo a riconoscere il colore.
