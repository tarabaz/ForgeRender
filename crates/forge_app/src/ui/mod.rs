//! Interfaccia utente (egui). Layout:
//! barra superiore (comandi), pannello sinistro (albero della scena),
//! pannello destro (proprietà), pannello inferiore (progressi, statistiche,
//! log), area centrale (viewport o render).

pub mod central;
pub mod gizmo;
pub(crate) mod inspector;
pub mod library;
pub mod navigator;
mod settings;
mod simple;
mod splash;
mod status;
pub mod style;
mod toolbar;
mod transform;
pub mod tree;

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use eframe::egui;

impl eframe::App for ForgeApp {
    /// Chiudendo il programma il controllo IA si spegne (file di contatto tolto).
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(s) = self.mcp.take() {
            s.shutdown();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        self.handle_dropped_files(&ctx);
        self.handle_shortcuts(&ctx);

        egui::Panel::top("toolbar").show(ui, |ui| toolbar::show(self, ui));
        egui::Panel::bottom("status")
            .resizable(true)
            .default_size(170.0)
            .min_size(70.0)
            .show(ui, |ui| status::show(self, ui));
        // Limiti di larghezza: un contenuto largo (es. molte miniature) non può più far
        // crescere un pannello fino a coprire la viewport; vale anche per le larghezze
        // salvate dalle sessioni precedenti.
        let total = ui.available_width();
        egui::Panel::left("tree")
            .resizable(true)
            .default_size(270.0)
            .min_size(180.0)
            .max_size((total * 0.3).max(200.0))
            .show(ui, |ui| tree::show(self, ui));
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(360.0)
            .min_size(260.0)
            .max_size((total * 0.45).max(280.0))
            .show(ui, |ui| match self.prefs.ui_mode {
                crate::prefs::UiMode::Simple => simple::show(self, ui),
                crate::prefs::UiMode::Advanced => inspector::show(self, ui),
            });
        egui::CentralPanel::default().show(ui, |ui| central::show(self, ui, frame));

        self.dialogs(&ctx);
    }
}

