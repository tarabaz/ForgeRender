# Versione 0.3.0: studio fotografico, vista isometrica, raggio delle luci, lingue, benvenuto

- **Data**: 29/10/2026.
- **Branch**: `claude/festive-heisenberg-d2ow6z`, poi `main`.
- **Commit**: studio `fa391da`, ortografica `72a250f`, raggio delle luci `e1060cb`, lingue
  `6529d26`, benvenuto e versione nel commit di questo report.
- **Macchina**: container di sviluppo (Xvfb, GPU software lavapipe, CPU 4 thread, senza OIDN).
- **Da verificare su Windows** (RTX 5070): aspetto reale, velocità, render con il denoiser.

## Cosa è stato implementato e come è verificato

| Funzione | Dove | Verifica |
|---|---|---|
| Studio fotografico: tela inclinata + 4 luci, scena di partenza | `presets.rs`, `forge_scene::primitives::studio_sweep`, preset luci `studio-fotografico` | test `studio_look_uses_sweep_and_swaps_backdrop_undoably`, `studio_demo_scene_is_ready`; screenshot 01 (l'immagine è un render CPU di questa scena, 400 campioni) |
| Colore delle luci (3200/5500/7500 K e per luce) | `ui/simple.rs` | lo stesso test: il colore non rende l'aspetto "Personalizzato" |
| Adatta la scena al prodotto | `presets::fit_scene` | test `fit_scene_follows_the_product` |
| Vista ortografica / isometrica (viewport e camera di render) | `forge_scene::camera`, `path_tracer.wgsl` | test `ortho_rays_are_parallel_and_match_frame_at_target`; `gpu_orthographic_matches_cpu`: RMSE GPU−CPU 0,0262 contro rumore CPU−CPU 0,0251; ortografica−prospettica 0,1060 |
| Raggio d'azione e sfumatura di puntiformi e spot | `RenderLight::range_window`, `lights.rs`, `path_tracer.wgsl`, `ui/central.rs` (bolla) | test `range_cuts_and_fades_independently_of_intensity` (valori esatti); `gpu_light_range_matches_cpu`: RMSE GPU−CPU 0,0089 contro rumore 0,0087; limitata−illimitata 0,1085; screenshot 03 |
| Lingue italiano/inglese, cambio immediato | `i18n.rs`, `lang/Lan_*.json` | test `english_covers_every_interface_text` (779 testi, segnaposto uguali), `fill_keeps_values_and_reorders`; screenshot 02 |
| Schermata di benvenuto | `ui/splash.rs`, `assets/splash.png` | test `splash_image_decodes`; screenshot 01 |

`cargo fmt`, `cargo clippy --workspace --all-targets -D warnings` e `cargo test --workspace`
verdi.

## Limiti noti

- Restano in italiano i **nomi** degli oggetti e delle luci già nella scena (sono dati del
  progetto, come in ogni programma) e i messaggi del log tecnico.
- Le categorie della libreria in inglese mostrano il nome originale del sito.
- Una lingua aggiunta come file nuovo si legge al prossimo avvio (i file compresi si cambiano
  subito).

## Immagini

- `01_benvenuto.jpg`: schermata di benvenuto all'avvio (italiano).
- `02_interfaccia_inglese.jpg`: modalità Semplice in inglese.
- `03_bolla_raggio_spot.jpg`: spot della demo del cocktail con raggio 0,9 m e sfumatura 50 %.
