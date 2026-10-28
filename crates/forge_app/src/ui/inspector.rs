use crate::app::{ForgeApp, InspectorTab, RenderView};
use crate::ui::style;
use eframe::egui;
use forge_math::Vec3;
use forge_render_core::ToneMapping;
use forge_scene::{light, Environment, Light, LightKind, Material, MaterialId};

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        for (tab, label) in [
            (InspectorTab::Object, "Oggetto"),
            (InspectorTab::Materials, "Materiali"),
            (InspectorTab::Lights, "Luci"),
            (InspectorTab::Environment, "Ambiente"),
            (InspectorTab::Camera, "Camera"),
            (InspectorTab::Render, "Render"),
        ] {
            ui.selectable_value(&mut app.tab, tab, label);
        }
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("inspector-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| match app.tab {
            InspectorTab::Object => crate::ui::transform::show(app, ui),
            InspectorTab::Materials => materials(app, ui),
            InspectorTab::Lights => lights(app, ui),
            InspectorTab::Environment => environment(app, ui),
            InspectorTab::Camera => camera(app, ui),
            InspectorTab::Render => render(app, ui),
        });
}

fn vec3_edit(ui: &mut egui::Ui, label: &str, v: &mut Vec3, speed: f64) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        for c in [&mut v.x, &mut v.y, &mut v.z] {
            changed |= ui.add(egui::DragValue::new(c).speed(speed).max_decimals(4)).changed();
        }
    });
    changed
}

fn unavailable(ui: &mut egui::Ui, what: &str, phase: &str) {
    ui.add_enabled(false, egui::Label::new(format!("{what}: non disponibile ({phase})")));
}

// ----------------------------------------------------------------------
// Materiali
// ----------------------------------------------------------------------

