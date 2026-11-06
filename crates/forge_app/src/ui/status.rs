use crate::app::ForgeApp;
use crate::i18n::{tf, tr, tr_dyn};
use crate::ui::style;
use eframe::egui;
use forge_render_core::SessionState;

fn fmt_dur(d: std::time::Duration) -> String {
    let s = d.as_secs_f64();
    if s < 1.0 {
        tf("{:.0} ms", &[format!("{:.0}", s * 1e3)])
    } else if s < 60.0 {
        tf("{s:.1} s", &[format!("{:.1}", s)])
    } else {
        tf(
            "{}m {:02}s",
            &[format!("{}", (s / 60.0) as u64), format!("{:02}", (s % 60.0) as u64)],
        )
    }
}

/// Utilizzo GPU e VRAM della scheda NVIDIA che fa il render (con il render su CPU, la prima).
fn gpu_label(app: &ForgeApp, ui: &mut egui::Ui, p: &forge_render_core::Progress) {
    use forge_telemetry::gpu;
    if !app.monitor.nvml_available() {
        ui.label(tr("GPU: lettura non disponibile")).on_hover_text(tr(
            "Utilizzo GPU e VRAM si leggono con il driver NVIDIA (NVML): con schede AMD o Intel, o senza driver NVIDIA, non sono disponibili",
        ));
        return;
    }
    let device = if p.backend.starts_with("GPU") { p.device.as_str() } else { "" };
    let Some(g) = gpu::matching(&app.monitor.gpus, device) else {
        ui.label(tr("GPU: lettura non disponibile"))
            .on_hover_text(tr("La scheda che fa il render non è NVIDIA: NVML legge solo le schede NVIDIA"));
        return;
    };
    let usage = g.usage_percent.map(|u| format!("{u}%")).unwrap_or_else(|| "—".into());
    let gb = |b: u64| format!("{:.1}", forge_telemetry::gib(b));
    ui.label(tf(
        "GPU: {} · VRAM {}/{} GB (libera {} GB)",
        &[usage, gb(g.vram_used_bytes), gb(g.vram_total_bytes), gb(g.vram_free_bytes)],
    ))
    .on_hover_text(tf(
        "{}: VRAM usata da tutti i programmi, letta dal driver NVIDIA ogni secondo",
        std::slice::from_ref(&g.name),
    ));
}

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let p = app.session.progress();
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let color = match p.state {
            SessionState::Rendering => style::ACCENT,
            SessionState::Completed => style::OK,
            SessionState::Failed(_) => ui.visuals().error_fg_color,
            _ => ui.visuals().text_color(),
        };
        ui.colored_label(color, egui::RichText::new(tr_dyn(&p.state.label())).strong());
        let target = if p.target == 0 { "∞".to_string() } else { p.target.to_string() };
        let bar = egui::ProgressBar::new(p.fraction())
            .desired_width(260.0)
            .text(tf("{} / {} campioni", &[format!("{}", p.samples), target.to_string()]));
        ui.add(bar);
        if p.active_fraction < 1.0 && p.state.is_active() {
            ui.label(tf(
                "ancora in lavorazione: {:.0}% dell'immagine",
                &[format!("{:.0}", p.active_fraction * 100.0)],
            ))
            .on_hover_text(tr("Campionamento adattivo: le zone già pulite non ricevono più campioni"));
        }
        // A destra: i calcoli che servono prima del render (BVH, nebbia), con il loro avanzamento.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if app.is_baking_fog() {
                use forge_render_core::fog::progress::{self, Phase};
                if ui
                    .small_button(crate::ui::icons::CLOSE)
                    .on_hover_text(tr(
                        "Annulla il calcolo della nebbia. Attenzione: verrà rifatto da capo quando avvii il render",
                    ))
                    .clicked()
                {
                    app.cancel_fog();
                }
                let hint = tr(
                    "Calcolo degli effetti (nebbia, foschia) prima del render, uno alla volta: parte da solo \
                     poco dopo l'ultima modifica, così quando avvii il render li trovi già pronti. Puoi continuare a lavorare.",
                );
                match progress::get() {
                    Some(p) => {
                        let phase = match p.phase {
                            Phase::Obstacles => tr("ostacoli"),
                            Phase::Simulation => tr("simulazione"),
                            Phase::Detail => tr("dettaglio"),
                            Phase::Haze => tr("foschia d'ambiente"),
                            Phase::Smoke => tr("fumo volumetrico"),
                        };
                        let pct = format!("{:.0}", p.fraction * 100.0);
                        // Più effetti: uno alla volta, la barra riparte da zero per ognuno.
                        let text = if p.fogs > 1 {
                            tf(
                                "Effetto {} di {}: {} {}%",
                                &[p.fog.to_string(), p.fogs.to_string(), phase.to_string(), pct],
                            )
                        } else if p.phase == Phase::Haze {
                            tf("Foschia d'ambiente: {}%", &[pct])
                        } else if p.phase == Phase::Smoke {
                            tf("Fumo volumetrico: {}%", &[pct])
                        } else {
                            tf("Nebbia: {} {}%", &[phase.to_string(), pct])
                        };
                        ui.add(egui::ProgressBar::new(p.fraction).desired_width(240.0).text(text))
                            .on_hover_text(hint);
                    }
                    None => {
                        ui.label(tr("Nebbia: calcolo…")).on_hover_text(hint);
                        ui.spinner();
                    }
                }
            }
            if app.is_building_bvh() {
                ui.label(tr("costruzione BVH…"));
                ui.spinner();
            }
        });
    });
    // Una riga sola: alla timeline resta l'altezza. Il resto delle misure si apre a richiesta.
    let open_id = egui::Id::NULL.with("statistiche-aperte");
    let mut open = ui.ctx().data_mut(|d| *d.get_persisted_mut_or(open_id, false));
    ui.horizontal(|ui| {
        let (icon, hint) = if open {
            ("−", tr("Meno dettagli"))
        } else {
            ("+", tr("Tutte le misure: passate, risoluzione, backend, memoria, processore"))
        };
        if ui.small_button(icon).on_hover_text(hint).clicked() {
            open = !open;
            ui.ctx().data_mut(|d| d.insert_persisted(open_id, open));
        }
        if !open {
            ui.label(tf("Tempo: {}", &[fmt_dur(p.elapsed).to_string()]));
            ui.separator();
            ui.label(tf("{:.2} Mcampioni/s", &[format!("{:.2}", p.msamples_per_sec())]));
            ui.separator();
            ui.label(tf("CPU: {:.0}%", &[format!("{:.0}", app.monitor.cpu_usage_percent)]));
            ui.separator();
            gpu_label(app, ui, &p);
        }
    });
    if open {
        stats_grid(app, ui, &p);
    }
    tabs_and_log(app, ui);
}

