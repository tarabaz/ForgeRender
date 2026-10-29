use crate::app::ForgeApp;
use crate::ui::style;
use eframe::egui;
use forge_render_core::SessionState;

fn fmt_dur(d: std::time::Duration) -> String {
    let s = d.as_secs_f64();
    if s < 1.0 {
        format!("{:.0} ms", s * 1e3)
    } else if s < 60.0 {
        format!("{s:.1} s")
    } else {
        format!("{}m {:02}s", (s / 60.0) as u64, (s % 60.0) as u64)
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
        ui.colored_label(color, egui::RichText::new(p.state.label()).strong());
        let target = if p.target == 0 { "∞".to_string() } else { p.target.to_string() };
        let bar = egui::ProgressBar::new(p.fraction())
            .desired_width(260.0)
            .text(format!("{} / {} campioni", p.samples, target));
        ui.add(bar);
        if p.active_fraction < 1.0 && p.state.is_active() {
            ui.label(format!("ancora in lavorazione: {:.0}% dell'immagine", p.active_fraction * 100.0))
                .on_hover_text("Campionamento adattivo: le zone già pulite non ricevono più campioni");
        }
        if app.is_preparing() {
            ui.spinner();
            ui.label("costruzione BVH…");
        }
    });
    egui::Grid::new("stats").num_columns(6).spacing([18.0, 2.0]).show(ui, |ui| {
        ui.label(format!("Tempo: {}", fmt_dur(p.elapsed)));
        ui.label(format!("Media/passata: {}", fmt_dur(p.avg_pass())));
        ui.label(format!("Ultima passata: {}", fmt_dur(p.last_pass)));
        ui.label(format!("Risoluzione: {}×{}", p.width, p.height));
        ui.label(format!("{:.2} Mcampioni/s", p.msamples_per_sec()));
        ui.label(format!(
            "Backend: {} {}",
            p.backend,
            if p.device.is_empty() {
                String::new()
            } else {
                format!("({})", p.device)
            }
        ));
        ui.end_row();
        ui.label(format!("CPU: {:.0}%", app.monitor.cpu_usage_percent));
        ui.label(format!(
            "RAM processo: {:.2} GB",
            forge_telemetry::gib(app.monitor.process_ram_bytes)
        ));
        ui.label(format!(
            "RAM sistema: {:.1}/{:.1} GB",
            forge_telemetry::gib(app.monitor.used_ram_bytes),
            forge_telemetry::gib(app.system.total_ram_bytes)
        ));
        ui.label("GPU: utilizzo non rilevato")
            .on_hover_text("La lettura di utilizzo GPU e VRAM (NVML) non è ancora integrata");
        match (&app.denoiser_info, p.denoiser.is_empty()) {
            (Err(_), _) => ui.label("Denoiser: OIDN non trovato"),
            (Ok(_), true) => ui.label("Denoiser: inattivo"),
            (Ok(_), false) => ui.label(format!("Denoiser: {}", p.denoiser)),
        };
        ui.label(egui::RichText::new(format!("{} ({} thread)", app.system.cpu_brand, app.system.logical_threads)).weak());
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
