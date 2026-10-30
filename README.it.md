# ForgeRender

🇬🇧 [Read in English](README.md) (versione di riferimento)

Renderer fotorealistico per modelli CAD, assiemi meccanici e scene 3D in generale, scritto in
Rust. Path tracing progressivo fisicamente basato su CPU e GPU, interfaccia desktop, pensato
per Windows 11 (sviluppato per Ryzen 9 9900X + RTX 5070) ma funzionante anche senza GPU dedicata.

> **Controlla ForgeRender con l'IA — MCP**
>
> Apri ForgeRender, accendi **Controllo IA** e chiedi a Claude di preparare la scena, importare un modello CAD, posizionare le luci da studio, assegnare i materiali, impostare la camera e lanciare il render. 25 comandi, modifiche visibili in tempo reale e anteprime dei render restituite all'assistente. Decidi tu quando l'IA può comandare: quando si scollega, ForgeRender resta aperto con il tuo lavoro.
>
> ![Controllo IA acceso e Claude collegato](docs/img/mcp/it/barra_controllato.png)
>
> **[Configura Claude Code o Claude Desktop →](docs/MCP.md)** · [English guide](docs/MCP.en.md)

> Vedi [docs/ROADMAP.it.md](docs/ROADMAP.it.md) per cosa funziona davvero oggi, cosa viene dopo
> e quanto è complesso ogni passo.

## Cosa fa oggi

- **Interfaccia Semplice / Avanzata** (selettore in alto, predefinita Semplice): "Foto prodotto in
  5 passi" con pulsanti grandi (importa, appoggia sulla base, inquadra, aspetto, materiali,
  formato 16:9 / 1:1 / 4:5 / 9:16, qualità Bozza / Bilanciata / Alta, render e salvataggio).
  La Avanzata ha tutti i controlli; le due modalità modificano gli stessi valori.
- **Studio fotografico**: scena di partenza con fondale in tela inclinato e quattro luci da
  studio; colore delle luci (calda 3200 K, neutra 5500 K, fredda 7500 K o per singola luce).
  Altri aspetti: Sfondo chiaro, Sfondo scuro, Studio scenografico. **Prepara scena prodotto** e
  **Adatta la scena al prodotto** ridimensionano fondale e luci sulle misure del prodotto;
  **Annulla preset** riporta la scena com'era. Luci e oggetti nuovi nascono vicino al prodotto.
- **Lingue**: italiano e inglese, cambio immediato di tutta l'interfaccia (in alto o in
  Impostazioni → Lingua). Le traduzioni sono file separati `lang/Lan_*.json`: se ne aggiungono
  altri copiando `Lan_eng.json`.
- **Controllo IA (MCP)**: comanda ForgeRender in linguaggio naturale tramite Claude Code o Claude Desktop, con il ponte `forgerender-mcp.exe` e l'interruttore "Controllo IA" nella barra. Consulta la [guida di configurazione MCP](docs/MCP.md) per installazione, esempi, strumenti disponibili e sicurezza.
- **Schermata di benvenuto** all'avvio con immagine (`assets/splash.png`), versione e commit;
  si chiude con "Chiudi" e si riapre da File.
- **Path tracing** su CPU (tutti i core) e su GPU (compute shader wgpu, Vulkan/DX12) con accumulo
  progressivo reale (16…1024 campioni, personalizzato o illimitato, modificabile durante il render).
  Next Event Estimation + Multiple Importance Sampling, Russian roulette, BVH SAH.
- **Denoiser**: Intel Open Image Denoise caricato a runtime, dispositivo CPU o CUDA, buffer guida
  albedo/normali, modalità solo finale o anteprima progressiva.
- **Materiali**: diffuso, metalli e plastiche GGX con compensazione multi-scattering, vetro con
  Fresnel esatto, vetro satinato, superfici emissive campionate come luci.
  Mappe PBR complete: colore base, rugosità/lucidità, metallicità, normal (OpenGL/DirectX),
  height (bump), **displacement vero**, occlusione ambientale, ritaglio con opacità; mapping UV o
  proiezione box in metri per i modelli CAD senza UV, con dimensione della ripetizione, rotazione
  e spostamento.
- **Libreria**: ~120 materiali pronti, più materiali e HDRI scaricati da ambientCG e Poly Haven
  o importati da cartelle/zip, conservati nella cartella locale `Libreria` con anteprime.
