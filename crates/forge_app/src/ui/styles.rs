//! Stile del render in cima alla scheda Render: fotorealistico, fumetto, illustrazione tecnica,
//! e i contorni a inchiostro (con qualsiasi stile). Passando sopra a ogni scelta si vede una
//! piccola immagine d'esempio (la scena di prova resa con quello stile).

use crate::app::ForgeApp;
use crate::i18n::tr;
use eframe::egui;
use forge_render_core::RenderStyle;

const PHOTOREAL_PNG: &[u8] = include_bytes!("../../../../assets/stili/fotorealistico.png");
const TOON_PNG: &[u8] = include_bytes!("../../../../assets/stili/fumetto.png");
const TECHNICAL_PNG: &[u8] = include_bytes!("../../../../assets/stili/tecnico.png");
const OUTLINES_PNG: &[u8] = include_bytes!("../../../../assets/stili/contorni.png");

fn name(s: RenderStyle) -> &'static str {
    match s {
        RenderStyle::Photoreal => tr("Fotorealistico"),
        RenderStyle::Toon => tr("Fumetto"),
        RenderStyle::Technical => tr("Illustrazione tecnica"),
    }
}

fn hint(s: RenderStyle) -> &'static str {
    match s {
        RenderStyle::Photoreal => tr("La luce come nella realtà: riflessi, rifrazioni, ombre morbide e luce di rimbalzo (path tracing)"),
        RenderStyle::Toon => tr(
            "Cartone animato (cel shading): colori pieni in tre fasce nette (luce, mezz'ombra, ombra) dalle luci \
             della scena, ombre nette e un riflesso secco sui materiali lucidi. Vetri e specchi restano veri",
        ),
        RenderStyle::Technical => tr(
            "Illustrazione da manuale tecnico (Gooch): ombreggiatura morbida da caldo in luce a freddo in ombra, \
             senza ombre nere, così si leggono bene forme e volumi. Bello con i contorni",
        ),
    }
}

fn png(s: RenderStyle) -> &'static [u8] {
    match s {
        RenderStyle::Photoreal => PHOTOREAL_PNG,
        RenderStyle::Toon => TOON_PNG,
        RenderStyle::Technical => TECHNICAL_PNG,
    }
}

/// Miniatura d'esempio (caricata una volta e tenuta nella memoria di egui).
fn thumb(ctx: &egui::Context, key: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let id = egui::Id::NULL.with(("stile-esempio", key));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let tex = ctx.load_texture(
        format!("stile-{key}"),
        egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}

/// Suggerimento con il testo e l'immagine d'esempio.
fn preview(ui: &mut egui::Ui, text: &str, key: &str, bytes: &[u8]) {
    ui.set_max_width(260.0);
    ui.label(text);
    if let Some(t) = thumb(ui.ctx(), key, bytes) {
        ui.add(egui::Image::new(&t).fit_to_exact_size(egui::vec2(240.0, 135.0)));
    }
}

/// Scelta dello stile e dei contorni. Vero se qualcosa è cambiato.
pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.strong(tr("Stile del render"));
    ui.horizontal_wrapped(|ui| {
        for s in RenderStyle::ALL {
            let key = format!("{s:?}");
            let r = ui.selectable_label(app.settings.style == s, name(s));
            let r = r.on_hover_ui(|ui| preview(ui, hint(s), &key, png(s)));
            if r.clicked() && app.settings.style != s {
                app.settings.style = s;
                changed = true;
            }
        }
    });
    let o = &mut app.settings.outlines;
    changed |= ui
        .checkbox(&mut o.enabled, tr("Contorni a inchiostro"))
        .on_hover_ui(|ui| {
            preview(
                ui,
                tr(
                    "Linee scure su sagome, confini tra oggetti e spigoli vivi, disegnate sopra l'immagine finale: \
                    vanno con qualsiasi stile",
                ),
                "contorni",
                OUTLINES_PNG,
            )
        })
        .changed();
    if o.enabled {
        ui.indent("contorni", |ui| {
            changed |= ui
                .add(egui::Slider::new(&mut o.width_px, 0.5..=6.0).suffix(" px").text(tr("Spessore")))
                .changed();
            ui.horizontal(|ui| {
                ui.label(tr("Colore"));
                changed |= ui.color_edit_button_rgb(&mut o.color).changed();
            });
            changed |= ui
                .checkbox(&mut o.creases, tr("Anche gli spigoli vivi"))
                .on_hover_text(tr("Linee anche dove due facce dello stesso oggetto formano un angolo netto"))
                .changed();
            if o.creases {
                changed |= ui
                    .add(
                        egui::Slider::new(&mut o.crease_deg, 10.0..=90.0)
                            .suffix("°")
                            .text(tr("Angolo degli spigoli")),
                    )
                    .on_hover_text(tr("Più basso = più linee (anche su angoli poco marcati)"))
                    .changed();
            }
        });
    }
    changed
}
