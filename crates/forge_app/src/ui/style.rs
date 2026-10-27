use eframe::egui;

/// Tema scuro professionale con accento arancione.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 5.0);
        style.spacing.button_padding = egui::vec2(8.0, 3.0);
        let v = &mut style.visuals;
        v.selection.bg_fill = egui::Color32::from_rgb(200, 100, 20);
        v.selection.stroke.color = egui::Color32::from_rgb(255, 170, 80);
        v.hyperlink_color = egui::Color32::from_rgb(255, 160, 60);
        v.panel_fill = egui::Color32::from_gray(30);
        v.window_fill = egui::Color32::from_gray(34);
        v.extreme_bg_color = egui::Color32::from_gray(18);
    });
}

pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 150, 50);
pub const WARN: egui::Color32 = egui::Color32::from_rgb(230, 190, 60);
pub const OK: egui::Color32 = egui::Color32::from_rgb(110, 200, 110);
