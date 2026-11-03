//! Scheda Effetti: foschia d'ambiente (con la polvere in sospensione) e adesivi (decal).

use crate::app::ForgeApp;
use crate::i18n::{tf, tk, tr};
use eframe::egui;
use forge_scene::{Decal, Haze, NodeIndex};

/// Preset della foschia: (nome, descrizione, foschia, polvere accesa).
type HazePreset = (&'static str, &'static str, fn() -> Haze, bool);

const HAZE_PRESETS: [HazePreset; 5] = [
    (
        tk("Velo leggero"),
        tk("Aria appena velata: lo sfondo si allontana, i fasci si intuiscono"),
        || Haze {
            enabled: true,
            density: 0.05,
            variation: 0.4,
            swirl_scale: 1.2,
            anisotropy: 0.5,
            ..Default::default()
        },
        false,
    ),
    (
        tk("Raggi di luce"),
        tk("Foschia da controluce: le luci e le spot disegnano lame di luce con le ombre degli oggetti"),
        || Haze {
            enabled: true,
            density: 0.2,
            variation: 0.5,
            swirl_scale: 0.8,
            anisotropy: 0.75,
            ..Default::default()
        },
        false,
    ),
    (
        tk("Soffitta polverosa"),
        tk("Aria calda e polverosa: raggi dorati e granelli che brillano dove passa la luce"),
        || Haze {
            enabled: true,
            density: 0.25,
            color: [0.95, 0.9, 0.8],
            variation: 0.45,
            swirl_scale: 0.9,
            anisotropy: 0.7,
            ..Default::default()
        },
        true,
    ),
    (
        tk("Foschia fitta"),
        tk("Aria densa con banchi e vortici: gli oggetti lontani sbiadiscono"),
        || Haze {
            enabled: true,
            density: 0.6,
            variation: 0.6,
            swirl_scale: 0.6,
            anisotropy: 0.4,
            ..Default::default()
        },
        false,
    ),
    (
        tk("Nebbia bassa"),
        tk("Nebbia che si stende a terra e si dirada salendo"),
        || Haze {
            enabled: true,
            density: 0.9,
            variation: 0.7,
            swirl_scale: 0.5,
            anisotropy: 0.3,
            falloff_height: 0.25,
            ..Default::default()
        },
        false,
    ),
];

/// Pulsanti "Foschia d'ambiente" e "Adesivo" della riga "Aggiungi effetto".
pub fn add_buttons(app: &mut ForgeApp, ui: &mut egui::Ui) {
    if ui
        .add_enabled(!app.scene.haze.enabled, egui::Button::new(tr("Foschia d'ambiente")))
        .on_hover_text(tr(
            "Aria velata in tutta la scena: rende visibili i raggi delle luci (con le ombre degli oggetti), \
             con banchi e vortici. Si può aggiungere la polvere in sospensione",
        ))
        .on_disabled_hover_text(tr("Già nella scena: le impostazioni sono qui sotto"))
        .clicked()
    {
        app.scene.haze = (HAZE_PRESETS[1].2)();
        app.shading_changed();
    }
    if ui
        .button(tr("Adesivo (decal)"))
        .on_hover_text(tr(
            "Un'immagine (logo, etichetta, PNG con trasparenza) applicata sull'oggetto selezionato, nel \
             punto che guardi: segue anche le superfici curve e si sposta con l'oggetto",
        ))
        .clicked()
    {
        add_decal(app);
    }
}

fn pick_image() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter(tr("Immagini"), &["png", "jpg", "jpeg", "webp", "bmp", "tga"])
        .pick_file()
}

/// Proporzioni dell'immagine (altezza / larghezza).
fn aspect_of(path: &std::path::Path) -> f32 {
    image::image_dimensions(path)
        .map(|(w, h)| h as f32 / w.max(1) as f32)
        .unwrap_or(1.0)
}

