//! Pannello "Lucciole / Bosco incantato" e punti nella viewport. Il calcolo sta in
//! `forge_scene::fireflies`: posizioni e ciclo dipendono solo dal tempo.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::icons;
use crate::viewport::project;
use eframe::egui;
use forge_scene::{Fireflies, FireflyColor};

/// Una coppia minimo e massimo, in secondi o W/sr.
fn range(ui: &mut egui::Ui, label: &str, v: &mut [f32; 2], max: f32, suffix: &str) -> bool {
    ui.label(label);
    let mut changed = false;
    ui.horizontal(|ui| {
        let speed = max / 200.0;
        let [low, high] = *v;
        changed |= ui
            .add(
                egui::DragValue::new(&mut v[0])
                    .range(0.0..=high)
                    .speed(speed)
                    .suffix(suffix)
                    .max_decimals(2),
            )
            .changed();
        ui.label("–");
        changed |= ui
            .add(
                egui::DragValue::new(&mut v[1])
                    .range(low..=max)
                    .speed(speed)
                    .suffix(suffix)
                    .max_decimals(2),
            )
            .changed();
    });
    ui.end_row();
    changed
}

fn percent(ui: &mut egui::Ui, label: &str, v: &mut f32, hint: &str) -> bool {
    ui.label(label);
    let r = ui
        .add(egui::Slider::new(v, 0.0..=1.0).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)))
        .on_hover_text(hint)
        .changed();
    ui.end_row();
    r
}

/// Volume che si vede dalla camera attuale: attorno al punto inquadrato, in proporzione alla distanza.
pub fn framed_volume(app: &ForgeApp) -> (forge_math::Vec3, forge_math::Vec3) {
    let cam = &app.view_camera;
    let d = (cam.position - cam.target).length().max(0.2);
    (cam.target + forge_math::Vec3::Y * 0.1 * d, forge_math::Vec3::new(d, d * 0.5, d))
}

