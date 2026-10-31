//! Finestra "Importa materiale da Blender (.blend)": si sceglie (o si trascina) un file `.blend`,
//! si vede che cosa si può convertire e lo si salva nella libreria locale.
//!
//! - **Sorgente**: materiali del file, mappe trovate e mancanti, immagini non usate, avvisi.
//! - **Parametri**: rapporto della conversione (valore in Blender → valore per ForgeRender, con
//!   lo stato) e gli slider, modificabili prima di importare.
//! - **Anteprima**: il materiale sullo shader ball delle anteprime, renderizzato con il vero
//!   motore di ForgeRender su un thread a parte; si rifà quando cambia un valore.
//!
//! La lettura del file gira in background (i `.blend` possono essere pesanti). La scena aperta
//! non viene toccata: si scrive solo nella libreria, con le funzioni che usa già la libreria.
//! I materiali interamente procedurali non si mostrano e non si importano.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr, tr_dyn};
use crate::ui::style;
use eframe::egui;
use forge_library::blenderkit::{self, Extracted, Slot, Status};
use forge_scene::Material;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

type Loaded = Result<(Vec<Extracted>, Vec<String>), String>;

/// Lato dell'anteprima nella finestra (punti dell'interfaccia): segue la colonna tra questi limiti.
const PREVIEW_MIN_UI: f32 = 160.0;
const PREVIEW_MAX_UI: f32 = 1024.0;

/// Categorie proposte (come quelle dei siti: nomi italiani nell'interfaccia).
const CATEGORIES: [&str; 8] = ["metal", "wood", "fabric", "leather", "stone", "plastic", "concrete", ""];

#[derive(Default)]
struct Preview {
    /// Valori del materiale mostrati nell'anteprima attuale.
    key: Option<String>,
    /// Chiave, lato in pixel e risultato del render in corso.
    pending: Option<(String, u32, Receiver<Vec<u8>>)>,
    texture: Option<egui::TextureHandle>,
}

#[derive(Default)]
pub struct BlendImportUi {
    pub open: bool,
    path: Option<PathBuf>,
    loading: Option<Receiver<Loaded>>,
    materials: Vec<Extracted>,
    skipped: Vec<String>,
    error: Option<String>,
    selected: usize,
    /// Materiale scelto con i valori modificabili e le mappe nella cartella temporanea.
    edited: Option<Material>,
    maps_dir: Option<PathBuf>,
    name: String,
    category: String,
    preview: Preview,
    status: Option<(String, bool)>,
}

impl Drop for BlendImportUi {
    fn drop(&mut self) {
        self.clear_maps();
    }
}

impl BlendImportUi {
    /// Legge un `.blend` in background.
    pub fn open_file(&mut self, path: PathBuf) {
        self.open = true;
        self.clear_maps();
        self.materials.clear();
        self.skipped.clear();
        self.error = None;
        self.status = None;
        self.edited = None;
        self.preview = Preview::default();
        let (tx, rx) = channel();
        let p = path.clone();
        std::thread::Builder::new()
            .name("forge-blend".into())
            .spawn(move || {
                let r = blenderkit::read_all(&p).map_err(|e| format!("{e:#}"));
                let _ = tx.send(r);
            })
            .ok();
        self.path = Some(path);
        self.loading = Some(rx);
    }

    /// Niente in corso: file letto e anteprima pronta (per gli screenshot automatici).
    pub fn is_settled(&self) -> bool {
        self.loading.is_none() && self.preview.pending.is_none() && (self.edited.is_none() || self.preview.texture.is_some())
    }