/// Raggio dalla camera della vista verso il centro dei componenti selezionati.
fn view_ray(app: &ForgeApp, among: &[NodeIndex]) -> Option<(forge_math::Vec3, forge_math::Vec3, f32)> {
    let mut b = forge_math::Aabb::EMPTY;
    for &i in among {
        b.grow(&app.scene.bounds_of(i));
    }
    if b.is_empty() {
        return None;
    }
    let cam = &app.scene.camera;
    let d = (b.center() - cam.position).normalize_or(forge_math::Vec3::NEG_Z);
    Some((cam.position, d, (b.max - b.min).max_element()))
}

fn add_decal(app: &mut ForgeApp) {
    let among: Vec<NodeIndex> = app.selection.iter().copied().collect();
    let Some((o, d, size)) = view_ray(app, &among) else {
        app.toast(tr("Seleziona prima l'oggetto su cui applicare l'adesivo"));
        return;
    };
    let Some(image) = pick_image() else { return };
    let up = app.scene.camera.up;
    let width = (size * 0.3).clamp(0.005, 2.0);
    let Some(mut decal) = app.scene.new_decal(&among, o, d, up, width, aspect_of(&image)) else {
        app.toast(tr(
            "L'oggetto selezionato non si vede dalla vista: giralo verso la camera e riprova",
        ));
        return;
    };
    decal.name = image
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| decal.name.clone());
    decal.image = Some(image);
    app.scene.decals.push(decal);
    app.scene.sync_effects();
    app.geometry_changed();
    app.appearance_changed();
}

/// Foschia d'ambiente e polvere (se presenti) e adesivi della scena.
pub fn sections(app: &mut ForgeApp, ui: &mut egui::Ui) {
    if app.scene.haze.enabled {
        haze(app, ui);
        ui.separator();
    }
    if !app.scene.decals.is_empty() {
        decals(app, ui);
        ui.separator();
    }
}

