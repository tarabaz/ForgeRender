---
id: scene.progetti
titolo: Progetti, pacchetti e scene demo
parole: progetto, salva, apri, frproject, frpack, pacchetto, zip, cartella, recenti, demo, esempi
ui: file.new_empty, file.open, file.save, file.save_as, file.save_pack, file.save_bundle, file.save_zip, file.demo, file.recent
correlate: scene.importa, export.immagine
---
# Progetti, pacchetti e scene demo

**Dove si trova:** menu **File**.

Un progetto (`.frproject`) salva la scena: modelli (come percorsi ai file importati), materiali e
la loro assegnazione ai pezzi, luci, ambiente, effetti, camere, animazione e impostazioni di
render. È un file di testo salvato in modo sicuro (un'interruzione durante il salvataggio non
lascia mai mezzo file); i percorsi sono relativi, quindi una cartella con progetto e modelli si
può spostare.

| Comando | Tasti | Cosa fa |
|---|---|---|
| Nuova scena vuota | Ctrl+N | Una scena senza oggetti né luci |
| Scene demo | | Scene pronte (nella cartella `samples` accanto al programma) |
| Apri progetto… | Ctrl+O | Apre un `.frproject` o un `.frpack` |
| Progetti recenti | | Gli ultimi progetti aperti |
| Salva progetto | Ctrl+S | La prima volta chiede dove |
| Salva progetto con nome… | Ctrl+Maiusc+S | Una copia con un altro nome |
| Salva pacchetto completo (.frpack)… | | Un solo file con dentro tutto: progetto, modelli importati, texture dei materiali e HDRI. Si apre su qualunque computer |
| Salva in una cartella con tutti i file… | | Il progetto e, accanto, i modelli, le texture e l'HDRI che usa: basta copiare la cartella |
| Salva come ZIP con tutti i file… | | La stessa cartella, compressa in uno ZIP da mandare o archiviare |

## File mancanti

Se un modello o una texture non è più dove dice il progetto, il progetto si apre lo stesso e la
console elenca cosa manca. Per portare un progetto su un altro computer usa il **pacchetto** o la
**cartella con tutti i file**.

## Chiusura

**File → Esci** chiede prima di perdere modifiche non salvate. Anche chiudere durante
un'esportazione chiede prima.
