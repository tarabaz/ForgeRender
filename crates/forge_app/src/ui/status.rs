use crate::app::ForgeApp;
use crate::i18n::{tf, tr, tr_dyn};
use crate::prefs::StatusItem;
use crate::ui::style;
use eframe::egui;
use forge_render_core::SessionState;
use std::time::{Duration, Instant};

/// FPS reali della viewport: tempo tra due disegni consecutivi, medio. Si aggiorna solo quando la
/// viewport si ridisegna (nessun ridisegno in più), e da ferma non dà un numero.
#[derive(Default)]
pub struct FpsMeter {
    last: Option<Instant>,
    /// Secondi per fotogramma, media mobile.
    average: f32,
}

impl FpsMeter {
    /// Pause più lunghe non sono un fotogramma lento ma una viewport ferma.
    const IDLE: f32 = 1.0;
    const WEIGHT: f32 = 0.1;

    pub fn frame(&mut self, now: Instant) {
        if let Some(last) = self.last {
            let dt = (now - last).as_secs_f32();
            if dt < Self::IDLE {
                self.average = if self.average > 0.0 {
                    self.average + (dt - self.average) * Self::WEIGHT
                } else {
                    dt
                };
            } else {
                self.average = 0.0;
            }
        }
        self.last = Some(now);
    }

    pub fn fps(&self, now: Instant) -> Option<f32> {
        let idle = self.last.is_none_or(|l| (now - l).as_secs_f32() >= Self::IDLE);
        (!idle && self.average > 0.0).then(|| 1.0 / self.average)
    }
}

/// Fotogrammi di un video in corso (render dell'animazione o sequenza).
struct VideoJob {
    done: usize,
    total: usize,
    /// In codifica; altrimenti si calcolano i fotogrammi.
    encoding: bool,
    remaining: Option<Duration>,
}

fn video_job(app: &ForgeApp) -> Option<VideoJob> {
    if let Some(run) = &app.timeline_ui.run {
        if let Some((p, since)) = run.encoding() {
            let (done, total) = p.get();
            let remaining = crate::video::eta(done, total, since.elapsed());
            return Some(VideoJob {
                done,
                total,
                encoding: true,
                remaining,
            });
        }
        let (done, total) = run.progress();
        let remaining = run.remaining();
        return Some(VideoJob {
            done,
            total,
            encoding: false,
            remaining,
        });
    }
    let run = app.sequence.run.as_ref()?;
    if let Some(p) = run.encoding() {
        let (done, total) = p.get();
        let remaining = run.encode_started().and_then(|s| crate::video::eta(done, total, s.elapsed()));
        return Some(VideoJob {
            done,
            total,
            encoding: true,
            remaining,
        });
    }
    let (done, total) = (run.done(), run.total());
    let remaining = (done >= 2).then(|| run.remaining()).flatten();
    Some(VideoJob {
        done,
        total,
        encoding: false,
        remaining,
    })
}

/// Tempo che manca al render dell'immagine, quando è già avanti abbastanza da stimarlo.
fn render_remaining(p: &forge_render_core::Progress) -> Option<Duration> {
    let f = p.fraction() as f64;
    (p.state == SessionState::Rendering && p.target > 0 && f > 0.02).then(|| p.elapsed.mul_f64((1.0 - f) / f))
}

pub fn item_label(i: StatusItem) -> &'static str {
    match i {
        StatusItem::Elapsed => tr("Tempo trascorso"),
        StatusItem::Remaining => tr("Tempo rimanente"),
        StatusItem::Frames => tr("Fotogrammi"),
        StatusItem::ViewportFps => tr("FPS viewport"),
        StatusItem::VideoFps => tr("FPS video"),
        StatusItem::RenderSpeed => tr("Velocità render"),
        StatusItem::Cpu => tr("CPU"),
        StatusItem::Gpu => tr("GPU"),
        StatusItem::Vram => tr("VRAM"),
        StatusItem::RamSystem => tr("RAM sistema"),
        StatusItem::AveragePass => tr("Media per passata"),
        StatusItem::LastPass => tr("Ultima passata"),
        StatusItem::Resolution => tr("Risoluzione"),
        StatusItem::Denoiser => tr("Denoiser"),
    }
}

