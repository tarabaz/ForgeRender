//! Finestra "Impostazioni video": formato, codec e qualità del file che si crea dai fotogrammi.
//! Le scelte restano nelle impostazioni del programma; contano solo per la codifica, il render
//! dei fotogrammi non cambia.

use crate::app::ForgeApp;
use crate::export_job::ExportLoad;
use crate::i18n::{tf, tr};
use crate::ui::style;
use crate::video::{self, Codec, Container, Preset, Rate, VideoSettings};
use eframe::egui;

fn load_label(l: ExportLoad) -> &'static str {
    match l {
        ExportLoad::Full => tr("Massima velocità"),
        ExportLoad::Shared => tr("Lascia il PC usabile"),
    }
}

fn load_hint(l: ExportLoad) -> &'static str {
    match l {
        ExportLoad::Full => tr("Usa tutta la potenza: il PC può rispondere più lentamente"),
        ExportLoad::Shared => tr("Priorità bassa e pause sulla GPU: gli altri programmi restano fluidi, il render dura di più"),
    }
}

fn preset_label(p: Preset) -> &'static str {
    match p {
        Preset::Standard => tr("Standard"),
        Preset::High => tr("Alta qualità"),
        Preset::VeryHigh => tr("Molto alta"),
        Preset::Editing => tr("Per il montaggio"),
        Preset::Lossless => tr("Senza perdita"),
        Preset::Uncompressed => tr("Non compresso"),
        Preset::Custom => tr("Personalizzato"),
    }
}

fn preset_hint(p: Preset) -> &'static str {
    match p {
        Preset::Standard => tr("File piccoli, compressione alta: per condividere il video"),
        Preset::High => tr("MP4 con compressione leggera: file circa tre volte più grandi dello standard"),
        Preset::VeryHigh => tr("MP4 con compressione minima: file molto più grandi, quasi nessun difetto visibile"),
        Preset::Editing => tr("ProRes in MOV: file grandi, fluidi da montare e da rilavorare"),
        Preset::Lossless => tr("FFV1 in MKV: ogni pixel identico ai fotogrammi, file grandi, per l'archivio"),
        Preset::Uncompressed => tr("AVI non compresso: file enormi, lo legge qualsiasi programma di montaggio"),
        Preset::Custom => tr("Scelte fatte a mano"),
    }
}

fn codec_label(k: Codec) -> &'static str {
    if k == Codec::Raw {
        tr("Non compresso (RGB)")
    } else {
        k.label()
    }
}

/// Riassunto delle impostazioni, per il menu di salvataggio.
pub fn summary(s: &VideoSettings) -> String {
    format!("{} · {} · {}", preset_label(s.preset), s.container.label(), codec_label(s.codec))
}

fn size_text(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1e9)
    } else {
        format!("{:.0} MB", bytes as f64 / 1e6)
    }
}

pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.video_open {
        return;
    }
    let caps = video::caps();
    let mut open = true;
    let mut s = app.prefs.video.clone();
    let (rw, rh) = (app.settings.width, app.settings.height);
    let timeline_fps = app.scene.timeline.fps.round().max(1.0) as u32;
    let frames = app.scene.timeline.work_frames().len() as u64;
    let mut out = app.video_out.clone();
    egui::Window::new(tr("Impostazioni video"))
        .id(egui::Id::NULL.with("win-video"))
        .open(&mut open)
        .resizable(false)
        .default_width(430.0)
        .show(ctx, |ui| {
            editor(ui, &mut s, caps, (rw, rh), timeline_fps, frames, &mut out);
        });
    app.video_open = open;
    app.video_out = out;
    if s != app.prefs.video {
        let mut p = app.prefs.clone();
        p.video = s;
        app.apply_prefs(p, false);
    }
}