fn materials(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let n_sel = app.selection.len();
    if n_sel == 0 {
        ui.label(
            egui::RichText::new("Nessun componente selezionato (clic nella viewport o nell'albero; Ctrl+clic per selezione multipla).")
                .weak(),
        );
    } else {
        let names: Vec<String> = app.selection.iter().take(3).map(|&i| app.scene.nodes[i].name.clone()).collect();
        ui.label(format!(
            "Selezione: {}{}",
            names.join(", "),
            if n_sel > 3 {
                format!(" e altri {}", n_sel - 3)
            } else {
                String::new()
            }
        ));
    }

    // Libreria raggruppata per categoria.
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong(format!("Libreria materiali ({})", app.scene.materials.materials.len()));
        let id = egui::Id::NULL.with("matlib-search");
        let mut q = ui.ctx().data(|d| d.get_temp::<String>(id).unwrap_or_default());
        if ui
            .add(egui::TextEdit::singleline(&mut q).hint_text("cerca…").desired_width(120.0))
            .changed()
        {
            ui.ctx().data_mut(|d| d.insert_temp(id, q.clone()));
        }
    });
    let query = ui
        .ctx()
        .data(|d| d.get_temp::<String>(egui::Id::NULL.with("matlib-search")).unwrap_or_default())
        .to_lowercase();
    let mut categories: Vec<String> = app.scene.materials.materials.iter().map(|m| m.category.clone()).collect();
    categories.sort();
    categories.dedup();
    egui::ScrollArea::vertical().id_salt("matlib").max_height(280.0).show(ui, |ui| {
        for cat in &categories {
            let items: Vec<&forge_scene::Material> = app
                .scene
                .materials
                .materials
                .iter()
                .filter(|m| &m.category == cat)
                .filter(|m| query.is_empty() || m.name.to_lowercase().contains(&query) || m.category.to_lowercase().contains(&query))
                .collect();
            if items.is_empty() {
                continue;
            }
            egui::CollapsingHeader::new(format!("{cat} ({})", items.len()))
                .id_salt(cat)
                .default_open(!query.is_empty())
                .show(ui, |ui| {
                    for m in items {
                        let usage = app.scene.assignments.values().filter(|v| **v == m.id).count();
                        let label = if usage > 0 {
                            format!("{}  ({usage})", m.name)
                        } else {
                            m.name.clone()
                        };
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            let col = if m.is_emissive() {
                                Vec3::from_array(m.emission)
                            } else {
                                Vec3::from_array(m.base_color)
                            };
                            ui.painter().rect_filled(r, 2.0, crate::viewport::to_egui_color(col));
                            if ui.selectable_label(app.selected_material == m.id, label).clicked() {
                                app.selected_material = m.id.clone();
                            }
                        });
                    }
                });
        }
    });

    if ui
        .button("Libreria con texture (ambientCG, Poly Haven)…")
        .on_hover_text("Materiali PBR veri con tutte le mappe: la tua libreria, import da cartella/zip e download")
        .clicked()
    {
        app.library.open = true;
    }

    // Assegnazione.
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(n_sel > 0, egui::Button::new("Assegna alla selezione")).clicked() {
            for &i in &app.selection {
                app.scene
                    .assignments
                    .insert(app.scene.nodes[i].id.clone(), app.selected_material.clone());
            }
            app.appearance_changed();
        }
        if ui
            .add_enabled(n_sel > 0, egui::Button::new("Rimuovi assegnazione"))
            .on_hover_text("Il componente torna a ereditare il materiale dal genitore o quello predefinito")
            .clicked()
        {
            for &i in &app.selection {
                app.scene.assignments.remove(&app.scene.nodes[i].id);
            }
            app.appearance_changed();
        }
    });
    ui.horizontal(|ui| {
        ui.label("Nome contiene:");
        ui.add(egui::TextEdit::singleline(&mut app.rule_pattern).desired_width(110.0));
        let pat = app.rule_pattern.trim().to_lowercase();
        let matches: Vec<usize> = if pat.is_empty() {
            Vec::new()
        } else {
            (0..app.scene.nodes.len())
                .filter(|&i| app.scene.nodes[i].name.to_lowercase().contains(&pat))
                .collect()
        };
        if ui
            .add_enabled(!matches.is_empty(), egui::Button::new(format!("Assegna ({})", matches.len())))
            .on_hover_text("Assegna il materiale a tutti i componenti il cui nome contiene il testo")
            .clicked()
        {
            for i in matches {
                app.scene
                    .assignments
                    .insert(app.scene.nodes[i].id.clone(), app.selected_material.clone());
            }
            app.appearance_changed();
        }
    });
    let is_default = app.scene.default_material == app.selected_material;
    if ui
        .add_enabled(
            !is_default,
            egui::Button::new("Usa come predefinito per i componenti senza assegnazione"),
        )
        .clicked()
    {
        app.scene.default_material = app.selected_material.clone();
        app.appearance_changed();
    }

    ui.horizontal(|ui| {
        if ui.button("Nuovo").clicked() {
            let id = app.scene.materials.unique_id("materiale");
            app.scene.materials.upsert(Material {
                id: id.clone(),
                name: "Nuovo materiale".into(),
                category: "Personalizzati".into(),
                ..Default::default()
            });
            app.selected_material = id;
            app.appearance_changed();
        }
        if ui.button("Duplica").clicked() {
            if let Some(m) = app.scene.materials.get(&app.selected_material).cloned() {
                let id = app.scene.materials.unique_id(&format!("{}-copia", m.id.0));
                app.scene.materials.upsert(Material {
                    id: id.clone(),
                    name: format!("{} (copia)", m.name),
                    category: "Personalizzati".into(),
                    ..m
                });
                app.selected_material = id;
                app.appearance_changed();
            }
        }
        let deletable = app.selected_material != MaterialId::new("default");
        if ui
            .add_enabled(deletable, egui::Button::new("Elimina"))
            .on_hover_text("I componenti che lo usavano tornano al materiale ereditato/predefinito")
            .clicked()
        {
            let id = app.selected_material.clone();
            app.scene.materials.materials.retain(|m| m.id != id);
            app.scene.assignments.retain(|_, v| *v != id);
            if app.scene.default_material == id {
                app.scene.default_material = MaterialId::new("default");
            }
            app.selected_material = MaterialId::new("default");
            app.appearance_changed();
        }
    });

    ui.separator();
    // Lato più lungo dei componenti selezionati (per contare le ripetizioni della texture).
    let obj_size = {
        let mut b = forge_math::Aabb::EMPTY;
        for &i in &app.selection {
            b.grow(&app.scene.bounds_of(i));
        }
        (!b.is_empty()).then(|| (b.max - b.min).max_element()).filter(|s| *s > 0.0)
    };
    let Some(m) = app.scene.materials.get_mut(&app.selected_material) else {
        return;
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Nome");
        changed |= ui.text_edit_singleline(&mut m.name).changed();
    });
    if m.has_textures() {
        egui::CollapsingHeader::new("Scala e posizionamento texture")
            .default_open(true)
            .show(ui, |ui| changed |= mapping_controls(ui, m, obj_size));
        if m.normal_texture.is_some() || m.height_texture.is_some() {
            egui::CollapsingHeader::new("Rilievo (normal, bump, displacement)")
                .default_open(true)
                .show(ui, |ui| changed |= relief_controls(ui, m));
        }
    }
    egui::Grid::new("mat-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Colore base");
        changed |= ui.color_edit_button_rgb(&mut m.base_color).changed();
        ui.end_row();
        ui.label("Metallic");
        changed |= ui.add(egui::Slider::new(&mut m.metallic, 0.0..=1.0)).changed();
        ui.end_row();
        ui.label("Roughness");
        changed |= ui.add(egui::Slider::new(&mut m.roughness, 0.0..=1.0)).changed();
        ui.end_row();
        ui.label("Specular");
        changed |= ui
            .add(egui::Slider::new(&mut m.specular, 0.0..=1.0))
            .on_hover_text("0.5 = riflesso fisico derivato dall'IOR")
            .changed();
        ui.end_row();
        ui.label("IOR");
        changed |= ui.add(egui::Slider::new(&mut m.ior, 1.0..=3.0)).changed();
        ui.end_row();
        ui.label("Transmission");
        changed |= ui
            .add(egui::Slider::new(&mut m.transmission, 0.0..=1.0))
            .on_hover_text("Trasparenza dielettrica (vetro). In questa versione la trasmissione è liscia.")
            .changed();
        ui.end_row();
        ui.label("Emissione");
        changed |= ui.color_edit_button_rgb(&mut m.emission).changed();
        ui.end_row();
        ui.label("Intensità emissione");
        changed |= ui
            .add(egui::DragValue::new(&mut m.emission_strength).speed(0.1).range(0.0..=10000.0))
            .changed();
        ui.end_row();
        ui.label("Categoria");
        changed |= ui.text_edit_singleline(&mut m.category).changed();
        ui.end_row();
    });
    ui.add_space(4.0);
    ui.strong("Texture");
    egui::Grid::new("tex-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Colore base");
        if texture_slot(ui, &mut m.base_color_texture, "base") {
            if m.base_color_texture.is_some() {
                m.base_color = [1.0; 3]; // la texture viene moltiplicata per il colore base
            }
            changed = true;
        }
        ui.end_row();
        ui.label("Roughness");
        if texture_slot(ui, &mut m.roughness_texture, "rough") {
            if m.roughness_texture.is_some() {
                m.roughness = 1.0; // la texture (canale R) viene moltiplicata per la roughness
            }
            changed = true;
        }
        ui.end_row();
        if m.roughness_texture.is_some() {
            ui.label("");
            changed |= ui
                .checkbox(&mut m.roughness_is_gloss, "È una mappa di lucidità (gloss)")
                .on_hover_text("Quixel e alcuni flussi Substance usano «gloss»: chiaro = lucido. Viene invertita.")
                .changed();
            ui.end_row();
        }
        ui.label("Metallicità");
        if texture_slot(ui, &mut m.metallic_texture, "metal") {
            if m.metallic_texture.is_some() {
                m.metallic = 1.0; // la texture viene moltiplicata per la metallicità
            }
            changed = true;
        }
        ui.end_row();
        ui.label("Normal map");
        changed |= texture_slot(ui, &mut m.normal_texture, "normal");
        ui.end_row();
        if m.normal_texture.is_some() {
            ui.label("");
            changed |= ui
                .checkbox(&mut m.normal_directx, "Formato DirectX")
                .on_hover_text("Le normal map «DX» (Unreal, Quixel, *_NormalDX) hanno il verde invertito rispetto a OpenGL")
                .changed();
            ui.end_row();
        }
        ui.label("Rilievo (height)");
        changed |= texture_slot(ui, &mut m.height_texture, "height");
        ui.end_row();
        ui.label("Occlusione (AO)");
        changed |= texture_slot(ui, &mut m.ao_texture, "ao");
        ui.end_row();
        if m.ao_texture.is_some() {
            ui.label("Intensità AO");
            changed |= ui
                .add(egui::Slider::new(&mut m.ao_strength, 0.0..=1.0))
                .on_hover_text("Scurisce le cavità. Il path tracer calcola già l'occlusione reale: valori bassi sono più fedeli.")
                .changed();
            ui.end_row();
        }
        ui.label("Opacità (ritaglio)");
        changed |= texture_slot(ui, &mut m.opacity_texture, "opacity");
        ui.end_row();
    });
    if changed {
        app.appearance_changed();
    }
    crate::ui::central::material_preview(app, ui);
}

