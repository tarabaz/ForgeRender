//! Finestra "Modifica materiale": si apre con un clic su un materiale della libreria (il clic
//! non lo applica più). A sinistra tutti i parametri (gli stessi del pannello Materiali), a
//! destra l'anteprima sullo shader ball, che si gira trascinando con il mouse, e le azioni:
//! applica alla selezione, salva nella libreria, esporta in uno zip (impostazioni + mappe).

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::style;
use eframe::egui;
use forge_library::store::{self, LibraryMaterial};
use forge_scene::Material;
use std::sync::mpsc::{channel, Receiver};

/// Da dove viene il materiale aperto.
#[derive(Clone)]
enum Source {
    /// Materiale predefinito del programma (si salva come copia nella libreria).
    Preset,
    /// Materiale della libreria (si salva al suo posto).
    Library(Box<LibraryMaterial>),
}

#[derive(Default)]
struct Preview {
    /// Chiave (valori, vista, lato) dell'immagine mostrata.
    key: Option<String>,
    /// Chiave, lato e risultato del render in corso.
    pending: Option<(String, u32, Receiver<Vec<u8>>)>,
    texture: Option<egui::TextureHandle>,
    /// Vista: rotazione in pianta e in alto/basso (gradi).
    yaw: f32,
    pitch: f32,
    /// Si sta trascinando: anteprima veloce (pochi campioni, metà risoluzione).
    dragging: bool,
}

#[derive(Default)]
pub struct MaterialEditorUi {
    pub open: bool,
    source: Option<Source>,
    original: Option<Material>,
    edited: Option<Material>,
    preview: Preview,
    status: Option<(String, bool)>,
}

impl MaterialEditorUi {
    /// Apre un materiale predefinito: si parte dalla versione del progetto se c'è (con le
    /// regolazioni fatte), altrimenti da quella del programma.
    pub fn open_preset(&mut self, m: &Material, scene_version: Option<&Material>) {
        self.open_with(Source::Preset, scene_version.unwrap_or(m).clone());
    }

    /// Apre un materiale della libreria (come per i predefiniti, con la versione del progetto).
    pub fn open_library(&mut self, m: &LibraryMaterial, scene_version: Option<&Material>) {
        self.open_with(Source::Library(Box::new(m.clone())), scene_version.unwrap_or(&m.material).clone());
    }

    fn open_with(&mut self, source: Source, m: Material) {
        self.source = Some(source);
        self.original = Some(m.clone());
        self.edited = Some(m);
        self.preview = Preview::default();
        self.status = None;
        self.open = true;
    }

    /// Id del materiale aperto (per evidenziare la tessera nella libreria).
    pub fn current_id(&self) -> Option<&forge_scene::MaterialId> {
        self.open.then_some(()).and(self.edited.as_ref().map(|m| &m.id))
    }
}

/// Lato in pixel dell'anteprima per il lato mostrato (gradini di 64, come nell'importatore).
fn preview_px(side: f32, ppp: f32, fast: bool) -> u32 {
    let px = ((side * ppp / 64.0).round() as u32 * 64).clamp(128, 1024);
    if fast {
        (px / 2).max(128)
    } else {
        px
    }
}

