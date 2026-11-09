---
id: ui.impostazioni
titolo: Impostazioni
parole: impostazioni, preferenze, opzioni, lingua, cartella libreria, percorsi, nuovi progetti, barra di stato, sfondo della viewport, colore di sfondo, antracite, griglia, griglia metrica, misura della cella, denoiser, anteprime
ui: edit.preferences, edit.reset_preferences
correlate: ui.finestra, mat.pannello, render.pannello
---
# Impostazioni

**Dove si trova:** **Modifica → Impostazioni…**. **Modifica → Ripristina impostazioni
predefinite** riporta ogni impostazione al valore iniziale.

Le impostazioni sono del programma, non del progetto: restano uguali qualunque cosa apri.

| Sezione | Cosa contiene |
|---|---|
| Lingua | Lingua di tutta l'interfaccia (italiano, inglese, o un file `lang/Lan_*.json` aggiunto accanto al programma). Cambia subito. Il manuale la segue. |
| Viewport | **Sfondo della viewport**: Antracite (predefinito, RGB 40, 44, 52, #282C34), Grigio Blender-like (#30343B), Scuro tecnico (#222831), Grigio chiaro (#353A42) o **Personalizzato**: lo scegli con un clic, e il colore personalizzato (dal selettore colore) resta anche mentre provi gli altri. **Griglia**: mostrala o nascondila, unità (mm, cm, m), misura della cella (1, 5, 10, 20, 50, 100 mm, 50 cm, 1 m o un valore a piacere), una linea principale ogni 5, 10, 20 o N celle, intensità. Sfondo e griglia cambiano subito la viewport e l'anteprima STEP; il render finale tiene il suo sfondo (ambiente e fondale) e non mostra mai la griglia. |
| Libreria e import | Risoluzione dei download, sito proposto (ambientCG o Poly Haven), cosa fare con la height map all'import (**Spento**, consigliato; un valore proposto del 3 % del lato della texture; oppure il valore del file MaterialX della fonte), qualità, campioni e dimensione dell'oggetto delle anteprime dei materiali, **Riduci il rumore** (il denoiser del render finale sulle anteprime), **Svuota cache anteprime** |
| Percorsi | Cartella della libreria (HDRI, Materiali, Texture). La variabile d'ambiente `FORGE_LIBRARY_DIR` ha la precedenza. |
| Rendering | Valori dei **nuovi** progetti: dispositivo, campioni per pixel (0 = senza limite), thread CPU (0 = automatico), riavvio automatico. I progetti salvati tengono i loro. |
| Barra di stato | Quali contatori mostrare in basso (fino a 9). |
| Informazioni | Versione, **Mostra all'avvio** per la schermata di benvenuto, **Chiedi la modalità all'avvio** (Semplice o Avanzata). |

## Dove vanno i file

Quello che salvi o esporti va di base nella cartella `Rendering` accanto al programma, oppure in
`Documenti\ForgeRender` se la cartella del programma non è scrivibile.