fn haze(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let mut changed = false;
    let mut dust_changed = false;
    let mut remove = false;
    let id = ui.make_persistent_id("foschia-ambiente");
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            ui.strong(tr("Foschia d'ambiente"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").on_hover_text(tr("Togli la foschia (e la polvere)")).clicked() {
                    remove = true;
                }
            });
        })
        .body(|ui| {
            let h = &mut app.scene.haze;
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Preset:"));
                for (name, tip, make, dust) in HAZE_PRESETS {
                    if ui.small_button(tr(name)).on_hover_text(tr(tip)).clicked() {
                        // Il seme resta: cambia l'aria, non la forma dei vortici.
                        let seed = h.seed;
                        *h = make();
                        h.seed = seed;
                        if dust && !app.scene.haze_dust.enabled {
                            app.scene.haze_dust.enabled = true;
                            dust_changed = true;
                        }
                        changed = true;
                    }
                }
            });
            let h = &mut app.scene.haze;
            egui::Grid::new("foschia").num_columns(2).show(ui, |ui| {
                ui.label(tr("Densità"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.density, 0.005..=3.0).logarithmic(true).max_decimals(3))
                    .on_hover_text(tr(
                        "Quanto è spessa l'aria (1/m): 0,05 = velo, 0,2 = raggi ben visibili, oltre 1 = nebbia",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Colore"));
                changed |= ui
                    .color_edit_button_rgb(&mut h.color)
                    .on_hover_text(tr(
                        "Colore della luce diffusa dalla foschia: bianco = aria neutra, caldo = polvere, freddo = nebbia",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Raggi in controluce"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.anisotropy, 0.0..=0.9))
                    .on_hover_text(tr(
                        "Quanto la foschia manda la luce in avanti: alto = raggi brillanti guardando verso la luce, \
                         basso = alone uniforme",
                    ))
                    .changed();
                ui.end_row();
                ui.label(tr("Vortici"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.variation, 0.0..=1.0))
                    .on_hover_text(tr("Quanto l'aria è irregolare: 0 = uniforme, 1 = banchi densi, riccioli e vuoti"))
                    .changed();
                ui.end_row();
                ui.label(tr("Grandezza dei vortici"));
                changed |= ui
                    .add(
                        egui::Slider::new(&mut h.swirl_scale, 0.1..=5.0)
                            .logarithmic(true)
                            .suffix(" m")
                            .max_decimals(2),
                    )
                    .on_hover_text(tr("Dimensione tipica dei banchi e dei riccioli"))
                    .changed();
                ui.end_row();
                ui.label(tr("Si dirada salendo"));
                changed |= ui
                    .add(egui::Slider::new(&mut h.falloff_height, 0.0..=5.0).suffix(" m").max_decimals(2))
                    .on_hover_text(tr("Altezza in cui la foschia si dimezza salendo dal punto più basso della scena: \
                         0 = uguale ovunque, piccola = nebbia a terra"))
                    .changed();
                ui.end_row();
                ui.label(tr("Forma"));
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut h.seed).range(1..=9999)).changed();
                    if ui
                        .small_button(tr("Nuova"))
                        .on_hover_text(tr("Altra disposizione dei vortici"))
                        .clicked()
                    {
                        h.seed = h.seed % 9999 + 1;
                        changed = true;
                    }
                });
                ui.end_row();
            });
            ui.separator();
            let dust = &mut app.scene.haze_dust;
            dust_changed |= ui
                .checkbox(&mut dust.enabled, tr("Polvere in sospensione"))
                .on_hover_text(tr(
                    "Granelli minuscoli nell'aria: al buio quasi non si vedono, dove li colpisce un raggio (o la luce \
                     di rimbalzo) brillano, come in soffitta controluce",
                ))
                .changed();
            if dust.enabled {
                egui::Grid::new("polvere").num_columns(2).show(ui, |ui| {
                    ui.label(tr("Quantità"));
                    dust_changed |= ui
                        .add(
                            egui::Slider::new(&mut dust.per_m3, 100.0..=30000.0)
                                .logarithmic(true)
                                .suffix(tr(" /m³"))
                                .max_decimals(0),
                        )
                        .on_hover_text(tr("Granelli per metro cubo (al massimo 150 000 in tutto)"))
                        .changed();
                    ui.end_row();
                    ui.label(tr("Dimensione"));
                    dust_changed |= ui
                        .add(egui::Slider::new(&mut dust.size_mm, 0.1..=3.0).suffix(" mm").max_decimals(2))
                        .on_hover_text(tr("Diametro medio dei granelli: più grandi = puntini più visibili e sfocati"))
                        .changed();
                    ui.end_row();
                    ui.label(tr("Colore"));
                    dust_changed |= ui.color_edit_button_rgb(&mut dust.color).changed();
                    ui.end_row();
                });
            }
        });
    if remove {
        app.scene.haze.enabled = false;
        dust_changed = true;
        changed = true;
    }
    if dust_changed && !(app.scene.haze.enabled && app.scene.haze_dust.enabled) {
        // Il nodo della polvere sparisce: gli indici dei nodi dopo di lui cambiano.
        app.selection.clear();
    }
    if dust_changed {
        app.scene.sync_effects();
        app.geometry_changed();
        app.appearance_changed();
    } else if changed {
        app.shading_changed();
    }
}