/// Scala, proiezione, rotazione e spostamento delle texture del materiale.
fn mapping_controls(ui: &mut egui::Ui, m: &mut Material, obj_size: Option<f32>) -> bool {
    let mut changed = false;
    let mp = &mut m.mapping;
    ui.horizontal(|ui| {
        ui.label("Proiezione");
        changed |= ui
            .radio_value(&mut mp.projection, forge_scene::Projection::Box, "Box (CAD)")
            .on_hover_text("Proiezione cubica in spazio mondo: non richiede UV, la scala è in metri reali")
            .changed();
        changed |= ui
            .radio_value(&mut mp.projection, forge_scene::Projection::Uv, "UV della mesh")
            .on_hover_text("Usa le coordinate UV del modello (glTF/OBJ esportati con UV)")
            .changed();
    });
    match mp.projection {
        forge_scene::Projection::Box => {
            ui.label("Dimensione di una ripetizione")
                .on_hover_text("Lato reale di una piastrella della texture: più piccola = più ripetizioni (es. mattoni più piccoli)");
            ui.horizontal(|ui| {
                changed |= ui
                    .add(
                        egui::Slider::new(&mut mp.scale, 0.002..=50.0)
                            .logarithmic(true)
                            .custom_formatter(|v, _| fmt_len(v as f32))
                            .custom_parser(parse_len),
                    )
                    .changed();
                if ui
                    .small_button("÷2")
                    .on_hover_text("Texture più piccola (il doppio delle ripetizioni)")
                    .clicked()
                {
                    mp.scale = (mp.scale * 0.5).max(0.0001);
                    changed = true;
                }
                if ui
                    .small_button("×2")
                    .on_hover_text("Texture più grande (metà delle ripetizioni)")
                    .clicked()
                {
                    mp.scale = (mp.scale * 2.0).min(1000.0);
                    changed = true;
                }
            });
            if let Some(size) = obj_size {
                let mut reps = size / mp.scale.max(1e-6);
                ui.horizontal(|ui| {
                    ui.label("Ripetizioni sull'oggetto");
                    if ui
                        .add(egui::DragValue::new(&mut reps).speed(0.1).range(0.01..=100000.0).max_decimals(1))
                        .on_hover_text(format!(
                            "Quante volte la texture si ripete sul lato più lungo dell'oggetto selezionato ({})",
                            fmt_len(size)
                        ))
                        .changed()
                    {
                        mp.scale = (size / reps.max(0.01)).clamp(0.0001, 1000.0);
                        changed = true;
                    }
                });
            }
        }
        forge_scene::Projection::Uv => {
            ui.horizontal(|ui| {
                ui.label("Ripetizioni");
                changed |= ui
                    .add(egui::Slider::new(&mut mp.scale, 0.01..=1000.0).logarithmic(true).max_decimals(2))
                    .on_hover_text("Quante volte la texture si ripete sull'intervallo UV 0–1 del modello")
                    .changed();
            });
        }
    }
    ui.horizontal(|ui| {
        ui.label("Rotazione");
        changed |= ui
            .add(egui::Slider::new(&mut mp.rotation_deg, -180.0..=180.0).suffix("°"))
            .changed();
    });
    ui.horizontal(|ui| {
        ui.label("Spostamento");
        changed |= ui.add(egui::DragValue::new(&mut mp.offset[0]).speed(0.01).prefix("U ")).changed();
        changed |= ui.add(egui::DragValue::new(&mut mp.offset[1]).speed(0.01).prefix("V ")).changed();
        if ui.small_button("Azzera").clicked() {
            mp.offset = [0.0; 2];
            mp.rotation_deg = 0.0;
            changed = true;
        }
    });
    changed
}

/// Normal map, bump e displacement vero.
fn relief_controls(ui: &mut egui::Ui, m: &mut Material) -> bool {
    let mut changed = false;
    egui::Grid::new("relief-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        if m.normal_texture.is_some() {
            ui.label("Intensità normal map");
            changed |= ui
                .add(egui::Slider::new(&mut m.normal_strength, 0.0..=4.0))
                .on_hover_text("Dettaglio fine dell'ombreggiatura (non cambia la sagoma)")
                .changed();
            ui.end_row();
        }
        if m.height_texture.is_some() {
            ui.label("Bump (mm)");
            changed |= ui
                .add(egui::Slider::new(&mut m.bump_mm, 0.0..=20.0).max_decimals(2))
                .on_hover_text("Bump mapping dalla height: solo ombreggiatura. 0 = spento (con la normal map di solito non serve)")
                .changed();
            ui.end_row();
            ui.label("Displacement (mm)");
            changed |= ui
                .add(egui::Slider::new(&mut m.displacement_mm, 0.0..=200.0).max_decimals(1))
                .on_hover_text(
                    "Displacement vero: la superficie viene suddivisa e spostata secondo la height map, \
                     cambia anche la sagoma e le ombre. 0 = spento. Più pesante da preparare.",
                )
                .changed();
            ui.end_row();
            if m.displacement_mm > 0.0 {
                ui.label("Livello medio");
                changed |= ui
                    .add(egui::Slider::new(&mut m.displacement_mid, 0.0..=1.0))
                    .on_hover_text("Grigio che lascia la superficie dov'è: 0.5 = metà in fuori e metà in dentro, 0 = solo in fuori")
                    .changed();
                ui.end_row();
            }
        }
    });
    if m.displacement_mm > 0.0 && m.height_texture.is_some() {
        ui.label(
            egui::RichText::new("Il displacement si vede nel render (non nella viewport di modellazione).")
                .small()
                .weak(),
        );
    }
    changed
}

fn fmt_len(m: f32) -> String {
    if m < 0.01 {
        format!("{:.1} mm", m * 1000.0)
    } else if m < 1.0 {
        format!("{:.1} cm", m * 100.0)
    } else {
        format!("{m:.2} m")
    }
}

/// Legge "25 cm", "3mm", "1.5 m" o un numero (metri).
fn parse_len(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".").to_lowercase();
    let (num, mul) = if let Some(x) = t.strip_suffix("mm") {
        (x, 0.001)
    } else if let Some(x) = t.strip_suffix("cm") {
        (x, 0.01)
    } else if let Some(x) = t.strip_suffix('m') {
        (x, 1.0)
    } else {
        (t.as_str(), 1.0)
    };
    num.trim().parse::<f64>().ok().map(|v| v * mul)
}

/// Slot di una texture: nome del file, "Scegli…" e rimozione. Restituisce `true` se cambiato.
fn texture_slot(ui: &mut egui::Ui, slot: &mut Option<std::path::PathBuf>, id: &str) -> bool {
    let mut changed = false;
    ui.push_id(id, |ui| {
        ui.horizontal(|ui| {
            match slot {
                Some(p) => {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    ui.label(name).on_hover_text(p.display().to_string());
                    if ui.small_button("×").on_hover_text("Rimuovi texture").clicked() {
                        *slot = None;
                        changed = true;
                    }
                }
                None => {
                    ui.label(egui::RichText::new("nessuna").weak());
                }
            }
            if ui.small_button("Scegli…").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Immagini", &["png", "jpg", "jpeg", "exr", "hdr"])
                    .pick_file()
                {
                    *slot = Some(p);
                    changed = true;
                }
            }
        });
    });
    changed
}

// ----------------------------------------------------------------------
// Luci
// ----------------------------------------------------------------------

