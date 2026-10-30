# Comandare ForgeRender con Claude (MCP)

Da ForgeRender 0.4.0 un assistente IA può impostare la scena e fare i render al posto tuo:
gli scrivi in italiano «metti uno sfondo da studio, aggiungi una luce calda a destra, fai il
pezzo in metallo satinato e fammi un render quadrato», e lui usa i comandi del programma.
Vedi ogni modifica nella finestra di ForgeRender mentre la fa, e lui vede il render che ottiene
(così può correggere luci e inquadratura da solo).

Il collegamento usa il **Model Context Protocol** (MCP), lo standard con cui Claude Code e
Claude Desktop parlano con i programmi esterni. ForgeRender non ha bisogno di internet né di
chiavi: è Claude Code ad avviarlo sul tuo PC con l'opzione `--mcp`.

## 1. Cosa serve

- ForgeRender **0.4.0 o successivo** compilato (`Build-ForgeRender.ps1`).
- **Claude Code** installato su Windows (il comando `claude` nel terminale).

L'eseguibile da usare è quello con il percorso fisso:

```
C:\Users\valer\Dev\RenderingEngine\RenderingEngine_pulito\target\release\forgerender.exe
```

Lo script di build ci copia accanto anche le DLL del denoiser, quindi il render ha anche la
riduzione del rumore. A differenza della cartella del portable, questo percorso non cambia a ogni
versione: la configurazione si fa una volta sola.

## 2. Collegarlo a Claude Code (una volta)

Claude Code avvia i programmi MCP quando parte, quindi conviene una **cartella dedicata**: la
finestra di ForgeRender si apre solo quando lavori lì, non in ogni progetto. Un comando alla
volta in PowerShell:

```
mkdir C:\Users\valer\ForgeRender-IA
```

```
cd C:\Users\valer\ForgeRender-IA
```

```
claude mcp add forgerender -- "C:\Users\valer\Dev\RenderingEngine\RenderingEngine_pulito\target\release\forgerender.exe" --mcp
```

- Senza `--scope` il collegamento vale solo per questa cartella.
- Tutto quello che viene dopo `--` è il comando che Claude Code lancia.

Controllo (apre e richiude ForgeRender per verificare che risponda):

```
claude mcp list
```

Deve comparire `forgerender` come connesso. Dentro Claude Code il comando `/mcp` mostra gli
strumenti di ForgeRender.

Per toglierlo: `claude mcp remove forgerender` (dalla stessa cartella).

### Claude Desktop (alternativa)

Impostazioni → Sviluppatore → Modifica configurazione; nel file `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "forgerender": {
      "command": "C:\\Users\\valer\\Dev\\RenderingEngine\\RenderingEngine_pulito\\target\\release\\forgerender.exe",
      "args": ["--mcp"]
    }
  }
}
```

Poi riavvia Claude Desktop.

## 3. Come si usa

Apri Claude Code nella cartella `ForgeRender-IA` (`cd C:\Users\valer\ForgeRender-IA`, poi
`claude`): si apre anche la finestra di ForgeRender, e in alto compare **"IA collegata (MCP)"**. La prima volta Claude
Code chiede il permesso per ogni strumento: puoi rispondere "sì, sempre" per quelli di
ForgeRender.

Esempi di richieste:

- «Apri `D:\Stampe\drago.stl` (è in millimetri) e prepara uno studio fotografico.»
- «Metti uno sfondo da studio scuro e una luce calda a destra, più un controluce freddo.»
- «Fai il drago in metallo satinato color bronzo, la base in plastica nera opaca.»
- «Inquadratura tre quarti con un 85 mm, formato 4:5 per Instagram.»
- «Fammi un render di prova veloce e dimmi cosa miglioreresti.»
- «La luce a destra è troppo forte: dimezzala e rifai il render a 512 campioni.»
- «Salva il progetto in `D:\Stampe\drago.frproject` e il render finale in `D:\Stampe\drago.png`.»

Claude guarda l'anteprima di ogni render (gli arriva l'immagine) e può correggere da solo.
Puoi sempre intervenire a mano nella finestra: lui legge lo stato con `scene_info` prima di
modificare.

Quando chiudi Claude Code la finestra resta aperta con "IA scollegata": il lavoro non si perde.

## 4. I comandi disponibili

Misure in **metri**, y verso l'alto. "Destra", "sinistra", "davanti" sono rispetto
all'inquadratura del render.

