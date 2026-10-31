//! Toolbar delle azioni rapide (seconda riga in alto): i comandi più usati, icona sopra e
//! testo sotto, a gruppi. Tutto il resto sta nei menu. A destra: Viewport | Render, Controllo
//! IA e Riavvio automatico.

use crate::app::{ForgeApp, InspectorTab, View};
use crate::commands::{self, Cmd};
use crate::i18n::tr;
use crate::ui::{icons, menubar, style};
use eframe::egui;

const HEIGHT: f32 = 46.0;

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.set_min_height(HEIGHT);
        for cmd in [Cmd::NewEmpty, Cmd::Open, Cmd::Save, Cmd::Import] {
            cmd_button(app, ui, cmd, short_label(cmd));
        }
        ui.separator();
        cmd_button(app, ui, Cmd::LibraryMine, tr("Libreria"));
        dropdown(ui, icons::ADD, tr("Aggiungi"), |ui| {
            for (label, kind) in crate::app::ADD_MENU {
                if ui.button(tr(label)).clicked() {
                    app.add_object(*kind);
                    ui.close();
                }
            }
        });
        dropdown(ui, icons::LIGHT, tr("Luci"), |ui| {
            for k in crate::app::NewLight::ALL {
                if ui.button(tr(k.label())).clicked() {
                    run(app, ui, Cmd::AddLight(k));
                    ui.close();
                }
            }
            ui.separator();
            if ui.button(tr("Pannello Luci…")).clicked() {
                run(app, ui, Cmd::Tab(InspectorTab::Lights));
                ui.close();
            }
        });
        dropdown(ui, icons::CAMERA, tr("Camera"), |ui| {
            for cmd in [Cmd::FrameAll, Cmd::FrameSelection, Cmd::ToggleOrtho, Cmd::Isometric] {
                menubar::item(app, ui, cmd);
            }
            ui.separator();
            for (i, (name, _)) in crate::ui::navigator::STANDARD_VIEWS.iter().enumerate() {
                if ui.button(tr(name)).clicked() {
                    run(app, ui, Cmd::StandardView(i));
                    ui.close();
                }
            }
            ui.separator();
            if ui.button(tr("Camera e obiettivo…")).clicked() {
                run(app, ui, Cmd::Tab(InspectorTab::Camera));
                ui.close();
            }
        });
        dropdown(ui, icons::ENVIRONMENT, tr("Ambiente"), |ui| {
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
            ui.separator();
            if ui.button(tr("Ambiente e HDRI…")).clicked() {
                run(app, ui, Cmd::Tab(InspectorTab::Environment));
                ui.close();
            }
        });
        // Materiale: un clic e si apre la libreria (niente menu).
        if tool_button(ui, icons::MATERIAL, tr("Materiale"), true, None)
            .on_hover_text(tr("Apri la libreria materiali"))
            .clicked()
        {
            run(app, ui, Cmd::LibraryMine);
        }
        ui.separator();
        render_button(app, ui);
        cmd_button(app, ui, Cmd::Stop, tr("Stop"));
        cmd_button(app, ui, Cmd::Restart, tr("Riavvia"));
        cmd_button(app, ui, Cmd::Export, tr("Esporta"));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut auto = app.auto_restart;
            let sp = commands::spec(Cmd::AutoRestart);
            if ui.checkbox(&mut auto, tr(sp.label)).on_hover_text(tr(sp.hint)).clicked() {
                run(app, ui, Cmd::AutoRestart);
            }
            ui.separator();
            // Da destra a sinistra: prima Render, poi Viewport.
            for (v, cmd) in [(View::Render, Cmd::ShowRender), (View::Viewport, Cmd::ShowViewport)] {
                let sp = commands::spec(cmd);
                if ui
                    .selectable_label(app.view == v, tr(sp.label))
                    .on_hover_text(tr(sp.hint))
                    .clicked()
                {
                    app.view = v;
                }
            }
            ui.separator();
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| menubar::ai_control(app, ui));
        });
    });
}

/// Testi brevi sotto le icone (il testo intero è nel suggerimento).
fn short_label(cmd: Cmd) -> &'static str {
    match cmd {
        Cmd::NewEmpty => tr("Nuovo"),
        Cmd::Open => tr("Apri"),
        Cmd::Save => tr("Salva"),
        Cmd::Import => tr("Importa"),
        _ => tr(commands::spec(cmd).label),
    }
}

