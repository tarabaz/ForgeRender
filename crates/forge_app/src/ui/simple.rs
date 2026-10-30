//! Pannello della modalità Semplice: la foto prodotto in cinque passi, con pulsanti grandi.
//!
//! Ogni pulsante usa le stesse funzioni della modalità Avanzata (`ForgeApp` e `presets`):
//! i valori sono quelli del renderer, quindi passando ad Avanzata si ritrovano identici.
//! Cosa si vede e quali valori impostano i preset: `docs/INTERFACCIA_SEMPLICE.md`.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::presets::{self, Look, Quality, FORMATS};
use crate::ui::style;
use eframe::egui;
use forge_render_core::{BackendKind, SessionState};

const BIG: f32 = 15.0;

/// Pulsante a tutta larghezza, alto e con il testo grande.
fn big_button(ui: &mut egui::Ui, text: &str, primary: bool) -> egui::Response {
    let mut rt = egui::RichText::new(text).size(BIG);
    if primary {
        rt = rt.strong().color(egui::Color32::WHITE);
    }
    let mut b = egui::Button::new(rt).min_size(egui::vec2(ui.available_width(), 36.0));
    if primary {
        b = b.fill(style::ACCENT);
    }
    ui.add(b)
}

/// Titolo di un passo ("1  Prodotto").
fn step(ui: &mut egui::Ui, n: u32, title: &str) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tf(" {n} ", &[format!("{}", n)]))
                .strong()
                .size(BIG)
                .background_color(style::ACCENT)
                .color(egui::Color32::WHITE),
        );
        ui.label(egui::RichText::new(title).strong().size(BIG + 1.0));
    });
    ui.add_space(4.0);
}

/// Scelta a tutta larghezza con titolo e spiegazione (aspetto e qualità).
fn choice(ui: &mut egui::Ui, selected: bool, title: &str, hint: &str) -> bool {
    let text = egui::RichText::new(format!("{title}\n")).size(BIG).strong();
    let mut job = egui::text::LayoutJob::default();
    text.append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::LEFT);
    // Sulla scelta evidenziata (sfondo arancione) il testo grigio non si leggerebbe.
    let hint = if selected {
        egui::RichText::new(hint).small().color(egui::Color32::WHITE)
    } else {
        egui::RichText::new(hint).small().weak()
    };
    hint.append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::LEFT);
    ui.add(egui::Button::selectable(selected, job).min_size(egui::vec2(ui.available_width(), 44.0)))
        .clicked()
}

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("simple-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(egui::RichText::new(tr("Foto prodotto in 5 passi")).size(BIG + 3.0).strong());
            ui.label(
                egui::RichText::new(tr(
                    "Tutte le opzioni sono nella modalità Avanzata (in alto): i valori restano gli stessi.",
                ))
                .small()
                .weak(),
            );
            product(app, ui);
            look(app, ui);
            materials(app, ui);
            format_quality(app, ui);
            render(app, ui);
            ui.add_space(12.0);
        });
}

fn product(app: &mut ForgeApp, ui: &mut egui::Ui) {
    step(ui, 1, tr("Prodotto"));
    if big_button(ui, tr("Importa modello…"), false)
        .on_hover_text(tr(crate::ui::toolbar::IMPORT_HINT))
        .clicked()
    {
        app.import_dialog();
    }
    let has_product = !presets::product_bounds(&app.scene).is_empty();
    ui.add_enabled_ui(has_product, |ui| {
        if big_button(ui, tr("Appoggia sulla base"), false)
            .on_hover_text(tr(
                "Porta il punto più basso del prodotto a terra (tutte le parti insieme). Ctrl+Z per annullare",
            ))
            .clicked()
        {
            app.drop_product_to_base();
        }
        if big_button(ui, tr("Inquadra il prodotto"), false)
            .on_hover_text(tr(
                "Centra la camera sul prodotto (senza pavimento e fondale); vale anche per il render",
            ))
            .clicked()
        {
            app.frame_product();
        }
        if big_button(ui, tr("Adatta la scena al prodotto"), false)
            .on_hover_text(tr(
                "Dopo aver spostato o scalato il prodotto: rifà fondale e luci sulle sue misure (tiene materiale del \
                 fondale e colori delle luci) e inquadra. Si può annullare",
            ))
            .clicked()
        {
            app.simple_fit_scene();
        }
    });
    if !has_product {
        ui.label(egui::RichText::new(tr("Importa un modello per cominciare.")).weak());
    }
}

