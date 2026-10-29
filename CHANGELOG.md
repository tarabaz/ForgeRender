# Novità di ForgeRender

La versione è in un solo punto: `version` in `[workspace.package]` del `Cargo.toml` alla radice.
Tutti i componenti la ereditano; il titolo della finestra mostra `ForgeRender v<versione> (<commit>)`.

## Regola per le versioni (0.x)

- **0.X.0** (minore): un gruppo di funzioni nuove consegnato a Valerio. Si incrementa nel commit
  che chiude il gruppo, insieme a questa pagina.
- **0.X.Y** (correzione): solo correzioni di difetti, senza funzioni nuove.
- Ogni incremento aggiunge qui una sezione con data e commit. Il numero cambia nel `Cargo.toml`
  (e nell'installer `packaging/windows/forgerender.iss`, solo come valore di riserva).

## 0.2.0 — 29/10/2026

**Interfaccia**
- Modalità **Semplice / Avanzata** con selettore in alto (predefinita Semplice): "Foto prodotto in
  5 passi" con pulsanti grandi.
  - Appoggia sulla base, Inquadra il prodotto.
  - Aspetti Sfondo chiaro / Sfondo scuro / Studio scenografico.
  - **Prepara scena prodotto** e Annulla preset.
  - Qualità Bozza / Bilanciata / Alta.
  - Formati 16:9, 1:1, 4:5, 9:16.
  - Valori: `docs/INTERFACCIA_SEMPLICE.md`.
- Menu **Impostazioni**: libreria e import, cartella della libreria, valori dei nuovi progetti,
  informazioni; salvate in `impostazioni.json`.
- **Canc** elimina dal progetto anche i componenti importati, senza toccare il file;
  ripristino con Ctrl+Z o da "Eliminati dal progetto".
- Menu File: "Importa modello…" con i formati veri.

**Libreria dei materiali**
- Riconoscimento delle mappe dal suffisso del nome. Prima `metal_plate_02` perdeva colore e
  displacement e usava l'AO come metallicità.
- Materiali importati male segnalati ("⚠ da riscaricare") e riscaricati da `scarica-tutto`.
- Mappe Poly Haven solo in EXR scaricate; mappe mancanti annotate; `bump` e `rough_ao` gestite.
- EXR a un canale (AO, rugosità, height di Poly Haven) letti; colore EXR lineare corretto; height
  del displacement a 16 bit.
- Displacement spento di default all'import; "Valore proposto" nel pannello.
- File MaterialX conservati, con i loro valori visibili nel pannello; "× mappa" accanto ai cursori.
- **Salva / Salva con nome / Ripristina** dei materiali della libreria.
- Categorie e tag (Poly Haven, ambientCG) con filtro; anteprime regolabili 64–512 px.
- `forgerender-cli libreria importa-polyhaven`: libreria dai download locali di Poly Haven
  (copia o spostamento).

**Render**
- Majorant 3D della nebbia (circa 1,7–2× più veloce sulla CPU nella scena del cocktail).
- Campionamento adattivo (spento di default) e diagnostica (`--adaptive-map`).

**Pacchetto e strumenti**
- Benchmark pronti nello zip portable (`samples\…\bench_*.frproject`); comandi nel README e nel
  LEGGIMI.
- Cartella `reports/` con le misure; quaderno condiviso con ChatGPT.
- `Build-ForgeRender.ps1`: aggiornamento da GitHub e compilazione locale con un comando. La build
  di GitHub Actions parte solo a mano.

## 0.1.0

Prima versione: path tracer CPU e GPU, import glTF/GLB, OBJ, STL, STEP, SolidWorks e Blender;
libreria materiali, denoiser OIDN, camera professionale, luci, nebbia volumetrica, progetti
`.frproject`, pacchetti Windows portable e installer.
