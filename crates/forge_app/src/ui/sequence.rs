//! Finestre "Viste e turntable" (Rendering → Viste e turntable…) e "Passaggi di render"
//! (File → Esporta passaggi di render…). La logica è in `crate::sequence`.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::sequence::{self, format_duration, PassSet, SeqKind, VIEWS};
use crate::ui::style;
use eframe::egui;

pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if app.sequence.run.is_some() {
        progress_modal(app, ctx);
    } else if app.sequence.open {
        options_window(app, ctx);
    }
    passes_window(app, ctx);
}

fn options_window(app: &mut ForgeApp, ctx: &egui::Context) {
    let mut open = true;
    let mut start = false;
    egui::Window::new(tr("Viste e turntable"))
        .open(&mut open)
        .resizable(false)
        .default_width(430.0)
        .show(ctx, |ui| {
            let o = &mut app.sequence.opts;
            ui.horizontal(|ui| {
                ui.selectable_value(&mut o.kind, SeqKind::Turntable, tr("Turntable (video)"));
                ui.selectable_value(&mut o.kind, SeqKind::Views, tr("Viste multiple"));
            });
            ui.separator();
            match o.kind {
                SeqKind::Views => {
                    ui.label(tr("Il prodotto gira su se stesso: camera, luci e fondale restano fermi."));
                    egui::Grid::new("viste").num_columns(2).show(ui, |ui| {
                        for (k, (_, label, _, _)) in VIEWS.iter().enumerate() {
                            ui.checkbox(&mut o.views[k], tr(label));
                            if k % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                }
                SeqKind::Turntable => {
                    egui::Grid::new("turntable").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        ui.label(tr("Rotazione tra due immagini"));
                        ui.add(
                            egui::DragValue::new(&mut o.step_deg)
                                .range(0.25..=45.0)
                                .speed(0.05)
                                .suffix("°")
                                .max_decimals(2),
                        )
                        .on_hover_text(tr("Più è piccola, più il movimento è fluido (e più immagini da calcolare)"));
                        ui.end_row();
                        ui.label(tr("Fotogrammi al secondo"));
                        egui::ComboBox::from_id_salt("fps")
                            .selected_text(format!("{}", o.fps))
                            .show_ui(ui, |ui| {
                                for f in [24, 25, 30, 50, 60] {
                                    ui.selectable_value(&mut o.fps, f, format!("{f}"));
                                }
                            });
                        ui.end_row();
                        ui.label(tr("Durata di un giro"));
                        let mut secs = sequence::turn_seconds(o.step_deg, o.fps);
                        if ui
                            .add(
                                egui::DragValue::new(&mut secs)
                                    .range(0.5..=120.0)
                                    .speed(0.1)
                                    .suffix(" s")
                                    .max_decimals(1),
                            )
                            .on_hover_text(tr("Cambiando la durata si ricalcola la rotazione tra due immagini"))
                            .changed()
                        {
                            o.step_deg = sequence::step_for_duration(secs, o.fps);
                        }
                        ui.end_row();
                        ui.label(tr("Giri nel video"));
                        ui.add(egui::DragValue::new(&mut o.turns).range(1..=20))
                            .on_hover_text(tr("Il giro si ripete senza rifare i render: il video gira in loop senza scatti"));
                        ui.end_row();
                        ui.label(tr("Senso di rotazione"));
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut o.clockwise, false, tr("Antiorario"));
                            ui.selectable_value(&mut o.clockwise, true, tr("Orario"));
                        });
                        ui.end_row();
                    });
                    let n = sequence::turntable_frames(o.step_deg);
                    let secs = sequence::turn_seconds(o.step_deg, o.fps) * o.turns as f32;
                    ui.label(
                        egui::RichText::new(tf(
                            "{} immagini (una ogni {}°) · video di {} s",
                            &[
                                format!("{n}"),
                                format!("{:.2}", sequence::effective_step(o.step_deg)),
                                format!("{secs:.1}"),
                            ],
                        ))
                        .strong(),
                    );
                    ui.checkbox(&mut o.make_video, tr("Crea il video MP4 (H.264)"));
                }
            }
            ui.separator();
            egui::Grid::new("comuni").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.label(tr("Campioni per immagine"));
                ui.add(egui::DragValue::new(&mut o.samples).range(16..=8192).speed(4.0))
                    .on_hover_text(tr(
                        "Uguali in tutte le immagini. Per un video senza sfarfallio almeno 256: il rumore \
                     residuo resta fermo da un'immagine all'altra e il denoiser lavora sempre allo stesso modo",
                    ));
                ui.end_row();
                ui.label(tr("Nome dei file"));
                ui.text_edit_singleline(&mut o.name);
                ui.end_row();
            });
            if o.kind == SeqKind::Turntable && o.samples < 256 {
                ui.colored_label(style::WARN, tr("Sotto i 256 campioni il video può sfarfallare"));
            }
            ui.checkbox(&mut o.fit_frame, tr("Inquadra il prodotto (sempre tutto dentro)"))
                .on_hover_text(tr(
                    "Stessa direzione della camera, distanza adattata. Nel turntable un'unica inquadratura \
                 che contiene il prodotto in ogni posizione del giro",
                ));
            ui.collapsing(tr("Passaggi di render per ogni immagine"), |ui| pass_checkboxes(ui, &mut o.passes));
            ui.separator();
            // Camera: quella di render attiva, altrimenti la vista attuale.
            match app.scene.active_camera.and_then(|i| app.scene.render_cameras.get(i)) {
                Some(c) => {
                    ui.label(tf("Camera: {}", std::slice::from_ref(&c.name)));
                }
                None => {
                    ui.horizontal(|ui| {
                        ui.label(tr("Camera: la vista attuale"));
                        if ui
                            .button(tr("Crea una camera da questa vista"))
                            .on_hover_text(tr("Così l'inquadratura resta salvata nel progetto"))
                            .clicked()
                        {
                            app.add_render_camera(None);
                        }
                    });
                }
            }
            let o = &mut app.sequence.opts;
            ui.horizontal(|ui| {
                if ui.button(tr("Cartella…")).clicked() {
                    if let Some(d) = rfd::FileDialog::new().set_title(tr("Cartella in cui salvare")).pick_folder() {
                        o.out_dir = Some(d);
                    }
                }
                match &o.out_dir {
                    Some(d) => ui.label(d.display().to_string()),
                    None => ui.colored_label(style::WARN, tr("scegli dove salvare")),
                };
            });
            // Stima dalla velocità dell'ultimo render.
            let frames = sequence::plan(o).len();
            let rate = app.session.progress().msamples_per_sec();
            if rate > 0.0 && frames > 0 {
                let px = app.settings.width as f64 * app.settings.height as f64;
                let per = px * o.samples as f64 / (rate * 1e6);
                ui.label(tf(
                    "Tempo stimato: {} ({} immagini da {} × {})",
                    &[
                        format_duration(std::time::Duration::from_secs_f64(per * frames as f64)),
                        format!("{frames}"),
                        format!("{}", app.settings.width),
                        format!("{}", app.settings.height),
                    ],
                ));
            } else {
                ui.label(tf(
                    "{} immagini da {} × {} (la stima del tempo arriva dopo un primo render)",
                    &[
                        format!("{frames}"),
                        format!("{}", app.settings.width),
                        format!("{}", app.settings.height),
                    ],
                ));
            }
            ui.add_space(4.0);
            let ready = o.out_dir.is_some() && frames > 0 && crate::presets::has_product(&app.scene);
            if ui
                .add_enabled(
                    ready,
                    egui::Button::new(egui::RichText::new(tr("Avvia")).strong()).fill(style::ACCENT),
                )
                .clicked()
            {
                start = true;
            }
        });
    if !open {
        app.sequence.open = false;
    }
    if start {
        app.start_sequence();
    }
}