fn look(app: &mut ForgeApp, ui: &mut egui::Ui) {
    step(ui, 2, tr("Aspetto"));
    let current = presets::matching_look(&app.scene, &app.settings);
    for l in Look::ALL {
        if choice(ui, current == Some(l), l.label(), l.hint()) {
            app.simple_apply_look(l);
        }
    }
    match current {
        Some(l) => {
            ui.label(egui::RichText::new(tf("Aspetto attuale: {}", &[l.label().to_string()])).small());
        }
        // L'avviso serve solo dopo un preset: una scena appena aperta ha le sue luci.
        None if app.preset_backup.is_some() => {
            ui.colored_label(
                style::WARN,
                tr(
                    "Personalizzato: luci, ambiente o visualizzazione sono stati cambiati (anche in Avanzata). \
                 Scegli un aspetto per riapplicarlo.",
                ),
            );
        }
        None => {
            ui.label(egui::RichText::new(tr("Scegli un aspetto, poi premi \"Prepara scena prodotto\".")).small());
        }
    }
    light_colors(app, ui);
    ui.add_space(4.0);
    if big_button(ui, tr("Prepara scena prodotto"), true)
        .on_hover_text(tr(
            "Appoggia il prodotto sulla base, aggiunge il fondale se manca, applica aspetto e qualità scelti e \
             inquadra. Si può annullare.",
        ))
        .clicked()
    {
        if let Some(l) = current {
            app.simple_look = l;
        }
        app.simple_prepare_scene();
    }
    if ui
        .add_enabled(
            app.preset_backup.is_some(),
            egui::Button::new(egui::RichText::new(tr("Annulla preset")).size(BIG - 1.0)),
        )
        .on_hover_text(tr(
            "Riporta luci, ambiente, qualità, fondale e posizione del prodotto a com'erano prima dell'ultimo preset",
        ))
        .clicked()
    {
        app.simple_undo_preset();
    }
    if let Some(m) = &app.simple_msg {
        ui.label(egui::RichText::new(m).color(egui::Color32::from_rgb(140, 200, 140)));
    }
}

fn materials(app: &mut ForgeApp, ui: &mut egui::Ui) {
    step(ui, 3, tr("Materiali"));
    ui.label(tr("Clicca un pezzo nella vista, poi scegli il materiale nella libreria."));
    match app.selection.iter().next().copied() {
        Some(i) => {
            let mat = app.scene.resolve_material(i).clone();
            let name = app.scene.materials.get(&mat).map(|m| m.name.clone()).unwrap_or_default();
            ui.label(
                egui::RichText::new(tf(
                    "Selezionato: {}{}  ·  materiale: {name}",
                    &[
                        app.scene.nodes[i].name.to_string(),
                        (if app.selection.len() > 1 {
                            tf(" e altri {}", &[format!("{}", app.selection.len() - 1)])
                        } else {
                            String::new()
                        })
                        .to_string(),
                        name.to_string(),
                    ],
                ))
                .strong(),
            );
        }
        None => {
            ui.label(egui::RichText::new(tr("Nessun pezzo selezionato.")).weak());
        }
    }
    if big_button(ui, tr("Apri la libreria materiali"), false)
        .on_hover_text(tr("Clic su un materiale: lo applica ai pezzi selezionati"))
        .clicked()
    {
        app.library.open = true;
    }
}

