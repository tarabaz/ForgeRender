use crate::app::{ForgeApp, View};
use crate::i18n::{tf, tk, tr};
use crate::ui::style;
use eframe::egui;
use forge_render_core::SessionState;

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        mode_switch(app, ui);
        ui.separator();
        language_switch(app, ui);
        ui.separator();
        ai_control(app, ui);
        ui.menu_button(tr("File"), |ui| {
            if ui.button(tr("Nuova scena vuota")).clicked() {
                app.new_empty_scene();
                ui.close();
            }
            if ui.button(tr("Nuova scena di test")).clicked() {
                app.new_test_scene();
                ui.close();
            }
            ui.separator();
            if ui.button(tr("Apri progetto…  (Ctrl+O)")).clicked() {
                app.open_project_dialog();
                ui.close();
            }
            if ui.button(tr("Salva progetto  (Ctrl+S)")).clicked() {
                app.save_project(false);
                ui.close();
            }
            if ui.button(tr("Salva progetto con nome…")).clicked() {
                app.save_project(true);
                ui.close();
            }
            ui.separator();
            if ui.button(tr("Importa modello…  (Ctrl+I)")).on_hover_text(tr(IMPORT_HINT)).clicked() {
                app.import_dialog();
                ui.close();
            }
            ui.separator();
            if ui.button(tr("Esporta immagine PNG…")).clicked() {
                app.export_png_dialog();
                ui.close();
            }
            ui.separator();
            if ui.button(tr("Impostazioni…")).clicked() {
                app.prefs_open = true;
                ui.close();
            }
            if ui.button(tr("Benvenuto e informazioni…")).clicked() {
                app.splash_open = true;
                ui.close();
            }
        });
        ui.menu_button(tr("Vista"), |ui| {
            if ui.button(tr("Inquadra tutto  (Home)")).clicked() {
                app.frame_all();
                ui.close();
            }
            if ui.button(tr("Inquadra selezione  (F)")).clicked() {
                app.frame_selection();
                ui.close();
            }
        });

        ui.menu_button(tr("Aggiungi"), |ui| {
            for (label, kind) in crate::app::ADD_MENU {
                if ui.button(tr(label)).clicked() {
                    app.add_object(*kind);
                    ui.close();
                }
            }
        });

        ui.separator();
        if ui.button(tr("Importa")).on_hover_text(tr(IMPORT_HINT)).clicked() {
            app.import_dialog();
        }
        if ui.button(tr("Salva")).on_hover_text(tr("Salva progetto (Ctrl+S)")).clicked() {
            app.save_project(false);
        }
        if ui
            .selectable_label(app.library.open, tr("Libreria"))
            .on_hover_text(tr(
                "Materiali con texture e HDRI: la tua libreria e i download da ambientCG / Poly Haven",
            ))
            .clicked()
        {
            app.library.open = !app.library.open;
        }
        if ui
            .selectable_label(app.prefs_open, tr("Impostazioni"))
            .on_hover_text(tr("Libreria e import, percorsi, valori dei nuovi progetti"))
            .clicked()
        {
            app.prefs_open = !app.prefs_open;
        }
        ui.separator();

        let state = app.render_state();
        let preparing = app.is_preparing();
        let (label, tip) = match state {
            SessionState::Rendering => (tr("Pausa"), tr("Mette in pausa l'accumulo dei campioni")),
            SessionState::Paused => (tr("Riprendi"), tr("Riprende ad accumulare campioni")),
            SessionState::Cancelled if !app.render_is_stale() => (tr("Riprendi"), tr("Riprende il rendering interrotto")),
            _ => (tr("Avvia render"), tr("Avvia il path tracing (F5)")),
        };
        if ui
            .add_enabled(!preparing, egui::Button::new(egui::RichText::new(label).strong()))
            .on_hover_text(tip)
            .clicked()
        {
            app.toggle_pause();
        }
        if ui
            .add_enabled(state.is_active(), egui::Button::new(tr("Stop")))
            .on_hover_text(tr("Ferma il rendering mantenendo l'immagine corrente"))
            .clicked()
        {
            app.stop_render();
        }
        if ui.button(tr("Riavvia")).on_hover_text(tr("Riparte da zero campioni")).clicked() {
            app.start_render();
        }
        if ui.button(tr("Esporta PNG")).clicked() {
            app.export_png_dialog();
        }
        ui.separator();
        ui.selectable_value(&mut app.view, View::Viewport, tr("Viewport"));
        ui.selectable_value(&mut app.view, View::Render, tr("Render"));
        ui.separator();
        if app.safe_mode {
            ui.colored_label(crate::ui::style::WARN, tr("Modalità sicura"))
                .on_hover_text(tr("Avviato senza accelerazione GPU: viewport 3D disattivata"));
        }
        if ui
            .checkbox(&mut app.auto_restart, tr("Riavvio automatico"))
            .on_hover_text(tr(
                "Quando cambi materiali, luci o camera il rendering riparte senza riutilizzare campioni incompatibili",
            ))
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
pub const IMPORT_HINT: &str = tk(
    "Importa un modello (Ctrl+I): glTF/GLB, OBJ, STL, Blender .blend (con Blender installato), \
     STEP (con il convertitore forgerender-step), SolidWorks SLDPRT/SLDASM (su Windows, con SolidWorks installato)",
);

