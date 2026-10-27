use crate::app::{ForgeApp, InspectorTab};
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
    let Some(m) = app.scene.materials.get_mut(&app.selected_material) else {
        return;
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Nome");
        changed |= ui.text_edit_singleline(&mut m.name).changed();
    });
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
        ui.label("Normal map");
        changed |= texture_slot(ui, &mut m.normal_texture, "normal");
        ui.end_row();
        if m.has_textures() {
            ui.label("Proiezione");
            ui.horizontal(|ui| {
                changed |= ui
                    .radio_value(&mut m.mapping.projection, forge_scene::Projection::Box, "Box (CAD)")
                    .on_hover_text("Proiezione cubica in spazio mondo: non richiede UV")
                    .changed();
                changed |= ui
                    .radio_value(&mut m.mapping.projection, forge_scene::Projection::Uv, "UV della mesh")
                    .changed();
            });
            ui.end_row();
            match m.mapping.projection {
                forge_scene::Projection::Box => {
                    ui.label("Dimensione ripetizione (m)");
                    changed |= ui
                        .add(egui::DragValue::new(&mut m.mapping.scale).speed(0.001).range(0.0001..=100.0))
                        .changed();
                }
                forge_scene::Projection::Uv => {
                    ui.label("Ripetizioni");
                    changed |= ui
                        .add(egui::DragValue::new(&mut m.mapping.scale).speed(0.05).range(0.001..=1000.0))
                        .changed();
                }
            }
            ui.end_row();
            ui.label("Rotazione (°)");
            changed |= ui.add(egui::Slider::new(&mut m.mapping.rotation_deg, -180.0..=180.0)).changed();
            ui.end_row();
            ui.label("Spostamento");
            ui.horizontal(|ui| {
                changed |= ui.add(egui::DragValue::new(&mut m.mapping.offset[0]).speed(0.01)).changed();
                changed |= ui.add(egui::DragValue::new(&mut m.mapping.offset[1]).speed(0.01)).changed();
            });
            ui.end_row();
        }
        if m.normal_texture.is_some() {
            ui.label("Intensità normal map");
            changed |= ui.add(egui::Slider::new(&mut m.normal_strength, 0.0..=4.0)).changed();
            ui.end_row();
        }
    });
    unavailable(ui, "Displacement, opacità e trasmissione ruvida", "fasi successive");
    if changed {
        app.appearance_changed();
    }
    crate::ui::central::material_preview(app, ui);
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
                    ui.label("Cono esterno (°)");
                    changed |= ui.add(egui::Slider::new(outer_angle_deg, 1.0..=170.0)).changed();
                    ui.end_row();
                    ui.label("Cono interno (°)");
                    changed |= ui.add(egui::Slider::new(inner_angle_deg, 0.0..=170.0)).changed();
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
                 Mappe gratuite CC0: polyhaven.com.",
            )
            .weak()
            .small(),
        );
    }
}

/// Sceglie una HDRI e la carica in background (così il primo render non blocca l'interfaccia).
fn pick_hdri(app: &mut ForgeApp) -> bool {
    let Some(path) = rfd::FileDialog::new().add_filter("HDRI (.hdr, .exr)", &["hdr", "exr"]).pick_file() else {
        return false;
    };
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
    true
}

// ----------------------------------------------------------------------
// Camera
// ----------------------------------------------------------------------

fn camera(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let mut changed = false;
    let mut cam = app.scene.camera.clone();
    ui.horizontal(|ui| {
        ui.label("Campo visivo verticale (°)");
        changed |= ui.add(egui::Slider::new(&mut cam.vfov_deg, 5.0..=120.0)).changed();
    });
    let focal = 12.0 / (cam.vfov_deg.to_radians() * 0.5).tan();
    ui.label(egui::RichText::new(format!("≈ obiettivo {focal:.0} mm (equivalente full-frame)")).weak());
    let r = app.scene.bounds().radius().max(0.01) as f64;
    changed |= vec3_edit(ui, "Posizione", &mut cam.position, r * 0.005);
    changed |= vec3_edit(ui, "Bersaglio", &mut cam.target, r * 0.005);
    if changed {
        app.scene.camera = cam;
        app.sync_orbit_from_camera();
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
    unavailable(ui, "Profondità di campo", "Fase 7");
}

// ----------------------------------------------------------------------
// Impostazioni di rendering
// ----------------------------------------------------------------------

fn render(app: &mut ForgeApp, ui: &mut egui::Ui) {
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
    unavailable(ui, "Denoiser (Intel Open Image Denoise)", "Fase 6");
    if changed {
        app.unsaved = true;
    }
    if app.render_is_stale() && !app.auto_restart {
        ui.colored_label(style::WARN, "Le impostazioni sono cambiate: premi Riavvia per applicarle.");
    }
}