fn editor(
    ui: &mut egui::Ui,
    s: &mut VideoSettings,
    caps: &video::Caps,
    render: (u32, u32),
    timeline_fps: u32,
    frames: u64,
    out: &mut Option<std::path::PathBuf>,
) {
    let before = s.clone();
    egui::Grid::new("video-preset").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Preset"));
        egui::ComboBox::from_id_salt("preset")
            .width(240.0)
            .selected_text(preset_label(s.preset))
            .show_ui(ui, |ui| {
                for p in Preset::ALL {
                    let usable = VideoSettings::preset_available(p, caps);
                    let hint = if usable {
                        preset_hint(p)
                    } else {
                        tr("Serve ffmpeg, accanto al programma o nel PATH")
                    };
                    let r = ui.add_enabled(usable, egui::Button::selectable(s.preset == p, preset_label(p)));
                    if r.on_hover_text(hint).on_disabled_hover_text(hint).clicked() {
                        s.apply_preset(p, caps);
                    }
                }
            });
        ui.end_row();
    });
    ui.label(egui::RichText::new(preset_hint(s.preset)).weak());
    ui.separator();
    egui::Grid::new("video-formato").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Contenitore"));
        ui.horizontal(|ui| {
            for c in Container::ALL {
                let usable = c.codecs().iter().any(|k| caps.can(c, *k));
                if ui
                    .add_enabled(usable, egui::Button::selectable(s.container == c, c.label()))
                    .on_disabled_hover_text(tr("Serve ffmpeg, accanto al programma o nel PATH"))
                    .clicked()
                {
                    s.container = c;
                }
            }
        });
        ui.end_row();
        s.sanitize(caps);
        ui.label(tr("Codec"));
        egui::ComboBox::from_id_salt("codec")
            .width(240.0)
            .selected_text(codec_label(s.codec))
            .show_ui(ui, |ui| {
                for k in s.codecs(caps) {
                    ui.selectable_value(&mut s.codec, k, codec_label(k));
                }
            });
        ui.end_row();
        s.sanitize(caps);
        let fps = s.fps.unwrap_or(timeline_fps);
        let (w, h) = s.output_size(render.0, render.1);
        if s.codec.has_rate() {
            ui.label(tr("Compressione"));
            ui.horizontal(|ui| {
                ui.add_enabled_ui(caps.has(s.codec), |ui| {
                    ui.selectable_value(&mut s.rate, Rate::Crf, tr("Qualità costante (CRF)"))
                        .on_hover_text(tr("La qualità è fissa, il file è grande quanto serve alla scena"))
                        .on_disabled_hover_text(tr("Serve ffmpeg, accanto al programma o nel PATH"));
                });
                ui.selectable_value(&mut s.rate, Rate::Bitrate, tr("Bitrate"))
                    .on_hover_text(tr("Il file ha una dimensione prevedibile, la qualità segue la scena"));
            });
            ui.end_row();
            match s.rate {
                Rate::Crf => {
                    ui.label("CRF");
                    ui.add(egui::Slider::new(&mut s.crf, 0..=51)).on_hover_text(tr(
                        "Più basso = qualità più alta e file più grande. 18 è già molto buono, 0 è senza perdita",
                    ));
                }
                Rate::Bitrate => {
                    ui.label(tr("Bitrate"));
                    let mut mbit = s.bitrate(w, h, fps).unwrap_or(0) as f32 / 1e6;
                    if ui
                        .add(egui::DragValue::new(&mut mbit).range(1.0..=400.0).speed(0.5).suffix(" Mbit/s"))
                        .changed()
                    {
                        s.bits_per_pixel = mbit * 1e6 / (w as f32 * h as f32 * fps.max(1) as f32).max(1.0);
                    }
                }
            }
            ui.end_row();
        }
        ui.label(tr("Pixel"));
        egui::ComboBox::from_id_salt("pixel")
            .width(240.0)
            .selected_text(s.pixel.label())
            .show_ui(ui, |ui| {
                for p in s.codec.pixels() {
                    if *p == video::Pixel::Yuv420 || caps.has(s.codec) || s.codec == Codec::Raw {
                        ui.selectable_value(&mut s.pixel, *p, p.label());
                    }
                }
            })
            .response
            .on_hover_text(tr(
                "I fotogrammi sono a 8 bit: i 10 bit servono alle lavorazioni successive, non aggiungono dettaglio",
            ));
        ui.end_row();
        ui.label(tr("Fotogrammi al secondo"));
        ui.horizontal(|ui| {
            let mut own = s.fps.is_some();
            if ui
                .checkbox(&mut own, tr("Diversi dalla timeline"))
                .on_hover_text(tr("I fotogrammi restano gli stessi: il video va più veloce o più lento"))
                .changed()
            {
                s.fps = own.then_some(timeline_fps);
            }
            match &mut s.fps {
                Some(f) => {
                    ui.add(egui::DragValue::new(f).range(1..=240));
                }
                None => {
                    ui.label(egui::RichText::new(format!("{timeline_fps}")).weak());
                }
            }
        });
        ui.end_row();
        ui.label(tr("Risoluzione"));
        ui.horizontal(|ui| {
            let mut own = s.size.is_some();
            if ui
                .checkbox(&mut own, tr("Diversa dal render"))
                .on_hover_text(tr("I fotogrammi vengono ridimensionati (Lanczos) solo nel video"))
                .changed()
            {
                s.size = own.then_some([render.0.max(16), render.1.max(16)]);
            }
            match &mut s.size {
                Some(v) => {
                    ui.add(egui::DragValue::new(&mut v[0]).range(16..=16384));
                    ui.label("×");
                    ui.add(egui::DragValue::new(&mut v[1]).range(16..=16384));
                }
                None => {
                    ui.label(egui::RichText::new(format!("{} × {}", render.0, render.1)).weak());
                }
            }
        });
        ui.end_row();
    });
    // Ogni modifica a mano allontana dal preset.
    if *s != before && s.preset == before.preset && s.preset != Preset::Custom {
        let mut reference = s.clone();
        reference.apply_preset(s.preset, caps);
        if reference != *s {
            s.preset = Preset::Custom;
        }
    }
    ui.separator();
    let fps = s.fps.unwrap_or(timeline_fps);
    let (w, h) = s.output_size(render.0, render.1);
    let seconds = frames as f32 / fps.max(1) as f32;
    let size = match s.estimate(render.0, render.1, fps, frames) {
        Some(e) if e.exact => tf("Dimensione del file: {}", &[size_text(e.bytes)]),
        Some(e) => tf("Dimensione stimata: circa {}, non garantita", &[size_text(e.bytes)]),
        None => tr("La dimensione del file dipende dalla scena").to_string(),
    };
    ui.label(tf(
        "{} × {}, {} fotogrammi a {} fps: {} s di video",
        &[
            w.to_string(),
            h.to_string(),
            frames.to_string(),
            fps.to_string(),
            format!("{seconds:.1}"),
        ],
    ));
    ui.label(size);
    ui.separator();
    egui::Grid::new("video-carico").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Durante l'esportazione"));
        egui::ComboBox::from_id_salt("carico")
            .width(240.0)
            .selected_text(load_label(s.load))
            .show_ui(ui, |ui| {
                for l in [ExportLoad::Full, ExportLoad::Shared] {
                    ui.selectable_value(&mut s.load, l, load_label(l)).on_hover_text(load_hint(l));
                }
            });
        ui.end_row();
    });
    ui.label(egui::RichText::new(load_hint(s.load)).weak());
    if caps.ffmpeg.is_none() {
        ui.colored_label(
            style::WARN,
            tr("ffmpeg non trovato: restano MP4 H.264 e AVI non compresso. Mettilo accanto al programma o nel PATH per gli altri formati."),
        );
    }
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button(tr("Scegli il file…")).clicked() {
            let ext = s.container.extension();
            let mut dialog = rfd::FileDialog::new()
                .set_directory(crate::prefs::rendering_dir())
                .set_file_name(format!("animazione.{ext}"))
                .add_filter(s.container.label(), &[ext]);
            dialog = dialog.set_title(tr("Dove salvare il video"));
            if let Some(p) = dialog.save_file() {
                *out = Some(p);
            }
        }
        match out {
            Some(p) => {
                let p = p.with_extension(s.container.extension());
                ui.label(p.display().to_string());
                if ui
                    .small_button(crate::ui::icons::CLOSE)
                    .on_hover_text(tr("Torna alla cartella dei fotogrammi"))
                    .clicked()
                {
                    *out = None;
                }
            }
            None => {
                ui.label(egui::RichText::new(tr("Nella cartella dei fotogrammi")).weak());
            }
        }
    });
}