fn lights(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let b = app.scene.bounds();
    let (center, radius) = if b.is_empty() {
        (Vec3::ZERO, 1.0)
    } else {
        (b.center(), b.radius())
    };
    ui.horizontal_wrapped(|ui| {
        ui.label("Preset:");
        for (id, name) in light::PRESETS {
            if ui
                .button(name)
                .on_hover_text("Sostituisce tutte le luci, adattandole alle dimensioni della scena")
                .clicked()
            {
                app.scene.lights = light::preset(id, center, radius).unwrap_or_default();
                app.selected_light = None;
                app.shading_changed();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Aggiungi:");
        let at = center + Vec3::new(1.0, 2.0, 1.5) * radius;
        let mut add = |l: Light| {
            app.scene.lights.push(l);
            app.selected_light = Some(app.scene.lights.len() - 1);
            app.shading_changed();
        };
        if ui.button("Area").clicked() {
            add(Light::area("Luce area", at, center, radius, radius, 5.0));
        }
        if ui.button("Punto").clicked() {
            add(Light::point("Luce puntiforme", at, 20.0 * radius * radius));
        }
        if ui.button("Spot").clicked() {
            add(Light {
                kind: LightKind::Spot {
                    inner_angle_deg: 25.0,
                    outer_angle_deg: 40.0,
                },
                target: center,
                ..Light::point("Spot", at, 40.0 * radius * radius)
            });
        }
        if ui.button("Direzionale").clicked() {
            add(Light {
                kind: LightKind::Directional,
                target: center,
                ..Light::point("Sole", at, 3.0)
            });
        }
    });
    ui.separator();

    let Some(i) = app.selected_light.filter(|&i| i < app.scene.lights.len()) else {
        ui.label(egui::RichText::new("Seleziona una luce nell'elenco a sinistra.").weak());
        return;
    };
    let mut changed = false;
    let mut delete = false;
    let mut duplicate = false;
    {
        let l = &mut app.scene.lights[i];
        ui.horizontal(|ui| {
            ui.label("Nome");
            changed |= ui.text_edit_singleline(&mut l.name).changed();
        });
        let mut kind_idx = match l.kind {
            LightKind::Area { .. } => 0,
            LightKind::Point => 1,
            LightKind::Spot { .. } => 2,
            LightKind::Directional => 3,
        };
        let before = kind_idx;
        egui::ComboBox::from_label("Tipo").selected_text(l.kind.label()).show_ui(ui, |ui| {
            ui.selectable_value(&mut kind_idx, 0, "Area (rettangolare)");
            ui.selectable_value(&mut kind_idx, 1, "Puntiforme");
            ui.selectable_value(&mut kind_idx, 2, "Spot");
            ui.selectable_value(&mut kind_idx, 3, "Direzionale");
        });
        if kind_idx != before {
            let r = radius;
            l.kind = match kind_idx {
                0 => LightKind::Area { width: r, height: r },
                1 => LightKind::Point,
                2 => LightKind::Spot {
                    inner_angle_deg: 25.0,
                    outer_angle_deg: 40.0,
                },
                _ => LightKind::Directional,
            };
            changed = true;
        }
        changed |= ui.checkbox(&mut l.enabled, "Accesa").changed();
        egui::Grid::new("light-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Colore");
            changed |= ui.color_edit_button_rgb(&mut l.color).changed();
            ui.end_row();
            ui.label("Temperatura");
            ui.horizontal(|ui| {
                for k in [2700.0, 4000.0, 5500.0, 6500.0, 9000.0] {
                    if ui.small_button(format!("{}K", k as u32)).clicked() {
                        l.color = light::kelvin_to_rgb(k);
                        changed = true;
                    }
                }
            });
            ui.end_row();
            let unit = match l.kind {
                LightKind::Area { .. } => "Radianza (W/sr/m²)",
                LightKind::Directional => "Irradianza (W/m²)",
                _ => "Intensità (W/sr)",
            };
            ui.label(unit);
            changed |= ui
                .add(egui::DragValue::new(&mut l.intensity).speed(0.05).range(0.0..=1e7))
                .changed();
            ui.end_row();
            match &mut l.kind {
                LightKind::Area { width, height } => {
                    ui.label("Larghezza (m)");
                    changed |= ui.add(egui::DragValue::new(width).speed(0.01).range(1e-4..=1e4)).changed();
                    ui.end_row();
                    ui.label("Altezza (m)");
                    changed |= ui.add(egui::DragValue::new(height).speed(0.01).range(1e-4..=1e4)).changed();
                    ui.end_row();
                }
                LightKind::Spot {
                    inner_angle_deg,
                    outer_angle_deg,
                } => {
                    // Apertura = angolo pieno del cono; sfumatura = parte del bordo che passa
                    // gradualmente da luce piena a buio (angolo interno = apertura × (1 − sfumatura)).
                    let mut soft = if *outer_angle_deg > 0.0 {
                        (1.0 - *inner_angle_deg / *outer_angle_deg).clamp(0.0, 1.0) * 100.0
                    } else {
                        0.0
                    };
                    ui.label("Apertura cono (°)");
                    if ui
                        .add(egui::Slider::new(outer_angle_deg, 1.0..=170.0))
                        .on_hover_text("Angolo pieno del fascio (si regola anche trascinando la maniglia piena del cono nella viewport)")
                        .changed()
                    {
                        *inner_angle_deg = *outer_angle_deg * (1.0 - soft / 100.0);
                        changed = true;
                    }
                    ui.end_row();
                    ui.label("Sfumatura bordo (%)");
                    if ui
                        .add(egui::Slider::new(&mut soft, 0.0..=100.0).suffix(" %"))
                        .on_hover_text("0 % = bordo netto, 100 % = sfuma dal centro (maniglia vuota nella viewport)")
                        .changed()
                    {
                        *inner_angle_deg = *outer_angle_deg * (1.0 - soft / 100.0);
                        changed = true;
                    }
                    ui.end_row();
                }
                _ => {}
            }
        });
        changed |= vec3_edit(ui, "Posizione", &mut l.position, (radius * 0.01) as f64);
        if !matches!(l.kind, LightKind::Point) {
            changed |= vec3_edit(ui, "Bersaglio", &mut l.target, (radius * 0.01) as f64);
            if ui.small_button("Punta al centro della scena").clicked() {
                l.target = center;
                changed = true;
            }
        }
        changed |= ui.checkbox(&mut l.cast_shadows, "Proietta ombre").changed();
        if matches!(l.kind, LightKind::Area { .. }) {
            changed |= ui.checkbox(&mut l.visible_to_camera, "Visibile alla camera").changed();
        }
        ui.horizontal(|ui| {
            duplicate = ui.button("Duplica").clicked();
            delete = ui.button("Elimina").clicked();
        });
    }
    if duplicate {
        let mut l = app.scene.lights[i].clone();
        l.name.push_str(" (copia)");
        app.scene.lights.push(l);
        app.selected_light = Some(app.scene.lights.len() - 1);
        changed = true;
    }
    if delete {
        app.scene.lights.remove(i);
        app.selected_light = None;
        changed = true;
    }
    if changed {
        app.shading_changed();
    }
}

// ----------------------------------------------------------------------
// Ambiente
// ----------------------------------------------------------------------

