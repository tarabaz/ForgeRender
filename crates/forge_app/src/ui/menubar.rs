//! Barra dei menu (prima riga in alto): modalità e lingua sempre visibili, poi i menu File,
//! Modifica, Scena, Oggetti, Vista, Rendering, Libreria, Strumenti, Guida. Le voci vengono
//! dall'elenco unico dei comandi (`crate::commands`).

use crate::app::{ForgeApp, InspectorTab, View};
use crate::commands::{self, Cmd};
use crate::i18n::{tf, tk, tr};
use crate::ui::{icons, style};
use eframe::egui;
use forge_render_core::BackendKind;

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        mode_switch(app, ui);
        language_switch(app, ui);
        ui.separator();
        ui.menu_button(tr("File"), |ui| file_menu(app, ui));
        ui.menu_button(tr("Modifica"), |ui| edit_menu(app, ui));
        ui.menu_button(tr("Scena"), |ui| scene_menu(app, ui));
        ui.menu_button(tr("Oggetti"), |ui| objects_menu(app, ui));
        ui.menu_button(tr("Vista"), |ui| view_menu(app, ui));
        ui.menu_button(tr("Rendering"), |ui| render_menu(app, ui));
        ui.menu_button(tr("Libreria"), |ui| {
            item(app, ui, Cmd::LibraryMine);
            item(app, ui, Cmd::LibraryOnline);
            ui.separator();
            item(app, ui, Cmd::LibraryFolder);
        });
        ui.menu_button(tr("Strumenti"), |ui| {
            let mut on = app.mcp.is_some();
            let sp = commands::spec(Cmd::AiControl);
            if ui
                .checkbox(&mut on, format!("{}  {}", sp.icon, tr(sp.label)))
                .on_hover_text(tr(sp.hint))
                .clicked()
            {
                run(app, ui, Cmd::AiControl);
            }
        });
        ui.menu_button(tr("Guida"), |ui| {
            item(app, ui, Cmd::Manual);
            item(app, ui, Cmd::Shortcuts);
            ui.separator();
            item(app, ui, Cmd::Welcome);
        });
        if app.safe_mode {
            ui.separator();
            ui.colored_label(style::WARN, tr("Modalità sicura"))
                .on_hover_text(tr("Avviato senza accelerazione GPU: viewport 3D disattivata"));
        }
    });
}

/// Voce di menu da un comando: icona, testo, scorciatoia a destra, suggerimento.
pub fn item(app: &mut ForgeApp, ui: &mut egui::Ui, cmd: Cmd) {
    let sp = commands::spec(cmd);
    let label = if cmd == Cmd::RenderToggle {
        commands::render_label(app)
    } else {
        sp.label
    };
    item_with(app, ui, cmd, &format!("{}  {}", sp.icon, tr(label)));
}

fn item_with(app: &mut ForgeApp, ui: &mut egui::Ui, cmd: Cmd, text: &str) {
    let sp = commands::spec(cmd);
    let mut b = egui::Button::new(text.to_string());
    if let Some(s) = sp.shortcut {
        b = b.shortcut_text(ui.ctx().format_shortcut(&s));
    }
    let mut r = ui.add_enabled(commands::enabled(app, cmd), b);
    if !sp.hint.is_empty() {
        r = r.on_hover_text(tr(sp.hint));
    }
    if r.clicked() {
        run(app, ui, cmd);
        ui.close();
    }
}

fn run(app: &mut ForgeApp, ui: &egui::Ui, cmd: Cmd) {
    let ctx = ui.ctx().clone();
    commands::run(app, &ctx, cmd);
}

fn tab_item(app: &mut ForgeApp, ui: &mut egui::Ui, tab: InspectorTab, text: &'static str) {
    let sp = commands::spec(Cmd::Tab(tab));
    item_with(app, ui, Cmd::Tab(tab), &format!("{}  {}", sp.icon, tr(text)));
}

