//! Scheda "Oggetto": trasformazioni non distruttive dei componenti selezionati.

use crate::app::ForgeApp;
use eframe::egui;
use forge_math::Vec3;
use forge_scene::UserTransform;

const MM: f32 = 1000.0;

fn axis_row(ui: &mut egui::Ui, label: &str, v: &mut Vec3, scale: f32, speed: f64, suffix: &str) -> bool {
    let mut changed = false;
    ui.label(label);
    ui.horizontal(|ui| {
        for (c, name) in [(&mut v.x, "X"), (&mut v.y, "Y"), (&mut v.z, "Z")] {
            let mut d = *c * scale;
            let r = ui.add(
                egui::DragValue::new(&mut d)
                    .speed(speed)
                    .max_decimals(3)
                    .prefix(format!("{name} "))
                    .suffix(suffix),
            );
            if r.changed() {
                *c = d / scale;
                changed = true;
            }
        }
    });
    ui.end_row();
    changed
}

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let sel: Vec<usize> = app.selection.iter().copied().collect();
    if sel.is_empty() {
        ui.label(
            egui::RichText::new("Seleziona un componente (clic nella viewport o nell'albero) per spostarlo, ruotarlo o ridimensionarlo.")
                .weak(),
        );
        tips(ui);
        return;
    }

    // Azioni rapide: valgono per tutti i componenti selezionati.
    let mut changed = false;
    // Duplica/elimina cambiano gli indici dei nodi: il resto del pannello si ridisegna al frame dopo.
    let mut structural = false;
    ui.horizontal_wrapped(|ui| {
        if ui
            .button("Duplica")
            .on_hover_text("Ctrl+D: copia identica nello stesso punto, già selezionata: trascinala col gizmo")
            .clicked()
        {
            app.duplicate_selection();
            structural = true;
        }
        if sel.iter().any(|&i| app.scene.is_copy(i) || app.scene.is_added(i))
            && ui
                .button("Elimina")
                .on_hover_text("Canc: elimina le copie e gli oggetti aggiunti selezionati")
                .clicked()
        {
            app.delete_selected_copies();
            structural = true;
        }
        if ui
            .button("Appoggia sul pavimento")
            .on_hover_text("Porta il punto più basso a quota 0")
            .clicked()
        {
            for &i in &sel {
                app.scene.drop_to_floor(i, 0.0);
            }
            changed = true;
        }
        if ui
            .button("Centra nell'origine")
            .on_hover_text("Porta il centro su X = 0, Z = 0")
            .clicked()
        {
            for &i in &sel {
                app.scene.center_on_origin(i);
            }
            changed = true;
        }
        if ui
            .button("Reimposta")
            .on_hover_text("Torna alla posizione del file originale")
            .clicked()
        {
            for &i in &sel {
                app.scene.nodes[i].user = UserTransform::default();
            }
            changed = true;
        }
    });
    if structural {
        return;
    }
    // Snap a contatto: impostazione dell'oggetto (salvata nel progetto), non globale.
    let mut contact = sel.iter().all(|&i| app.scene.nodes[i].contact_snap);
    if ui
        .checkbox(&mut contact, "Snap a contatto")
        .on_hover_text(
            "Spostandolo con il gizmo si ferma quando tocca un altro oggetto e non lo attraversa \
             (può comunque scivolare lungo le superfici). Vale per questo oggetto.",
        )
        .changed()
    {
        for &i in &sel {
            app.scene.nodes[i].contact_snap = contact;
        }
        app.unsaved = true;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Ruota 90°:");
        for (name, axis) in [("X", Vec3::X), ("Y", Vec3::Y), ("Z", Vec3::Z)] {
            if ui.button(name).clicked() {
                for &i in &sel {
                    let r = &mut app.scene.nodes[i].user.rotation_deg;
                    *r += axis * 90.0;
                    *r = Vec3::new(wrap180(r.x), wrap180(r.y), wrap180(r.z));
                }
                changed = true;
            }
        }
    });
    ui.separator();

    if sel.len() > 1 {
        ui.label(format!(
            "{} componenti selezionati: i valori numerici si modificano su un componente alla volta.",
            sel.len()
        ));
    } else {
        let i = sel[0];
        ui.strong(&app.scene.nodes[i].name);
        ui.label(egui::RichText::new(app.scene.nodes[i].id.to_string()).weak().small());
        let bounds = app.scene.bounds_of(i);
        let mut user = app.scene.nodes[i].user;
        egui::Grid::new("xf-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            // Dimensioni reali: modificandole si cambia la scala.
            if !bounds.is_empty() {
                let size = bounds.extent();
                let mut target = size;
                if axis_row(ui, "Dimensioni", &mut target, MM, 0.5, " mm") {
                    let lock = ui
                        .ctx()
                        .data(|d| d.get_temp::<bool>(egui::Id::NULL.with("xf-lock")).unwrap_or(true));
                    for k in 0..3 {
                        if size[k] > 1e-9 && (target[k] - size[k]).abs() > 1e-9 {
                            let f = target[k].max(1e-6) / size[k];
                            if lock {
                                user.scale *= f;
                            } else {
                                user.scale[k] *= f;
                            }
                        }
                    }
                    changed = true;
                }
                ui.label("");
                let mut lock = ui
                    .ctx()
                    .data(|d| d.get_temp::<bool>(egui::Id::NULL.with("xf-lock")).unwrap_or(true));
                if ui.checkbox(&mut lock, "Mantieni proporzioni").changed() {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::NULL.with("xf-lock"), lock));
                }
                ui.end_row();
            }
            changed |= axis_row(ui, "Spostamento", &mut user.translation, MM, 1.0, " mm");
            changed |= axis_row(ui, "Rotazione", &mut user.rotation_deg, 1.0, 0.5, "°");
            ui.label("Scala");
            ui.horizontal(|ui| {
                let mut pct = user.scale.x * 100.0;
                let uniform = (user.scale.x - user.scale.y).abs() < 1e-6 && (user.scale.x - user.scale.z).abs() < 1e-6;
                if uniform {
                    if ui
                        .add(egui::DragValue::new(&mut pct).speed(0.5).range(0.01..=100000.0).suffix(" %"))
                        .changed()
                    {
                        user.scale = Vec3::splat(pct / 100.0);
                        changed = true;
                    }
                } else {
                    for k in 0..3 {
                        let mut p = user.scale[k] * 100.0;
                        if ui
                            .add(egui::DragValue::new(&mut p).speed(0.5).range(0.01..=100000.0).suffix(" %"))
                            .changed()
                        {
                            user.scale[k] = p / 100.0;
                            changed = true;
                        }
                    }
                }
                for (label, f) in [("×0.1", 0.1), ("×10", 10.0), ("×25.4", 25.4)] {
                    if ui
                        .small_button(label)
                        .on_hover_text("Scala rapida (×25.4 = da pollici a mm)")
                        .clicked()
                    {
                        user.scale *= f;
                        changed = true;
                    }
                }
            });
            ui.end_row();
        });
        if changed {
            app.scene.nodes[i].user = user;
        }
        if !bounds.is_empty() {
            let c = bounds.center() * MM;
            ui.label(
                egui::RichText::new(format!(
                    "Centro attuale: X {:.1}  Y {:.1}  Z {:.1} mm · base a quota {:.1} mm",
                    c.x,
                    c.y,
                    c.z,
                    bounds.min.y * MM
                ))
                .weak()
                .small(),
            );
        }
    }
    if changed {
        app.geometry_changed();
    }
    ui.separator();
    tips(ui);
}

fn tips(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new(
            "Gizmo nella viewport: W sposta · E ruota · R scala · Q solo selezione · Ctrl durante il trascinamento = snap · Esc annulla.\n\
             H nasconde la selezione · Shift+H nasconde tutto il resto · Alt+H mostra tutto · Ctrl+Z / Ctrl+Y annulla e ripeti.\n\
             Ctrl+D duplica · Canc elimina una copia.\n\
             Alt + trascina sposta la selezione sul piano orizzontale, Alt + Shift + trascina in verticale.\n\
             Le trasformazioni non modificano il file CAD: vengono salvate nel progetto.",
        )
        .weak()
        .small(),
    );
}

fn wrap180(a: f32) -> f32 {
    let w = (a + 180.0).rem_euclid(360.0) - 180.0;
    if (w + 180.0).abs() < 1e-4 {
        180.0
    } else {
        w
    }
}
