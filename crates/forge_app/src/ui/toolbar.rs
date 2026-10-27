use crate::app::{ForgeApp, View};
use eframe::egui;
use forge_render_core::SessionState;

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Nuova scena vuota").clicked() {
                app.new_empty_scene();
                ui.close();
            }
            if ui.button("Nuova scena di test").clicked() {
                app.new_test_scene();
                ui.close();
            }
            ui.separator();
            if ui.button("Apri progetto…  (Ctrl+O)").clicked() {
                app.open_project_dialog();
                ui.close();
            }
            if ui.button("Salva progetto  (Ctrl+S)").clicked() {
                app.save_project(false);
                ui.close();
            }
            if ui.button("Salva progetto con nome…").clicked() {
                app.save_project(true);
                ui.close();
            }
            ui.separator();
            if ui.button("Importa mesh OBJ/STL…  (Ctrl+I)").clicked() {
                app.import_dialog();
                ui.close();
            }
            ui.add_enabled(false, egui::Button::new("Importa STEP…"))
                .on_disabled_hover_text("Non ancora disponibile: importazione STEP tramite Open CASCADE (Fase 5)");
            ui.add_enabled(false, egui::Button::new("Importa da SolidWorks (SLDPRT/SLDASM)…"))
                .on_disabled_hover_text("Non ancora disponibile: bridge con le API di SolidWorks (Fase 5)");
            ui.separator();
            if ui.button("Esporta immagine PNG…").clicked() {
                app.export_png_dialog();
                ui.close();
            }
        });
        ui.menu_button("Vista", |ui| {
            if ui.button("Inquadra tutto  (Home)").clicked() {
                app.frame_all();
                ui.close();
            }
            if ui.button("Inquadra selezione  (F)").clicked() {
                app.frame_selection();
                ui.close();
            }
        });

        ui.separator();
        if ui.button("Importa").on_hover_text("Importa mesh OBJ/STL (Ctrl+I)").clicked() {
            app.import_dialog();
        }
        if ui.button("Salva").on_hover_text("Salva progetto (Ctrl+S)").clicked() {
            app.save_project(false);
        }
        ui.separator();

        let state = app.render_state();
        let preparing = app.is_preparing();
        let (label, tip) = match state {
            SessionState::Rendering => ("Pausa", "Mette in pausa l'accumulo dei campioni"),
            SessionState::Paused => ("Riprendi", "Riprende ad accumulare campioni"),
            SessionState::Cancelled if !app.render_is_stale() => ("Riprendi", "Riprende il rendering interrotto"),
            _ => ("Avvia render", "Avvia il path tracing (F5)"),
        };
        if ui
            .add_enabled(!preparing, egui::Button::new(egui::RichText::new(label).strong()))
            .on_hover_text(tip)
            .clicked()
        {
            app.toggle_pause();
        }
        if ui
            .add_enabled(state.is_active(), egui::Button::new("Stop"))
            .on_hover_text("Ferma il rendering mantenendo l'immagine corrente")
            .clicked()
        {
            app.stop_render();
        }
        if ui.button("Riavvia").on_hover_text("Riparte da zero campioni").clicked() {
            app.start_render();
        }
        if ui.button("Esporta PNG").clicked() {
            app.export_png_dialog();
        }
        ui.separator();
        ui.selectable_value(&mut app.view, View::Viewport, "Viewport");
        ui.selectable_value(&mut app.view, View::Render, "Render");
        ui.separator();
        if app.safe_mode {
            ui.colored_label(crate::ui::style::WARN, "Modalità sicura")
                .on_hover_text("Avviato senza accelerazione GPU: viewport 3D disattivata");
        }
        ui.checkbox(&mut app.auto_restart, "Riavvio automatico")
            .on_hover_text("Quando cambi materiali, luci o camera il rendering riparte senza riutilizzare campioni incompatibili");
    });
}