fn decals(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.strong(tf("Adesivi ({})", &[app.scene.decals.len().to_string()]));
    let mut geometry = false;
    let mut look = false;
    let mut remove = None;
    let mut replace = None;
    let mut reposition = None;
    for (i, d) in app.scene.decals.iter_mut().enumerate() {
        ui.push_id(("adesivo", d.n), |ui| {
            let id = ui.make_persistent_id("adesivo");
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    geometry |= ui
                        .checkbox(&mut d.enabled, "")
                        .on_hover_text(tr("Visibile (togliendo la spunta resta nella scena, nascosto)"))
                        .changed();
                    ui.label(&d.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").on_hover_text(tr("Elimina l'adesivo")).clicked() {
                            remove = Some(i);
                        }
                    });
                })
                .body(|ui| {
                    ui.horizontal(|ui| {
                        let name = d
                            .image
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| tr("nessuna immagine").into());
                        ui.label(name);
                        if ui.small_button(tr("Cambia immagine…")).clicked() {
                            replace = Some(i);
                        }
                    });
                    egui::Grid::new("adesivo-grid").num_columns(2).show(ui, |ui| {
                        let aspect = d.height / d.width.max(1e-6);
                        ui.label(tr("Larghezza"));
                        if ui
                            .add(
                                egui::Slider::new(&mut d.width, 0.002..=3.0)
                                    .logarithmic(true)
                                    .suffix(" m")
                                    .max_decimals(3),
                            )
                            .on_hover_text(tr("L'altezza segue, con le proporzioni dell'immagine"))
                            .changed()
                        {
                            d.height = d.width * aspect;
                            d.depth = d.depth.max(d.width.max(d.height) * 0.25);
                            geometry = true;
                        }
                        ui.end_row();
                        ui.label(tr("Rotazione"));
                        geometry |= ui.add(egui::Slider::new(&mut d.rotation_deg, -180.0..=180.0).suffix("°")).changed();
                        ui.end_row();
                        ui.label(tr("Sposta"));
                        ui.horizontal(|ui| {
                            for (k, label) in [(0, "↔"), (1, "↕")] {
                                geometry |= ui
                                    .add(
                                        egui::DragValue::new(&mut d.offset[k])
                                            .speed(0.0005)
                                            .prefix(format!("{label} "))
                                            .suffix(" m")
                                            .max_decimals(4),
                                    )
                                    .on_hover_text(tr("Sposta l'adesivo sulla superficie (orizzontale, verticale)"))
                                    .changed();
                            }
                        });
                        ui.end_row();
                        ui.label(tr("Profondità"));
                        geometry |= ui
                            .add(
                                egui::Slider::new(&mut d.depth, 0.001..=1.0)
                                    .logarithmic(true)
                                    .suffix(" m")
                                    .max_decimals(3),
                            )
                            .on_hover_text(tr(
                                "Fin dove arriva la proiezione davanti e dietro il punto: più alta per superfici \
                                 molto curve, più bassa se l'adesivo sbuca anche sul lato opposto di un oggetto sottile",
                            ))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Ruvidità"));
                        look |= ui
                            .add(egui::Slider::new(&mut d.roughness, 0.0..=1.0))
                            .on_hover_text(tr("0 = adesivo lucido (riflessi netti), 1 = carta opaca"))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Metallicità"));
                        look |= ui
                            .add(egui::Slider::new(&mut d.metallic, 0.0..=1.0))
                            .on_hover_text(tr("Per etichette metallizzate (oro, argento)"))
                            .changed();
                        ui.end_row();
                    });
                    if ui
                        .button(tr("Riposiziona dalla vista"))
                        .on_hover_text(tr("Rimette l'adesivo al centro dell'oggetto visto dalla camera di adesso"))
                        .clicked()
                    {
                        reposition = Some(i);
                    }
                    ui.label(
                        egui::RichText::new(tr("Usa un PNG con la trasparenza: le parti trasparenti lasciano vedere l'oggetto"))
                            .weak()
                            .small(),
                    );
                });
        });
    }
    if let Some(i) = replace {
        if let Some(p) = pick_image() {
            let d = &mut app.scene.decals[i];
            d.height = d.width * aspect_of(&p);
            d.image = Some(p);
            geometry = true;
            look = true;
        }
    }
    if let Some(i) = reposition {
        let target = app.scene.find(&app.scene.decals[i].target);
        if let Some((o, dir, _)) = target.and_then(|t| view_ray(app, &[t])) {
            let up = app.scene.camera.up;
            let old = app.scene.decals[i].clone();
            if let Some(nd) = app
                .scene
                .new_decal(&[target.unwrap()], o, dir, up, old.width, old.height / old.width.max(1e-6))
            {
                app.scene.decals[i] = Decal {
                    n: old.n,
                    name: old.name,
                    image: old.image,
                    roughness: old.roughness,
                    metallic: old.metallic,
                    enabled: old.enabled,
                    ..nd
                };
                geometry = true;
            }
        }
    }
    if let Some(i) = remove {
        app.scene.decals.remove(i);
        app.selection.clear();
        geometry = true;
    }
    if geometry || look {
        app.scene.sync_effects();
        if geometry {
            app.geometry_changed();
        }
        app.appearance_changed();
    }
}
