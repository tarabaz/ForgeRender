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
        if app.is_preparing() {
            ui.spinner();
            ui.label(tr("costruzione BVH…"));
        }
    });
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
        ui.label(tr("GPU: utilizzo non rilevato"))
            .on_hover_text(tr("La lettura di utilizzo GPU e VRAM (NVML) non è ancora integrata"));
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
    ui.separator();
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