    fn clear_maps(&mut self) {
        if let Some(d) = self.maps_dir.take() {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    /// Sceglie il materiale `i`: le mappe vanno in una cartella temporanea (servono
    /// all'anteprima), i valori diventano modificabili.
    fn select(&mut self, i: usize) {
        self.selected = i;
        self.clear_maps();
        self.edited = None;
        self.preview = Preview::default();
        let Some(x) = self.materials.get(i) else { return };
        self.name = x.name.clone();
        let metal = x.material.metallic > 0.5 || x.images.iter().any(|im| im.slot == Slot::Metallic);
        self.category = if metal { "metal".into() } else { String::new() };
        if !x.is_importable() {
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let dir = std::env::temp_dir().join(format!("forge_blend_anteprima_{}_{stamp}", std::process::id()));
        match std::fs::create_dir_all(&dir)
            .map_err(anyhow::Error::from)
            .and_then(|_| x.material_with_maps(&dir))
        {
            Ok(m) => {
                self.edited = Some(m);
                self.maps_dir = Some(dir);
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dir);
                self.error = Some(format!("{e:#}"));
            }
        }
    }

    fn poll(&mut self) {
        let Some(rx) = &self.loading else { return };
        if let Ok(r) = rx.try_recv() {
            self.loading = None;
            match r {
                Ok((materials, skipped)) => {
                    self.materials = materials;
                    self.skipped = skipped;
                    if self.materials.is_empty() {
                        self.error = Some(tr("Nel file non c'è un materiale convertibile.").into());
                    } else {
                        // Si parte dal primo importabile.
                        let first = self.materials.iter().position(Extracted::is_importable).unwrap_or(0);
                        self.select(first);
                    }
                }
                Err(e) => self.error = Some(e),
            }
        }
    }
}

/// Anteprima del materiale modificato, grande quanto la colonna (segue la finestra): si rifà
/// quando cambiano i valori o la dimensione (una alla volta; intanto quella vecchia si scala).
fn preview_image(ui: &mut egui::Ui, pv: &mut Preview, m: &Material) {
    // Lato mostrato: tutta la larghezza della colonna, senza superare l'altezza libera.
    let side = ui
        .available_width()
        .min(ui.available_height() - 90.0)
        .clamp(PREVIEW_MIN_UI, PREVIEW_MAX_UI);
    // Pixel renderizzati: il lato reale sullo schermo, a gradini di 64 per non rifarla a ogni
    // pixel di trascinamento della finestra.
    let px = ((side * ui.ctx().pixels_per_point() / 64.0).round() as u32 * 64).clamp(128, 1024);
    let key = format!("{px}|{m:?}");
    if let Some((k, n, rx)) = &pv.pending {
        if let Ok(rgba) = rx.try_recv() {
            let img = egui::ColorImage::from_rgba_unmultiplied([*n as usize; 2], &rgba);
            match &mut pv.texture {
                Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                None => pv.texture = Some(ui.ctx().load_texture("blend-import-preview", img, egui::TextureOptions::LINEAR)),
            }
            pv.key = Some(k.clone());
            pv.pending = None;
        }
    }
    if pv.pending.is_none() && pv.key.as_ref() != Some(&key) {
        let (tx, rx) = channel();
        let mat = m.clone();
        std::thread::Builder::new()
            .name("forge-blend-preview".into())
            .spawn(move || {
                let _ = tx.send(crate::ui::central::render_preview(&mat, px));
            })
            .ok();
        pv.pending = Some((key, px, rx));
    }
    if pv.pending.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
    }
    let size = egui::vec2(side, side);
    match &pv.texture {
        Some(t) => {
            ui.image((t.id(), size));
        }
        None => {
            let (r, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            ui.painter().rect_filled(r, 4.0, egui::Color32::from_gray(40));
        }
    }
    ui.horizontal(|ui| {
        if pv.pending.is_some() {
            ui.spinner();
        }
        ui.label(
            egui::RichText::new(tf(
                "Shader ball, {} campioni sulla {} (Impostazioni, Anteprime dei materiali)",
                &[
                    format!("{}", crate::ui::central::preview_samples()),
                    crate::ui::central::preview_device().to_string(),
                ],
            ))
            .small()
            .weak(),
        );
    });
}

fn status_text(s: Status) -> (String, egui::Color32) {
    match s {
        Status::Exact => (tr("esatto").into(), egui::Color32::from_rgb(140, 200, 140)),
        Status::Approximate => (tr("approssimato").into(), style::WARN),
        Status::Unsupported => (tr("non supportato").into(), egui::Color32::from_rgb(230, 110, 100)),
    }
}

/// Finestra dell'importatore.
pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    let ui_state = &mut app.blend_import;
    ui_state.poll();
    if !ui_state.open {
        return;
    }
    let mut open = true;
    let mut import = false;
    egui::Window::new(tr("Importa materiale da Blender"))
        .id(egui::Id::NULL.with("win-importa-blend"))
        .open(&mut open)
        .default_size([1000.0, 640.0])
        .resizable(true)
        .show(ctx, |ui| {
            let s = &mut app.blend_import;
            ui.horizontal(|ui| {
                if ui.button(tr("Scegli file .blend…")).clicked() {
                    if let Some(p) = rfd::FileDialog::new().add_filter("Blender", &["blend"]).pick_file() {
                        s.open_file(p);
                    }
                }
                match &s.path {
                    Some(p) => ui.label(egui::RichText::new(p.display().to_string()).small()),
                    None => ui.label(egui::RichText::new(tr("…oppure trascina qui un file .blend")).weak()),
                };
                if s.loading.is_some() {
                    ui.spinner();
                    ui.label(tr("Lettura del file…"));
                }
            });
            if let Some(e) = &s.error {
                ui.colored_label(egui::Color32::from_rgb(230, 110, 100), e);
            }
            ui.separator();
            if s.materials.is_empty() {
                ui.label(
                    egui::RichText::new(tr(
                        "Si leggono i materiali del file (Principled BSDF o Metallic BSDF): mappe impacchettate o \
                         accanto al file, valori dei nodi. I materiali interamente procedurali non si importano.",
                    ))
                    .weak(),
                );
                return;
            }
            ui.columns(3, |cols| {
                source_column(&mut cols[0], s);
                params_column(&mut cols[1], s);
                import |= preview_column(&mut cols[2], s);
            });
        });
    let s = &mut app.blend_import;
    s.open = open;
    if !open {
        s.clear_maps();
    }
    if import {
        if let (Some(x), Some(m), Some(path)) = (s.materials.get(s.selected), s.edited.as_ref(), s.path.as_ref()) {
            let name = s.name.trim().to_string();
            let source = blenderkit::source_of(path, &x.name);
            let cats: Vec<String> = [s.category.clone()].into_iter().filter(|c| !c.is_empty()).collect();
            let mut values = m.clone();
            values.name = name.clone();
            s.status = Some(match blenderkit::save_to_library(x, &values, &name, &source, &cats) {
                Ok(saved) => {
                    app.library.refresh();
                    (
                        tf(
                            "«{}» salvato nella libreria ({}).",
                            &[name.clone(), saved.dir.display().to_string()],
                        ),
                        false,
                    )
                }
                Err(e) => (format!("{e:#}"), true),
            });
        }
    }
}

fn source_column(ui: &mut egui::Ui, s: &mut BlendImportUi) {
    ui.strong(tr("Sorgente Blender"));
    egui::ScrollArea::vertical().id_salt("blend-src").show(ui, |ui| {
        ui.label(tf("Materiali nel file ({})", &[format!("{}", s.materials.len())]));
        let mut pick = None;
        for (i, x) in s.materials.iter().enumerate() {
            let mut label = x.name.clone();
            if !x.is_importable() {
                label = tf("{} — procedurale", std::slice::from_ref(&label));
            } else if x.is_partly_procedural() {
                label = tf("{} — in parte procedurale", std::slice::from_ref(&label));
            }
            if ui.selectable_label(i == s.selected, label).clicked() && i != s.selected {
                pick = Some(i);
            }
        }
        if let Some(i) = pick {
            s.select(i);
        }
        for k in &s.skipped {
            ui.label(
                egui::RichText::new(tf("Non convertibile: {}", std::slice::from_ref(k)))
                    .small()
                    .weak(),
            );
        }
        let Some(x) = s.materials.get(s.selected) else { return };
        ui.separator();
        ui.label(tf("Shader: {}", &[x.shader.trim_start_matches("ShaderNode").to_string()]));
        ui.add_space(4.0);
        ui.strong(tr("Mappe trovate"));
        for slot in [Slot::Color, Slot::Metallic, Slot::Roughness, Slot::Normal, Slot::Height] {
            match x.images.iter().find(|i| i.slot == slot) {
                Some(img) => {
                    ui.label(format!("• {}: {}", tr_dyn(slot.label()), img.name))
                        .on_hover_text(&img.origin);
                }
                None => {
                    ui.label(egui::RichText::new(format!("— {}: {}", tr_dyn(slot.label()), tr("nessuna mappa"))).weak());
                }
            }
        }
        if !x.unused_images.is_empty() {
            ui.add_space(4.0);
            ui.label(tr("Immagini del file non usate (nessuno slot in ForgeRender):"));
            for n in &x.unused_images {
                ui.label(egui::RichText::new(format!("• {n}")).small());
            }
        }
        for n in &x.notes {
            ui.add_space(4.0);
            let color = if x.is_importable() {
                style::WARN
            } else {
                egui::Color32::from_rgb(230, 110, 100)
            };
            ui.colored_label(color, n);
        }
    });
}

fn params_column(ui: &mut egui::Ui, s: &mut BlendImportUi) {
    ui.strong(tr("Conversione dei parametri"));
    let Some(x) = s.materials.get(s.selected) else { return };
    if !x.is_importable() {
        ui.colored_label(
            egui::Color32::from_rgb(230, 110, 100),
            tr("Materiale interamente procedurale: ForgeRender usa mappe, non nodi. Non si importa."),
        );
        return;
    }
    let params = x.params.clone();
    egui::ScrollArea::vertical().id_salt("blend-params").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Nome"));
            ui.text_edit_singleline(&mut s.name);
        });
        ui.horizontal(|ui| {
            ui.label(tr("Categoria"));
            egui::ComboBox::from_id_salt("blend-cat")
                .selected_text(if s.category.is_empty() {
                    tr("nessuna").to_string()
                } else {
                    forge_library::categories::label(&s.category)
                })
                .show_ui(ui, |ui| {
                    for c in CATEGORIES {
                        let text = if c.is_empty() {
                            tr("nessuna").to_string()
                        } else {
                            forge_library::categories::label(c)
                        };
                        ui.selectable_value(&mut s.category, c.to_string(), text);
                    }
                });
        });
        if let Some(m) = s.edited.as_mut() {
            ui.add_space(4.0);
            egui::Grid::new("blend-values").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(tr("Colore base"));
                ui.horizontal(|ui| {
                    ui.color_edit_button_rgb(&mut m.base_color);
                    if m.base_color_texture.is_some() {
                        ui.label(egui::RichText::new(tr("× mappa")).small().weak());
                    }
                });
                ui.end_row();
                ui.label(tr("Metallic"));
                ui.add(egui::Slider::new(&mut m.metallic, 0.0..=1.0));
                ui.end_row();
                ui.label(tr("Roughness"));
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut m.roughness, 0.0..=1.0));
                    if m.roughness_texture.is_some() {
                        ui.checkbox(&mut m.roughness_is_gloss, tr("glossiness"));
                    }
                });
                ui.end_row();
                ui.label(tr("IOR"));
                ui.add(egui::Slider::new(&mut m.ior, 1.0..=3.0));
                ui.end_row();
            });
            egui::CollapsingHeader::new(tr("Scala e posizionamento texture"))
                .default_open(true)
                .show(ui, |ui| {
                    crate::ui::inspector::mapping_controls(ui, m, None);
                });
            if m.normal_texture.is_some() || m.height_texture.is_some() {
                egui::CollapsingHeader::new(tr("Rilievo (normal, bump, displacement)"))
                    .default_open(true)
                    .show(ui, |ui| {
                        crate::ui::inspector::relief_controls(ui, m);
                    });
            }
        }
        ui.add_space(6.0);
        ui.strong(tr("Da Blender a ForgeRender"));
        egui::Grid::new("blend-report")
            .num_columns(4)
            .striped(true)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new(tr("Parametro")).small().strong());
                ui.label(egui::RichText::new(tr("Blender")).small().strong());
                ui.label(egui::RichText::new(tr("ForgeRender")).small().strong());
                ui.label(egui::RichText::new(tr("Stato")).small().strong());
                ui.end_row();
                for p in &params {
                    let (st, col) = status_text(p.status);
                    ui.label(egui::RichText::new(tr_dyn(p.name)).small());
                    ui.label(egui::RichText::new(&p.blender).small());
                    ui.label(egui::RichText::new(&p.forge).small());
                    let r = ui.colored_label(col, egui::RichText::new(st).small());
                    if !p.note.is_empty() {
                        r.on_hover_text(&p.note);
                    }
                    ui.end_row();
                }
            });
        ui.label(
            egui::RichText::new(tr("Passa il mouse sullo stato per leggere la nota della conversione."))
                .small()
                .weak(),
        );
    });
}