/// Tutte le misure del render e della macchina.
fn stats_grid(app: &ForgeApp, ui: &mut egui::Ui, p: &forge_render_core::Progress) {
    egui::Grid::new("stats").num_columns(6).spacing([18.0, 2.0]).show(ui, |ui| {
        ui.label(tf("Tempo: {}", &[fmt_dur(p.elapsed).to_string()]));
        ui.label(tf("Media/passata: {}", &[fmt_dur(p.avg_pass()).to_string()]));
        ui.label(tf("Ultima passata: {}", &[fmt_dur(p.last_pass).to_string()]));
        ui.label(tf("Risoluzione: {}×{}", &[format!("{}", p.width), format!("{}", p.height)]));
        ui.label(tf("{:.2} Mcampioni/s", &[format!("{:.2}", p.msamples_per_sec())]));
        ui.label(tf(
            "Backend: {} {}",
            &[
                p.backend.to_string(),
                (if p.device.is_empty() {
                    String::new()
                } else {
                    format!("({})", p.device)
                })
                .to_string(),
            ],
        ));
        ui.end_row();
        ui.label(tf("CPU: {:.0}%", &[format!("{:.0}", app.monitor.cpu_usage_percent)]));
        ui.label(tf(
            "RAM processo: {:.2} GB",
            &[format!("{:.2}", forge_telemetry::gib(app.monitor.process_ram_bytes))],
        ));
        ui.label(tf(
            "RAM sistema: {:.1}/{:.1} GB",
            &[
                format!("{:.1}", forge_telemetry::gib(app.monitor.used_ram_bytes)),
                format!("{:.1}", forge_telemetry::gib(app.system.total_ram_bytes)),
            ],
        ));
        gpu_label(app, ui, p);
        match (&app.denoiser_info, p.denoiser.is_empty()) {
            (Err(_), _) => ui.label(tr("Denoiser: OIDN non trovato")),
            (Ok(_), true) => ui.label(tr("Denoiser: inattivo")),
            (Ok(_), false) => ui.label(tf("Denoiser: {}", std::slice::from_ref(&p.denoiser))),
        };
        ui.label(
            egui::RichText::new(tf(
                "{} ({} thread)",
                &[app.system.cpu_brand.to_string(), format!("{}", app.system.logical_threads)],
            ))
            .weak(),
        );
        ui.end_row();
    });
}

/// Schede Timeline e Console.
fn tabs_and_log(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.bottom_tab, crate::app::BottomTab::Timeline, tr("Timeline"));
        ui.selectable_value(&mut app.bottom_tab, crate::app::BottomTab::Console, tr("Console"));
    });
    if app.bottom_tab == crate::app::BottomTab::Timeline {
        if app.prefs.timeline_detached {
            ui.label(egui::RichText::new(tr("La timeline è in una finestra a parte.")).weak());
        } else {
            crate::ui::timeline::panel(app, ui);
            return;
        }
    }
    let lines = app.logs.lines();
    egui::ScrollArea::vertical()
        .id_salt("log")
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for l in lines.iter().rev().take(300).rev() {
                let color = match l.level {
                    tracing::Level::ERROR => ui.visuals().error_fg_color,
                    tracing::Level::WARN => style::WARN,
                    _ => ui.visuals().weak_text_color(),
                };
                ui.label(egui::RichText::new(&l.message).color(color).monospace().small());
            }
        });
}