fn environment(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let mut changed = false;
    let current = match app.scene.environment {
        Environment::Uniform { .. } => 0,
        Environment::Gradient { .. } => 1,
        Environment::Hdri { .. } => 2,
    };
    let mut kind = current;
    ui.horizontal(|ui| {
        ui.radio_value(&mut kind, 0, "Uniforme");
        ui.radio_value(&mut kind, 1, "Cielo a gradiente");
        ui.radio_value(&mut kind, 2, "Mappa HDRI");
    });
    if kind != current {
        match kind {
            0 => {
                app.scene.environment = Environment::Uniform {
                    color: [0.8; 3],
                    strength: 0.5,
                };
                changed = true;
            }
            1 => {
                app.scene.environment = Environment::default();
                changed = true;
            }
            _ => changed |= pick_hdri(app),
        }
    }
    let mut repick = false;
    egui::Grid::new("env-grid")
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| match &mut app.scene.environment {
            Environment::Uniform { color, strength } => {
                ui.label("Colore");
                changed |= ui.color_edit_button_rgb(color).changed();
                ui.end_row();
                ui.label("Intensità");
                changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).logarithmic(true)).changed();
                ui.end_row();
            }
            Environment::Gradient {
                zenith,
                horizon,
                ground,
                strength,
            } => {
                ui.label("Zenit");
                changed |= ui.color_edit_button_rgb(zenith).changed();
                ui.end_row();
                ui.label("Orizzonte");
                changed |= ui.color_edit_button_rgb(horizon).changed();
                ui.end_row();
                ui.label("Suolo");
                changed |= ui.color_edit_button_rgb(ground).changed();
                ui.end_row();
                ui.label("Intensità");
                changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).logarithmic(true)).changed();
                ui.end_row();
            }
            Environment::Hdri {
                path,
                strength,
                rotation_deg,
            } => {
                ui.label("File");
                ui.horizontal(|ui| {
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    ui.label(name).on_hover_text(path.display().to_string());
                    repick = ui.small_button("Cambia…").clicked();
                });
                ui.end_row();
                ui.label("Intensità");
                changed |= ui.add(egui::Slider::new(strength, 0.0..=20.0).logarithmic(true)).changed();
                ui.end_row();
                ui.label("Rotazione (°)");
                changed |= ui.add(egui::Slider::new(rotation_deg, -180.0..=180.0)).changed();
                ui.end_row();
            }
        });
    if repick {
        changed |= pick_hdri(app);
    }
    changed |= hdri_library(app, ui);
    if changed {
        app.shading_changed();
    }
    if ui
        .checkbox(&mut app.settings.environment_visible, "Ambiente visibile come sfondo")
        .on_hover_text("Se disattivato lo sfondo è nero, ma l'ambiente continua a illuminare e a riflettersi")
        .changed()
    {
        app.unsaved = true;
    }
    if matches!(app.scene.environment, Environment::Hdri { .. }) {
        ui.label(
            egui::RichText::new(
                "La HDRI illumina la scena con importance sampling (sole e finestre campionati direttamente). \
                 Mappe gratuite CC0: polyhaven.com (anche dal pulsante «Scarica HDRI…»).",
            )
            .weak()
            .small(),
        );
    }
}

/// Elenco delle HDRI della cartella `Libreria/HDRI`: un clic e diventano l'ambiente.
fn hdri_library(app: &mut ForgeApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.separator();
    ui.horizontal(|ui| {
        ui.strong("HDRI della libreria");
        if ui
            .small_button("Apri cartella")
            .on_hover_text(forge_library::paths::hdri_dir().display().to_string())
            .clicked()
        {
            crate::ui::library::open_folder(&forge_library::paths::hdri_dir());
        }
        if ui
            .small_button("Scarica HDRI…")
            .on_hover_text("HDRI gratuite (CC0) da Poly Haven")
            .clicked()
        {
            app.library.open_hdri_tab();
        }
    });
    let list = app.library.hdri_list();
    if list.is_empty() {
        ui.label(
            egui::RichText::new("Nessuna HDRI: copia i file .hdr/.exr nella cartella HDRI della libreria, oppure scaricane una.")
                .weak()
                .small(),
        );
    }
    let current = match &app.scene.environment {
        Environment::Hdri { path, .. } => Some(path.clone()),
        _ => None,
    };
    let ctx = ui.ctx().clone();
    // Colonne a larghezza fissa calcolate sullo spazio disponibile: il pannello non si
    // allarga mai per far stare le miniature (con molte HDRI usciva dallo schermo).
    // Al massimo 2 per riga, larghe quanto metà pannello; le altre vanno sotto (scorrimento).
    let gap = ui.spacing().item_spacing.x;
    let avail = ui.available_width() - 12.0; // spazio per la barra di scorrimento
    let cols = if avail >= 260.0 { 2 } else { 1 };
    let cell = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).clamp(100.0, 400.0);
    let mut clicked = None;
    egui::ScrollArea::vertical().id_salt("hdri-list").max_height(520.0).show(ui, |ui| {
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                for p in row {
                    let name = p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let tex = app.library.hdri_thumb(&ctx, p);
                    let selected = current.as_ref() == Some(p);
                    let r = crate::ui::library::thumb_card(ui, tex, egui::vec2(cell, cell * 0.5), &name, "", selected);
                    if r.on_hover_text(p.display().to_string()).clicked() {
                        clicked = Some(p.clone());
                    }
                }
            });
        }
    });
    if let Some(p) = clicked {
        set_hdri(app, p);
        changed = true;
    }
    changed
}

/// Sceglie una HDRI da file.
fn pick_hdri(app: &mut ForgeApp) -> bool {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("HDRI (.hdr, .exr)", &["hdr", "exr"])
        .set_directory(forge_library::paths::hdri_dir())
        .pick_file()
    else {
        return false;
    };
    set_hdri(app, path);
    true
}

/// Imposta la HDRI come ambiente e la carica in background (così il primo render non blocca l'interfaccia).
pub(crate) fn set_hdri(app: &mut ForgeApp, path: std::path::PathBuf) {
    let p2 = path.clone();
    std::thread::spawn(move || {
        if let Err(e) = forge_render_core::env_map::load_cached(&p2) {
            tracing::error!("HDRI non caricata: {e:#}");
        }
    });
    let (strength, rotation_deg) = match app.scene.environment {
        Environment::Hdri {
            strength, rotation_deg, ..
        } => (strength, rotation_deg),
        _ => (1.0, 0.0),
    };
    app.scene.environment = Environment::Hdri {
        path,
        strength,
        rotation_deg,
    };
}

// ----------------------------------------------------------------------
// Camera
// ----------------------------------------------------------------------

fn camera(app: &mut ForgeApp, ui: &mut egui::Ui) {
    render_cameras(app, ui);
    ui.separator();

    // Parametri della camera di rendering attiva, oppure della vista se non ce n'è una.
    let active = app.scene.active_camera;
    ui.strong(match active {
        Some(i) => format!("Parametri: {}", app.scene.render_cameras[i].name),
        None => "Parametri della vista".to_string(),
    });
    let mut changed = false;
    let mut cam = app.scene.render_camera().clone();
    changed |= lens_controls(app, ui, &mut cam);
    ui.add_space(4.0);
    let r = app.scene.bounds().radius().max(0.01) as f64;
    changed |= vec3_edit(ui, "Posizione", &mut cam.position, r * 0.005);
    changed |= vec3_edit(ui, "Bersaglio", &mut cam.target, r * 0.005);
    if changed {
        let through = app.viewing_through_active();
        match active {
            Some(i) => {
                app.scene.render_cameras[i].camera = cam.clone();
                if through {
                    app.scene.camera = cam;
                    app.sync_orbit_from_camera();
                }
            }
            None => {
                app.scene.camera = cam;
                app.sync_orbit_from_camera();
            }
        }
        app.shading_changed();
    }
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        let views = [
            ("Frontale", 0.0, 0.0),
            ("Destra", std::f32::consts::FRAC_PI_2, 0.0),
            ("Alto", 0.0, 1.55),
            ("Isometrica", 0.785, 0.615),
        ];
        for (name, yaw, pitch) in views {
            if ui.button(name).clicked() {
                app.orbit.yaw = yaw;
                app.orbit.pitch = pitch;
                app.frame_all();
            }
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Inquadra tutto").clicked() {
            app.frame_all();
        }
        if ui.button("Inquadra selezione").clicked() {
            app.frame_selection();
        }
    });
    ui.label(
        egui::RichText::new("Viewport: trascina = orbita · Shift/destro/centrale = pan · rotella = zoom · doppio clic = inquadra")
            .weak()
            .small(),
    );
}

