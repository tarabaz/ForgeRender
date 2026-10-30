//! Schermata di benvenuto all'avvio: immagine `assets/splash.png` con il logo `assets/logo.png`
//! e la versione in alto a sinistra; sotto, versione, commit, rendering e, in piccolo, le
//! licenze dei componenti open source. Resta davanti finché non si preme "Chiudi"; si riapre dal
//! menu File. "Mostra all'avvio" è in Impostazioni → Informazioni.

use crate::app::ForgeApp;
use crate::i18n::tr;
use crate::ui::style;
use eframe::egui;

const SPLASH_PNG: &[u8] = include_bytes!("../../../../assets/splash.png");
const LOGO_PNG: &[u8] = include_bytes!("../../../../assets/logo.png");
const WIDTH: f32 = 640.0;

pub fn show(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.splash_open {
        return;
    }
    if app.splash_texture.is_none() {
        app.splash_texture = decode(SPLASH_PNG).map(|img| ctx.load_texture("splash", img, egui::TextureOptions::LINEAR));
    }
    if app.logo_texture.is_none() {
        app.logo_texture = decode(LOGO_PNG).map(|img| ctx.load_texture("logo", img, egui::TextureOptions::LINEAR));
    }
    let mut close = false;
    // Il clic fuori o Esc non la chiudono: solo il pulsante.
    let _ = egui::Modal::new(egui::Id::NULL.with("splash")).show(ctx, |ui| {
        ui.set_width(WIDTH);
        if let Some(tex) = &app.splash_texture {
            let [w, h] = tex.size();
            let size = egui::vec2(WIDTH, WIDTH * h as f32 / w.max(1) as f32);
            let rect = ui.add(egui::Image::new((tex.id(), size)).corner_radius(6.0)).rect;
            overlay(ui, rect, app.logo_texture.as_ref());
        }
        ui.add_space(10.0);
        ui.label(tr(
            "Path tracer per foto di prodotto: CPU e GPU, materiali PBR, luci da studio, denoiser.",
        ));
        ui.add_space(6.0);
        egui::Grid::new("splash-info").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label(egui::RichText::new(tr("Versione")).weak());
            ui.label(env!("CARGO_PKG_VERSION"));
            ui.end_row();
            ui.label(egui::RichText::new(tr("Commit")).weak());
            ui.label(env!("FORGE_GIT_SHA"));
            ui.end_row();
            ui.label(egui::RichText::new(tr("Rendering")).weak());
            ui.label(tr("CPU (tutti i core) e GPU (wgpu: Vulkan, DirectX 12, Metal)"));
            ui.end_row();
        });
        ui.add_space(6.0);
        ui.label(egui::RichText::new(tr(
            "ForgeRender usa componenti open source: Intel Open Image Denoise (licenza Apache 2.0, con oneTBB e oneDNN); egui, wgpu e altre librerie Rust (MIT / Apache 2.0); Open CASCADE (LGPL 2.1, solo nel convertitore STEP). Testi completi nella cartella licenze del programma.",
        ))
        .small()
        .weak());
        ui.add_space(10.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let b = egui::Button::new(egui::RichText::new(tr("Chiudi")).size(16.0).strong())
                .fill(style::ACCENT)
                .min_size(egui::vec2(120.0, 32.0));
            if ui.add(b).clicked() {
                close = true;
            }
        });
    });
    if close {
        app.splash_open = false;
    }
}

/// Logo e versione in alto a sinistra, disegnati sopra l'immagine.
fn overlay(ui: &egui::Ui, rect: egui::Rect, logo: Option<&egui::TextureHandle>) {
    let painter = ui.painter_at(rect);
    let margin = rect.width() * 0.035;
    let mut below = rect.min + egui::vec2(margin, margin);
    let logo_w = rect.width() * 0.17;
    if let Some(tex) = logo {
        let [w, h] = tex.size();
        let size = egui::vec2(logo_w, logo_w * h as f32 / w.max(1) as f32);
        let r = egui::Rect::from_min_size(below, size);
        painter.image(
            tex.id(),
            r,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        below = r.left_bottom() + egui::vec2(0.0, 4.0);
    }
    painter.text(
        below,
        egui::Align2::LEFT_TOP,
        format!("v. {}", env!("CARGO_PKG_VERSION")),
        egui::FontId::proportional(13.0),
        style::ACCENT,
    );
}

fn decode(bytes: &[u8]) -> Option<egui::ColorImage> {
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
        }
        Err(e) => {
            tracing::warn!("immagine della schermata di benvenuto non leggibile: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn splash_image_decodes() {
        let img = super::decode(super::SPLASH_PNG).expect("assets/splash.png");
        assert!(img.size[0] >= 320 && img.size[1] >= 180, "{:?}", img.size);
        let logo = super::decode(super::LOGO_PNG).expect("assets/logo.png");
        assert!(logo.size[1] >= 128, "{:?}", logo.size);
    }
}
