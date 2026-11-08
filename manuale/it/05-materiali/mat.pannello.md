---
id: mat.pannello
titolo: Pannello Materiali e libreria
parole: materiale, materiali, libreria, assegna, applica, sostituisci, anteprima, nuovo materiale, duplica, materiale predefinito, emissione, selettore colore, ral, ambientcg, poly haven
ui: panel.materials, library.mine, library.online, library.import_blend, library.folder
correlate: mat.roughness, mat.metallic, mat.transmission, mat.texture
---
# Pannello Materiali e libreria

**Dove si trova:** modalità Avanzata → scheda **Selezione** (il materiale del pezzo scelto) o **Scena → Materiali**; menu **Libreria**; in modalità
Semplice **Apri la libreria materiali**.

## Assegnare i materiali

1. Seleziona uno o più pezzi (clic nella viewport o nel pannello Scena).
2. Clicca un materiale nella **Libreria materiali…** o nell'elenco dei materiali della scena,
   poi **Assegna alla selezione**.

| Comando | Cosa fa |
|---|---|
| Assegna alla selezione | I pezzi selezionati prendono il materiale |
| Rimuovi assegnazione | Il pezzo torna a ereditare il materiale dal genitore o quello predefinito |
| Nome contiene | Assegna il materiale a tutti i pezzi il cui nome contiene il testo |
| Usa come predefinito per i componenti senza assegnazione | Il materiale predefinito della scena |
| Sostituisci [materiale] con «selezionato» | Ogni pezzo con quel materiale (anche ereditato, e il predefinito) passa a quello selezionato; **Annulla sostituzione** lo riporta |
| Nuovo / Duplica / Elimina | I tuoi materiali (nella categoria *Personalizzati*) |

## La libreria

- **Libreria compresa:** 125 materiali pronti in 13 categorie (metalli, alluminio anodizzato,
  vernici, plastiche, gomme, vetri, liquidi, ceramiche e pietre, legni, tessuti e pelle, asfalto,
  coperture, emissivi). Un pacchetto aggiuntivo porta altri 109 materiali con texture.
- **La mia libreria:** materiali con texture e HDRI salvati sul computer.
- **Scarica da ambientCG / Poly Haven…:** materiali e HDRI gratuiti (CC0).
- **Importa materiale da Blender…:** mappe e valori di un materiale `.blend`, con anteprima.
- Ogni anteprima è calcolata con il path tracer, quindi mostra il materiale vero.

## Valori del materiale

| Controllo | Intervallo | Predefinito | Pagina |
|---|---|---|---|
| Colore base | colore | grigio 0,8 | il colore della superficie (RGB, CMYK o palette, RAL compresi) |
| Metallic | 0 – 1 | 0 | [Metallic](mat.metallic) |
| Roughness | 0 – 1 | 0,5 | [Roughness](mat.roughness) |
| Specular | 0 – 1 | 0,5 | [Specular](mat.specular) |
| IOR | 1 – 3 | 1,5 | [IOR](mat.ior) |
| Transmission | 0 – 1 | 0 | [Transmission](mat.transmission) |
| Emissione e Intensità emissione | colore, 0 – 10000 | nero, 0 | la superficie emette luce e illumina la scena (LED, display) |
| Texture | mappe | nessuna | [Texture e rilievo](mat.texture) |

Il **selettore colore** si apre cliccando il riquadro di un colore: **RGB** (ruota, valori,
codice esadecimale), **CMYK** e **Palette** (RAL Classic con valori indicativi, o palette tue
`.ase`, `.json`, `.csv`). **Applica** tiene il colore, **Annulla** o Esc rimette quello di prima.