fn run(app: &mut ForgeApp, ui: &egui::Ui, cmd: Cmd) {
    let ctx = ui.ctx().clone();
    commands::run(app, &ctx, cmd);
}

/// Suggerimento: testo intero del comando, scorciatoia e spiegazione.
fn hint(ui: &egui::Ui, cmd: Cmd) -> String {
    let sp = commands::spec(cmd);
    let mut s = tr(sp.label).to_string();
    if let Some(k) = sp.shortcut {
        s.push_str(&format!("  ({})", ui.ctx().format_shortcut(&k)));
    }
    if !sp.hint.is_empty() {
        s.push('\n');
        s.push_str(tr(sp.hint));
    }
    s
}

fn cmd_button(app: &mut ForgeApp, ui: &mut egui::Ui, cmd: Cmd, label: &str) {
    let sp = commands::spec(cmd);
    let enabled = commands::enabled(app, cmd);
    let r = tool_button(ui, sp.icon, label, enabled, None);
    let r = r.on_hover_text(hint(ui, cmd));
    if r.clicked() {
        run(app, ui, cmd);
    }
}

/// Avvia render: il pulsante più evidente (arancione), il testo segue lo stato.
fn render_button(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let active = app.render_state() == forge_render_core::SessionState::Rendering;
    let icon = if active { icons::PAUSE } else { icons::RENDER };
    let enabled = commands::enabled(app, Cmd::RenderToggle);
    let r = tool_button(ui, icon, tr(commands::render_label(app)), enabled, Some(style::ACCENT));
    let r = r.on_hover_text(hint(ui, Cmd::RenderToggle));
    if r.clicked() {
        run(app, ui, Cmd::RenderToggle);
    }
}

/// Pulsante con menu a tendina (icona, testo e ▾).
fn dropdown(ui: &mut egui::Ui, icon: &str, label: &str, content: impl FnOnce(&mut egui::Ui)) {
    let r = tool_button(ui, icon, &format!("{label} \u{E136}"), true, None);
    egui::Popup::menu(&r).show(content);
}

/// Pulsante della toolbar: icona grande sopra, testo piccolo sotto. `fill` = pulsante pieno
/// (solo "Avvia render").
fn tool_button(ui: &mut egui::Ui, icon: &str, label: &str, enabled: bool, fill: Option<egui::Color32>) -> egui::Response {
    let icon_font = egui::FontId::proportional(19.0);
    let text_font = egui::FontId::proportional(12.0);
    let text_color = ui.visuals().text_color();
    let icon_g = ui.painter().layout_no_wrap(icon.to_string(), icon_font, text_color);
    let text_g = ui.painter().layout_no_wrap(label.to_string(), text_font, text_color);
    let w = text_g.size().x.max(icon_g.size().x).max(if fill.is_some() { 92.0 } else { 40.0 }) + 14.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, HEIGHT), egui::Sense::click());
    let resp = if enabled {
        resp
    } else {
        resp.on_hover_cursor(egui::CursorIcon::NotAllowed)
    };
    if ui.is_rect_visible(rect) {
        let vis = ui.style().interact(&resp);
        let bg = match fill {
            Some(c) if enabled => {
                if resp.hovered() {
                    c
                } else {
                    c.gamma_multiply(0.85)
                }
            }
            _ if enabled && (resp.hovered() || resp.is_pointer_button_down_on()) => vis.bg_fill,
            _ => egui::Color32::TRANSPARENT,
        };
        ui.painter().rect_filled(rect, 4.0, bg);
        let color = match (enabled, fill.is_some()) {
            (false, _) => ui.visuals().weak_text_color(),
            (true, true) => egui::Color32::WHITE,
            (true, false) => vis.text_color(),
        };
        let gap = 3.0;
        let total = icon_g.size().y + gap + text_g.size().y;
        let top = rect.center().y - total / 2.0;
        ui.painter()
            .galley_with_override_text_color(egui::pos2(rect.center().x - icon_g.size().x / 2.0, top), icon_g.clone(), color);
        ui.painter().galley_with_override_text_color(
            egui::pos2(rect.center().x - text_g.size().x / 2.0, top + icon_g.size().y + gap),
            text_g,
            color,
        );
    }
    if enabled {
        resp
    } else {
        // Un pulsante spento non deve reagire al clic.
        let mut r = resp;
        r.flags.remove(egui::response::Flags::CLICKED);
        r
    }
}