// ----------------------------------------------------------------------
// Impostazioni di rendering
// ----------------------------------------------------------------------

fn render(app: &mut ForgeApp, ui: &mut egui::Ui) {
    if !app.scene.render_cameras.is_empty() {
        camera_picker(app, ui, "render-tab-camera");
        ui.separator();
    }
    let mut changed = false;
    let prev_backend = app.settings.backend;
    egui::ComboBox::from_label("Dispositivo")
        .selected_text(app.settings.backend.label())
        .show_ui(ui, |ui| {
            for b in [forge_render_core::BackendKind::Cpu, forge_render_core::BackendKind::Gpu] {
                ui.selectable_value(&mut app.settings.backend, b, b.label());
            }
        });
    if app.settings.backend != prev_backend {
        changed = true;
    }
    if app.settings.backend == forge_render_core::BackendKind::Gpu {
        match app.gpu_backend() {
            Ok(g) => {
                let i = g.info();
                ui.label(
                    egui::RichText::new(format!("{} · {} · {}", i.name, i.backend, i.device_type))
                        .weak()
                        .small(),
                );
            }
            Err(e) => {
                ui.colored_label(style::WARN, format!("GPU non disponibile: {e}"));
            }
        }
    }
    let s = &mut app.settings;
    let max_t = app.system.logical_threads;
    ui.horizontal(|ui| {
        ui.label("Thread CPU");
        changed |= ui
            .add(egui::Slider::new(&mut s.threads, 0..=max_t))
            .on_hover_text("0 = automatico (thread logici − 1, per lasciare reattiva l'interfaccia)")
            .changed();
        if s.threads == 0 {
            ui.label(egui::RichText::new(format!("auto: {}", s.effective_threads())).weak());
        }
    });

    ui.separator();
    ui.strong("Risoluzione");
    ui.horizontal_wrapped(|ui| {
        for (name, w, h) in [
            ("HD", 1280, 720),
            ("Full HD", 1920, 1080),
            ("QHD", 2560, 1440),
            ("4K", 3840, 2160),
            ("Quadrato", 1080, 1080),
            ("Anteprima", 640, 360),
        ] {
            if ui.selectable_label(s.width == w && s.height == h, name).clicked() {
                s.width = w;
                s.height = h;
                changed = true;
            }
        }
    });
    ui.horizontal(|ui| {
        changed |= ui.add(egui::DragValue::new(&mut s.width).range(16..=16384).suffix(" px")).changed();
        ui.label("×");
        changed |= ui
            .add(egui::DragValue::new(&mut s.height).range(16..=16384).suffix(" px"))
            .changed();
    });

    ui.separator();
    ui.strong("Campioni per pixel");
    ui.horizontal_wrapped(|ui| {
        for n in forge_render_core::SamplesPreset::FIXED {
            if ui.selectable_label(s.target_samples == n, n.to_string()).clicked() {
                s.target_samples = n;
                changed = true;
            }
        }
        let custom = s.target_samples != 0 && !forge_render_core::SamplesPreset::FIXED.contains(&s.target_samples);
        if ui.selectable_label(custom, "Personalizzato").clicked() {
            s.target_samples = app.custom_samples;
            changed = true;
        }
        if ui
            .selectable_label(s.target_samples == 0, "Illimitato")
            .on_hover_text("Il rendering continua finché non lo fermi")
            .clicked()
        {
            s.target_samples = 0;
            changed = true;
        }
    });
    if s.target_samples != 0
        && !forge_render_core::SamplesPreset::FIXED.contains(&s.target_samples)
        && ui
            .add(
                egui::DragValue::new(&mut s.target_samples)
                    .range(1..=1_000_000)
                    .prefix("campioni: "),
            )
            .changed()
    {
        app.custom_samples = s.target_samples;
        changed = true;
    }
    ui.label(
        egui::RichText::new("Si può cambiare durante il rendering: l'accumulo prosegue senza ripartire.")
            .weak()
            .small(),
    );
    unavailable(ui, "Arresto a soglia di rumore", "Fase 6");

    ui.separator();
    ui.strong("Path tracing");
    egui::Grid::new("pt-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Rimbalzi massimi");
        changed |= ui.add(egui::Slider::new(&mut s.max_bounces, 1..=64)).changed();
        ui.end_row();
        ui.label("Russian roulette da");
        changed |= ui.add(egui::Slider::new(&mut s.russian_roulette_start, 1..=16)).changed();
        ui.end_row();
        ui.label("Clamp indiretto");
        changed |= ui
            .add(egui::DragValue::new(&mut s.clamp_indirect).speed(0.1).range(0.0..=1000.0))
            .on_hover_text("0 = disattivato (fisicamente corretto). Valori > 0 riducono i firefly introducendo un bias.")
            .changed();
        ui.end_row();
        ui.label("Seed");
        changed |= ui.add(egui::DragValue::new(&mut s.seed)).changed();
        ui.end_row();
    });

    ui.separator();
    ui.strong("Visualizzazione (non riavvia il render)");
    egui::ComboBox::from_label("Tone mapping")
        .selected_text(s.tone_mapping.label())
        .show_ui(ui, |ui| {
            for t in [ToneMapping::AcesFilmic, ToneMapping::Standard] {
                changed |= ui.selectable_value(&mut s.tone_mapping, t, t.label()).changed();
            }
        });
    changed |= ui
        .add(egui::Slider::new(&mut s.exposure_ev, -6.0..=6.0).text("Esposizione (EV)"))
        .changed();

    ui.separator();
    changed |= denoiser(app, ui);
    if changed {
        app.unsaved = true;
    }
    if app.render_is_stale() && !app.auto_restart {
        ui.colored_label(style::WARN, "Le impostazioni sono cambiate: premi Riavvia per applicarle.");
    }
}

