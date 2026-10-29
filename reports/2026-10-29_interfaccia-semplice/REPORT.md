# Interfaccia Semplice / Avanzata (fasi 1–3)

- **Data**: 29/10/2026.
- **Branch**: `claude/festive-heisenberg-d2ow6z`, poi `main`.
- **Commit**: fase 1 `d40e3d4`, fasi 2–3 nel commit di questo report.
- **Macchina**: container di sviluppo, interfaccia con Xvfb e GPU software (lavapipe), render
  sulla CPU a 3 thread, senza denoiser (OIDN non presente nel container).
- **Da verificare su Windows** (RTX 5070): aspetto reale dell'interfaccia e render con il
  denoiser.

## Cosa è stato implementato (verificato)

| Funzione | Dove | Verifica |
|---|---|---|
| Selettore **Semplice / Avanzata** in alto a sinistra, salvato nelle impostazioni (`ui_mode`, predefinito Semplice) | `prefs.rs`, `ui/toolbar.rs`, `ui/mod.rs` | screenshot 01 e 06; test delle impostazioni |
| Pannello Semplice in 5 passi con pulsanti grandi | `ui/simple.rs` | screenshot 01–04 |
| Importa modello, **Appoggia sulla base** (annullabile con Ctrl+Z), **Inquadra il prodotto** (anche la camera di render; tiene conto dei formati verticali) | `app.rs` (`drop_product_to_base`, `frame_product`), `presets.rs` | test `prepare_scene_drops_adds_backdrop_and_configures`; screenshot |
| Aspetti **Sfondo chiaro / Sfondo scuro / Studio scenografico** | `presets.rs` | test di render reale: luminosità media 0,773 (chiaro) contro 0,039 (scuro); screenshot 02–04 |
| **Prepara scena prodotto**: appoggia, fondale se manca (mai doppio), aspetto, qualità, inquadratura | `presets::prepare_scene`, `ForgeApp::simple_prepare_scene` | test; avvio con `--prepara` e render |
| **Annulla preset**: ripristina solo i valori toccati dal preset | `PresetBackup` | test `undo_restores_only_what_the_preset_changed` (una modifica successiva alla risoluzione resta) |
| Indicatore **"Personalizzato"** se luci, ambiente o visualizzazione sono stati cambiati (anche in Avanzata) | `presets::matching_look` | test |
| Qualità **Bozza / Bilanciata / Alta** (campioni, rimbalzi, denoiser finale) | `presets::Quality` | test; screenshot 05 (in Avanzata: 256 campioni, 12 rimbalzi dopo "Bilanciata") |
| Formati **Orizzontale, Quadrato, Verticale 4:5, Storie 9:16** | `presets::FORMATS` | screenshot |
| Materiali: pezzo selezionato e materiale, pulsante della libreria | `ui/simple.rs` | screenshot |
| Avvia render / Pausa / Riprendi con barra di avanzamento; Salva immagine | `ui/simple.rs` | screenshot 02–04 (render in corso) |
| Menu File: "Importa modello…" con i formati veri | `ui/toolbar.rs` | screenshot |

Tolte le voci grigie e false "Importa STEP… / SolidWorks… — Non ancora disponibile": entrambi
gli import funzionano già da "Importa".

**Modalità Avanzata**: invariata. Tutte le schede e i parametri restano raggiungibili. I valori
sono gli stessi della Semplice, perché le due modalità leggono e scrivono gli stessi dati del
renderer. Non è stato aggiunto nessun controllo fittizio.

## Test eseguiti

- `cargo fmt --all --check`: ok.
- `cargo clippy --workspace --exclude forge_step --all-targets --release -- -D warnings`: ok.
- `cargo test --release --workspace --exclude forge_step`: tutti verdi, nessun fallimento
  (compresi i 4 test nuovi di `presets` e il round trip di `ui_mode`).
- `cargo build --release -p forge_app`: ok.
- Prove dell'app con Xvfb:
  - `--modalita semplice|avanzata`;
  - `samples/.../GlassHurricaneCandleHolder.glb --prepara chiaro|scuro|scenografico --render
    --screenshot …`;
  - `--modalita avanzata --prepara chiaro --tab render` (stessi valori in Avanzata).

## Screenshot

| File | Contenuto |
|---|---|
| `01_semplice_avvio.jpg` | Modalità Semplice all'avvio (scena di test) |
| `02_prepara_sfondo_chiaro.jpg` | Modello importato, "Prepara scena" con Sfondo chiaro, render in corso |
| `03_prepara_sfondo_scuro.jpg` | Stesso modello con Sfondo scuro (tre punti luce) |
| `04_prepara_studio_scenografico.jpg` | Stesso modello con Studio scenografico |
| `05_avanzata_stessi_valori.jpg` | Modalità Avanzata dopo "Prepara scena": 256 campioni, 12 rimbalzi, ACES, 0 EV |
| `06_avanzata_avvio.jpg` | Modalità Avanzata all'avvio |

I render negli screenshot sono a 32 campioni, sulla CPU e senza denoiser: il rumore è normale e
non rappresenta la qualità finale.

## Problemi aperti

- I valori dei preset (luci, colori del fondale, campioni) sono una prima proposta: si
  decidono nella scheda `docs/INTERFACCIA_SEMPLICE.md`.
- "Annulla preset" vale per l'ultimo preset applicato (un solo livello).
- La sezione Materiali della Semplice usa la libreria esistente: non c'è ancora una scelta
  rapida di materiali "tipici" (plastica, metallo, vetro…). Va valutata.
- L'interfaccia è in italiano; la localizzazione è in roadmap (dopo questa fase).
- **Non verificato**: aspetto su Windows con DPI alto; render con il denoiser e la GPU RTX.
