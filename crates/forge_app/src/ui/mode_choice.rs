//! Scelta della modalità all'avvio (dopo la schermata di benvenuto): Semplice o Avanzata, con due
//! righe su cosa offre ognuna e dove si cambia dopo. Torna a ogni avvio finché non si spunta
//! "Non mostrare più" (si riattiva in Impostazioni → Informazioni).

use crate::app::ForgeApp;
use crate::i18n::tr;
use crate::prefs::UiMode;
use crate::ui::style;
use eframe::egui;

const WIDTH: f32 = 560.0;

pub fn show(app: &mut ForgeApp, ctx: &egui::Context) {
    // Dopo la schermata di benvenuto, mai insieme.
    if !app.mode_choice_open || app.splash_open {
        return;
    }
    let mut chosen = None;
    let current = app.prefs.ui_mode;
    let _ = egui::Modal::new(egui::Id::NULL.with("scelta-modalita")).show(ctx, |ui| {
        ui.set_width(WIDTH);
        ui.heading(tr("Come vuoi lavorare?"));
        ui.label(tr("Scegli la modalità dell'interfaccia. Scena e impostazioni sono le stesse nelle due modalità."));
        ui.add_space(10.0);
        ui.columns(2, |cols| {
            let cards = [
                (
                    UiMode::Simple,
                    tr("Semplice"),
                    tr("Pannello guidato in pochi passi: importi il prodotto, prepari la scena con un clic, scegli aspetto e qualità e avvii il render. Ideale per iniziare o per foto veloci."),
                ),
                (
                    UiMode::Advanced,
                    tr("Avanzata"),
                    tr("Tutti i controlli: oggetti e trasformazioni, materiali e texture, luci, ambiente, effetti, camera e obiettivo, impostazioni di render e denoiser."),
                ),
            ];
            for (ui, (mode, title, text)) in cols.iter_mut().zip(cards) {
                if card(ui, title, text, mode == current) {
                    chosen = Some(mode);
                }
            }
        });
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(tr(
                "Puoi cambiarla quando vuoi dal selettore \"Modalità: Semplice | Avanzata\" in alto a sinistra.",
            ))
            .weak(),
        );
        ui.add_space(6.0);
        ui.checkbox(&mut app.mode_choice_dont_ask, tr("Non mostrare più all'avvio"))
            .on_hover_text(tr("Si riattiva in Impostazioni, Informazioni"));
    });
    if let Some(mode) = chosen {
        app.mode_choice_open = false;
        let p = crate::prefs::Preferences {
            ui_mode: mode,
            ask_ui_mode: !app.mode_choice_dont_ask,
            ..app.prefs.clone()
        };
        app.apply_prefs(p, false);
    }
}

/// Riquadro cliccabile con titolo e descrizione; restituisce `true` al clic.
fn card(ui: &mut egui::Ui, title: &str, text: &str, current: bool) -> bool {
    let stroke = if current {
        egui::Stroke::new(2.0, style::ACCENT)
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    let frame = egui::Frame::group(ui.style()).stroke(stroke).corner_radius(6.0).inner_margin(12.0);
    let resp = frame
        .show(ui, |ui| {
            ui.set_min_height(150.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(title).size(18.0).strong());
                ui.add_space(4.0);
                ui.label(text);
                ui.add_space(8.0);
                let b = egui::Button::new(egui::RichText::new(tr("Usa questa modalità")).strong()).fill(if current {
                    style::ACCENT
                } else {
                    ui.visuals().widgets.inactive.bg_fill
                });
                ui.add(b).clicked()
            })
            .inner
        })
        .inner;
    resp
}
