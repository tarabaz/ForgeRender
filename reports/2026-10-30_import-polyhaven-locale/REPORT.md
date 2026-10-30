# Import della libreria Poly Haven scaricata in locale (fase 4)

- **Data**: download finito il 29/10/2026 alle 23:13; spostamento e riparazioni nella notte
  tra il 29 e il 30/10/2026.
- **Macchina**: PC di Valerio (Windows 11), disco `Y:`.
- **Download**: `Y:\ForgeRender\Assets\PolyHaven` (script `tools/material-library-polyhaven`).
- **Libreria**: `Y:\ForgeRender\Libreria`.
- **Comando**:
  `forgerender-cli libreria importa-polyhaven Y:\ForgeRender\Assets\PolyHaven --libreria Y:\ForgeRender\Libreria --sposta`
  (prima con `--solo-elenco`: 865 da importare, 3 già presenti, 1 incompleto, 5,6 s).

## Download

| Voce | Valore |
|---|---|
| Cartelle | 869 (tutti i materiali del catalogo Poly Haven) |
| Complete (`index.csv`) | 868 |
| Errori (`errors.txt`) | 1: `medieval_blocks_05: Remote end closed connection without response` |

## Spostamento

| Voce | Valore |
|---|---|
| Importati (spostati) | **865**, di cui 3 con avvisi (vedi sotto) |
| Già in libreria, saltati | 3 (aerial_asphalt_01, aerial_beach_01, aerial_beach_02: prova in copia `--max 3`) |
| Errori | **0** |
| Incompleti, non toccati | 1 (medieval_blocks_05) |
| Tempo | 197,3 s (rename sullo stesso disco) |
| Materiali Poly Haven in libreria | **868**, 15,28 GB |
| Scala reale | tutti dal catalogo (nessuna nota "dimensioni reali non disponibili") |

Controllo di tutta la libreria dopo le riparazioni (ogni `materiale.json` letto, ogni texture
citata esiste): 0 problemi. Mappe collegate:

| Slot | Materiali |
|---|---|
| colore, rugosità, normal, AO | 868 |
| height | 865 (senza: Fabric Pattern 05, Fabric Pattern 07, Fine Grained Wood: Poly Haven non dà la `disp`) |
| metallicità | 40 |
| opacità | 13 |

## Materiali importati male e riparati

Trovati controllando i residui nei download:

| Materiale | Problema | Causa |
|---|---|---|
| Rough Linen | rugosità = `rough_linen_anisotropy_rotation` | "anisotropy" non era una parola nota, quindi decideva "rough" del nome del materiale (e in ordine alfabetico veniva prima della `rough` vera) |
| Book Pattern | nessun colore | Poly Haven non dà `diff`, solo varianti `col1`/`col2` |
| Leather Red 02, Leather Red 03 | nessun colore | solo varianti `coll1`/`coll2` |

- **Codice** (`forge_library::detect`): "anisotropy", "translucent", "ior" ignorate;
  `col1..3` e `coll1..3` riconosciute come colore (vince la variante 1, come `col_01` in
  Fabric Pattern 05). Test `polyhaven_anisotropy_and_color_variants` con i nomi reali.
- **Libreria**: spostati dai download `book_pattern_col1`, `leather_red_02_coll1`,
  `leather_red_03_coll1`, `rough_linen_rough`; riportato `rough_linen_anisotropy_rotation` nei
  download; aggiornati i 4 `materiale.json` (slot, colore base 1,0 come gli altri materiali con
  texture, tolta la nota "nessuna mappa di colore"). Nessun file cancellato. Non c'erano
  anteprime da rigenerare.
- **Non verificato**: l'aspetto di questi 4 materiali nell'app (da guardare nella finestra
  Libreria).

## Rimasto nei download (non cancellato)

Totale nella cartella dei download dopo le riparazioni: **3884 file, 13,17 GB**, compresi i
file completi dei 3 materiali importati in copia e di medieval_blocks_05. Per i materiali
spostati (dati del comando, prima delle riparazioni):

| Tipo | File | MB |
|---|---|---|
| nor_dx .exr | 864 | 6379,6 |
| arm .exr | 864 | 4866,7 |
| rough_ao .exr | 80 | 532,5 |
| bump .exr | 91 | 468,1 |
| spec .exr | 104 | 407,8 |
| .blend | 865 | 192,6 |
| anisotropy_strength .exr | 30 | 144,5 |
| spec_ior .exr | 30 | 138,7 |
| anisotropy_rotation .exr | 29 (30 dopo la riparazione) | 136,3 |
| varianti di colore (col/coll), translucent, page, ref, normal_gl e normal_dx doppie (broken_wall), me_arm, rough_diff, rough | 23 (19 dopo la riparazione) | 128,5 |
| .gltf | 865 | 2,4 |

Valerio decide se cancellarli. Attenzione: rilanciare lo script di download con
`--skip-existing` riscaricherebbe i file spostati.

## Da decidere

- EXR o PNG: gli EXR di Poly Haven sono convertiti da PNG con compressione DWAA (con perdita).
- Riscaricare `medieval_blocks_05` e importarlo con lo stesso comando.