fn format_quality(app: &mut ForgeApp, ui: &mut egui::Ui) {
    step(ui, 4, tr("Formato e qualità"));
    let s = &mut app.settings;
    let mut format_changed = false;
    egui::Grid::new("simple-formats").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        let w = (ui.available_width() - 6.0) / 2.0;
        for (k, (name, desc, fw, fh)) in FORMATS.iter().enumerate() {
            let sel = s.width == *fw && s.height == *fh;
            if ui
                .add(egui::Button::selectable(sel, egui::RichText::new(tr(name)).size(BIG - 1.0)).min_size(egui::vec2(w, 30.0)))
                .on_hover_text(tr(desc))
                .clicked()
            {
                s.width = *fw;
                s.height = *fh;
                format_changed = true;
            }
            if k % 2 == 1 {
                ui.end_row();
            }
        }
    });
    ui.label(
        egui::RichText::new(tf("Immagine: {} × {} pixel", &[format!("{}", s.width), format!("{}", s.height)]))
            .small()
            .weak(),
    );
    if format_changed {
        // Il formato cambia l'inquadratura (soprattutto in verticale): si riadatta.
        app.frame_product();
    }
    ui.add_space(6.0);
    let current = Quality::matching(&app.settings);
    for q in Quality::ALL {
        let (spp, _) = q.values();
        if choice(
            ui,
            current == Some(q),
            q.label(),
            &tf("{} · {spp} campioni", &[q.hint().to_string(), format!("{}", spp)]),
        ) {
            app.simple_apply_quality(q);
        }
    }
    if current.is_none() {
        ui.colored_label(
            style::WARN,
            tf(
                "Qualità personalizzata in Avanzata: {} campioni, {} rimbalzi.",
                &[format!("{}", app.settings.target_samples), format!("{}", app.settings.max_bounces)],
            ),
        );
    }
    ui.add_space(6.0);
    // Dispositivo: due pulsanti ben visibili. Non si aspetta la preparazione della GPU: finché
    // non è pronta il pulsante resta attivo.
    let gpu_ok = app.gpu_ready() != Some(false);
    ui.label(egui::RichText::new(tr("Dispositivo di calcolo")).size(BIG - 1.0));
    let mut backend = app.settings.backend;
    egui::Grid::new("simple-device").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        let w = (ui.available_width() - 6.0) / 2.0;
        let gpu = ui.add_enabled(
            gpu_ok,
            egui::Button::selectable(
                backend == BackendKind::Gpu,
                egui::RichText::new(tr("Scheda video (GPU)")).size(BIG - 1.0),
            )
            .min_size(egui::vec2(w, 30.0)),
        );
        let gpu = if gpu_ok {
            gpu.on_hover_text(tr("Molto più veloce con una scheda video recente"))
        } else {
            gpu.on_disabled_hover_text(tr("Nessuna GPU utilizzabile: il render usa il processore"))
        };
        if gpu.clicked() {
            backend = BackendKind::Gpu;
        }
        if ui
            .add(
                egui::Button::selectable(
                    backend == BackendKind::Cpu,
                    egui::RichText::new(tr("Processore (CPU)")).size(BIG - 1.0),
                )
                .min_size(egui::vec2(w, 30.0)),
            )
            .on_hover_text(tr("Usa tutti i core del processore: funziona su ogni PC"))
            .clicked()
        {
            backend = BackendKind::Cpu;
        }
        ui.end_row();
    });
    if backend != app.settings.backend {
        app.settings.backend = backend;
        app.simple_gpu_checked = true;
    }
}

fn render(app: &mut ForgeApp, ui: &mut egui::Ui) {
    step(ui, 5, tr("Render"));
    let state = app.render_state();
    let label = match state {
        SessionState::Rendering => tr("Pausa"),
        SessionState::Paused => tr("Riprendi"),
        _ => tr("Avvia render"),
    };
    if ui
        .add_enabled_ui(!app.is_preparing(), |ui| big_button(ui, label, true))
        .inner
        .on_hover_text("F5")
        .clicked()
    {
        app.toggle_pause();
    }
    let p = app.session.progress();
    if p.target > 0 && (state.is_active() || p.samples > 0) {
        ui.add(
            egui::ProgressBar::new((p.samples as f32 / p.target as f32).min(1.0))
                .text(tf("{} / {} campioni", &[format!("{}", p.samples), format!("{}", p.target)])),
        );
    }
    if app.is_preparing() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(tr("Preparazione della scena…"));
        });
    }
    if big_button(ui, tr("Salva immagine…"), false)
        .on_hover_text(tr("Salva il render come PNG"))
        .clicked()
    {
        app.export_png_dialog();
    }
}

/// Colore delle luci: tre temperature rapide per tutte, poi un colore per ciascuna luce.
/// Cambiare il colore non fa uscire dall'aspetto scelto (non diventa "Personalizzato").
fn light_colors(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let lights: Vec<usize> = (0..app.scene.lights.len())
        .filter(|&i| !matches!(app.scene.lights[i].kind, forge_scene::LightKind::Portal { .. }))
        .collect();
    if lights.is_empty() {
        return;
    }
    egui::CollapsingHeader::new(egui::RichText::new(tr("Colore delle luci")).size(BIG - 1.0))
        .id_salt("simple-light-colors")
        .show(ui, |ui| {
            let mut changed = false;
            ui.horizontal(|ui| {
                for (name, k, tip) in [
                    (tr("Calda"), 3200.0, tr("3200 K: luce gialla come una lampadina")),
                    (tr("Neutra"), 5500.0, tr("5500 K: luce bianca da studio")),
                    (tr("Fredda"), 7500.0, tr("7500 K: luce azzurrina")),
                ] {
                    if ui
                        .add(egui::Button::new(egui::RichText::new(name).size(BIG - 1.0)).min_size(egui::vec2(80.0, 28.0)))
                        .on_hover_text(tf("Tutte le luci: {tip}", &[tip.to_string()]))
                        .clicked()
                    {
                        presets::set_lights_kelvin(&mut app.scene, k);
                        changed = true;
                    }
                }
            });
            for i in lights {
                ui.horizontal(|ui| {
                    let l = &mut app.scene.lights[i];
                    changed |= ui.color_edit_button_rgb(&mut l.color).changed();
                    ui.label(&l.name);
                });
            }
            if changed {
                app.shading_changed();
            }
        });
}