fn file_menu(app: &mut ForgeApp, ui: &mut egui::Ui) {
    item(app, ui, Cmd::NewEmpty);
    item(app, ui, Cmd::NewTest);
    ui.separator();
    item(app, ui, Cmd::Open);
    let recent = app.prefs.recent_projects.clone();
    ui.menu_button(format!("{}  {}", icons::RECENT, tr("Progetti recenti")), |ui| {
        if recent.is_empty() {
            ui.label(egui::RichText::new(tr("Nessun progetto recente")).weak());
        }
        for (i, p) in recent.iter().enumerate() {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if ui.button(name).on_hover_text(p.display().to_string()).clicked() {
                run(app, ui, Cmd::OpenRecent(i));
                ui.close();
            }
        }
    });
    ui.separator();
    item(app, ui, Cmd::Save);
    item(app, ui, Cmd::SaveAs);
    ui.separator();
    item(app, ui, Cmd::Import);
    item(app, ui, Cmd::Export);
    ui.separator();
    item(app, ui, Cmd::Exit);
}

fn edit_menu(app: &mut ForgeApp, ui: &mut egui::Ui) {
    item(app, ui, Cmd::Undo);
    item(app, ui, Cmd::Redo);
    ui.separator();
    item(app, ui, Cmd::Duplicate);
    item(app, ui, Cmd::Delete);
    ui.separator();
    item(app, ui, Cmd::Hide);
    item(app, ui, Cmd::Isolate);
    item(app, ui, Cmd::ShowAll);
    ui.separator();
    item(app, ui, Cmd::Preferences);
    item(app, ui, Cmd::ResetPreferences);
}

fn scene_menu(app: &mut ForgeApp, ui: &mut egui::Ui) {
    item(app, ui, Cmd::PrepareScene);
    item(app, ui, Cmd::FitScene);
    item(app, ui, Cmd::UndoPreset);
    ui.separator();
    ui.menu_button(format!("{}  {}", icons::SCENE, tr("Aspetto")), |ui| {
        let current = crate::presets::matching_look(&app.scene, &app.settings);
        for l in crate::presets::Look::ALL {
            if ui
                .add(egui::Button::selectable(current == Some(l), l.label()))
                .on_hover_text(l.hint())
                .clicked()
            {
                app.simple_apply_look(l);
                ui.close();
            }
        }
    });
    item(app, ui, Cmd::DropToBase);
    item(app, ui, Cmd::FrameProduct);
    ui.separator();
    tab_item(app, ui, InspectorTab::Environment, tk("Ambiente e HDRI…"));
    tab_item(app, ui, InspectorTab::Effects, tk("Effetti e nebbia…"));
}

fn objects_menu(app: &mut ForgeApp, ui: &mut egui::Ui) {
    for (label, kind) in crate::app::ADD_MENU {
        if ui.button(format!("{}  {}", icons::ADD, tr(label))).clicked() {
            app.add_object(*kind);
            ui.close();
        }
    }
    ui.separator();
    ui.menu_button(format!("{}  {}", icons::LIGHT, tr("Luce")), |ui| {
        for k in crate::app::NewLight::ALL {
            if ui.button(tr(k.label())).clicked() {
                run(app, ui, Cmd::AddLight(k));
                ui.close();
            }
        }
    });
    ui.separator();
    tab_item(app, ui, InspectorTab::Object, tk("Posizione e dimensioni…"));
    tab_item(app, ui, InspectorTab::Materials, tk("Materiali…"));
    tab_item(app, ui, InspectorTab::Lights, tk("Luci…"));
}

fn view_menu(app: &mut ForgeApp, ui: &mut egui::Ui) {
    item(app, ui, Cmd::FrameAll);
    item(app, ui, Cmd::FrameSelection);
    ui.separator();
    item(app, ui, Cmd::ToggleOrtho);
    item(app, ui, Cmd::Isometric);
    ui.menu_button(format!("{}  {}", icons::VIEWS, tr("Viste standard")), |ui| {
        for (i, (name, _)) in crate::ui::navigator::STANDARD_VIEWS.iter().enumerate() {
            if ui.button(tr(name)).clicked() {
                run(app, ui, Cmd::StandardView(i));
                ui.close();
            }
        }
    });
    ui.separator();
    for (v, cmd) in [(View::Viewport, Cmd::ShowViewport), (View::Render, Cmd::ShowRender)] {
        let sp = commands::spec(cmd);
        if ui
            .add(egui::Button::selectable(app.view == v, format!("{}  {}", sp.icon, tr(sp.label))))
            .on_hover_text(tr(sp.hint))
            .clicked()
        {
            app.view = v;
            ui.close();
        }
    }
    ui.separator();
    tab_item(app, ui, InspectorTab::Camera, tk("Camera e obiettivo…"));
}

