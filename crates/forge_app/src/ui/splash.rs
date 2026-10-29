//! Schermata di benvenuto all'avvio: immagine `assets/splash.png` (compresa nel programma),
//! nome, versione e commit. Resta davanti finché non si preme "Chiudi"; si riapre dal menu File.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::style;
use eframe::egui;

const SPLASH_PNG: &[u8] = include_bytes!("../../../../assets/splash.png");
const WIDTH: f32 = 640.0;

pub fn show(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.splash_open {
        return;
    }
    if app.splash_texture.is_none() {
        app.splash_texture = decode().map(|img| ctx.load_texture("splash", img, egui::TextureOptions::LINEAR));
    }
    let mut close = false;
    let mut show_at_start = app.prefs.show_splash;
    // Il clic fuori o Esc non la chiudono: solo il pulsante.
    let _ = egui::Modal::new(egui::Id::NULL.with("splash")).show(ctx, |ui| {
        ui.set_width(WIDTH);
        if let Some(tex) = &app.splash_texture {
            let [w, h] = tex.size();
            let size = egui::vec2(WIDTH, WIDTH * h as f32 / w.max(1) as f32);
            ui.add(egui::Image::new((tex.id(), size)).corner_radius(6.0));
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("ForgeRender").size(28.0).strong().color(style::ACCENT));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(tf("versione {}", &[env!("CARGO_PKG_VERSION").to_string()])).size(16.0));
        });
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
            ui.label(egui::RichText::new(tr("Formati")).weak());
            ui.label(tr("glTF/GLB, OBJ, STL, Blender, STEP, SolidWorks"));
            ui.end_row();
            ui.label(egui::RichText::new(tr("Interfaccia")).weak());
            ui.label(tr("Lingua e modalità Semplice/Avanzata in alto a sinistra"));
            ui.end_row();
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut show_at_start, tr("Mostra all'avvio"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let b = egui::Button::new(egui::RichText::new(tr("Chiudi")).size(16.0).strong())
                    .fill(style::ACCENT)
                    .min_size(egui::vec2(120.0, 32.0));
                if ui.add(b).clicked() {
                    close = true;
                }
            });
        });
    });
    if show_at_start != app.prefs.show_splash {
        let p = crate::prefs::Preferences {
            show_splash: show_at_start,
            ..app.prefs.clone()
        };
        app.apply_prefs(p, false);
    }
    if close {
        app.splash_open = false;
    }
}

fn decode() -> Option<egui::ColorImage> {
    match image::load_from_memory(SPLASH_PNG) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
        }
        Err(e) => {
            tracing::warn!("splash.png non leggibile: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn splash_image_decodes() {
        let img = super::decode().expect("assets/splash.png");
        assert!(img.size[0] >= 320 && img.size[1] >= 180, "{:?}", img.size);
    }
}
