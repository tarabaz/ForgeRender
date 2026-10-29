# Poly Haven Material Library — strumenti

Strumenti separati dal motore ForgeRender per creare una libreria locale di **tutti i materiali PBR di Poly Haven**. Non scarica HDRI né modelli 3D.

## Requisiti

- Windows 10/11 o Linux/macOS
- Python 3.10+ (nessun pacchetto pip richiesto)
- Connessione Internet

## Prova sicura: 3 materiali a 2K

Da PowerShell, nella cartella del repository:

```powershell
py -3 tools/material-library-polyhaven/download_polyhaven_materials.py --output "D:\ForgeRender\Assets\PolyHaven" --resolution 2k --limit 3
```

Controlla le cartelle e il peso totale. Se va tutto bene, avvia il download completo:

```powershell
py -3 tools/material-library-polyhaven/download_polyhaven_materials.py --output "D:\ForgeRender\Assets\PolyHaven" --resolution 2k --skip-existing
```

Puoi usare anche il file `download_2k.ps1` incluso. Modifica prima la variabile `$libraryPath` se vuoi un percorso diverso.

## Opzioni

- `--resolution 1k|2k|4k|8k|16k`: risoluzione desiderata (default 2K). Se una mappa non esiste a quella risoluzione, sceglie quella disponibile più vicina.
- `--limit 3`: prova con pochi materiali.
- `--ids metal_plate_02 rock_wall_02`: scarica solo gli ID indicati.
- `--skip-existing`: conserva i file già scaricati e riprende saltando quelli esistenti.

Esempio per un singolo materiale:

```powershell
py -3 tools/material-library-polyhaven/download_polyhaven_materials.py --output "D:\ForgeRender\Assets\PolyHaven" --resolution 4k --ids metal_plate_02
```

## Struttura creata

```text
PolyHaven/
  README.txt
  index.csv
  metal_plate_02/
    material.json
    maps/
      metal_plate_02_diff_4k.jpg
      metal_plate_02_metal_4k.exr
      metal_plate_02_nor_gl_4k.exr
      ...
```

Il contenuto esatto dipende dalle mappe e dai formati pubblicati da Poly Haven. Lo script seleziona un formato per ogni tipo di mappa: JPG preferito per il colore base, EXR preferito per le mappe dati quando disponibile, altrimenti PNG/JPG/TIFF. Non converte né ricomprime le texture; EXR resta EXR. Gli archivi ZIP eventualmente restituiti dall'API vengono estratti in sicurezza.

## Attenzioni

- Il download completo può occupare molto spazio e richiedere tempo. Parti dalla prova a 2K.
- Lo script non modifica il codice o gli asset del renderer e non committa texture nel repository: la libreria va salvata fuori dalla cartella Git.
- L'API viene consultata per ogni materiale; se Poly Haven cambia la struttura dell'API, potrebbero servire aggiornamenti.
- La cartella contiene un `material.json` per materiale e un `index.csv` globale. In caso di errori viene scritto anche `errors.txt`.
- Attribuzione consigliata: “Textures from Poly Haven” con link a https://polyhaven.com/. Licenza: CC0; dettagli su https://polyhaven.com/license.

## Risoluzione dei problemi

- Se `py` non è riconosciuto, installa Python 3.10+ e abilita il launcher Python.
- Per riprendere dopo un'interruzione, rilancia con `--skip-existing`.
- Se l'API non è raggiungibile, riprova più tardi. Lo script usa un User-Agent identificativo e ritenta i download in caso di errori di rete.
