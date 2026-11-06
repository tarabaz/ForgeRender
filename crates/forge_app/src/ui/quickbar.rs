//! Toolbar delle azioni rapide (seconda riga in alto): i comandi più usati, a gruppi. Tutti i
//! controlli passano da `Control`: stessa altezza, stessi bordi, icona e testo sulla stessa riga;
//! il colore distingue l'azione principale, non la misura. A destra: Viewport | Render, Riavvio
//! automatico e, in fondo, AI MCP con il suo ingranaggio. Con la finestra stretta i gruppi di
//! sinistra che non entrano si tolgono (sono anche nei menu), a partire dai comandi dei file:
//! quelli del render restano sempre interi.

use crate::app::{ForgeApp, InspectorTab, View};
use crate::commands::{self, Cmd};
use crate::i18n::{tf, tr};
use crate::ui::{icons, menubar, style};
use eframe::egui;

/// Altezza di tutti i controlli della barra.
const HEIGHT: f32 = 28.0;
const RADIUS: f32 = 4.0;
const ICON_SIZE: f32 = 15.0;
const TEXT_SIZE: f32 = 12.5;
/// Margine interno ai lati e spazio tra icona e testo.
const PAD_X: f32 = 8.0;
const ICON_GAP: f32 = 5.0;
/// Spazio occupato da un separatore tra due gruppi.
const SEPARATOR: f32 = 16.0;

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    // Gruppo di destra misurato al fotogramma prima: il gruppo di sinistra prende solo lo spazio
    // che resta e, se la finestra è stretta, si taglia invece di finirci sotto.
    let right_id = ui.make_persistent_id("barra-rapida-destra");
    let right_w: f32 = ui.ctx().data(|d| d.get_temp(right_id)).unwrap_or(360.0);
    ui.horizontal(|ui| {
        ui.set_min_height(HEIGHT);
        let left_w = (ui.available_width() - right_w - 12.0).max(0.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(left_w, HEIGHT), egui::Sense::hover());
        let mut left = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        left.set_clip_rect(rect.intersect(ui.clip_rect()));
        left_group(app, &mut left, left_w);
        let used = ui
            .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let start = ui.cursor().max.x;
                // Da destra: AI MCP in fondo, poi Riavvio automatico, poi Viewport | Render.
                ai_block(app, ui);
                menubar::ai_controlled_label(app, ui);
                ui.separator();
                let mut auto = app.auto_restart;
                let sp = commands::spec(Cmd::AutoRestart);
                let label = tr(sp.label);
                let w = text_width(ui, label) + ui.spacing().icon_width + ui.spacing().icon_spacing + PAD_X;
                let check = egui::Checkbox::new(&mut auto, label);
                if ui.add_sized([w, HEIGHT], check).on_hover_text(tr(sp.hint)).clicked() {
                    run(app, ui, Cmd::AutoRestart);
                }
                ui.separator();
                // Da destra a sinistra: prima Render, poi Viewport.
                for (v, cmd) in [(View::Render, Cmd::ShowRender), (View::Viewport, Cmd::ShowViewport)] {
                    let sp = commands::spec(cmd);
                    let look = if app.view == v { Look::Selected } else { Look::Plain };
                    if tool_button(ui, "", tr(sp.label), true, look).on_hover_text(tr(sp.hint)).clicked() {
                        app.view = v;
                    }
                }
                start - ui.min_rect().min.x
            })
            .inner;
        if (used - right_w).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(right_id, used));
            ui.ctx().request_repaint();
        }
    });
}