fn item_hint(i: StatusItem) -> &'static str {
    match i {
        StatusItem::Elapsed => tr("Tempo del render in corso"),
        StatusItem::Remaining => {
            tr("Stima dai fotogrammi già finiti, o dai campioni per un'immagine sola: si aggiorna mentre il lavoro procede")
        }
        StatusItem::Frames => tr("Fotogrammi finiti su quelli totali, durante il render e la codifica"),
        StatusItem::ViewportFps => {
            tr("Fotogrammi al secondo davvero disegnati dalla viewport, media mobile. Da ferma non si ridisegna: niente numero")
        }
        StatusItem::VideoFps => tr("Frequenza impostata per il video: non dice quanto è veloce il programma"),
        StatusItem::RenderSpeed => tr("Milioni di campioni calcolati al secondo"),
        StatusItem::Cpu => tr("Utilizzo della CPU, aggiornato ogni secondo"),
        StatusItem::Gpu => tr("Utilizzo della GPU: solo schede NVIDIA, letto dal driver ogni secondo"),
        StatusItem::Vram => tr("Memoria video usata da tutti i programmi: solo schede NVIDIA"),
        StatusItem::RamSystem => tr("Memoria usata da tutto il sistema"),
        StatusItem::AveragePass => tr("Tempo medio di una passata del render"),
        StatusItem::LastPass => tr("Tempo dell'ultima passata del render"),
        StatusItem::Resolution => tr("Risoluzione del render"),
        StatusItem::Denoiser => tr("Stato del denoiser"),
    }
}

/// Sezione delle Impostazioni: quali contatori mostrare.
pub fn settings(ui: &mut egui::Ui, p: &mut crate::prefs::Preferences) {
    use crate::prefs::MAX_STATUS_ITEMS;
    ui.heading(tr("Barra di stato"));
    ui.label(tf(
        "Contatori in tempo reale: al massimo {}, più Backend e processore che restano sempre a destra.",
        &[MAX_STATUS_ITEMS.to_string()],
    ));
    ui.add_space(4.0);
    let full = p.status_items.len() >= MAX_STATUS_ITEMS;
    egui::Grid::new("contatori").num_columns(2).spacing([24.0, 4.0]).show(ui, |ui| {
        for (k, i) in StatusItem::ALL.into_iter().enumerate() {
            let mut on = p.status_items.contains(&i);
            let free = on || !full;
            let r = ui.add_enabled(free, egui::Checkbox::new(&mut on, item_label(i)));
            let r = r.on_hover_text(item_hint(i));
            let r = r.on_disabled_hover_text(tf("Limite raggiunto: {} contatori più i 2 fissi", &[MAX_STATUS_ITEMS.to_string()]));
            if r.changed() {
                if on {
                    p.status_items.push(i);
                } else {
                    p.status_items.retain(|x| *x != i);
                }
            }
            if k % 2 == 1 {
                ui.end_row();
            }
        }
    });
    ui.add_space(6.0);
    ui.label(tf(
        "Scelti: {} di {}",
        &[p.status_items.len().to_string(), MAX_STATUS_ITEMS.to_string()],
    ));
    if ui.button(tr("Contatori predefiniti")).clicked() {
        p.status_items = StatusItem::defaults();
    }
}

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

/// Lettura della scheda NVIDIA che fa il render (con il render su CPU, la prima), o perché non c'è.
fn gpu_sample<'a>(app: &'a ForgeApp, p: &forge_render_core::Progress) -> Result<&'a forge_telemetry::gpu::GpuSample, &'static str> {
    if !app.monitor.nvml_available() {
        return Err(tr(
            "Utilizzo GPU e VRAM si leggono con il driver NVIDIA (NVML): con schede AMD o Intel, o senza driver NVIDIA, non sono disponibili",
        ));
    }
    let device = if p.backend.starts_with("GPU") { p.device.as_str() } else { "" };
    forge_telemetry::gpu::matching(&app.monitor.gpus, device)
        .ok_or(tr("La scheda che fa il render non è NVIDIA: NVML legge solo le schede NVIDIA"))
}

/// Larghezza di una casella: non si stringe mai, così quello che segue non si sposta.
fn slot_width(previous: f32, widest: f32, text: f32) -> f32 {
    previous.max(widest).max(text).ceil()
}

fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    ui.painter()
        .layout_no_wrap(text.to_string(), font, ui.visuals().text_color())
        .size()
        .x
}