- **Luci e ambiente**: luci ad area, puntiformi, spot e direzionali, preset da studio; ambiente
  uniforme, a gradiente o HDRI (.hdr/.exr) con importance sampling. Puntiformi e spot hanno un
  **raggio d'azione** con sfumatura del bordo, indipendente dall'intensità, disegnato come una
  bolla nella viewport (CPU e GPU).
- **Nebbia volumetrica**: nebbia da ghiaccio secco procedurale (con seed, riproducibile) che
  esce da un emettitore puntiforme, cade, si deposita, si allarga e scorre attorno agli oggetti;
  resa come vero mezzo partecipante (delta tracking, fase Henyey–Greenstein, ombre nella nebbia)
  su CPU e GPU.
- **Camere**: camera orbitale della viewport più camere di rendering con obiettivo professionale:
  focale in mm, diaframma f/, profondità di campo (piano di fuoco e limiti vicino/lontano
  disegnati nella viewport), lamelle, decentramento. **Vista ortografica e isometrica** nella
  viewport (icone accanto al navigatore, tasto 5 del tastierino) e nella camera di render.
- **Import**: glTF/GLB (esportazione consigliata da SolidWorks), OBJ (con .mtl), STL, Blender
  `.blend` (tramite Blender installato); STEP con Open CASCADE e SolidWorks SLDPRT/SLDASM sono
  sperimentali.