/// Anteprima del materiale modificato, che si gira trascinando. Restituisce se è cambiata la vista.
fn preview_image(ui: &mut egui::Ui, pv: &mut Preview, m: &Material) {
    let side = ui.available_width().min(ui.available_height() - 140.0).clamp(160.0, 1024.0);
    let fast = pv.dragging;
    let px = preview_px(side, ui.ctx().pixels_per_point(), fast);
    let samples = if fast { 8 } else { crate::ui::central::preview_samples() };
    let key = format!("{px}|{samples}|{:.1}|{:.1}|{m:?}", pv.yaw, pv.pitch);
    if let Some((k, n, rx)) = &pv.pending {
        if let Ok(rgba) = rx.try_recv() {
            let img = egui::ColorImage::from_rgba_unmultiplied([*n as usize; 2], &rgba);
            match &mut pv.texture {
                Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                None => pv.texture = Some(ui.ctx().load_texture("material-editor-preview", img, egui::TextureOptions::LINEAR)),
            }
            pv.key = Some(k.clone());
            pv.pending = None;
        }
    }
    if pv.pending.is_none() && pv.key.as_ref() != Some(&key) {
        let (tx, rx) = channel();
        let (mat, yaw, pitch) = (m.clone(), pv.yaw, pv.pitch);
        std::thread::Builder::new()
            .name("forge-material-editor".into())
            .spawn(move || {
                let _ = tx.send(crate::ui::central::render_preview_view(&mat, px, yaw, pitch, samples));
            })
            .ok();
        pv.pending = Some((key, px, rx));
    }
    if pv.pending.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
    }
    let size = egui::vec2(side, side);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::drag());
    match &pv.texture {
        Some(t) => {
            ui.painter().image(
                t.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        None => {
            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(40));
        }
    }
    // Trascina per girare la vista attorno all'oggetto; doppio clic = vista iniziale.
    if resp.dragged() {
        let d = resp.drag_delta();
        pv.yaw += d.x * 0.5;
        pv.pitch = (pv.pitch + d.y * 0.4).clamp(-70.0, 70.0);
        pv.dragging = true;
    } else if pv.dragging {
        pv.dragging = false;
    }
    if resp.double_clicked() {
        (pv.yaw, pv.pitch) = (0.0, 0.0);
    }
    let resp = resp.on_hover_text(tr(
        "Trascina per girare la vista attorno all'oggetto; doppio clic per tornare alla vista iniziale",
    ));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    ui.horizontal(|ui| {
        if pv.pending.is_some() {
            ui.spinner();
        }
        ui.label(
            egui::RichText::new(tf(
                "Shader ball, {} campioni sulla {} · trascina per girare",
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

/// Azione chiesta dai pulsanti (si esegue dopo il disegno).
enum Action {
    Apply,
    Save,
    Export,
    Revert,
}

pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.material_editor.open {
        return;
    }
    let mut open = true;
    let mut action = None;
    let selection = app.selection.len();
    let title = app
        .material_editor
        .edited
        .as_ref()
        .map(|m| tf("Modifica materiale: {}", std::slice::from_ref(&m.name)))
        .unwrap_or_else(|| tr("Modifica materiale").to_string());
    egui::Window::new(title)
        .id(egui::Id::NULL.with("win-modifica-materiale"))
        .open(&mut open)
        .default_size([980.0, 680.0])
        .resizable(true)
        .show(ctx, |ui| {
            let e = &mut app.material_editor;
            let (Some(m), Some(original)) = (e.edited.as_mut(), e.original.as_ref()) else {
                return;
            };
            let modified = m != original;
            ui.columns(2, |cols| {
                egui::ScrollArea::vertical().id_salt("mat-editor-params").show(&mut cols[0], |ui| {
                    let origin = match &e.source {
                        Some(Source::Library(l)) => tf("Libreria: {}", &[l.dir.display().to_string()]),
                        _ => tr("Materiale predefinito di ForgeRender (si salva come copia nella tua libreria)").to_string(),
                    };
                    ui.label(egui::RichText::new(origin).small().weak());
                    ui.add_space(4.0);
                    crate::ui::inspector::material_form(ui, m, None);
                });
                let ui = &mut cols[1];
                ui.strong(tr("Anteprima"));
                preview_image(ui, &mut e.preview, m);
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(selection > 0, egui::Button::new(tr("Applica alla selezione")))
                        .on_hover_text(if selection > 0 {
                            tr("Mette il materiale, con queste impostazioni, sui pezzi selezionati")
                        } else {
                            tr("Seleziona prima uno o più pezzi nella viewport o nell'albero")
                        })
                        .on_disabled_hover_text(tr("Seleziona prima uno o più pezzi nella viewport o nell'albero"))
                        .clicked()
                    {
                        action = Some(Action::Apply);
                    }
                    let save_hint = match &e.source {
                        Some(Source::Library(_)) => tr("Salva le nuove impostazioni nel materiale della libreria"),
                        _ => tr("Salva una copia nella tua libreria (i predefiniti del programma non cambiano)"),
                    };
                    if ui.button(tr("Salva nella libreria")).on_hover_text(save_hint).clicked() {
                        action = Some(Action::Save);
                    }
                    if ui
                        .button(tr("Esporta zip…"))
                        .on_hover_text(tr(
                            "Un file .zip con materiale.json (tutte le impostazioni) e le mappe: si reinserisce come materiale predefinito",
                        ))
                        .clicked()
                    {
                        action = Some(Action::Export);
                    }
                    if ui.add_enabled(modified, egui::Button::new(tr("Annulla modifiche"))).clicked() {
                        action = Some(Action::Revert);
                    }
                });
                if modified {
                    ui.label(egui::RichText::new(tr("Modifiche non ancora salvate")).small().color(style::WARN));
                }
                if let Some((msg, err)) = &e.status {
                    let color = if *err {
                        egui::Color32::from_rgb(230, 110, 100)
                    } else {
                        egui::Color32::from_rgb(140, 200, 140)
                    };
                    ui.colored_label(color, msg);
                }
            });
        });
    app.material_editor.open = open;
    let Some(action) = action else { return };
    let Some(m) = app.material_editor.edited.clone() else { return };
    match action {
        Action::Revert => {
            app.material_editor.edited = app.material_editor.original.clone();
            app.material_editor.status = None;
        }
        Action::Apply => {
            // Il materiale del progetto prende queste impostazioni.
            app.scene.materials.upsert(m.clone());
            for &i in &app.selection {
                app.scene.assignments.insert(app.scene.nodes[i].id.clone(), m.id.clone());
            }
            app.selected_material = m.id.clone();
            app.appearance_changed();
            app.material_editor.status = Some((tf("«{}» applicato alla selezione.", std::slice::from_ref(&m.name)), false));
        }
        Action::Save => {
            let result = match app.material_editor.source.clone() {
                Some(Source::Library(l)) => store::update(&l.dir, &m),
                _ => store::save_as(&m, &m.name, &format!("ForgeRender, dal predefinito «{}»", m.id.0)),
            };
            app.material_editor.status = Some(match result {
                Ok(saved) => {
                    app.library.material_saved(&saved);
                    app.library.refresh();
                    // Da qui si salva al posto della copia appena creata.
                    app.material_editor.source = Some(Source::Library(Box::new(saved.clone())));
                    app.material_editor.original = Some(m.clone());
                    // Se il progetto usa il materiale, prende le nuove impostazioni.
                    if app.scene.materials.get(&m.id).is_some() {
                        app.scene.materials.upsert(m.clone());
                        app.appearance_changed();
                    }
                    (tf("Salvato nella libreria: {}", &[saved.dir.display().to_string()]), false)
                }
                Err(e) => (tf("Salvataggio non riuscito: {}", &[format!("{e:#}")]), true),
            });
        }
        Action::Export => {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Zip", &["zip"])
                .set_file_name(forge_library::export::suggested_name(&m))
                .save_file()
            else {
                return;
            };
            app.material_editor.status = Some(match forge_library::export::material_zip(&m, &path) {
                Ok(n) => (
                    tf("Esportato con {} mappe: {}", &[n.to_string(), path.display().to_string()]),
                    false,
                ),
                Err(e) => (tf("Esportazione non riuscita: {}", &[format!("{e:#}")]), true),
            });
        }
    }
}