fn render_menu(app: &mut ForgeApp, ui: &mut egui::Ui) {
    item(app, ui, Cmd::RenderToggle);
    item(app, ui, Cmd::Stop);
    item(app, ui, Cmd::Restart);
    ui.separator();
    ui.menu_button(format!("{}  {}", icons::GPU, tr("Dispositivo di calcolo")), |ui| {
        let gpu_ok = app.gpu_ready() != Some(false);
        let mut backend = app.settings.backend;
        ui.add_enabled_ui(gpu_ok, |ui| {
            ui.radio_value(&mut backend, BackendKind::Gpu, tr("Scheda video (GPU)"))
                .on_disabled_hover_text(tr("Nessuna GPU utilizzabile: il render usa il processore"));
        });
        ui.radio_value(&mut backend, BackendKind::Cpu, tr("Processore (CPU)"));
        if backend != app.settings.backend {
            app.settings.backend = backend;
            app.simple_gpu_checked = true;
            ui.close();
        }
    });
    ui.menu_button(format!("{}  {}", icons::QUALITY, tr("Qualità")), |ui| {
        let current = crate::presets::Quality::matching(&app.settings);
        for q in crate::presets::Quality::ALL {
            let (spp, _) = q.values();
            if ui
                .add(egui::Button::selectable(current == Some(q), q.label()))
                .on_hover_text(tf("{} · {spp} campioni", &[q.hint().to_string(), format!("{}", spp)]))
                .clicked()
            {
                app.simple_apply_quality(q);
                ui.close();
            }
        }
    });
    ui.menu_button(format!("{}  {}", icons::IMAGE, tr("Formato")), |ui| {
        for (name, desc, w, h) in crate::presets::FORMATS {
            let sel = app.settings.width == w && app.settings.height == h;
            if ui
                .add(egui::Button::selectable(sel, format!("{}  ({w}×{h})", tr(name))))
                .on_hover_text(tr(desc))
                .clicked()
            {
                app.settings.width = w;
                app.settings.height = h;
                app.frame_product();
                ui.close();
            }
        }
    });
    let mut auto = app.auto_restart;
    let sp = commands::spec(Cmd::AutoRestart);
    if ui.checkbox(&mut auto, tr(sp.label)).on_hover_text(tr(sp.hint)).clicked() {
        run(app, ui, Cmd::AutoRestart);
    }
    ui.separator();
    tab_item(app, ui, InspectorTab::Render, tk("Impostazioni di render e denoiser…"));
    ui.separator();
    item(app, ui, Cmd::Export);
}

/// "Controllo IA": l'utente decide quando un assistente (Claude, tramite `forgerender-mcp`)
/// può comandare questa finestra. "Controllato da IA" compare solo mentre è collegato.
pub fn ai_control(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let on = app.mcp.is_some();
    let hint = if on {
        tr("Acceso: Claude può comandare questa finestra (tramite forgerender-mcp). Clic per spegnere.")
    } else {
        tr("Spento: nessun assistente può comandare ForgeRender. Clic per permettere a Claude di collegarsi.")
    };
    if ui
        .selectable_label(on, format!("{}  {}", icons::AI, tr("Controllo IA")))
        .on_hover_text(hint)
        .clicked()
    {
        run(app, ui, Cmd::AiControl);
    }
    if let Some(s) = app.mcp.as_ref().filter(|s| s.connected()) {
        let last = s.last.clone().unwrap_or_else(|| "—".into());
        ui.label(egui::RichText::new(tr("Controllato da IA")).color(style::ACCENT).strong())
            .on_hover_text(tf("Comandi eseguiti: {}, ultimo: {}", &[s.calls.to_string(), last]));
    }
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
    ui.label(egui::RichText::new(tr("Modalità")).weak());
    ui.selectable_value(&mut mode, UiMode::Simple, egui::RichText::new(tr("Semplice")).strong())
        .on_hover_text(tr("Pannello guidato: prepara la scena, scegli aspetto e qualità, avvia il render"));
    ui.selectable_value(&mut mode, UiMode::Advanced, egui::RichText::new(tr("Avanzata")).strong())
        .on_hover_text(tr("Tutti i controlli: oggetti, materiali, luci, ambiente, effetti, camera, render"));
    if mode != app.prefs.ui_mode {
        let p = crate::prefs::Preferences {
            ui_mode: mode,
            ..app.prefs.clone()
        };
        app.apply_prefs(p, false);
    }
}
