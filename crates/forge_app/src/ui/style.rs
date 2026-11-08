use eframe::egui;

/// Tema scuro professionale. L'arancione è solo per l'azione principale (Avvia render) e per ciò
/// che è selezionato nella scena ([`scene_selection`]): schede e interruttori scelti sono grigio
/// chiaro, così l'arancione resta un segnale.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 5.0);
        style.spacing.button_padding = egui::vec2(8.0, 3.0);
        // Testo piccolo leggibile: egui lo fa di 9 punti, troppo poco per note e suggerimenti.
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(SMALL_TEXT, egui::FontFamily::Proportional),
        );
        let v = &mut style.visuals;
        v.selection.bg_fill = egui::Color32::from_gray(78);
        v.selection.stroke.color = egui::Color32::from_gray(245);
        // Testi normali più chiari e testi attenuati ancora leggibili sul fondo scuro (contrasto
        // almeno 4,5:1 sul grigio 30 dei pannelli).
        v.widgets.noninteractive.fg_stroke.color = egui::Color32::from_gray(190);
        v.weak_text_color = Some(egui::Color32::from_gray(140));
        v.hyperlink_color = egui::Color32::from_rgb(255, 160, 60);
        v.panel_fill = egui::Color32::from_gray(30);
        v.window_fill = egui::Color32::from_gray(34);
        v.extreme_bg_color = egui::Color32::from_gray(18);
    });
}

/// Misura del testo piccolo (note, suggerimenti sotto i controlli).
pub const SMALL_TEXT: f32 = 12.0;

/// Arancione per ciò che è selezionato nella scena (righe dell'albero), da usare in quella parte
/// dell'interfaccia al posto del grigio delle schede.
pub fn scene_selection(ui: &mut egui::Ui) {
    let s = &mut ui.visuals_mut().selection;
    s.bg_fill = egui::Color32::from_rgb(200, 100, 20);
    s.stroke.color = egui::Color32::WHITE;
}

pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 150, 50);
pub const WARN: egui::Color32 = egui::Color32::from_rgb(230, 190, 60);
pub const OK: egui::Color32 = egui::Color32::from_rgb(110, 200, 110);