impl ForgeApp {
    fn import_options_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut go = false;
        egui::Window::new(tr("Opzioni di importazione"))
            .id(egui::Id::NULL.with("win-opzioni-di-importazione"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                for f in self.import_queue.iter().take(8) {
                    ui.label(f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
                }
                if self.import_queue.len() > 8 {
                    ui.label(tf("… e altri {}", &[format!("{}", self.import_queue.len() - 8)]));
                }
                ui.separator();
                let o = &mut self.import_opts;
                egui::Grid::new("import-opts").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("Unità (STL / OBJ)"));
                    egui::ComboBox::from_id_salt("unit")
                        .selected_text(tr(o.unit.label()))
                        .show_ui(ui, |ui| {
                            for u in forge_scene::LengthUnit::ALL {
                                ui.selectable_value(&mut o.unit, u, tr(u.label()));
                            }
                        });
                    ui.end_row();
                    ui.label(tr("Asse verticale"));
                    egui::ComboBox::from_id_salt("up")
                        .selected_text(tr(o.up.label()))
                        .show_ui(ui, |ui| {
                            for u in [forge_import::UpAxis::Auto, forge_import::UpAxis::Y, forge_import::UpAxis::Z] {
                                ui.selectable_value(&mut o.up, u, tr(u.label()));
                            }
                        })
                        .response
                        .on_hover_text(tr("Automatico: STL con Z in alto; OBJ, STEP e SolidWorks con Y in alto"));
                    ui.end_row();
                    ui.label(tr("Qualità triangolazione (STEP)"));
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut o.step_quality, forge_import::StepQuality::Bassa, tr("Bassa"));
                        ui.radio_value(&mut o.step_quality, forge_import::StepQuality::Media, tr("Media"));
                        ui.radio_value(&mut o.step_quality, forge_import::StepQuality::Alta, tr("Alta"));
                    });
                    ui.end_row();
                    ui.label(tr("Posizione"));
                    ui.checkbox(&mut o.drop_to_floor, tr("Appoggia sul pavimento dopo l'importazione"));
                    ui.end_row();
                    ui.label(tr("Angolo di piega normali"));
                    ui.add(egui::Slider::new(&mut o.crease_angle_deg, 0.0..=90.0).suffix("°"))
                        .on_hover_text(tr(
                            "Solo per mesh senza normali (STL): sotto questo angolo le facce vengono smussate",
                        ));
                    ui.end_row();
                });
                ui.label(
                    egui::RichText::new(tr(
                        "Lo STEP dichiara le proprie unità; i file SolidWorks richiedono SolidWorks installato su questo PC.",
                    ))
                    .weak()
                    .small(),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    go = ui.button(egui::RichText::new(tr("Importa")).strong()).clicked();
                    if ui.button(tr("Annulla")).clicked() {
                        self.import_queue.clear();
                        self.show_import_options = false;
                    }
                });
            });
        if !open {
            self.import_queue.clear();
            self.show_import_options = false;
        }
        if go {
            self.run_import_queue();
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let files: Vec<std::path::PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        let (projects, models): (Vec<_>, Vec<_>) = files.into_iter().partition(|f| {
            f.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case(forge_project::EXTENSION))
        });
        if let Some(p) = projects.first() {
            self.open_project(p);
        }
        self.ask_import(models);
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (save, open, import, frame_all, frame_sel, render) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::S),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::O),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::I),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Home),
                i.consume_key(egui::Modifiers::NONE, egui::Key::F),
                i.consume_key(egui::Modifiers::NONE, egui::Key::F5),
            )
        });
        // Gizmo e annulla/ripeti (Ctrl+Maiusc+Z = ripeti, come Ctrl+Y).
        let (redo, undo) = ctx.input_mut(|i| {
            let redo = i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z)
                || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y);
            (redo, i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z))
        });
        let (dup, del) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::D),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
            )
        });
        // 5 (anche del tastierino): prospettica / ortografica, come in Blender.
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Num5)) {
            self.toggle_ortho();
        }
        if dup {
            self.duplicate_selection();
        }
        if del {
            self.delete_selected_copies();
        }
        let (hide, isolate, unhide) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::H),
                i.consume_key(egui::Modifiers::SHIFT, egui::Key::H),
                i.consume_key(egui::Modifiers::ALT, egui::Key::H),
            )
        });
        if hide || isolate {
            self.hide_selection(isolate);
        }
        if unhide {
            self.show_all();
        }
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if !self.gizmo.is_dragging() {
            use crate::ui::gizmo::GizmoMode;
            ctx.input_mut(|i| {
                for (k, m) in [
                    (egui::Key::Q, GizmoMode::Select),
                    (egui::Key::W, GizmoMode::Move),
                    (egui::Key::E, GizmoMode::Rotate),
                    (egui::Key::R, GizmoMode::Scale),
                ] {
                    if i.consume_key(egui::Modifiers::NONE, k) {
                        self.gizmo.mode = m;
                    }
                }
            });
        }
        if save {
            self.save_project(false);
        }
        if open {
            self.open_project_dialog();
        }
        if import {
            self.import_dialog();
        }
        if frame_all {
            self.frame_all();
        }
        if frame_sel {
            self.frame_selection();
        }
        if render {
            self.start_render();
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        crate::ui::library::window(self, ctx);
        crate::ui::settings::window(self, ctx);
        crate::ui::splash::show(self, ctx);
        if self.show_import_options {
            self.import_options_window(ctx);
        }
        if let Some(p) = &self.pending {
            egui::Window::new(tr("Operazione in corso"))
                .id(egui::Id::NULL.with("win-operazione-in-corso"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -200.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.strong(&p.label);
                    });
                    ui.label(&p.message);
                    ui.label(egui::RichText::new(tf("{:.0} s", &[format!("{:.0}", p.started.elapsed().as_secs_f32())])).weak());
                    if !self.import_queue.is_empty() {
                        ui.label(tf("altri {} file in coda", &[format!("{}", self.import_queue.len())]));
                    }
                });
        }
        if let Some(msg) = self.error.clone() {
            let mut open = true;
            egui::Window::new(tr("Errore"))
                .id(egui::Id::NULL.with("win-errore"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(&msg).color(ui.visuals().error_fg_color));
                    if ui.button("OK").clicked() {
                        self.error = None;
                    }
                });
            if !open {
                self.error = None;
            }
        }
        if let Some(msg) = self.info.clone() {
            let mut open = true;
            egui::Window::new(tr("Informazioni"))
                .id(egui::Id::NULL.with("win-informazioni"))
                .collapsible(false)
                .open(&mut open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| ui.label(&msg));
                    if ui.button("OK").clicked() {
                        self.info = None;
                    }
                });
            if !open {
                self.info = None;
            }
        }
    }
}