- **Modifica**: gizmo sposta/ruota/scala con snap (anche per le luci, con le maniglie del cono
  dello spot, e per l'emettitore della nebbia), snap a contatto per oggetto, annulla/ripeti, nascondi (H), duplica, appoggio
  esatto sul pavimento, navigatore degli assi stile Blender, albero dei componenti, assegnazione
  dei materiali persistente per id stabile.
- Avvio / pausa / ripresa / stop / riavvio automatico quando la scena cambia, export PNG.
- Progetti `.frproject` (JSON versionato, salvataggio atomico, percorsi relativi).
- CLI per render senza interfaccia, benchmark e import nella libreria.

## Scaricare la versione Windows

La build su GitHub Actions **non** parte ai push: si avvia a mano da *Actions → CI → Run workflow*
(oppure si compila in locale con `Build-ForgeRender.ps1`). Nella sezione *Artifacts* del run ci
sono due pacchetti:

| Pacchetto | Contenuto |
|---|---|
| `ForgeRender-portable-x64` | zip: si estrae e si avvia `forgerender.exe`, nessuna installazione |
| `ForgeRender-setup-x64` | installer (menu Start, disinstallazione, associazione `.frproject`) |

Entrambi sono autonomi: CRT statico, solo DLL di sistema di Windows più le librerie di Open
Image Denoise incluse.

## Compilare in locale

Requisiti: Rust stable (`rustup`); su Windows i *Visual Studio Build Tools* (C++).

```
cargo test --workspace --release
cargo run --release -p forge_app            # interfaccia grafica
cargo run --release -p forge_cli -- bench   # benchmark
```

Pacchetti Windows in locale: `powershell -ExecutionPolicy Bypass -File packaging\windows\package.ps1`
(per l'installer serve [Inno Setup 6](https://jrsoftware.org/isdl.php)).

## Benchmark: comandi da lanciare

Elenco di tutte le misure che si possono fare, con a cosa serve ciascuna. I risultati veri (e i
dati grezzi) vanno in [`reports/`](reports/README.md), una cartella per esperimento; quelli già
fatti sono in [docs/BENCHMARK.md](docs/BENCHMARK.md). Regola: si riportano solo numeri misurati.

### Preparazione (una volta)

I comandi sono per PowerShell. Basta lo zip `ForgeRender-portable-x64`:
- estrailo e apri PowerShell **nella cartella estratta**;
- i progetti di benchmark sono già dentro, in `samples\scene\cocktail-ghiaccio-secco\`;
- non serve né Python né il repository.

In alternativa usa la cartella del repository con `forgerender-cli.exe` compilato da
`Build-ForgeRender.ps1`.

```powershell
cd C:\percorso\ForgeRender-portable-x64          # cartella dello zip estratto
$cli = ".\forgerender-cli.exe"                    # nel repository: ".\target\release\forgerender-cli.exe"
```

I progetti di benchmark sono derivati dalla demo del cocktail, tutti senza profondità di campo
e con la camera fissa. Se cambia la demo si rigenerano con
`reports/strumenti/crea_progetti_bench.py`, e la CI controlla che siano allineati.

Python (`py -m pip install numpy pillow`) serve solo per gli script di confronto delle immagini
in `reports/strumenti/`. Se non puoi usarlo, metti le immagini in `reports/AAAA-MM-GG_.../` e
l'analisi la fa l'agente.

| Progetto | Cosa inquadra | Seed |
|---|---|---|
| `bench_intera` | l'intera demo | 12345 |
| `bench_vicina` | ravvicinata sulla nebbia | 12345 |
| `bench_ad` | l'intera demo | dato da `--seed` |

Le righe da leggere nell'output del render sono:
- `rendering N s | M ms/passata | X Mcampioni/s`
- `geometria compilata … in N ms`
- `nebbia «…»: simulazione … in N ms`

### 1. Hardware rilevato

```powershell
& $cli info
```
**A cosa serve**: verificare CPU, thread e RAM viste dal programma. Il nome va scritto in ogni
report.

### 2. Benchmark standard (CPU, GPU, denoiser)

```powershell
& $cli bench --json bench.json
& $cli bench --spp 64 --width 1920 --height 1080 --threads 1,4,8,12,24 --json bench-1080p.json
```
**A cosa serve**:
- **CPU**: scalabilità al variare dei thread.
- **GPU**: la riga `GPU` riporta Mcampioni/s e quante volte va più veloce di 1 thread.
- **Denoiser**: OIDN su CPU e CUDA, in ms per immagine.

La scena di test è piccola (11 022 triangoli): misura la velocità pura del path tracer.
**Confronto**: la tabella del 9900X in `docs/BENCHMARK.md`. Se cambia di molto senza modifiche
al codice, cambiano driver o impostazioni di energia.

### 3. Scena reale pesante, CPU contro GPU (nebbia, vetro, texture)

```powershell
$p = "samples\scene\cocktail-ghiaccio-secco"
& $cli render --project $p\bench_intera.frproject --backend cpu --width 320 --height 400 --spp 32 --quiet --out intera_cpu.png
& $cli render --project $p\bench_intera.frproject --backend gpu --width 320 --height 400 --spp 32 --quiet --out intera_gpu.png
& $cli render --project $p\bench_vicina.frproject --backend cpu --width 320 --height 400 --spp 32 --quiet --out vicina_cpu.png
& $cli render --project $p\bench_vicina.frproject --backend gpu --width 320 --height 400 --spp 32 --quiet --out vicina_gpu.png
```
**A cosa serve**:
- Quanto rende la GPU su una scena vera, non sulla scena di test.
- Misura il **majorant 3D della nebbia** sulla RTX 5070, che nel container non si è potuto
  misurare.

**Cosa guardare**: la riga `ms/passata`. Prima del render c'è la simulazione della nebbia, che
dura qualche decina di secondi e non conta nel tempo di rendering.

**Confronto**: [report majorant](reports/2026-10-29_majorant-3d-nebbia/REPORT.md). Per
confrontarti con i numeri del container aggiungi `--threads 3` alla riga CPU. Per tempi più
stabili alza la risoluzione (es. `--width 960 --height 1200 --spp 64`).

### 4. Campionamento adattivo (conviene sulla mia scena?)

```powershell
$b = "samples\scene\cocktail-ghiaccio-secco\bench_ad.frproject"
& $cli render --project $b --backend gpu --width 320 --height 400 --spp 1024 --seed 99  --quiet --out ref.png
& $cli render --project $b --backend gpu --width 320 --height 400 --spp 1024 --seed 100 --quiet --out ref2.png
& $cli render --project $b --backend gpu --width 320 --height 400 --spp 32  --quiet --out uni32.png
& $cli render --project $b --backend gpu --width 320 --height 400 --spp 128 --quiet --out uni128.png
& $cli render --project $b --backend gpu --width 320 --height 400 --spp 1024 --noise-threshold 0.08 --quiet --out ad08.png
& $cli render --project $b --backend gpu --width 320 --height 400 --spp 1024 --noise-threshold 0.04 --quiet --out ad04.png
py reports\strumenti\confronta_immagini.py ref.png ref2.png -- uni32.png uni128.png ad08.png ad04.png
```
**A cosa serve**: capire se fermare i pixel già puliti fa risparmiare tempo a parità di
qualità.

**Cosa guardare**:
- Annota il tempo di ogni render.
- Calcola **tempo × MSE**: più è basso, meglio è. Si confronta adattivo contro uniforme.
- Con il threshold attivo il CLI stampa anche i campioni medi per pixel.

Ripeti con `--backend cpu` per confrontare i due backend.

**Diagnostica (dove vanno i campioni)**: aggiungi `--adaptive-map mappa04.png` a un render
adattivo. Scrive un PNG a 16 bit con i campioni esatti di ogni pixel e stampa la quota dei
campioni nella metà "B" usata per stimare l'errore (dovrebbe essere circa 0,5). Poi:

```powershell
py reports\strumenti\analisi_adattivo.py --rif ref.png ref2.png --mappa mappa04.png --img ad04.png --img uni128.png --csv riquadri.csv
```

Per le zone scure, medie e chiare del riferimento stampa: quota di pixel, quota di campioni,
errore di ogni immagine e differenza media con il suo errore standard.

**Confronto**: [report campionamento adattivo](reports/2026-10-29_campionamento-adattivo/REPORT.md).
Nel container, su questa scena, l'adattivo era meno efficiente.

### 5. Import e BVH di una mesh grande

```powershell
& $cli render --mesh C:\percorso\modello_grande.stl --unit mm --spp 16 --width 640 --height 360 --quiet --out mesh.png
```
**A cosa serve**: il tempo di caricamento delle scene CAD pesanti. Funziona anche con
`.step`, `.glb` e `.obj`.

**Cosa guardare**: la riga `geometria compilata: N triangoli, BVH M nodi, in T ms` e il tempo
totale prima che parta il render.

**Confronto**: toro da 1 milione di triangoli in `docs/BENCHMARK.md`.

### 6. Profilo della simulazione della nebbia

```powershell
$env:FORGE_FOG_PROFILE = "1"
& $cli render --project samples\scene\cocktail-ghiaccio-secco\bench_intera.frproject --spp 1 --width 160 --height 200 --quiet --out nebbia.png
Remove-Item Env:FORGE_FOG_PROFILE
```
**A cosa serve**: capire quanto costa ogni fase della simulazione fluida (forze, trasporto,
pressione…), per decidere dove ottimizzare.

**Cosa guardare**: le righe `nebbia «…»: simulazione … passi in N ms` e `profilo nebbia: …`.

### 7. Risoluzione e campioni della scena finale

```powershell
& $cli render --project samples\scene\cocktail-ghiaccio-secco\cocktail-ghiaccio-secco.frproject --backend gpu --quiet --out finale_gpu.png
& $cli render --project samples\scene\cocktail-ghiaccio-secco\cocktail-ghiaccio-secco.frproject --backend gpu --denoise finale --cuda --quiet --out finale_gpu_dn.png
```
**A cosa serve**: il tempo reale di un'immagine finita, con le impostazioni del progetto
(risoluzione, campioni, profondità di campo), con e senza denoiser.

### 8. Test di correttezza (prima di fidarsi dei tempi)

```powershell
cargo test --release --workspace --exclude forge_step
cargo test --release -p forge_render_gpu -- --nocapture
cargo test --release -p forge_render_cpu --test portal -- --nocapture
cargo test --release -p forge_render_cpu --test adaptive -- --nocapture
```
**A cosa serve**:
- **Tutti i test**: confermano che le immagini sono giuste. Tra questi ci sono white furnace,
  luci contro il risultato analitico, nebbia, vetro, portali e adattivo.
- **Test della GPU**: sulla RTX 5070 controllano che CPU e GPU diano la stessa immagine.
- `--nocapture` stampa gli errori misurati (portali, adattivo) da confrontare con i report.

### Dove scrivere i risultati

Crea `reports/AAAA-MM-GG_argomento/` con un `REPORT.md` (macchina, commit, comandi, tabella) e i
file di output e le immagini. Poi aggiungi una riga all'indice in `reports/README.md`.

## Documentazione

- [CHANGELOG.md](CHANGELOG.md) — novità di ogni versione e regola per i numeri di versione
- [reports/](reports/README.md) — report delle misure con dati grezzi, immagini e comandi per rifarle
- [docs/MCP.md](docs/MCP.md) — comandare ForgeRender con Claude Code (MCP): installazione,
  comandi, sicurezza
- [docs/ROADMAP.it.md](docs/ROADMAP.it.md) — stato, prossimi passi con la complessità, piani tecnici
  ([English](docs/ROADMAP.md))
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — architettura, scelte tecniche, rischi
- [docs/BENCHMARK.md](docs/BENCHMARK.md) — misure eseguite (solo dati reali)
- [docs/STATO_LAVORI.md](docs/STATO_LAVORI.md) — diario dello sviluppo