| Strumento | Cosa fa |
|---|---|
| `scene_info` | Stato della scena: prodotto (centro e misure), componenti con materiale, luci, camera, ambiente, impostazioni e stato del render |
| `new_scene` | Scena nuova: studio dimostrativo o vuota |
| `open_project` / `save_project` | Apre o salva un `.frproject` |
| `import_model` | Importa glTF/GLB, OBJ, STL, STEP, Blender, SolidWorks (unità e asse per STL/OBJ) |
| `prepare_studio` | Aspetto completo: studio fotografico, sfondo chiaro, scuro, scenografico; qualità |
| `fit_scene_to_product` | Rifà fondale e luci sulle misure del prodotto |
| `undo_preset` | Annulla l'ultimo `prepare_studio` / `fit_scene_to_product` |
| `add_object` | Pavimento, fondale, fondale fotografico, parete, cubo, sfera, cilindro, tazza |
| `transform_objects` | Sposta, ruota, scala, appoggia a terra (annullabile con Ctrl+Z) |
| `set_visibility` | Mostra o nasconde componenti |
| `add_light` | Luce area (softbox), punto, spot, direzionale; per lato (destra, sinistra, alto, davanti, dietro, controluce…), colore in Kelvin, intensità relativa, raggio d'azione |
| `edit_light` / `remove_light` | Modifica o elimina una luce (per numero o nome) |
| `set_all_lights_color` | Temperatura colore di tutte le luci |
| `list_materials` | Cerca tra i ~120 materiali pronti e la libreria scaricata |
| `apply_material` | Applica un materiale esistente a componenti o a tutto il prodotto |
| `create_material` | Crea un materiale da parametri (colore, metallic, roughness, vetro, emissione) e lo applica |
| `set_camera` | Vista (frontale, tre quarti, isometrica…), angoli, focale, ortografica, zoom, profondità di campo |
| `set_environment` | Ambiente uniforme, gradiente, HDRI o nero; visibile o no |
| `set_render_settings` | Formato (16:9, 1:1, 4:5, 9:16), risoluzione, campioni, qualità, CPU/GPU, denoiser, esposizione |
| `render` | Render, attesa della fine, PNG salvato e anteprima restituita a Claude |
| `stop_render` / `save_image` | Ferma il render; salva l'ultima immagine |

Dove finiscono i render senza un percorso: nella cartella `render` accanto al progetto, oppure
in `renders` accanto ai log (portable: cartella del programma; installata:
`%LOCALAPPDATA%\ForgeRender\renders`).

## 5. Sicurezza

- Claude non può eseguire codice nel programma: solo i comandi della tabella, con gli argomenti
  controllati (numeri nei limiti, percorsi assoluti).
- Un file esistente **non viene sovrascritto** senza `overwrite: true`, che Claude deve usare
  solo se glielo chiedi. Il progetto aperto si può risalvare sul suo file.
- I materiali della libreria non vengono modificati: `create_material` crea un materiale nuovo
  del progetto (id `ia-…`).
- Trasformazioni e preset si annullano come quelli fatti a mano (Ctrl+Z, "Annulla preset").

## 6. Se qualcosa non va

- **"ForgeRender non risponde: la finestra è ridotta a icona…"**: i comandi si eseguono come
  i clic, a ogni aggiornamento della finestra. Tienila aperta (non ridotta a icona) e senza
  finestre di dialogo davanti.
- **`claude mcp list` dice "failed"**: controlla il percorso dell'eseguibile; prova a lanciare
  a mano `forgerender.exe --mcp` (si apre la finestra e aspetta comandi: chiudila).
- **Render lento**: per le prove chiedi pochi campioni (32–64) e risoluzione bassa; per il finale
  la GPU (`set_render_settings` con device gpu).
- Il registro dei comandi ricevuti è nel log di ForgeRender (righe "MCP: …"), visibile in basso
  nella finestra e in `forgerender.log`.

## 7. Per sviluppatori

- `forgerender --mcp`: `crates/forge_app/src/mcp/`.
  - `rpc.rs`: JSON-RPC 2.0 su stdin/stdout, una riga per messaggio (protocollo 2025-06-18,
    accetta anche 2025-03-26 e 2024-11-05); `initialize`, `tools/list`, `tools/call`, `ping`.
  - `mod.rs`: un thread legge stdin; `tools/call` passa alla finestra con un canale e si
    esegue in `ForgeApp::tick`, quindi nessun accesso concorrente alla scena. Import, apertura
    dei progetti e render completano nei fotogrammi successivi (`tools::Waiter`).
  - `tools.rs`: schema e implementazione degli strumenti, che chiamano le stesse funzioni dei
    pulsanti (`simple_prepare_scene`, `import_path`, `start_render`…).
  - `ops.rs`: parti senza finestra (argomenti, lati delle luci, ricerca di luci, componenti e
    materiali, stato della scena), con test.
- I log vanno su **stderr** (stdout è riservato al protocollo).
- Per aggiungere uno strumento: definizione in `tools::list`, ramo in `dispatch` (un test
  controlla che ogni nome abbia il suo ramo), test della parte pura in `ops.rs`.
- Prova completa senza Claude: uno script che lancia `forgerender --mcp` e scrive le richieste
  su stdin (esempio nel report `reports/2026-10-30_mcp/`).