fn pass_checkboxes(ui: &mut egui::Ui, p: &mut PassSet) {
    ui.checkbox(&mut p.albedo, tr("Albedo (colore dei materiali senza luce)"));
    ui.checkbox(&mut p.normals, tr("Normali (orientamento delle superfici)"));
    ui.checkbox(&mut p.depth, tr("Profondità (PNG 16 bit ed EXR in metri)"));
    ui.checkbox(&mut p.mask, tr("Maschera del prodotto (per scontornare)"));
    ui.checkbox(&mut p.ids, tr("ID dei pezzi (un colore per pezzo, per selezionarli)"));
}

fn progress_modal(app: &mut ForgeApp, ctx: &egui::Context) {
    let mut stop = false;
    let Some(run) = &app.sequence.run else { return };
    let render = app.session.progress();
    egui::Modal::new(egui::Id::NULL.with("sequenza")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading(tr("Viste e turntable"));
        if run.is_finishing() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr("Ultimi salvataggi e creazione del video…"));
            });
            return;
        }
        let (done, total) = (run.done(), run.total());
        let frac = (done as f32 + render.fraction()) / total.max(1) as f32;
        ui.label(tf(
            "{} ({} di {})",
            &[run.current_label().to_string(), format!("{}", done + 1), format!("{total}")],
        ));
        ui.add(egui::ProgressBar::new(frac).show_percentage());
        let left = run.remaining().map(format_duration).unwrap_or_else(|| tr("calcolo…").to_string());
        ui.label(tf("Trascorso {} · mancano circa {}", &[format_duration(run.elapsed()), left]));
        ui.add_space(6.0);
        if ui
            .button(tr("Interrompi"))
            .on_hover_text(tr("Le immagini già fatte restano; la scena torna com'era"))
            .clicked()
        {
            stop = true;
        }
    });
    if stop {
        app.cancel_sequence();
    }
}

fn passes_window(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.sequence.passes_open {
        return;
    }
    let mut open = true;
    let mut export = false;
    let busy = app.sequence.passes_job.is_some();
    let has_render = app.last_render_scene_arc().is_some() && app.session.latest_frame().is_some();
    egui::Window::new(tr("Passaggi di render"))
        .open(&mut open)
        .resizable(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            ui.label(tr(
                "Dall'ultimo render, stessa inquadratura e stesse dimensioni: per la post-produzione.",
            ));
            ui.checkbox(&mut app.sequence.export_beauty, tr("Immagine finale (PNG)"));
            pass_checkboxes(ui, &mut app.sequence.export_passes);
            ui.add_space(4.0);
            if busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr("Calcolo dei passaggi…"));
                });
            } else if !has_render {
                ui.colored_label(style::WARN, tr("Avvia prima un rendering"));
            }
            let any = app.sequence.export_passes.any() || app.sequence.export_beauty;
            if ui
                .add_enabled(!busy && has_render && any, egui::Button::new(tr("Esporta…")).fill(style::ACCENT))
                .clicked()
            {
                export = true;
            }
        });
    if !open {
        app.sequence.passes_open = false;
    }
    if export {
        if let Some(p) = rfd::FileDialog::new()
            .set_file_name("render.png")
            .add_filter("PNG", &["png"])
            .save_file()
        {
            app.export_passes(p);
        }
    }
}
