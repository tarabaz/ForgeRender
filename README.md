# ForgeRender

Renderer fotorealistico per modelli CAD e assiemi meccanici, scritto in Rust.
Path tracing progressivo fisicamente basato, interfaccia desktop in italiano,
pensato per Windows 11 (Ryzen 9 9900X + RTX 5070) ma funzionante anche senza GPU.

> Stato: **Milestone 1 completata** (renderer CPU, GUI, progetti). Vedi
> [docs/ROADMAP.md](docs/ROADMAP.md) per cosa è realmente operativo e cosa no.

## Cosa fa oggi

- Path tracer CPU multithread con accumulo progressivo reale dei campioni
  (16…1024, personalizzato o illimitato), modificabile durante il render.
- Next Event Estimation + Multiple Importance Sampling, Russian roulette, BVH SAH.
- Materiali: diffuso, metalli e plastiche GGX (campionamento VNDF), vetro con Fresnel esatto, emissivi.
  Texture colore/roughness/normal map con mapping UV o proiezione box (per CAD senza UV).
  Libreria di ~120 materiali pronti in 11 categorie, con ricerca.
- Luci ad area, puntiformi, spot, direzionali; ambiente uniforme, a gradiente o HDRI (.hdr/.exr)
  con importance sampling; 5 preset da studio.
- Import STEP/STP (assiemi con nomi, colori, trasformazioni; Open CASCADE in un processo separato),
  OBJ (con .mtl) e STL; SolidWorks SLDPRT/SLDASM sperimentale tramite SolidWorks installato.
- Trasformazioni non distruttive: sposta, ruota, ridimensiona (anche in mm), appoggia sul pavimento.
- Viewport 3D wgpu (orbita/pan/zoom, selezione col clic), albero dei componenti,
  assegnazione materiali persistente per id stabile (singola, multipla, per nome, ereditata).
- Avvio / pausa / ripresa / stop / riavvio automatico quando la scena cambia, export PNG.
- Progetti `.frproject` (JSON versionato, salvataggio atomico, percorsi relativi).
- CLI per render headless e benchmark.

## Scaricare la versione Windows

Ogni push compila automaticamente su GitHub Actions (job *Build e pacchetti (Windows)*)
due pacchetti scaricabili dalla pagina del run, sezione *Artifacts*:

| Pacchetto | Contenuto |
|---|---|
| `ForgeRender-portable-x64` | zip: si estrae e si avvia `forgerender.exe`, nessuna installazione |
| `ForgeRender-setup-x64` | installer (menu Start, disinstallazione, associazione `.frproject`) |

Entrambi sono eseguibili autonomi: CRT collegato staticamente, solo DLL di sistema di Windows.

## Compilare in locale

Requisiti: Rust stable (`rustup`), su Windows i *Visual Studio Build Tools* (C++).

```
cargo test --workspace --release
cargo run --release -p forge_app            # interfaccia grafica
cargo run --release -p forge_cli -- bench   # benchmark
```

Pacchetti Windows in locale: `powershell -ExecutionPolicy Bypass -File packaging\windows\package.ps1`
(per l'installer serve [Inno Setup 6](https://jrsoftware.org/isdl.php)).

## Documentazione

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — architettura, scelte tecniche, rischi
- [docs/ROADMAP.md](docs/ROADMAP.md) — fasi, stato reale delle funzionalità
- [docs/BENCHMARK.md](docs/BENCHMARK.md) — misure eseguite (solo dati reali)
- [docs/STATO_LAVORI.md](docs/STATO_LAVORI.md) — punto di ripresa dello sviluppo
