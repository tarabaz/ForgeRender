//! Scheda di una scena demo, aperta insieme alla scena: che cosa mostra, le funzioni del motore
//! che usa e come rifarla da zero. Si chiude con "Chiudi" o avviando il render.

use crate::app::ForgeApp;
use crate::commands::{self, Cmd};
use crate::demos::{CATEGORIES, DEMOS};
use crate::i18n::tr;
use crate::ui::style;
use eframe::egui;

const WIDTH: f32 = 560.0;

pub fn show(app: &mut ForgeApp, ctx: &egui::Context) {
    // Dopo le finestre dell'avvio, mai insieme.
    if app.splash_open || app.mode_choice_open {
        return;
    }
    let Some(d) = app.demo_info.and_then(|i| DEMOS.get(i)) else {
        return;
    };
    let mut open = true;
    let mut action = None;
    egui::Window::new(tr(d.title))
        .id(egui::Id::NULL.with("scheda-scena-demo"))
        .open(&mut open)
        .collapsible(true)
        .resizable(false)
        .default_width(WIDTH)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(WIDTH);
            ui.label(
                egui::RichText::new(format!("{} · {}", tr("Scena demo"), tr(CATEGORIES[d.category])))
                    .small()
                    .color(style::ACCENT),
            );
            ui.add_space(4.0);
            ui.label(tr(d.summary));
            ui.add_space(8.0);
            ui.strong(tr("Che cosa usa"));
            for f in d.features {
                ui.label(format!("•  {}", tr(f)));
            }
            ui.add_space(8.0);
            ui.strong(tr("Come rifarla"));
            for (k, s) in d.steps.iter().enumerate() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(format!("{}.", k + 1)).strong());
                    ui.label(tr(s));
                });
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(tr("Avvia il render")).clicked() {
                    action = Some(Cmd::RenderToggle);
                }
                if ui.button(tr("Chiudi")).clicked() {
                    app.demo_info = None;
                }
            });
        });
    if !open {
        app.demo_info = None;
    }
    if let Some(cmd) = action {
        app.demo_info = None;
        commands::run(app, ctx, cmd);
    }
}