pub fn panel(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.separator();
    let mut changed = false;
    let region = framed_volume(app);
    let f = &mut app.scene.fireflies;
    ui.horizontal(|ui| {
        changed |= ui.checkbox(&mut f.enabled, tr("Lucciole / Bosco incantato")).changed();
        if ui
            .button(tr("Bosco incantato"))
            .on_hover_text(tr(
                "Molte lucciole lente, accensioni sparse, palette calda e luce morbida: tutto resta modificabile",
            ))
            .clicked()
        {
            *f = Fireflies::enchanted_forest(region.0, region.1);
            changed = true;
        }
    });
    if !f.enabled {
        finish(app, changed);
        return;
    }
    let group = |ui: &mut egui::Ui, id: &str, title: &str, open: bool, body: &mut dyn FnMut(&mut egui::Ui)| {
        egui::CollapsingHeader::new(title).id_salt(id).default_open(open).show(ui, |ui| {
            egui::Grid::new(id).num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| body(ui));
        });
    };
    group(ui, "lucciole-generale", tr("Generale e quantità"), true, &mut |ui| {
        ui.label(tr("Quantità"));
        changed |= ui.add(egui::Slider::new(&mut f.count, 1..=3000).logarithmic(true)).changed();
        ui.end_row();
        ui.label(tr("Seme"));
        changed |= ui
            .add(egui::DragValue::new(&mut f.seed))
            .on_hover_text(tr("Un altro seme, un altro stormo: lo stesso seme dà sempre lo stesso"))
            .changed();
        ui.end_row();
    });
    group(ui, "lucciole-volume", tr("Volume e movimento"), false, &mut |ui| {
        ui.label(tr("Centro"));
        ui.horizontal(|ui| {
            for v in [&mut f.center.x, &mut f.center.y, &mut f.center.z] {
                changed |= ui.add(egui::DragValue::new(v).speed(0.01).suffix(" m").max_decimals(2)).changed();
            }
        });
        ui.end_row();
        ui.label(tr("Dimensioni"));
        ui.horizontal(|ui| {
            for v in [&mut f.size.x, &mut f.size.y, &mut f.size.z] {
                changed |= ui
                    .add(egui::DragValue::new(v).range(0.05..=100.0).speed(0.01).suffix(" m").max_decimals(2))
                    .changed();
            }
        });
        ui.end_row();
        ui.label(tr("Rotazione"));
        changed |= ui
            .add(egui::Slider::new(&mut f.yaw_deg, -180.0..=180.0).suffix("°"))
            .on_hover_text(tr("Gira il volume attorno alla verticale; gli assi del volo seguono il volume"))
            .changed();
        ui.end_row();
        ui.label(tr("Velocità"));
        changed |= ui
            .add(
                egui::Slider::new(&mut f.speed, 0.01..=2.0)
                    .logarithmic(true)
                    .suffix(" m/s")
                    .max_decimals(2),
            )
            .changed();
        ui.end_row();
        changed |= percent(
            ui,
            tr("Variazione di velocità"),
            &mut f.speed_spread,
            tr("Quanto cambia la velocità tra una lucciola e l'altra"),
        );
        changed |= percent(ui, tr("Movimento verticale"), &mut f.vertical, tr("Rispetto a quello orizzontale"));
        changed |= percent(
            ui,
            tr("Fluidità"),
            &mut f.smooth,
            tr("Alta: curve larghe. Bassa: più guizzi e cambi di direzione"),
        );
        changed |= percent(
            ui,
            tr("Dispersione"),
            &mut f.spread,
            tr("Bassa: stormo raccolto al centro del volume. Alta: lucciole dappertutto"),
        );
    });
    group(ui, "lucciole-ciclo", tr("Ciclo di accensione e spegnimento"), false, &mut |ui| {
        changed |= range(ui, tr("Spenta"), &mut f.off_s, 60.0, " s");
        changed |= range(ui, tr("Accensione"), &mut f.rise_s, 10.0, " s");
        changed |= range(ui, tr("Accesa"), &mut f.on_s, 30.0, " s");
        changed |= range(ui, tr("Spegnimento"), &mut f.fall_s, 10.0, " s");
    });
    egui::CollapsingHeader::new(tr("Palette colori e percentuali"))
        .id_salt("lucciole-palette")
        .show(ui, |ui| {
            let shares = f.shares();
            let mut remove = None;
            for (k, p) in f.palette.iter_mut().enumerate() {
                ui.push_id(k, |ui| {
                    ui.horizontal(|ui| {
                        changed |= crate::ui::color_picker::rgb_edit(ui, &mut p.color);
                        changed |= ui.add(egui::Slider::new(&mut p.share, 0.0..=1.0).show_value(false)).changed();
                        ui.label(format!("{:.0} %", shares.get(k).copied().unwrap_or(0.0) * 100.0))
                            .on_hover_text(tr("Parte effettiva: le percentuali vengono riportate a 100"));
                        if ui.small_button(icons::DELETE).clicked() {
                            remove = Some(k);
                        }
                    });
                });
            }
            if let Some(k) = remove.filter(|_| f.palette.len() > 1) {
                f.palette.remove(k);
                changed = true;
            }
            if ui.small_button(tr("+ Colore")).clicked() {
                f.palette.push(FireflyColor {
                    color: [1.0, 0.9, 0.4],
                    share: 0.3,
                });
                changed = true;
            }
        });
    group(ui, "lucciole-tinta", tr("Variazione tonale"), false, &mut |ui| {
        changed |= percent(
            ui,
            tr("Lucciole variate"),
            &mut f.tint_share,
            tr("Quante ricevono una tinta leggermente diversa da quella della palette"),
        );
        changed |= percent(
            ui,
            tr("Ampiezza"),
            &mut f.tint_amount,
            tr("Fino a 30° di tinta: resta nella stessa famiglia di colore"),
        );
    });
    group(ui, "lucciole-luce", tr("Intensità luminosa e alone"), false, &mut |ui| {
        changed |= range(ui, tr("Intensità"), &mut f.intensity, 5.0, " W/sr");
        ui.label(tr("Luce sulla scena"));
        changed |= ui
            .add(egui::Slider::new(&mut f.scene_light, 0.0..=20.0).max_decimals(2))
            .on_hover_text(tr(
                "Quanto illuminano davvero oggetti e superfici. 0 = solo punti visibili, senza luce sulla scena",
            ))
            .changed();
        ui.end_row();
        ui.label(tr("Corpo"));
        changed |= ui
            .add(
                egui::Slider::new(&mut f.body_mm, 0.5..=40.0)
                    .logarithmic(true)
                    .suffix(" mm")
                    .max_decimals(1),
            )
            .changed();
        ui.end_row();
        ui.label(tr("Grandezza alone"));
        changed |= ui.add(egui::Slider::new(&mut f.halo_size, 0.0..=30.0).max_decimals(1)).changed();
        ui.end_row();
        ui.label(tr("Forza alone"));
        changed |= ui
            .add(egui::Slider::new(&mut f.halo_strength, 0.0..=4.0).max_decimals(2))
            .on_hover_text(tr("Solo l'aspetto dei punti: non cambia la luce sulla scena"))
            .changed();
        ui.end_row();
    });
    group(ui, "lucciole-qualita", tr("Qualità e prestazioni"), false, &mut |ui| {
        ui.label(tr("Luci al massimo"));
        changed |= ui
            .add(egui::Slider::new(&mut f.max_lights, 1..=64))
            .on_hover_text(tr(
                "Oltre questo numero le lucciole vicine diventano una luce sola con la stessa potenza: meno rumore, ombre meno precise",
            ))
            .changed();
        ui.end_row();
        ui.label(tr("Raggio della luce"));
        changed |= ui
            .add(
                egui::Slider::new(&mut f.light_range, 0.1..=20.0)
                    .logarithmic(true)
                    .suffix(" m")
                    .max_decimals(2),
            )
            .on_hover_text(tr("Fin dove arriva la luce di una lucciola"))
            .changed();
        ui.end_row();
        ui.label(tr("Ombre"));
        changed |= ui.checkbox(&mut f.shadows, tr("Gli oggetti fermano la luce")).changed();
        ui.end_row();
    });
    finish(app, changed);
}

fn finish(app: &mut ForgeApp, changed: bool) {
    if changed {
        app.shading_changed();
    }
}

/// Le lucciole accese nella viewport: un punto del loro colore, che segue il ciclo.
pub fn draw(app: &ForgeApp, painter: &egui::Painter, rect: egui::Rect) {
    let f = &app.scene.fireflies;
    if !f.enabled {
        return;
    }
    let cam = &app.view_camera;
    let t = app.scene.timeline.time;
    let sparks = f.sparks(t);
    for s in &sparks {
        let Some(p) = project(cam, rect, s.position) else { continue };
        let [r, g, b] = s.color.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8);
        let color = egui::Color32::from_rgb(r, g, b);
        painter.circle_filled(p, 6.0 * s.level, color.gamma_multiply(0.18 * s.level));
        painter.circle_filled(p, 1.2 + 1.3 * s.level, color.gamma_multiply(s.level));
    }
    painter.text(
        rect.left_bottom() + egui::vec2(10.0, -48.0),
        egui::Align2::LEFT_BOTTOM,
        tf("Lucciole: {} accese su {}", &[sparks.len().to_string(), f.count.to_string()]),
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(225, 235, 200),
    );
}