/// Regolazioni della riduzione del rumore (Open Image Denoise). Nessuna di
/// queste riavvia il render: la sessione ripulisce di nuovo l'immagine corrente.
fn denoiser(app: &mut ForgeApp, ui: &mut egui::Ui) -> bool {
    use forge_render_core::{DenoiseMode, DenoiseQuality};
    let mut changed = false;
    ui.strong("Riduzione del rumore (denoiser)");
    let info = app.denoiser_info.clone();
    match &info {
        Ok(v) => {
            ui.label(egui::RichText::new(v).weak().small());
        }
        Err(e) => {
            ui.colored_label(
                style::WARN,
                "Open Image Denoise non trovato: il render resta senza riduzione del rumore.",
            )
            .on_hover_text(e);
            return false;
        }
    }
    let status = app.session.progress().denoiser;
    let d = &mut app.settings.denoise;
    egui::ComboBox::from_label("Modalità")
        .selected_text(d.mode.label())
        .show_ui(ui, |ui| {
            for m in [DenoiseMode::Final, DenoiseMode::Progressive, DenoiseMode::Off] {
                changed |= ui.selectable_value(&mut d.mode, m, m.label()).changed();
            }
        })
        .response
        .on_hover_text(
            "Solo immagine finale: ripulisce a fine render, in pausa o quando lo fermi.\n\
             Anteprima veloce: ripulisce anche durante il render a 8, 16, 32… campioni, così vedi subito il risultato.",
        );
    if d.mode == DenoiseMode::Off {
        return changed;
    }
    changed |= ui
        .checkbox(&mut d.use_cuda, "Usa CUDA (GPU NVIDIA)")
        .on_hover_text(
            "Esegue il denoiser sulla GPU NVIDIA con i core tensor: molto più veloce, ideale per l'anteprima.\n\
             Richiede una GPU Volta o successiva (RTX 20/30/40/50, Quadro RTX) e il driver NVIDIA; nessun CUDA Toolkit da installare.\n\
             Se non è disponibile si usa automaticamente la CPU.",
        )
        .changed();
    if d.use_cuda && status.contains("· CPU") {
        ui.colored_label(style::WARN, "CUDA non disponibile su questa GPU: il denoiser usa la CPU.");
    }
    egui::Grid::new("dn-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        let quality = |ui: &mut egui::Ui, id: &str, q: &mut DenoiseQuality| {
            let mut c = false;
            egui::ComboBox::from_id_salt(id).selected_text(q.label()).show_ui(ui, |ui| {
                for v in [DenoiseQuality::High, DenoiseQuality::Balanced, DenoiseQuality::Fast] {
                    c |= ui.selectable_value(q, v, v.label()).changed();
                }
            });
            c
        };
        ui.label("Qualità finale");
        changed |= quality(ui, "dn-q-final", &mut d.quality);
        ui.end_row();
        if d.mode == DenoiseMode::Progressive {
            ui.label("Qualità anteprima");
            changed |= quality(ui, "dn-q-preview", &mut d.preview_quality);
            ui.end_row();
            ui.label("Prima anteprima a");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut d.first_preview_samples)
                        .range(1..=1024)
                        .suffix(" campioni"),
                )
                .on_hover_text("Poi a ogni raddoppio dei campioni (es. 8, 16, 32, 64…)")
                .changed();
            ui.end_row();
        }
        ui.label("Intensità");
        let mut pct = d.strength * 100.0;
        if ui
            .add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %"))
            .on_hover_text("Miscela tra l'immagine grezza (0 %) e quella ripulita (100 %): utile se il filtro appiattisce dettagli fini")
            .changed()
        {
            d.strength = pct / 100.0;
            changed = true;
        }
        ui.end_row();
    });
    changed |= ui
        .checkbox(&mut d.use_guides, "Preserva i dettagli (albedo e normali)")
        .on_hover_text("Guida il filtro con colore e orientamento delle superfici: spigoli, testi e texture restano nitidi")
        .changed();
    ui.horizontal(|ui| {
        ui.label("Vista:");
        for v in [RenderView::Final, RenderView::Raw, RenderView::Albedo, RenderView::Normals] {
            ui.selectable_value(&mut app.render_view, v, v.label());
        }
    });
    if !status.is_empty() {
        ui.label(egui::RichText::new(status).weak().small());
    }
    changed
}

/// Elenco delle camere di rendering salvate nel progetto.
fn render_cameras(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.strong("Camere di rendering");
    ui.label(
        egui::RichText::new("Il render usa la camera attiva: nella viewport ti muovi liberamente senza perdere l'inquadratura.")
            .weak()
            .small(),
    );
    let active = app.scene.active_camera;
    if ui
        .radio(active.is_none(), "Nessuna: il render usa la vista della viewport")
        .clicked()
        && active.is_some()
    {
        app.set_active_camera(None);
    }
    let mut action: Option<(usize, &str)> = None;
    for i in 0..app.scene.render_cameras.len() {
        ui.horizontal(|ui| {
            if ui
                .radio(active == Some(i), "")
                .on_hover_text("Usa questa camera per il render")
                .clicked()
                && active != Some(i)
            {
                action = Some((i, "attiva"));
            }
            let name = &mut app.scene.render_cameras[i].name;
            if ui.add(egui::TextEdit::singleline(name).desired_width(110.0)).changed() {
                app.unsaved = true;
            }
            if ui
                .small_button("Guarda")
                .on_hover_text("Guarda attraverso: porta la vista su questa inquadratura")
                .clicked()
            {
                action = Some((i, "guarda"));
            }
            if ui
                .small_button("Da vista")
                .on_hover_text("Imposta dalla vista: questa camera prende l'inquadratura attuale della viewport")
                .clicked()
            {
                action = Some((i, "imposta"));
            }
            if ui.small_button("Elimina").on_hover_text("Elimina la camera").clicked() {
                action = Some((i, "elimina"));
            }
        });
    }
    match action {
        Some((i, "attiva")) => {
            app.set_active_camera(Some(i));
            app.look_through(i);
        }
        Some((i, "guarda")) => app.look_through(i),
        Some((i, "imposta")) => {
            app.scene.render_cameras[i].camera = app.scene.camera.clone();
            app.shading_changed();
        }
        Some((i, "elimina")) => {
            app.scene.render_cameras.remove(i);
            let new_active = match app.scene.active_camera {
                Some(a) if a == i => None,
                Some(a) if a > i => Some(a - 1),
                other => other,
            };
            app.set_active_camera(new_active);
        }
        _ => {}
    }
    ui.horizontal(|ui| {
        if ui
            .button("+ Nuova camera dalla vista")
            .on_hover_text("Crea una camera di rendering con l'inquadratura attuale")
            .clicked()
        {
            app.add_render_camera(None);
        }
        if app.scene.active_camera.is_some() {
            ui.checkbox(&mut app.camera_follow, "Pilota")
                .on_hover_text("La camera attiva segue i movimenti della viewport (per regolare l'inquadratura muovendoti nella scena)");
        }
    });
    if app.scene.active_camera.is_some() && !app.viewing_through_active() && !app.camera_follow {
        ui.label(
            egui::RichText::new("Vista libera: il render resta sull'inquadratura della camera attiva («Guarda» per tornarci).")
                .weak()
                .small(),
        );
    }
}