/// "Controllo IA": l'utente decide quando un assistente (Claude, tramite `forgerender-mcp`)
/// può comandare questa finestra. "Controllato da IA" compare solo mentre è collegato.
fn ai_control(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let on = app.mcp.is_some();
    let hint = if on {
        tr("Acceso: Claude può comandare questa finestra (tramite forgerender-mcp). Clic per spegnere.")
    } else {
        tr("Spento: nessun assistente può comandare ForgeRender. Clic per permettere a Claude di collegarsi.")
    };
    if ui.selectable_label(on, tr("Controllo IA")).on_hover_text(hint).clicked() {
        let ctx = ui.ctx().clone();
        crate::mcp::set_enabled(app, &ctx, !on);
    }
    if let Some(s) = app.mcp.as_ref().filter(|s| s.connected()) {
        let last = s.last.clone().unwrap_or_else(|| "—".into());
        ui.label(egui::RichText::new(tr("Controllato da IA")).color(style::ACCENT).strong())
            .on_hover_text(tf("Comandi eseguiti: {}, ultimo: {}", &[s.calls.to_string(), last]));
    }
    ui.separator();
}

/// Lingua dell'interfaccia: cambia subito tutte le scritte e resta nelle Impostazioni.
pub fn language_switch(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let current = app
        .languages
        .iter()
        .find(|l| l.code == app.prefs.language)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| "Italiano".into());
    let mut code = app.prefs.language.clone();
    egui::ComboBox::from_id_salt("ui-language")
        .selected_text(current)
        .width(90.0)
        .show_ui(ui, |ui| {
            for l in &app.languages {
                ui.selectable_value(&mut code, l.code.clone(), &l.name);
            }
        })
        .response
        .on_hover_text(tr("Lingua dell'interfaccia (file nella cartella lang accanto al programma)"));
    if code != app.prefs.language {
        let p = crate::prefs::Preferences {
            language: code,
            ..app.prefs.clone()
        };
        app.apply_prefs(p, false);
    }
}

/// Selettore Semplice | Avanzata: cambia solo il pannello di destra, i valori restano gli stessi.
fn mode_switch(app: &mut ForgeApp, ui: &mut egui::Ui) {
    use crate::prefs::UiMode;
    let mut mode = app.prefs.ui_mode;
    let big = |t: &str| egui::RichText::new(t).strong().size(15.0);
    ui.label(egui::RichText::new(tr("Modalità")).weak());
    ui.selectable_value(&mut mode, UiMode::Simple, big(tr("Semplice")))
        .on_hover_text(tr("Pannello guidato: prepara la scena, scegli aspetto e qualità, avvia il render"));
    ui.selectable_value(&mut mode, UiMode::Advanced, big(tr("Avanzata")))
        .on_hover_text(tr("Tutti i controlli: oggetti, materiali, luci, ambiente, effetti, camera, render"));
    if mode != app.prefs.ui_mode {
        let p = crate::prefs::Preferences {
            ui_mode: mode,
            ..app.prefs.clone()
        };
        app.apply_prefs(p, false);
    }
}
