use crate::app::{ForgeApp, View};
use eframe::egui;
use forge_render_core::SessionState;

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        mode_switch(app, ui);
        ui.separator();
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
            if ui.button("Importa modello…  (Ctrl+I)").on_hover_text(IMPORT_HINT).clicked() {
                app.import_dialog();
                ui.close();
            }
            ui.separator();
            if ui.button("Esporta immagine PNG…").clicked() {
                app.export_png_dialog();
                ui.close();
            }
            ui.separator();
            if ui.button("Impostazioni…").clicked() {
                app.prefs_open = true;
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

        ui.menu_button("Aggiungi", |ui| {
            for (label, kind) in crate::app::ADD_MENU {
                if ui.button(*label).clicked() {
                    app.add_object(*kind);
                    ui.close();
                }
            }
        });

        ui.separator();
        if ui.button("Importa").on_hover_text(IMPORT_HINT).clicked() {
            app.import_dialog();
        }
        if ui.button("Salva").on_hover_text("Salva progetto (Ctrl+S)").clicked() {
            app.save_project(false);
        }
        if ui
            .selectable_label(app.library.open, "Libreria")
            .on_hover_text("Materiali con texture e HDRI: la tua libreria e i download da ambientCG / Poly Haven")
            .clicked()
        {
            app.library.open = !app.library.open;
        }
        if ui
            .selectable_label(app.prefs_open, "Impostazioni")
            .on_hover_text("Libreria e import, percorsi, valori dei nuovi progetti")
            .clicked()
        {
            app.prefs_open = !app.prefs_open;
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
        if ui
            .checkbox(&mut app.auto_restart, "Riavvio automatico")
            .on_hover_text("Quando cambi materiali, luci o camera il rendering riparte senza riutilizzare campioni incompatibili")
            .changed()
        {
            let p = crate::prefs::Preferences {
                auto_restart: app.auto_restart,
                ..app.prefs.clone()
            };
            app.apply_prefs(p, false);
        }
    });
}

/// Formati che l'import accetta davvero (vedi `ForgeApp::import_dialog`).
pub const IMPORT_HINT: &str = "Importa un modello (Ctrl+I): glTF/GLB, OBJ, STL, Blender .blend (con Blender installato), \
     STEP (con il convertitore forgerender-step), SolidWorks SLDPRT/SLDASM (su Windows, con SolidWorks installato)";

/// Selettore Semplice | Avanzata: cambia solo il pannello di destra, i valori restano gli stessi.
fn mode_switch(app: &mut ForgeApp, ui: &mut egui::Ui) {
    use crate::prefs::UiMode;
    let mut mode = app.prefs.ui_mode;
    let big = |t: &str| egui::RichText::new(t).strong().size(15.0);
    ui.label(egui::RichText::new("Modalità").weak());
    ui.selectable_value(&mut mode, UiMode::Simple, big("Semplice"))
        .on_hover_text("Pannello guidato: prepara la scena, scegli aspetto e qualità, avvia il render");
    ui.selectable_value(&mut mode, UiMode::Advanced, big("Avanzata"))
        .on_hover_text("Tutti i controlli: oggetti, materiali, luci, ambiente, effetti, camera, render");
    if mode != app.prefs.ui_mode {
        let p = crate::prefs::Preferences {
            ui_mode: mode,
            ..app.prefs.clone()
        };
        app.apply_prefs(p, false);
    }
}