/// Colonna dell'anteprima; `true` se si chiede di importare.
fn preview_column(ui: &mut egui::Ui, s: &mut BlendImportUi) -> bool {
    ui.strong(tr("Anteprima"));
    let Some(m) = s.edited.clone() else {
        ui.label(egui::RichText::new(tr("Nessuna anteprima per questo materiale.")).weak());
        return false;
    };
    preview_image(ui, &mut s.preview, &m);
    ui.add_space(8.0);
    let ok = !s.name.trim().is_empty();
    let clicked = ui
        .add_enabled(ok, egui::Button::new(tr("Importa nella libreria")))
        .on_hover_text(tr("Salva mappe e valori come materiale della libreria (la scena non cambia)"))
        .clicked();
    if let Some((msg, err)) = &s.status {
        let col = if *err {
            egui::Color32::from_rgb(230, 110, 100)
        } else {
            egui::Color32::from_rgb(140, 200, 140)
        };
        ui.colored_label(col, msg);
    }
    if let Some(p) = &s.path {
        if blenderkit::is_blenderkit_download(p) {
            ui.label(
                egui::RichText::new(tr(
                    "Materiale BlenderKit: di solito con licenza Royalty Free, va bene nei tuoi render ma non si ridistribuisce.",
                ))
                .small()
                .weak(),
            );
        }
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Percorso completo su un .blend vero (non nel repository): lettura in background, materiale
    /// scelto con le mappe, salvataggio in una libreria temporanea con le funzioni della libreria.
    /// `FORGE_BLENDERKIT_BLEND=…\file.blend cargo test -- --ignored import_real_blend`.
    #[test]
    #[ignore]
    fn import_real_blend_into_temp_library() {
        let blend = PathBuf::from(std::env::var("FORGE_BLENDERKIT_BLEND").expect("FORGE_BLENDERKIT_BLEND"));
        let lib = std::env::temp_dir().join(format!("forge_blend_lib_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&lib);
        std::env::set_var("FORGE_LIBRARY_DIR", &lib);
        let mut ui = BlendImportUi::default();
        ui.open_file(blend.clone());
        let t0 = std::time::Instant::now();
        while ui.loading.is_some() && t0.elapsed().as_secs() < 60 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            ui.poll();
        }
        assert!(ui.error.is_none(), "{:?}", ui.error);
        let m = ui.edited.clone().expect("materiale con le mappe");
        assert!(m.base_color_texture.as_ref().is_some_and(|p| p.is_file()));
        let x = &ui.materials[ui.selected];
        let saved = blenderkit::save_to_library(x, &m, "Prova import", &blenderkit::source_of(&blend, &x.name), &["metal".into()])
            .expect("salvato");
        assert!(saved.dir.starts_with(&lib));
        let back = forge_library::store::load(&saved.dir).unwrap();
        assert_eq!(back.material.name, "Prova import");
        assert!(back
            .material
            .texture_slots()
            .into_iter()
            .flatten()
            .all(|p| p.is_file() && p.starts_with(&saved.dir)));
        drop(ui);
        let _ = std::fs::remove_dir_all(&lib);
    }
}