/// Comandi a sinistra: file, aggiunte, materiale, render.
fn left_group(app: &mut ForgeApp, ui: &mut egui::Ui, width: f32) {
    const FILES: [Cmd; 4] = [Cmd::NewEmpty, Cmd::Open, Cmd::Save, Cmd::Import];
    // Larghezze misurate sui testi veri: valgono per ogni lingua e ogni scala.
    let gap = ui.spacing().item_spacing.x;
    let sum = |ui: &egui::Ui, items: &[(&str, &str)]| -> f32 { items.iter().map(|(i, l)| control_width(ui, i, l) + gap).sum() };
    let file_items: Vec<_> = FILES.iter().map(|c| (commands::spec(*c).icon, short_label(*c))).collect();
    let files = sum(ui, &file_items) + SEPARATOR;
    let add_labels = add_items();
    let add_refs: Vec<(&str, &str)> = add_labels.iter().map(|(i, l)| (*i, l.as_str())).collect();
    let add = sum(ui, &add_refs) + SEPARATOR;
    let render = sum(
        ui,
        &[
            (icons::RENDER, tr(commands::render_label(app))),
            (commands::spec(Cmd::Stop).icon, tr("Stop")),
            (commands::spec(Cmd::Restart).icon, tr("Riavvia")),
            (commands::spec(Cmd::Export).icon, tr("Esporta")),
        ],
    );
    if width >= files + add + render {
        for cmd in FILES {
            cmd_button(app, ui, cmd, short_label(cmd));
        }
        ui.separator();
    }
    if width >= add + render {
        add_group(app, ui);
        ui.separator();
    }
    render_button(app, ui);
    cmd_button(app, ui, Cmd::Stop, tr("Stop"));
    cmd_button(app, ui, Cmd::Restart, tr("Riavvia"));
    cmd_button(app, ui, Cmd::Export, tr("Esporta"));
}

/// Icone e testi del gruppo delle aggiunte (quelli a tendina hanno anche la freccia).
fn add_items() -> [(&'static str, String); 5] {
    let menu = |label: &'static str| format!("{} \u{E136}", tr(label));
    [
        (icons::ADD, menu("Aggiungi")),
        (icons::LIGHT, menu("Luci")),
        (icons::CAMERA, menu("Camera")),
        (icons::ENVIRONMENT, menu("Ambiente")),
        (icons::MATERIAL, tr("Materiale").to_string()),
    ]
}

/// Aggiunte alla scena e materiale.
fn add_group(app: &mut ForgeApp, ui: &mut egui::Ui) {
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
    if tool_button(ui, icons::MATERIAL, tr("Materiale"), true, Look::Plain)
        .on_hover_text(tr("Apri la libreria materiali"))
        .clicked()
    {
        run(app, ui, Cmd::LibraryMine);
    }
}

/// AI MCP in fondo a destra: interruttore con lo stato e ingranaggio delle impostazioni, alti come
/// gli altri controlli. Il layout va da destra a sinistra: si parte dall'ingranaggio.
fn ai_block(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let on = app.mcp.is_some();
    let connected = app.mcp.as_ref().is_some_and(|s| s.connected());
    // Ingranaggio: collegare o scollegare gli assistenti (Claude Desktop, Claude Code, Codex).
    let g = tool_button(ui, icons::SETTINGS, "", true, Look::Plain).on_hover_text(tr(
        "Impostazioni dell'integrazione AI: collega o scollega Claude Desktop, Claude Code e Codex",
    ));
    if g.clicked() {
        run(app, ui, Cmd::AiSetup);
    }
    let hint = if on {
        tr("Acceso: Claude può comandare questa finestra (tramite forgerender-mcp). Clic per spegnere.")
    } else {
        tr("Spento: nessun assistente può comandare ForgeRender. Clic per permettere a Claude di collegarsi.")
    };
    let mut hint = hint.to_string();
    if let Some(s) = app.mcp.as_ref().filter(|s| s.connected()) {
        hint.push_str(&format!(
            "\n{}",
            tf(
                "Comandi eseguiti: {}, ultimo: {}",
                &[s.calls.to_string(), s.last.clone().unwrap_or_else(|| "—".into())]
            )
        ));
    }
    let label = format!("AI MCP {}", if on { "ON" } else { "OFF" });
    let look = if on { Look::Active } else { Look::Off };
    let r = tool_button(ui, icons::AI, &label, true, look).on_hover_text(hint);
    // Collegato: bordo arancione (un assistente sta comandando).
    if connected {
        ui.painter()
            .rect_stroke(r.rect, RADIUS, egui::Stroke::new(1.5, style::ACCENT), egui::StrokeKind::Inside);
    }
    if r.clicked() {
        run(app, ui, Cmd::AiControl);
    }
}

/// Testi brevi nei pulsanti (il testo intero è nel suggerimento).
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
    let r = tool_button(ui, sp.icon, label, enabled, Look::Plain);
    let r = r.on_hover_text(hint(ui, cmd));
    if r.clicked() {
        run(app, ui, cmd);
    }
}