/// Testo allineato a sinistra in una casella che tiene la larghezza più grande vista finora (e
/// almeno `widest`): i numeri cambiano senza spostare quello che viene dopo.
fn steady_text(ui: &mut egui::Ui, id: egui::Id, text: egui::RichText, widest: f32) -> egui::Response {
    let previous = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(0.0);
    let width = slot_width(previous, widest, text_width(ui, text.text()));
    ui.data_mut(|d| d.insert_temp(id, width));
    let height = ui.text_style_height(&egui::TextStyle::Body);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let galley = egui::WidgetText::from(text).into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, egui::TextStyle::Body);
    let pos = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, ui.visuals().text_color());
    response
}

/// I testi più lunghi che un contatore può mostrare: le caselle sono già larghe abbastanza dal
/// primo fotogramma. Quello che non è previsto qui allarga la casella la prima volta che appare.
fn widest_texts(i: StatusItem) -> Vec<String> {
    let dur = || tf("{}m {:02}s", &["888".to_string(), "88".to_string()]);
    match i {
        StatusItem::Elapsed => vec![tf("Tempo: {}", &[dur()])],
        StatusItem::Remaining => vec![
            tf("Codifica: mancano {}", &[dur()]),
            tf("Fotogrammi: mancano {}", &[dur()]),
            tf("Tempo rimanente: {}", &[dur()]),
            tr("Calcolo stima…").to_string(),
        ],
        StatusItem::Frames => vec![tf("Fotogrammi: {} / {}", &["8888".to_string(), "8888".to_string()])],
        StatusItem::ViewportFps => vec![tf("FPS viewport: {}", &["8888".to_string()])],
        StatusItem::VideoFps => vec![tf("FPS video: {}", &["888".to_string()])],
        StatusItem::RenderSpeed => vec![tf("{:.2} Mcampioni/s", &["8888.88".to_string()])],
        StatusItem::Cpu => vec![tf("CPU: {:.0}%", &["100".to_string()])],
        StatusItem::Gpu => vec![tf("GPU: {}", &["100%".to_string()])],
        StatusItem::Vram => vec![tf(
            "VRAM {}/{} GB (libera {} GB)",
            &["88.8".to_string(), "88.8".to_string(), "88.8".to_string()],
        )],
        StatusItem::RamSystem => vec![tf("RAM sistema: {:.1}/{:.1} GB", &["888.8".to_string(), "888.8".to_string()])],
        StatusItem::AveragePass => vec![tf("Media/passata: {}", &[dur()])],
        StatusItem::LastPass => vec![tf("Ultima passata: {}", &[dur()])],
        StatusItem::Resolution => vec![tf("Risoluzione: {}×{}", &["8888".to_string(), "8888".to_string()])],
        StatusItem::Denoiser => vec![tr("Denoiser: inattivo").to_string()],
    }
}

