//! Interfaccia utente (egui). Layout:
//! barra superiore (comandi), pannello sinistro (albero della scena),
//! pannello destro (proprietà), pannello inferiore (progressi, statistiche,
//! log), area centrale (viewport o render).

pub mod central;
mod inspector;
mod status;
pub mod style;
mod toolbar;
pub mod tree;

use crate::app::ForgeApp;
use eframe::egui;

impl eframe::App for ForgeApp {
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
        egui::Panel::left("tree")
            .resizable(true)
            .default_size(270.0)
            .min_size(180.0)
            .show(ui, |ui| tree::show(self, ui));
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(340.0)
            .min_size(260.0)
            .show(ui, |ui| inspector::show(self, ui));
        egui::CentralPanel::default().show(ui, |ui| central::show(self, ui, frame));

        self.dialogs(&ctx);
    }
}

impl ForgeApp {
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let files: Vec<_> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        for f in files {
            self.open_any(&f);
        }
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
        if let Some(msg) = self.error.clone() {
            let mut open = true;
            egui::Window::new("Errore")
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
            egui::Window::new("Informazioni")
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