/// Tendina per scegliere la camera usata dal render (vista o camere di rendering).
pub(crate) fn camera_picker(app: &mut ForgeApp, ui: &mut egui::Ui, id: &str) {
    let current = match app.scene.active_camera {
        Some(i) => app.scene.render_cameras[i].name.clone(),
        None => "Vista della viewport".to_string(),
    };
    let mut choice: Option<Option<usize>> = None;
    egui::ComboBox::from_id_salt(id)
        .selected_text(format!("Camera: {current}"))
        .width(200.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(app.scene.active_camera.is_none(), "Vista della viewport")
                .clicked()
            {
                choice = Some(None);
            }
            for (i, c) in app.scene.render_cameras.iter().enumerate() {
                if ui.selectable_label(app.scene.active_camera == Some(i), &c.name).clicked() {
                    choice = Some(Some(i));
                }
            }
        })
        .response
        .on_hover_text("Camera usata dal render (le camere si creano nella scheda Camera)");
    if let Some(c) = choice {
        if c != app.scene.active_camera {
            app.set_active_camera(c);
        }
    }
}

/// Diaframmi standard (stop interi) delle fotocamere.
const F_STOPS: [f32; 11] = [1.2, 1.4, 2.0, 2.8, 4.0, 5.6, 8.0, 11.0, 16.0, 22.0, 32.0];

fn fmt_m(m: f32) -> String {
    if m.is_infinite() {
        "∞".into()
    } else if m < 1.0 {
        format!("{:.0} cm", m * 100.0)
    } else {
        format!("{m:.2} m")
    }
}

/// Obiettivo: focale, profondità di campo (diaframma, fuoco, lamelle) e decentramento.
fn lens_controls(app: &mut ForgeApp, ui: &mut egui::Ui, cam: &mut forge_scene::Camera) -> bool {
    let mut changed = false;
    egui::Grid::new("lens-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Obiettivo");
        ui.horizontal(|ui| {
            let mut mm = cam.focal_mm();
            if ui
                .add(
                    egui::DragValue::new(&mut mm)
                        .range(4.0..=2000.0)
                        .speed(0.5)
                        .suffix(" mm")
                        .max_decimals(0),
                )
                .on_hover_text("Focale equivalente full frame (sensore 36×24 mm)")
                .changed()
            {
                cam.set_focal_mm(mm);
                changed = true;
            }
            ui.label(egui::RichText::new(format!("{:.1}° vert.", cam.vfov_deg)).weak().small());
        });
        ui.end_row();
        ui.label("");
        ui.horizontal_wrapped(|ui| {
            for mm in [14.0, 24.0, 35.0, 50.0, 85.0, 135.0, 200.0] {
                if ui.small_button(format!("{mm:.0}")).clicked() {
                    cam.set_focal_mm(mm);
                    changed = true;
                }
            }
        });
        ui.end_row();
        ui.label("Profondità di campo");
        changed |= ui
            .checkbox(&mut cam.lens.dof, "attiva")
            .on_hover_text("Sfocato ottico reale (lente sottile): nitido sul piano di fuoco, sfocato davanti e dietro")
            .changed();
        ui.end_row();
        if cam.lens.dof {
            ui.label("Diaframma");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("fstop")
                    .selected_text(format!("f/{}", trim(cam.lens.f_stop)))
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        for f in F_STOPS {
                            if ui
                                .selectable_label((cam.lens.f_stop - f).abs() < 1e-3, format!("f/{}", trim(f)))
                                .clicked()
                            {
                                cam.lens.f_stop = f;
                                changed = true;
                            }
                        }
                    })
                    .response
                    .on_hover_text("Numero f: più basso = più sfocato (zona nitida stretta), più alto = più nitido");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut cam.lens.f_stop)
                            .range(0.7..=64.0)
                            .speed(0.05)
                            .prefix("f/")
                            .max_decimals(1),
                    )
                    .changed();
            });
            ui.end_row();
            ui.label("Messa a fuoco");
            ui.horizontal(|ui| {
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut cam.lens.focus_distance)
                            .range(0.01..=10000.0)
                            .speed(0.01)
                            .suffix(" m")
                            .max_decimals(3),
                    )
                    .changed();
                if ui
                    .selectable_label(app.focus_pick, "Scegli col clic")
                    .on_hover_text("Clicca nella viewport il punto da mettere a fuoco")
                    .clicked()
                {
                    app.focus_pick = !app.focus_pick;
                    app.view = crate::app::View::Viewport;
                }
                if ui
                    .add_enabled(!app.selection.is_empty(), egui::Button::new("Sulla selezione"))
                    .on_hover_text("Mette a fuoco il centro dei componenti selezionati")
                    .clicked()
                {
                    let mut b = forge_math::Aabb::EMPTY;
                    for &i in &app.selection {
                        b.grow(&app.scene.exact_bounds_of(i));
                    }
                    if !b.is_empty() {
                        cam.lens.focus_distance = (b.center() - cam.position).dot(cam.forward()).max(0.01);
                        changed = true;
                    }
                }
            });
            ui.end_row();
            let (near, far) = cam.dof_limits();
            ui.label("Zona nitida");
            ui.label(
                egui::RichText::new(format!(
                    "da {} a {}{}",
                    fmt_m(near),
                    fmt_m(far),
                    if far.is_finite() {
                        format!("  (profondità {})", fmt_m(far - near))
                    } else {
                        String::new()
                    }
                ))
                .strong(),
            )
            .on_hover_text("Calcolata come su una fotocamera full frame (circolo di confusione 0,03 mm)");
            ui.end_row();
            ui.label("Lamelle");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("blades")
                    .selected_text(if cam.lens.blades < 3 {
                        "circolare".to_string()
                    } else {
                        format!("{}", cam.lens.blades)
                    })
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        changed |= ui.selectable_value(&mut cam.lens.blades, 0, "circolare").changed();
                        for n in 5..=9 {
                            changed |= ui.selectable_value(&mut cam.lens.blades, n, format!("{n} lamelle")).changed();
                        }
                    })
                    .response
                    .on_hover_text("Forma del bokeh: le luci sfocate diventano poligoni come negli obiettivi veri");
                if cam.lens.blades >= 3 {
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut cam.lens.blade_rotation_deg)
                                .range(-180.0..=180.0)
                                .suffix("°"),
                        )
                        .on_hover_text("Rotazione del diaframma")
                        .changed();
                }
            });
            ui.end_row();
        }
        ui.label("Decentramento");
        ui.horizontal(|ui| {
            let mut x = cam.lens.shift_x * 100.0;
            let mut y = cam.lens.shift_y * 100.0;
            let rx = ui.add(
                egui::DragValue::new(&mut x)
                    .range(-100.0..=100.0)
                    .speed(0.2)
                    .prefix("orizz. ")
                    .suffix(" %"),
            );
            let ry = ui.add(
                egui::DragValue::new(&mut y)
                    .range(-100.0..=100.0)
                    .speed(0.2)
                    .prefix("vert. ")
                    .suffix(" %"),
            );
            if rx.changed() || ry.changed() {
                cam.lens.shift_x = x / 100.0;
                cam.lens.shift_y = y / 100.0;
                changed = true;
            }
            ry.on_hover_text(
                "Shift (come un obiettivo decentrabile): sposta l'inquadratura senza inclinare la camera, \
                 così le verticali restano dritte (prodotti alti, architettura)",
            );
        });
        ui.end_row();
    });
    changed
}

/// "2.8" invece di "2.80", "8" invece di "8.0".
fn trim(v: f32) -> String {
    let s = format!("{v:.1}");
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}