/// Un contatore della barra.
fn item(app: &ForgeApp, ui: &mut egui::Ui, i: StatusItem, p: &forge_render_core::Progress, job: Option<&VideoJob>) {
    let gib = forge_telemetry::gib;
    let widest = widest_texts(i).iter().map(|t| text_width(ui, t)).fold(0.0, f32::max);
    let id = egui::Id::NULL.with(("contatore", i));
    let text = match i {
        StatusItem::Elapsed => tf("Tempo: {}", &[fmt_dur(p.elapsed)]),
        StatusItem::Remaining => {
            let left = job.and_then(|j| j.remaining).or_else(|| render_remaining(p));
            match (left, job) {
                (Some(d), Some(j)) if j.encoding => tf("Codifica: mancano {}", &[fmt_dur(d)]),
                (Some(d), Some(_)) => tf("Fotogrammi: mancano {}", &[fmt_dur(d)]),
                (Some(d), None) => tf("Tempo rimanente: {}", &[fmt_dur(d)]),
                (None, Some(_)) => tr("Calcolo stima…").to_string(),
                (None, None) => tf("Tempo rimanente: {}", &["—".to_string()]),
            }
        }
        StatusItem::Frames => match job {
            Some(j) => tf("Fotogrammi: {} / {}", &[j.done.to_string(), j.total.to_string()]),
            None => tf("Fotogrammi: {} / {}", &["—".to_string(), "—".to_string()]),
        },
        StatusItem::ViewportFps => match app.viewport_fps.fps(Instant::now()) {
            Some(f) => tf("FPS viewport: {}", &[format!("{f:.0}")]),
            None => tf("FPS viewport: {}", &["—".to_string()]),
        },
        StatusItem::VideoFps => tf("FPS video: {}", &[format!("{:.0}", app.scene.timeline.fps)]),
        StatusItem::RenderSpeed => tf("{:.2} Mcampioni/s", &[format!("{:.2}", p.msamples_per_sec())]),
        StatusItem::Cpu => tf("CPU: {:.0}%", &[format!("{:.0}", app.monitor.cpu_usage_percent)]),
        StatusItem::Gpu | StatusItem::Vram => match gpu_sample(app, p) {
            Err(why) => {
                steady_text(ui, id.with("assente"), tr("GPU: lettura non disponibile").into(), 0.0).on_hover_text(why);
                return;
            }
            Ok(g) if i == StatusItem::Gpu => tf("GPU: {}", &[g.usage_percent.map(|u| format!("{u}%")).unwrap_or_else(|| "—".into())]),
            Ok(g) => tf(
                "VRAM {}/{} GB (libera {} GB)",
                &[
                    format!("{:.1}", gib(g.vram_used_bytes)),
                    format!("{:.1}", gib(g.vram_total_bytes)),
                    format!("{:.1}", gib(g.vram_free_bytes)),
                ],
            ),
        },
        StatusItem::RamSystem => tf(
            "RAM sistema: {:.1}/{:.1} GB",
            &[
                format!("{:.1}", gib(app.monitor.used_ram_bytes)),
                format!("{:.1}", gib(app.system.total_ram_bytes)),
            ],
        ),
        StatusItem::AveragePass => tf("Media/passata: {}", &[fmt_dur(p.avg_pass())]),
        StatusItem::LastPass => tf("Ultima passata: {}", &[fmt_dur(p.last_pass)]),
        StatusItem::Resolution => tf("Risoluzione: {}×{}", &[format!("{}", p.width), format!("{}", p.height)]),
        StatusItem::Denoiser => match (&app.denoiser_info, p.denoiser.is_empty()) {
            (Err(_), _) => tr("Denoiser: OIDN non trovato").to_string(),
            (Ok(_), true) => tr("Denoiser: inattivo").to_string(),
            (Ok(_), false) => tf("Denoiser: {}", std::slice::from_ref(&p.denoiser)),
        },
    };
    steady_text(ui, id, text.into(), widest).on_hover_text(item_hint(i));
}

/// Riga dei contatori: quelli scelti partono da sinistra e vanno a capo se serve; Backend e
/// processore restano fissi all'estrema destra, su due righe.
fn counters(app: &ForgeApp, ui: &mut egui::Ui, p: &forge_render_core::Progress) {
    let job = video_job(app);
    let device = if p.device.is_empty() {
        String::new()
    } else {
        format!("({})", p.device)
    };
    let backend = tf("Backend: {} {}", &[p.backend.to_string(), device]);
    let cpu = tf(
        "{} ({} thread)",
        &[app.system.cpu_brand.to_string(), format!("{}", app.system.logical_threads)],
    );
    let font = egui::TextStyle::Body.resolve(ui.style());
    let color = ui.visuals().text_color();
    let measure = |ui: &egui::Ui, t: &str| ui.painter().layout_no_wrap(t.to_string(), font.clone(), color).size().x;
    let fixed = measure(ui, &backend).max(measure(ui, &cpu)).min(ui.available_width() * 0.45).ceil();
    ui.horizontal_top(|ui| {
        let left = (ui.available_width() - fixed - 12.0).max(40.0);
        ui.allocate_ui(egui::vec2(left, 0.0), |ui| {
            ui.set_width(left);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                for i in &app.prefs.status_items {
                    item(app, ui, *i, p, job.as_ref());
                }
            });
        });
        ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.add(egui::Label::new(backend).truncate());
            ui.add(egui::Label::new(egui::RichText::new(cpu).weak()).truncate());
        });
    });
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
        let state = egui::RichText::new(tr_dyn(&p.state.label())).strong().color(color);
        steady_text(ui, egui::Id::NULL.with("stato-render"), state, 0.0);
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
    counters(app, ui, &p);
    tabs_and_log(app, ui);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_never_shrinks() {
        assert_eq!(slot_width(0.0, 80.0, 60.0), 80.0);
        assert_eq!(slot_width(80.0, 80.0, 95.2), 96.0);
        assert_eq!(slot_width(96.0, 80.0, 40.0), 96.0);
    }

    #[test]
    fn every_counter_has_a_widest_text() {
        assert!(StatusItem::ALL.iter().all(|&i| !widest_texts(i).is_empty()));
    }
}
