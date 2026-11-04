# Libreria dei materiali di serie

Come si rifà `assets/materiali` (compresi nel programma) e il pacchetto aggiuntivo.

| File | A cosa serve |
|---|---|
| `tabella.json` | Per ogni materiale: zip di partenza, id, nome italiano, categoria, compreso (`true`) o aggiuntivo (`false`), dispersione (`abbe`) e campi da cambiare (`set`). |
| `sostituti.txt` | Materiali CC0 (Poly Haven, ambientCG) scaricati al posto di quelli BlenderKit, che non si possono ridistribuire. |
| `leggimi.js` | Scrive i `LEGGIMI.md` delle due librerie con la fonte di ogni materiale. |

## Passi

1. Sostituti CC0 (una volta; quelli già scaricati si saltano):

   ```text
   cargo run --release -p forge_library --example sostituti_cc0 -- tools\libreria-serie\sostituti.txt <sostituti>
   ```

2. Libreria compresa, pacchetto aggiuntivo e zip da conservare:

   ```text
   cargo run --release -p forge_render_core --example libreria_serie -- tools\libreria-serie\tabella.json "<zip originali>" <sostituti> assets\materiali assets\materiali-aggiuntivi <cartella zip>
   ```

   - `assets\materiali`: compresi, mappe ridotte a 1024 px (va su git);
   - `assets\materiali-aggiuntivi`: aggiuntivi a 2K (ignorata da git, letta dal programma compilato in locale);
   - `<cartella zip>\MATERIALI COMPRESI` e `MATERIALI AGGIUNTIVI`: gli stessi materiali come zip di "Esporta zip…" a 2K.

   Le cartelle che iniziano con `_` (come `assets\materiali\_vecchi`) non si toccano.

3. LEGGIMI:

   ```text
   node tools\libreria-serie\leggimi.js tools\libreria-serie\tabella.json tools\libreria-serie\sostituti.txt assets\materiali\LEGGIMI.md assets\materiali-aggiuntivi\LEGGIMI.md
   ```

## Regole

- Gli zip BlenderKit (categoria "… BlenderKit" o mappe chiamate `colore.jpg`, `rilievo.jpg`, il formato dell'import BlenderKit) si escludono; lo strumento elenca gli zip che non sono nella tabella.
- Ogni materiale trasparente deve avere la dispersione: senza, lo strumento si ferma.
- Mappe: EXR e PNG diventano JPEG di qualità 95 (il motore legge rugosità, normal, metallicità e occlusione a 8 bit), la height un PNG a 16 bit (il motore la legge a 16 bit), il colore lineare delle EXR diventa sRGB.
- Al massimo cinque materiali con mappe per categoria nella libreria compresa (lo controlla il test `builtin_library_is_complete`).