/// Avvia render: l'azione principale (arancione, alta come gli altri), il testo segue lo stato.
fn render_button(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let active = app.render_state() == forge_render_core::SessionState::Rendering;
    let icon = if active { icons::PAUSE } else { icons::RENDER };
    let enabled = commands::enabled(app, Cmd::RenderToggle);
    let r = tool_button(ui, icon, tr(commands::render_label(app)), enabled, Look::Primary);
    let r = r.on_hover_text(hint(ui, Cmd::RenderToggle));
    if r.clicked() {
        run(app, ui, Cmd::RenderToggle);
    }
}

/// Pulsante con menu a tendina (icona, testo e ▾).
fn dropdown(ui: &mut egui::Ui, icon: &str, label: &str, content: impl FnOnce(&mut egui::Ui)) {
    let r = tool_button(ui, icon, &format!("{label} \u{E136}"), true, Look::Plain);
    egui::Popup::menu(&r).show(content);
}

/// Aspetto di un controllo: cambia il colore, mai la misura.
#[derive(Clone, Copy, PartialEq)]
enum Look {
    Plain,
    /// Azione principale (solo "Avvia render").
    Primary,
    /// Scheda scelta.
    Selected,
    /// Acceso (verde).
    Active,
    /// Spento (testo attenuato).
    Off,
}

fn icon_font() -> egui::FontId {
    egui::FontId::proportional(ICON_SIZE)
}

fn text_font() -> egui::FontId {
    egui::FontId::proportional(TEXT_SIZE)
}

fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    let color = ui.visuals().text_color();
    ui.painter().layout_no_wrap(text.to_string(), text_font(), color).size().x
}

/// Larghezza di un controllo con questa icona e questo testo.
fn control_width(ui: &egui::Ui, icon: &str, label: &str) -> f32 {
    let color = ui.visuals().text_color();
    let icon_w = ui.painter().layout_no_wrap(icon.to_string(), icon_font(), color).size().x;
    let gap = if icon.is_empty() || label.is_empty() { 0.0 } else { ICON_GAP };
    (icon_w + gap + text_width(ui, label) + PAD_X * 2.0).max(HEIGHT)
}

/// Il controllo della barra: unico punto che decide misure, bordi e allineamento.
fn tool_button(ui: &mut egui::Ui, icon: &str, label: &str, enabled: bool, look: Look) -> egui::Response {
    let w = control_width(ui, icon, label);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, HEIGHT), egui::Sense::click());
    let resp = if enabled {
        resp
    } else {
        resp.on_hover_cursor(egui::CursorIcon::NotAllowed)
    };
    if ui.is_rect_visible(rect) {
        let vis = ui.style().interact(&resp);
        let hot = enabled && (resp.hovered() || resp.is_pointer_button_down_on());
        let visuals = ui.visuals();
        let plain = if hot { vis.bg_fill } else { visuals.widgets.inactive.weak_bg_fill };
        let bg = match look {
            Look::Primary if enabled => style::ACCENT.gamma_multiply(if hot { 1.0 } else { 0.85 }),
            Look::Selected => visuals.selection.bg_fill,
            Look::Active if !hot => style::OK.gamma_multiply(0.18),
            _ => plain,
        };
        ui.painter().rect_filled(rect, RADIUS, bg);
        let color = match look {
            _ if !enabled => visuals.weak_text_color(),
            Look::Primary | Look::Selected => egui::Color32::WHITE,
            Look::Active => style::OK,
            Look::Off => visuals.weak_text_color(),
            Look::Plain => vis.text_color(),
        };
        let icon_g = ui.painter().layout_no_wrap(icon.to_string(), icon_font(), color);
        let text_g = ui.painter().layout_no_wrap(label.to_string(), text_font(), color);
        let gap = if icon.is_empty() || label.is_empty() { 0.0 } else { ICON_GAP };
        let mut x = rect.center().x - (icon_g.size().x + gap + text_g.size().x) / 2.0;
        // Icona e testo centrati in altezza, sulla stessa linea in tutti i controlli.
        let icon_w = icon_g.size().x;
        ui.painter()
            .galley(egui::pos2(x, rect.center().y - icon_g.size().y / 2.0), icon_g, color);
        x += icon_w + gap;
        ui.painter()
            .galley(egui::pos2(x, rect.center().y - text_g.size().y / 2.0), text_g, color);
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
