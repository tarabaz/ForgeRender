use crate::app::{ForgeApp, InspectorTab, NewLight, RenderView};
use crate::i18n::{tf, tr};
use crate::ui::manual::{help_button, HelpExt};
use crate::ui::style;
use eframe::egui;
use forge_math::Vec3;
use forge_render_core::ToneMapping;
use forge_scene::{light, Environment, LightKind, Material, MaterialId};

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        for (tab, label) in [
            (InspectorTab::Object, tr("Oggetto")),
            (InspectorTab::Materials, tr("Materiali")),
            (InspectorTab::Lights, tr("Luci")),
            (InspectorTab::Environment, tr("Ambiente")),
            (InspectorTab::Effects, tr("Effetti")),
            (InspectorTab::Camera, tr("Camera")),
            (InspectorTab::Render, tr("Render")),
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
            InspectorTab::Effects => effects(app, ui),
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

// ----------------------------------------------------------------------
// Materiali
// ----------------------------------------------------------------------

/// Salva / Salva con nome / Ripristina del materiale selezionato nella libreria con texture.
fn library_save_controls(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let id = app.selected_material.clone();
    let Some(current) = app.scene.materials.get(&id).cloned() else {
        return;
    };
    let lib_dir = forge_library::store::dir_of(&id);
    let saved = lib_dir
        .as_ref()
        .and_then(|_| app.library.saved_version(&id).map(|m| m.material.clone()));
    let dirty = saved.as_ref().is_some_and(|s| *s != current);
    let name_key = egui::Id::NULL.with("mat-save-as-name");
    let mut save_as_name: Option<String> = ui.ctx().data(|d| d.get_temp(name_key));
    let mut status: Option<(String, bool)> = None;

    ui.horizontal_wrapped(|ui| {
        if let Some(dir) = &lib_dir {
            if ui
                .add_enabled(dirty || saved.is_none(), egui::Button::new(tr("Salva")))
                .on_hover_text(tf("Salva le modifiche nella libreria ({})", &[format!("{}", dir.display())]))
                .clicked()
            {
                match forge_library::store::update(dir, &current) {
                    Ok(m) => {
                        app.scene.materials.upsert(m.material.clone());
                        app.library.material_saved(&m);
                        app.appearance_changed();
                        status = Some((tf("«{}» salvato nella libreria.", std::slice::from_ref(&m.material.name)), false));
                    }
                    Err(e) => status = Some((tf("Salvataggio non riuscito: {e:#}", &[format!("{:#}", e)]), true)),
                }
            }
        }
        let label = if lib_dir.is_some() {
            tr("Salva con nome…")
        } else {
            tr("Salva nella libreria…")
        };
        if ui
            .button(label)
            .on_hover_text(tr(
                "Salva una copia nella libreria con un nome nuovo (con tutte le texture); l'originale non cambia",
            ))
            .clicked()
        {
            save_as_name = Some(if lib_dir.is_some() {
                tf("{} (copia)", std::slice::from_ref(&current.name))
            } else {
                current.name.clone()
            });
        }
        if let (Some(_), Some(saved)) = (&lib_dir, &saved) {
            if ui
                .add_enabled(dirty, egui::Button::new(tr("Ripristina")))
                .on_hover_text(tr("Torna alle impostazioni salvate nella libreria"))
                .clicked()
            {
                app.scene.materials.upsert(saved.clone());
                app.appearance_changed();
            }
        }
        if dirty {
            ui.colored_label(crate::ui::style::WARN, tr("● modifiche non salvate nella libreria"));
        }
    });

    if let Some(name) = &mut save_as_name {
        let mut close = false;
        ui.horizontal(|ui| {
            ui.label(tr("Nome:"));
            let r = ui.add(egui::TextEdit::singleline(name).desired_width(180.0));
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.add_enabled(!name.trim().is_empty(), egui::Button::new(tr("Salva"))).clicked() || (enter && !name.trim().is_empty()) {
                let source = if lib_dir.is_some() {
                    tf(
                        "copia di «{}»",
                        &[saved.as_ref().map(|m| m.name.as_str()).unwrap_or(&current.name).to_string()],
                    )
                } else {
                    tr("salvato dal progetto").to_string()
                };
                match forge_library::store::save_as(&current, name, &source) {
                    Ok(m) => {
                        // Il progetto usa la copia al posto dell'originale (come "Salva con nome").
                        let new_id = m.material.id.clone();
                        app.scene.materials.upsert(m.material.clone());
                        for v in app.scene.assignments.values_mut() {
                            if *v == id {
                                *v = new_id.clone();
                            }
                        }
                        if app.scene.default_material == id {
                            app.scene.default_material = new_id.clone();
                        }
                        // L'originale della libreria torna come salvato (si riapplica dalla libreria).
                        if lib_dir.is_some() {
                            app.scene.materials.materials.retain(|x| x.id != id);
                        }
                        app.selected_material = new_id;
                        app.library.material_saved(&m);
                        app.appearance_changed();
                        status = Some((
                            tf("Copia «{}» salvata nella libreria.", std::slice::from_ref(&m.material.name)),
                            false,
                        ));
                        close = true;
                    }
                    Err(e) => status = Some((tf("Salvataggio non riuscito: {e:#}", &[format!("{:#}", e)]), true)),
                }
            }
            if ui.button(tr("Annulla")).clicked() {
                close = true;
            }
        });
        if close {
            save_as_name = None;
        }
    }
    ui.ctx().data_mut(|d| match &save_as_name {
        Some(n) => {
            d.insert_temp(name_key, n.clone());
        }
        None => d.remove::<String>(name_key),
    });
    // Messaggio dell'ultimo salvataggio, finché resta selezionato lo stesso materiale.
    let status_key = egui::Id::NULL.with("mat-save-status");
    if let Some((msg, err)) = status {
        let sel = app.selected_material.0.clone();
        ui.ctx().data_mut(|d| d.insert_temp(status_key, (sel, msg, err)));
    }
    if let Some((_, msg, err)) = ui
        .ctx()
        .data(|d| d.get_temp::<(String, String, bool)>(status_key))
        .filter(|(sel, _, _)| *sel == app.selected_material.0)
    {
        let col = if err {
            crate::ui::style::WARN
        } else {
            egui::Color32::from_rgb(140, 200, 140)
        };
        ui.colored_label(col, egui::RichText::new(msg).small());
    }
}

fn materials(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let n_sel = app.selection.len();
    if n_sel == 0 {
        ui.label(
            egui::RichText::new(tr(
                "Nessun componente selezionato (clic nella viewport o nell'albero; Ctrl+clic per selezione multipla).",
            ))
            .weak(),
        );
    } else {
        let names: Vec<String> = app.selection.iter().take(3).map(|&i| app.scene.nodes[i].name.clone()).collect();
        ui.label(tf(
            "Selezione: {}{}",
            &[
                names.join(", ").to_string(),
                (if n_sel > 3 {
                    tf(" e altri {}", &[format!("{}", n_sel - 3)])
                } else {
                    String::new()
                })
                .to_string(),
            ],
        ));
    }

    // Solo i materiali della scena (assegnati, predefinito, selezionato): tutti gli altri sono
    // nella finestra Libreria, senza doppioni qui.
    ui.add_space(4.0);
    let mut used: Vec<MaterialId> = app.scene.assignments.values().cloned().collect();
    used.push(app.scene.default_material.clone());
    used.push(app.selected_material.clone());
    used.sort();
    used.dedup();
    let mut items: Vec<Material> = used.iter().filter_map(|id| app.scene.materials.get(id).cloned()).collect();
    items.sort_by_key(|m| crate::i18n::tr_dyn(&m.name).to_lowercase());
    ui.horizontal(|ui| {
        ui.strong(tf("Materiali della scena ({})", &[format!("{}", items.len())]));
        if ui
            .button(tr("Libreria materiali…"))
            .on_hover_text(tr("Tutti i materiali: libreria ufficiale e scaricati"))
            .clicked()
        {
            app.library.open = true;
        }
    });
    let ctx = ui.ctx().clone();
    let card = 64.0;
    egui::ScrollArea::vertical().id_salt("scene-mats").max_height(260.0).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for m in &items {
                let usage = app.scene.assignments.values().filter(|v| **v == m.id).count();
                let sub = if usage > 0 {
                    tf("{} componenti", &[format!("{}", usage)])
                } else if app.scene.default_material == m.id {
                    tr("predefinito della scena").to_string()
                } else {
                    String::new()
                };
                let tex = app.library.preset_thumb(&ctx, m, 128);
                let name = crate::i18n::tr_dyn(&m.name);
                let r = crate::ui::library::thumb_card(ui, tex, egui::vec2(card, card), &name, &sub, app.selected_material == m.id);
                if r.on_hover_text(name.clone()).clicked() {
                    app.selected_material = m.id.clone();
                }
            }
        });
    });

    // Assegnazione.
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(n_sel > 0, egui::Button::new(tr("Assegna alla selezione"))).clicked() {
            for &i in &app.selection {
                app.scene
                    .assignments
                    .insert(app.scene.nodes[i].id.clone(), app.selected_material.clone());
            }
            app.appearance_changed();
        }
        if ui
            .add_enabled(n_sel > 0, egui::Button::new(tr("Rimuovi assegnazione")))
            .on_hover_text(tr("Il componente torna a ereditare il materiale dal genitore o quello predefinito"))
            .clicked()
        {
            for &i in &app.selection {
                app.scene.assignments.remove(&app.scene.nodes[i].id);
            }
            app.appearance_changed();
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Nome contiene:"));
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
            .add_enabled(
                !matches.is_empty(),
                egui::Button::new(tf("Assegna ({})", &[format!("{}", matches.len())])),
            )
            .on_hover_text(tr("Assegna il materiale a tutti i componenti il cui nome contiene il testo"))
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
            egui::Button::new(tr("Usa come predefinito per i componenti senza assegnazione")),
        )
        .clicked()
    {
        app.scene.default_material = app.selected_material.clone();
        app.appearance_changed();
    }

    ui.horizontal(|ui| {
        if ui.button(tr("Nuovo")).clicked() {
            let id = app.scene.materials.unique_id("materiale");
            app.scene.materials.upsert(Material {
                id: id.clone(),
                name: tr("Nuovo materiale").into(),
                category: tr("Personalizzati").into(),
                ..Default::default()
            });
            app.selected_material = id;
            app.appearance_changed();
        }
        if ui.button(tr("Duplica")).clicked() {
            if let Some(m) = app.scene.materials.get(&app.selected_material).cloned() {
                let id = app.scene.materials.unique_id(&format!("{}-copia", m.id.0));
                app.scene.materials.upsert(Material {
                    id: id.clone(),
                    name: tf("{} (copia)", std::slice::from_ref(&m.name)),
                    category: tr("Personalizzati").into(),
                    ..m
                });
                app.selected_material = id;
                app.appearance_changed();
            }
        }
        let deletable = app.selected_material != MaterialId::new("default");
        if ui
            .add_enabled(deletable, egui::Button::new(tr("Elimina")))
            .on_hover_text(tr("I componenti che lo usavano tornano al materiale ereditato/predefinito"))
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
    library_save_controls(app, ui);

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
        ui.label(tr("Nome"));
        changed |= ui.text_edit_singleline(&mut m.name).changed();
    });
    if m.has_textures() {
        egui::CollapsingHeader::new(tr("Scala e posizionamento texture"))
            .default_open(true)
            .show(ui, |ui| changed |= mapping_controls(ui, m, obj_size));
        if m.normal_texture.is_some() || m.height_texture.is_some() {
            egui::CollapsingHeader::new(tr("Rilievo (normal, bump, displacement)"))
                .default_open(true)
                .show(ui, |ui| changed |= relief_controls(ui, m));
        }
    }
    changed |= mtlx_values(ui, m);
    // Con una mappa il valore del cursore la moltiplica: il materiale segue la mappa.
    let from_map = |ui: &mut egui::Ui, tex: &Option<std::path::PathBuf>| {
        if let Some(p) = tex {
            ui.label(egui::RichText::new(tr("× mappa")).small().weak()).on_hover_text(tf(
                "Il valore viene dalla mappa {} (il cursore la moltiplica: 1 = mappa così com'è)",
                &[format!("{}", p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default())],
            ));
        }
    };
    egui::Grid::new("mat-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Colore base"));
        ui.horizontal(|ui| {
            changed |= ui.color_edit_button_rgb(&mut m.base_color).changed();
            from_map(ui, &m.base_color_texture);
        });
        ui.end_row();
        ui.label(tr("Metallic"));
        ui.horizontal(|ui| {
            changed |= ui.add(egui::Slider::new(&mut m.metallic, 0.0..=1.0)).changed();
            from_map(ui, &m.metallic_texture);
        });
        ui.end_row();
        ui.horizontal(|ui| {
            ui.label(tr("Roughness"));
            help_button(ui, "mat.roughness");
        });
        ui.horizontal(|ui| {
            changed |= ui
                .add(egui::Slider::new(&mut m.roughness, 0.0..=1.0))
                .help("mat.roughness")
                .changed();
            from_map(ui, &m.roughness_texture);
        });
        ui.end_row();
        ui.label(tr("Specular"));
        changed |= ui
            .add(egui::Slider::new(&mut m.specular, 0.0..=1.0))
            .on_hover_text(tr("0.5 = riflesso fisico derivato dall'IOR"))
            .changed();
        ui.end_row();
        ui.horizontal(|ui| {
            ui.label("IOR");
            help_button(ui, "mat.ior");
        });
        changed |= ui.add(egui::Slider::new(&mut m.ior, 1.0..=3.0)).help("mat.ior").changed();
        ui.end_row();
        ui.label(tr("Transmission"));
        changed |= ui
            .add(egui::Slider::new(&mut m.transmission, 0.0..=1.0))
            .on_hover_text(tr("Trasparenza dielettrica (vetro, liquidi)"))
            .changed();
        ui.end_row();
        if m.transmission > 0.0 {
            ui.label(tr("Colore in volume"));
            changed |= ui
                .color_edit_button_rgb(&mut m.volume_color)
                .on_hover_text(tr("Colore della luce dopo «Profondità colore» di percorso dentro l'oggetto. \
                     Per i liquidi usa questo e lascia bianco il colore base: più scuro dove è \
                     profondo, più chiaro sui bordi, come nella realtà."))
                .changed();
            ui.end_row();
            ui.label(tr("Profondità colore"));
            changed |= ui
                .add(
                    egui::DragValue::new(&mut m.volume_depth_cm)
                        .range(0.01..=100000.0)
                        .speed(0.1)
                        .suffix(tr(" cm")),
                )
                .changed();
            ui.end_row();
            ui.label(tr("Priorità volume"));
            changed |= ui
                .add(egui::DragValue::new(&mut m.priority).range(-100..=100))
                .on_hover_text(tr("Liquido in un bicchiere: modella il liquido un filo più grande della cavità \
                     (che entri nello spessore del vetro) e dagli priorità più alta del vetro. \
                     Il motore usa allora l'interfaccia vetro-liquido giusta (IOR da 1,5 a 1,33)."))
                .changed();
            ui.end_row();
        }
        ui.label(tr("Emissione"));
        changed |= ui.color_edit_button_rgb(&mut m.emission).changed();
        ui.end_row();
        ui.label(tr("Intensità emissione"));
        changed |= ui
            .add(egui::DragValue::new(&mut m.emission_strength).speed(0.1).range(0.0..=10000.0))
            .changed();
        ui.end_row();
        ui.label(tr("Categoria"));
        changed |= ui.text_edit_singleline(&mut m.category).changed();
        ui.end_row();
    });
    ui.add_space(4.0);
    ui.strong(tr("Texture"));
    egui::Grid::new("tex-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Colore base"));
        if texture_slot(ui, &mut m.base_color_texture, "base") {
            if m.base_color_texture.is_some() {
                m.base_color = [1.0; 3]; // la texture viene moltiplicata per il colore base
            }
            changed = true;
        }
        ui.end_row();
        ui.label(tr("Roughness"));
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
                .checkbox(&mut m.roughness_is_gloss, tr("È una mappa di lucidità (gloss)"))
                .on_hover_text(tr(
                    "Quixel e alcuni flussi Substance usano «gloss»: chiaro = lucido. Viene invertita.",
                ))
                .changed();
            ui.end_row();
        }
        ui.label(tr("Metallicità"));
        if texture_slot(ui, &mut m.metallic_texture, "metal") {
            if m.metallic_texture.is_some() {
                m.metallic = 1.0; // la texture viene moltiplicata per la metallicità
            }
            changed = true;
        }
        ui.end_row();
        ui.label(tr("Normal map"));
        changed |= texture_slot(ui, &mut m.normal_texture, "normal");
        ui.end_row();
        if m.normal_texture.is_some() {
            ui.label("");
            changed |= ui
                .checkbox(&mut m.normal_directx, tr("Formato DirectX"))
                .on_hover_text(tr(
                    "Le normal map «DX» (Unreal, Quixel, *_NormalDX) hanno il verde invertito rispetto a OpenGL",
                ))
                .changed();
            ui.end_row();
        }
        ui.label(tr("Rilievo (height)"));
        changed |= texture_slot(ui, &mut m.height_texture, "height");
        ui.end_row();
        ui.label(tr("Occlusione (AO)"));
        changed |= texture_slot(ui, &mut m.ao_texture, "ao");
        ui.end_row();
        if m.ao_texture.is_some() {
            ui.label(tr("Intensità AO"));
            changed |= ui
                .add(egui::Slider::new(&mut m.ao_strength, 0.0..=1.0))
                .on_hover_text(tr(
                    "Scurisce le cavità. Il path tracer calcola già l'occlusione reale: valori bassi sono più fedeli.",
                ))
                .changed();
            ui.end_row();
        }
        ui.label(tr("Opacità (ritaglio)"));
        changed |= texture_slot(ui, &mut m.opacity_texture, "opacity");
        ui.end_row();
    });
    if changed {
        app.appearance_changed();
    }
    crate::ui::central::material_preview(app, ui);
}

/// Scala, proiezione, rotazione e spostamento delle texture del materiale.
pub(crate) fn mapping_controls(ui: &mut egui::Ui, m: &mut Material, obj_size: Option<f32>) -> bool {
    let mut changed = false;
    let mp = &mut m.mapping;
    ui.horizontal(|ui| {
        ui.label(tr("Proiezione"));
        changed |= ui
            .radio_value(&mut mp.projection, forge_scene::Projection::Box, tr("Box (CAD)"))
            .on_hover_text(tr("Proiezione cubica sfumata: non richiede UV, la scala è in metri reali"))
            .changed();
        changed |= ui
            .radio_value(&mut mp.projection, forge_scene::Projection::Uv, tr("UV della mesh"))
            .on_hover_text(tr("Usa le coordinate UV del modello (glTF/OBJ esportati con UV)"))
            .changed();
    });
    match mp.projection {
        forge_scene::Projection::Box => {
            ui.label(tr("Dimensione di una ripetizione")).on_hover_text(tr(
                "Lato reale di una piastrella della texture: più piccola = più ripetizioni (es. mattoni più piccoli)",
            ));
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
                    .small_button(tr("÷2"))
                    .on_hover_text(tr("Texture più piccola (il doppio delle ripetizioni)"))
                    .clicked()
                {
                    mp.scale = (mp.scale * 0.5).max(0.0001);
                    changed = true;
                }
                if ui
                    .small_button(tr("×2"))
                    .on_hover_text(tr("Texture più grande (metà delle ripetizioni)"))
                    .clicked()
                {
                    mp.scale = (mp.scale * 2.0).min(1000.0);
                    changed = true;
                }
            });
            if let Some(size) = obj_size {
                let mut reps = size / mp.scale.max(1e-6);
                ui.horizontal(|ui| {
                    ui.label(tr("Ripetizioni sull'oggetto"));
                    if ui
                        .add(egui::DragValue::new(&mut reps).speed(0.1).range(0.01..=100000.0).max_decimals(1))
                        .on_hover_text(tf(
                            "Quante volte la texture si ripete sul lato più lungo dell'oggetto selezionato ({})",
                            &[fmt_len(size).to_string()],
                        ))
                        .changed()
                    {
                        mp.scale = (size / reps.max(0.01)).clamp(0.0001, 1000.0);
                        changed = true;
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                use forge_scene::TextureAnchor;
                ui.label(tr("Ancoraggio"));
                for (a, label, hint) in [
                    (
                        TextureAnchor::Component,
                        tr("Pezzo"),
                        tr("La texture segue il singolo pezzo quando lo sposti o lo ruoti"),
                    ),
                    (
                        TextureAnchor::Assembly,
                        tr("Assieme"),
                        tr("Una sola proiezione per tutto l'assieme: venature continue tra i pezzi, segue l'assieme"),
                    ),
                    (
                        TextureAnchor::World,
                        tr("Mondo"),
                        tr("Texture ferma nella scena: l'oggetto ci scorre sopra quando lo muovi (come fino alla 0.14)"),
                    ),
                ] {
                    changed |= ui.radio_value(&mut mp.anchor, a, label).on_hover_text(hint).changed();
                }
            });
        }
        forge_scene::Projection::Uv => {
            ui.horizontal(|ui| {
                ui.label(tr("Ripetizioni"));
                changed |= ui
                    .add(egui::Slider::new(&mut mp.scale, 0.01..=1000.0).logarithmic(true).max_decimals(2))
                    .on_hover_text(tr("Quante volte la texture si ripete sull'intervallo UV 0–1 del modello"))
                    .changed();
            });
        }
    }
    ui.horizontal(|ui| {
        ui.label(tr("Rotazione"));
        changed |= ui
            .add(egui::Slider::new(&mut mp.rotation_deg, -180.0..=180.0).suffix("°"))
            .changed();
    });
    ui.horizontal(|ui| {
        ui.label(tr("Spostamento"));
        changed |= ui
            .add(egui::DragValue::new(&mut mp.offset[0]).speed(0.01).prefix(tr("U ")))
            .changed();
        changed |= ui
            .add(egui::DragValue::new(&mut mp.offset[1]).speed(0.01).prefix(tr("V ")))
            .changed();
        if ui.small_button(tr("Azzera")).clicked() {
            mp.offset = [0.0; 2];
            mp.rotation_deg = 0.0;
            changed = true;
        }
    });
    changed
}

/// Valori scritti nel file MaterialX scaricato con il materiale (se c'è): si mostrano e si
/// applicano su richiesta. Il file si rilegge solo quando cambia materiale (cache per percorso).
fn mtlx_values(ui: &mut egui::Ui, m: &mut Material) -> bool {
    let Some(dir) = forge_library::store::dir_of(&m.id) else {
        return false;
    };
    let Some(path) = forge_library::mtlx::find_in(&dir) else {
        return false;
    };
    let key = egui::Id::NULL.with(("mtlx", path.to_string_lossy().into_owned()));
    let values: std::sync::Arc<Vec<forge_library::mtlx::MtlxValue>> = ui.ctx().data_mut(|d| {
        d.get_temp(key).unwrap_or_else(|| {
            let v = std::sync::Arc::new(forge_library::mtlx::read(&path).unwrap_or_default());
            d.insert_temp(key, v.clone());
            v
        })
    });
    let mut changed = false;
    egui::CollapsingHeader::new(tr("Valori del file del materiale (MaterialX)"))
        .default_open(false)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(tf(
                    "{}: valori scritti dalla fonte (non collegati a mappe).",
                    &[format!("{}", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default())],
                ))
                .small()
                .weak(),
            );
            let shown: Vec<_> = values.iter().filter(|v| v.ty != "filename" && v.ty != "string").collect();
            if shown.is_empty() {
                ui.label(egui::RichText::new(tr("Nessun valore numerico nel file.")).small());
            }
            for v in shown {
                ui.label(
                    egui::RichText::new(format!("{}.{} = {}", v.node, v.input, v.value))
                        .small()
                        .monospace(),
                );
            }
            if let (Some(scale), true) = (forge_library::mtlx::displacement_scale(&values), m.height_texture.is_some()) {
                let mm = scale * 1000.0;
                if ui
                    .button(tf("Usa la scala del displacement del file ({mm:.1} mm)", &[format!("{:.1}", mm)]))
                    .on_hover_text(tr("MaterialX esprime la scala in unità della scena (metri): valore × 1000 = mm. \
                         Da verificare sul risultato: applicalo e guarda il render."))
                    .clicked()
                {
                    m.displacement_mm = mm.clamp(0.0, 1000.0);
                    changed = true;
                }
            }
        });
    changed
}

/// Normal map, bump e displacement vero.
pub(crate) fn relief_controls(ui: &mut egui::Ui, m: &mut Material) -> bool {
    let mut changed = false;
    egui::Grid::new("relief-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        if m.normal_texture.is_some() {
            ui.label(tr("Intensità normal map"));
            changed |= ui
                .add(egui::Slider::new(&mut m.normal_strength, 0.0..=4.0))
                .on_hover_text(tr("Dettaglio fine dell'ombreggiatura (non cambia la sagoma)"))
                .changed();
            ui.end_row();
        }
        if m.height_texture.is_some() {
            ui.label(tr("Bump (mm)"));
            changed |= ui
                .add(egui::Slider::new(&mut m.bump_mm, 0.0..=20.0).max_decimals(2))
                .on_hover_text(tr(
                    "Bump mapping dalla height: solo ombreggiatura. 0 = spento (con la normal map di solito non serve)",
                ))
                .changed();
            ui.end_row();
            ui.label(tr("Displacement (mm)"));
            changed |= ui
                .add(egui::Slider::new(&mut m.displacement_mm, 0.0..=200.0).max_decimals(1))
                .on_hover_text(tr(
                    "Displacement vero: la superficie viene suddivisa e spostata secondo la height map, \
                     cambia anche la sagoma e le ombre. 0 = spento. Più pesante da preparare.",
                ))
                .changed();
            ui.end_row();
            if m.displacement_mm <= 0.0 {
                ui.label("");
                if ui
                    .small_button(tr("Valore proposto"))
                    .on_hover_text(tr(
                        "Accende il displacement al 3 % del lato della texture: per materiali con rilievo vero \
                         (pietre, mattoni, cortecce). Per metalli, vernici e plastiche lascialo a 0.",
                    ))
                    .clicked()
                {
                    m.displacement_mm = forge_library::store::default_displacement_mm(Some(m.mapping.scale));
                    changed = true;
                }
                ui.end_row();
            } else {
                ui.label(tr("Livello medio"));
                changed |= ui
                    .add(egui::Slider::new(&mut m.displacement_mid, 0.0..=1.0))
                    .on_hover_text(tr(
                        "Grigio che lascia la superficie dov'è: 0.5 = metà in fuori e metà in dentro, 0 = solo in fuori",
                    ))
                    .changed();
                ui.end_row();
            }
        }
    });
    if m.displacement_mm > 0.0 && m.height_texture.is_some() {
        ui.label(
            egui::RichText::new(tr("Il displacement si vede nel render (non nella viewport di modellazione)."))
                .small()
                .weak(),
        );
    }
    changed
}

fn fmt_len(m: f32) -> String {
    if m < 0.01 {
        tf("{:.1} mm", &[format!("{:.1}", m * 1000.0)])
    } else if m < 1.0 {
        tf("{:.1} cm", &[format!("{:.1}", m * 100.0)])
    } else {
        tf("{m:.2} m", &[format!("{:.2}", m)])
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
                    if ui.small_button(tr("×")).on_hover_text(tr("Rimuovi texture")).clicked() {
                        *slot = None;
                        changed = true;
                    }
                }
                None => {
                    ui.label(egui::RichText::new("nessuna").weak());
                }
            }
            if ui.small_button(tr("Scegli…")).clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter(tr("Immagini"), &["png", "jpg", "jpeg", "exr", "hdr"])
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
    // Preset e luci nuove vicino a selezione o prodotto, non a tutta la scena (pavimento compreso).
    let (center, radius) = app.anchor();
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Preset:"));
        for (id, name) in light::PRESETS {
            if ui
                .button(tr(name))
                .on_hover_text(tr("Sostituisce tutte le luci, adattandole alle dimensioni della scena"))
                .clicked()
            {
                app.scene.lights = light::preset(id, center, radius).unwrap_or_default();
                app.selected_light = None;
                app.shading_changed();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Aggiungi:"));
        for kind in NewLight::ALL {
            let mut r = ui.button(tr(kind.label()));
            if kind == NewLight::Portal {
                r = r.on_hover_text(tr("Per gli interni illuminati dall'HDRI: il renderer cerca la luce del cielo \
                 attraverso questo rettangolo invece che a caso, molta meno grana. \
                 Selezionare prima il vetro o il foro della finestra lo mette già al posto giusto."));
            }
            if r.clicked() {
                app.add_light(kind);
            }
        }
    });
    ui.separator();

    let Some(i) = app.selected_light.filter(|&i| i < app.scene.lights.len()) else {
        ui.label(egui::RichText::new(tr("Seleziona una luce nell'elenco a sinistra.")).weak());
        return;
    };
    let mut changed = false;
    let mut delete = false;
    let mut duplicate = false;
    {
        let l = &mut app.scene.lights[i];
        ui.horizontal(|ui| {
            ui.label(tr("Nome"));
            changed |= ui.text_edit_singleline(&mut l.name).changed();
        });
        let mut kind_idx = match l.kind {
            LightKind::Area { .. } => 0,
            LightKind::Point => 1,
            LightKind::Spot { .. } => 2,
            LightKind::Directional => 3,
            LightKind::Portal { .. } => 4,
        };
        let before = kind_idx;
        egui::ComboBox::from_label(tr("Tipo"))
            .selected_text(tr(l.kind.label()))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut kind_idx, 0, tr("Area (rettangolare)"));
                ui.selectable_value(&mut kind_idx, 1, tr("Puntiforme"));
                ui.selectable_value(&mut kind_idx, 2, tr("Spot"));
                ui.selectable_value(&mut kind_idx, 3, tr("Direzionale"));
                ui.selectable_value(&mut kind_idx, 4, tr("Portale (finestra)"));
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
                4 => LightKind::Portal { width: r, height: r },
                _ => LightKind::Directional,
            };
            changed = true;
        }
        changed |= ui.checkbox(&mut l.enabled, tr("Accesa")).changed();
        let portal = matches!(l.kind, LightKind::Portal { .. });
        if portal {
            ui.label(
                egui::RichText::new(tr("Il portale non emette luce: indica da dove entra quella dell'ambiente (HDRI). \
                     Mettilo sul foro della finestra, con la freccia verso l'interno e grande quanto \
                     l'apertura. Togli o rendi invisibile il vetro della finestra, altrimenti blocca i raggi."))
                .small()
                .weak(),
            );
        }
        egui::Grid::new("light-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            if portal {
                if let LightKind::Portal { width, height } = &mut l.kind {
                    ui.label(tr("Larghezza (m)"));
                    changed |= ui.add(egui::DragValue::new(width).speed(0.01).range(1e-4..=1e4)).changed();
                    ui.end_row();
                    ui.label(tr("Altezza (m)"));
                    changed |= ui.add(egui::DragValue::new(height).speed(0.01).range(1e-4..=1e4)).changed();
                    ui.end_row();
                }
                return;
            }
            ui.label(tr("Colore"));
            changed |= ui.color_edit_button_rgb(&mut l.color).changed();
            ui.end_row();
            ui.label(tr("Temperatura"));
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
                LightKind::Area { .. } => tr("Radianza (W/sr/m²)"),
                LightKind::Directional => tr("Irradianza (W/m²)"),
                _ => tr("Intensità (W/sr)"),
            };
            ui.label(unit);
            changed |= ui
                .add(egui::DragValue::new(&mut l.intensity).speed(0.05).range(0.0..=1e7))
                .changed();
            ui.end_row();
            match &mut l.kind {
                LightKind::Area { width, height } => {
                    ui.label(tr("Larghezza (m)"));
                    changed |= ui.add(egui::DragValue::new(width).speed(0.01).range(1e-4..=1e4)).changed();
                    ui.end_row();
                    ui.label(tr("Altezza (m)"));
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
                    ui.label(tr("Apertura cono (°)"));
                    if ui
                        .add(egui::Slider::new(outer_angle_deg, 1.0..=170.0))
                        .on_hover_text(tr(
                            "Angolo pieno del fascio (si regola anche trascinando la maniglia piena del cono nella viewport)",
                        ))
                        .changed()
                    {
                        *inner_angle_deg = *outer_angle_deg * (1.0 - soft / 100.0);
                        changed = true;
                    }
                    ui.end_row();
                    ui.label(tr("Sfumatura bordo (%)"));
                    if ui
                        .add(egui::Slider::new(&mut soft, 0.0..=100.0).suffix(" %"))
                        .on_hover_text(tr("0 % = bordo netto, 100 % = sfuma dal centro (maniglia vuota nella viewport)"))
                        .changed()
                    {
                        *inner_angle_deg = *outer_angle_deg * (1.0 - soft / 100.0);
                        changed = true;
                    }
                    ui.end_row();
                }
                _ => {}
            }
            if matches!(l.kind, LightKind::Spot { .. }) {
                // Fascio visibile: foschia nel cono (forma = apertura e sfumatura del cono).
                let mut beam = l.beam_density > 0.0;
                ui.label(tr("Fascio visibile"));
                if ui
                    .checkbox(&mut beam, tr("Volumetrico"))
                    .on_hover_text(tr(
                        "Mostra il cono di luce nell'aria, come in una sala con un po' di foschia: forma e bordo seguono l'apertura e la sfumatura del cono",
                    ))
                    .changed()
                {
                    l.beam_density = if beam { 0.5 } else { 0.0 };
                    if beam && l.beam_length <= 0.0 {
                        l.beam_length = ((l.target - l.position).length() * 1.1).max(0.1);
                    }
                    changed = true;
                }
                ui.end_row();
                if l.beam_density > 0.0 {
                    ui.label(tr("Intensità del fascio"));
                    changed |= ui
                        .add(egui::Slider::new(&mut l.beam_density, 0.02..=5.0).logarithmic(true))
                        .on_hover_text(tr("Densità della foschia nel cono: più alta = fascio più marcato e luminoso"))
                        .changed();
                    ui.end_row();
                    ui.label(tr("Lunghezza del fascio (m)"));
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut l.beam_length)
                                .speed((radius * 0.01).max(0.001))
                                .range(0.01..=1e4),
                        )
                        .on_hover_text(tr("Fin dove si vede il fascio (sfuma nell'ultimo quinto)"))
                        .changed();
                    ui.end_row();
                }
            }
            if l.has_range() {
                // Raggio d'azione: indipendente dall'intensità (la bolla nella viewport).
                let mut limited = l.range > 0.0;
                ui.label(tr("Raggio d'azione"));
                if ui
                    .checkbox(&mut limited, tr("Limitato"))
                    .on_hover_text(tr(
                        "Spento = luce fisica, arriva ovunque. Acceso = la luce si ferma al bordo della bolla",
                    ))
                    .changed()
                {
                    l.range = if limited {
                        ((l.position - center).length() * 1.5).max(radius).max(0.1)
                    } else {
                        0.0
                    };
                    changed = true;
                }
                ui.end_row();
                if l.range > 0.0 {
                    ui.label(tr("Distanza massima (m)"));
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut l.range)
                                .speed((radius * 0.01).max(0.001))
                                .range(0.01..=1e4),
                        )
                        .on_hover_text(tr("Fin dove arriva la luce (bolla piena nella viewport)"))
                        .changed();
                    ui.end_row();
                    let mut soft = l.range_softness * 100.0;
                    ui.label(tr("Sfumatura del raggio (%)"));
                    if ui
                        .add(egui::Slider::new(&mut soft, 0.0..=100.0).suffix(" %"))
                        .on_hover_text(tr(
                            "0 % = taglio netto al bordo, 100 % = sfuma dalla sorgente al bordo (bolla tratteggiata)",
                        ))
                        .changed()
                    {
                        l.range_softness = soft / 100.0;
                        changed = true;
                    }
                    ui.end_row();
                }
            }
        });
        changed |= vec3_edit(ui, tr("Posizione"), &mut l.position, (radius * 0.01) as f64);
        if !matches!(l.kind, LightKind::Point) {
            changed |= vec3_edit(ui, tr("Bersaglio"), &mut l.target, (radius * 0.01) as f64);
            if ui.small_button(tr("Punta al centro della scena")).clicked() {
                l.target = center;
                changed = true;
            }
        }
        if !portal {
            changed |= ui.checkbox(&mut l.cast_shadows, tr("Proietta ombre")).changed();
        }
        if matches!(l.kind, LightKind::Area { .. }) {
            changed |= ui.checkbox(&mut l.visible_to_camera, tr("Visibile alla camera")).changed();
        }
        ui.horizontal(|ui| {
            duplicate = ui.button(tr("Duplica")).clicked();
            delete = ui.button(tr("Elimina")).clicked();
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
        ui.radio_value(&mut kind, 0, tr("Uniforme"));
        ui.radio_value(&mut kind, 1, tr("Cielo a gradiente"));
        ui.radio_value(&mut kind, 2, tr("Mappa HDRI"));
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
                ui.label(tr("Colore"));
                changed |= ui.color_edit_button_rgb(color).changed();
                ui.end_row();
                ui.label(tr("Intensità"));
                changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).logarithmic(true)).changed();
                ui.end_row();
            }
            Environment::Gradient {
                zenith,
                horizon,
                ground,
                strength,
            } => {
                ui.label(tr("Zenit"));
                changed |= ui.color_edit_button_rgb(zenith).changed();
                ui.end_row();
                ui.label(tr("Orizzonte"));
                changed |= ui.color_edit_button_rgb(horizon).changed();
                ui.end_row();
                ui.label(tr("Suolo"));
                changed |= ui.color_edit_button_rgb(ground).changed();
                ui.end_row();
                ui.label(tr("Intensità"));
                changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).logarithmic(true)).changed();
                ui.end_row();
            }
            Environment::Hdri {
                path,
                strength,
                rotation_deg,
            } => {
                ui.label(tr("File"));
                ui.horizontal(|ui| {
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    ui.label(name).on_hover_text(path.display().to_string());
                    repick = ui.small_button(tr("Cambia…")).clicked();
                });
                ui.end_row();
                ui.label(tr("Intensità"));
                changed |= ui.add(egui::Slider::new(strength, 0.0..=20.0).logarithmic(true)).changed();
                ui.end_row();
                ui.label(tr("Rotazione (°)"));
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
        .checkbox(&mut app.settings.environment_visible, tr("Ambiente visibile come sfondo"))
        .on_hover_text(tr(
            "Se disattivato lo sfondo è nero, ma l'ambiente continua a illuminare e a riflettersi",
        ))
        .changed()
    {
        app.unsaved = true;
    }
    if matches!(app.scene.environment, Environment::Hdri { .. }) {
        ui.label(
            egui::RichText::new(tr(
                "La HDRI illumina la scena con importance sampling (sole e finestre campionati direttamente). \
                 Mappe gratuite CC0: polyhaven.com (anche dal pulsante «Scarica HDRI…»).",
            ))
            .weak()
            .small(),
        );
    }
}

/// Effetti della scena: in alto i tipi che si possono aggiungere, sotto quelli presenti,
/// ciascuno con la spunta per disattivarlo senza cancellarlo e le opzioni a scomparsa.
fn effects(app: &mut ForgeApp, ui: &mut egui::Ui) {
    // Sopra la selezione: centro del lato superiore dei componenti selezionati.
    let above_selection = {
        let mut b = forge_math::Aabb::EMPTY;
        for &i in &app.selection {
            b.grow(&app.scene.bounds_of(i));
        }
        (!b.is_empty()).then(|| Vec3::new(b.center().x, b.max.y + 0.03, b.center().z))
    };
    ui.strong(tr("Aggiungi effetto"));
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(tr("Ghiaccio secco"))
            .on_hover_text(tr(
                "Nebbia pesante da un emettitore (sopra l'oggetto selezionato o al centro della scena): \
                 cade, riempie i contenitori, trabocca e si stende a terra",
            ))
            .clicked()
        {
            let n = app.scene.fogs.len() + 1;
            app.scene.fogs.push(forge_scene::FogVolume {
                name: if n == 1 {
                    tr("Ghiaccio secco").into()
                } else {
                    tf("Ghiaccio secco {n}", &[format!("{}", n)])
                },
                seed: n as u32,
                emitter: above_selection.unwrap_or(Vec3::new(0.0, 0.3, 0.0)),
                ..Default::default()
            });
            app.selected_fog = Some(app.scene.fogs.len() - 1);
            app.selected_light = None;
            app.shading_changed();
        }
        for (name, tip) in [
            (tr("Foschia d'ambiente"), tr("Aria velata e raggi di luce visibili (in roadmap)")),
            (tr("Adesivo (decal)"), tr("Immagine applicata su una superficie (in roadmap)")),
        ] {
            ui.add_enabled(false, egui::Button::new(name)).on_disabled_hover_text(tip);
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.strong(tf("Nella scena ({})", &[format!("{}", app.scene.fogs.len())]));
        if app.is_baking_fog() {
            ui.spinner();
            ui.label(egui::RichText::new(tr("calcolo della nebbia…")).small());
        }
    });
    if app.scene.fogs.is_empty() {
        ui.label(egui::RichText::new(tr("Nessun effetto: aggiungine uno dai pulsanti qui sopra.")).weak());
        return;
    }
    let mut remove = None;
    let mut changed = false;
    let mut select = None;
    let selected = app.selected_fog;
    let mut zone_mode = app.fog_gizmo_zone;
    for (i, f) in app.scene.fogs.iter_mut().enumerate() {
        ui.push_id(("nebbia", i), |ui| {
            let id = ui.make_persistent_id("effetto");
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
            // Appena selezionato (anche dalla viewport) l'effetto si apre; poi lo si può richiudere.
            let opened_for = egui::Id::NULL.with("effetto-aperto-per-selezione");
            let last: Option<usize> = ui.ctx().data(|d| d.get_temp(opened_for)).flatten();
            if selected == Some(i) && last != Some(i) {
                state.set_open(true);
                ui.ctx().data_mut(|d| d.insert_temp(opened_for, Some(i)));
            }
            state
                .show_header(ui, |ui| {
                    changed |= ui
                        .checkbox(&mut f.enabled, "")
                        .on_hover_text(tr("Attivo nel render (togliendo la spunta resta nella scena, spento)"))
                        .changed();
                    if ui
                        .selectable_label(selected == Some(i), &f.name)
                        .on_hover_text(tr("Seleziona: il gizmo sposta l'emettitore"))
                        .clicked()
                    {
                        select = Some(i);
                    }
                    ui.label(egui::RichText::new(tr("ghiaccio secco")).small().weak());
                    if ui.small_button(tr("Elimina")).clicked() {
                        remove = Some(i);
                    }
                })
                .body(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("Nome"));
                        ui.text_edit_singleline(&mut f.name);
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("Il gizmo sposta"));
                        for (zone, name, tip) in [
                            (
                                false,
                                tr("Emettitore"),
                                tr("Sposta il punto da cui esce la nebbia (la zona lo segue)"),
                            ),
                            (
                                true,
                                tr("Zona"),
                                tr("Sposta il riquadro simulato lasciando l'emettitore dov'è (es. più in alto per gli sbuffi)"),
                            ),
                        ] {
                            if ui.selectable_label(zone_mode == zone, name).on_hover_text(tip).clicked() {
                                zone_mode = zone;
                                select = Some(i);
                            }
                        }
                    });
                    changed |= vec3_edit(ui, tr("Emettitore (m)"), &mut f.emitter, 0.01);
                    if let Some(p) = above_selection {
                        if ui
                            .small_button(tr("Metti sopra la selezione"))
                            .on_hover_text(tr("Sposta l'emettitore sul lato superiore dei componenti selezionati"))
                            .clicked()
                        {
                            f.emitter = p;
                            changed = true;
                        }
                    }
                    changed |= vec3_edit(ui, tr("Zona simulata (m)"), &mut f.size, 0.01);
                    ui.horizontal(|ui| {
                        changed |= vec3_edit(ui, tr("Spostamento zona (m)"), &mut f.zone_offset, 0.01);
                        if f.zone_offset != Vec3::ZERO
                            && ui
                                .small_button(tr("Azzera"))
                                .on_hover_text(tr("Zona di nuovo attorno all'emettitore"))
                                .clicked()
                        {
                            f.zone_offset = Vec3::ZERO;
                            changed = true;
                        }
                    });
                    let (lo, hi) = f.bounds();
                    if (f.emitter.cmplt(lo) | f.emitter.cmpgt(hi)).any() {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            tr("L'emettitore è fuori dalla zona: non uscirà nebbia."),
                        );
                    }
                    // Preset scenografici: impostano tutto il look; l'emettitore resta dov'è.
                    ui.label(tr("Preset (passa sopra per vedere dove mettere l'emettitore):"));
                    ui.horizontal_wrapped(|ui| {
                        for p in forge_scene::FOG_PRESETS.iter() {
                            if ui.small_button(tr(p.name)).on_hover_text(tr(p.hint)).clicked() {
                                p.apply(f);
                                changed = true;
                            }
                        }
                    });
                    egui::Grid::new("fog-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label(tr("Seed"));
                        ui.horizontal(|ui| {
                            changed |= ui.add(egui::DragValue::new(&mut f.seed)).changed();
                            if ui
                                .small_button(tr("Nuovo"))
                                .on_hover_text(tr("Un'altra forma con gli stessi parametri"))
                                .clicked()
                            {
                                f.seed = f.seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453) % 100_000;
                                changed = true;
                            }
                        });
                        ui.end_row();
                        ui.label(tr("Tempo simulato (s)"));
                        changed |= ui
                            .add(egui::Slider::new(&mut f.sim_time, 0.0..=30.0).max_decimals(1))
                            .on_hover_text(tr(
                                "Secondi di emissione: la nebbia è calcolata in quell'istante (più tempo = pozza più grande)",
                            ))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Quantità emessa"));
                        changed |= ui.add(egui::Slider::new(&mut f.emission_rate, 0.0..=5.0)).changed();
                        ui.end_row();
                        ui.label(tr("Raggio sorgente (m)"));
                        changed |= ui
                            .add(egui::Slider::new(&mut f.source_radius, 0.01..=1.0).logarithmic(true))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Densità"));
                        changed |= ui
                            .add(egui::Slider::new(&mut f.density, 0.0..=500.0).logarithmic(true))
                            .on_hover_text(tr("Coefficiente di estinzione della nebbia piena (1/m): più alto = più opaca"))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Scala dettagli (m)"));
                        changed |= ui
                            .add(egui::Slider::new(&mut f.scale, 0.005..=2.0).logarithmic(true))
                            .on_hover_text(tr("Dimensione di sbuffi, filamenti e vortici"))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Turbolenza"));
                        changed |= ui.add(egui::Slider::new(&mut f.turbulence, 0.0..=1.0)).changed();
                        ui.end_row();
                        ui.label(tr("Risalita"));
                        changed |= ui
                            .add(egui::Slider::new(&mut f.rise, 0.0..=1.0))
                            .on_hover_text(tr("Quanto la nebbia, scaldandosi, risale a sbuffi prima di svanire \
                             (0 = resta a terra come un liquido; alta = sbuffi sopra la sorgente)"))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Dissolvenza"));
                        changed |= ui
                            .add(egui::Slider::new(&mut f.dissipation, 0.0..=4.0))
                            .on_hover_text(tr("Velocità con cui la nebbia si dissolve nell'aria (0 = mai)"))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Colore scattering"));
                        changed |= ui.color_edit_button_rgb(&mut f.albedo).changed();
                        ui.end_row();
                        ui.label(tr("Intensità scattering"));
                        let mut a = f.albedo.iter().cloned().fold(0.0f32, f32::max);
                        if ui
                            .add(egui::Slider::new(&mut a, 0.0..=1.0))
                            .on_hover_text(tr("Quanta luce la nebbia diffonde invece di assorbire (1 = bianca, 0 = fumo nero)"))
                            .changed()
                        {
                            let cur = f.albedo.iter().cloned().fold(0.0f32, f32::max).max(1e-4);
                            f.albedo = f.albedo.map(|v| (v / cur * a).clamp(0.0, 1.0));
                            if cur <= 1e-4 {
                                f.albedo = [a; 3];
                            }
                            changed = true;
                        }
                        ui.end_row();
                        ui.label(tr("Anisotropia"));
                        changed |= ui
                            .add(egui::Slider::new(&mut f.anisotropy, -0.9..=0.9))
                            .on_hover_text(tr("0 = diffusione uniforme; positiva = più luminosa in controluce"))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Collisioni"));
                        changed |= ui
                            .checkbox(&mut f.collisions, tr("Non attraversa gli oggetti"))
                            .on_hover_text(tr("La nebbia si appoggia sugli oggetti e ci scorre attorno"))
                            .changed();
                        ui.end_row();
                        ui.label(tr("Qualità griglia"));
                        ui.horizontal_wrapped(|ui| {
                            for (name, cells, res) in forge_scene::FOG_QUALITY {
                                let on = f.sim_cells == cells && f.resolution == res;
                                let hint = tf(
                                    "Simulazione fino a {cells} celle, dettaglio {res} celle sul lato più lungo della zona. \
                                     Ogni livello in su rende il moto più fine ma il calcolo 3–5 volte più lento \
                                     (Ultra: minuti e qualche centinaio di MB di memoria).",
                                    &[format!("{}", cells), format!("{}", res)],
                                );
                                if ui.selectable_label(on, tr(name)).on_hover_text(hint).clicked() {
                                    f.sim_cells = cells;
                                    f.resolution = res;
                                    changed = true;
                                }
                            }
                        });
                        ui.end_row();
                        ui.label(tr("Dettaglio"));
                        changed |= ui
                            .add(egui::Slider::new(&mut f.resolution, 32..=400))
                            .on_hover_text(tr("Celle della griglia del dettaglio (sbuffi, bordi) sul lato più lungo. \
                                 Il moto lo decide la qualità della griglia; una zona più piccola dà celle più fini."))
                            .changed();
                        ui.end_row();
                    });
                });
        });
    }
    app.fog_gizmo_zone = zone_mode;
    if let Some(i) = select {
        app.selected_fog = Some(i);
        app.selected_light = None;
        app.selection.clear();
    }
    if let Some(i) = remove {
        app.scene.fogs.remove(i);
        app.selected_fog = None;
        changed = true;
    }
    if changed {
        app.shading_changed();
    }
}

/// Elenco delle HDRI della cartella `Libreria/HDRI`: un clic e diventano l'ambiente.
fn hdri_library(app: &mut ForgeApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.separator();
    ui.horizontal(|ui| {
        ui.strong(tr("HDRI della libreria"));
        if ui
            .small_button(tr("Apri cartella"))
            .on_hover_text(forge_library::paths::hdri_dir().display().to_string())
            .clicked()
        {
            crate::ui::library::open_folder(&forge_library::paths::hdri_dir());
        }
        if ui
            .small_button(tr("Scarica HDRI…"))
            .on_hover_text(tr("HDRI gratuite (CC0) da Poly Haven"))
            .clicked()
        {
            app.library.open_hdri_tab();
        }
    });
    let list = app.library.hdri_list();
    if list.is_empty() {
        ui.label(
            egui::RichText::new(tr(
                "Nessuna HDRI: copia i file .hdr/.exr nella cartella HDRI della libreria, oppure scaricane una.",
            ))
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
        .add_filter(tr("HDRI (.hdr, .exr)"), &["hdr", "exr"])
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
    ui.horizontal(|ui| {
        if ui
            .selectable_label(app.aim_pick, tr("Punta su oggetto"))
            .on_hover_text(tr("Poi clicca un oggetto nella viewport: la camera lo inquadra, lo mette a fuoco \
                 e ci gira attorno restando agganciata"))
            .clicked()
        {
            app.aim_pick = !app.aim_pick;
            app.focus_pick = false;
            app.view = crate::app::View::Viewport;
        }
        let locked = app
            .aim_lock
            .as_ref()
            .and_then(|id| app.scene.find(id))
            .map(|i| app.scene.nodes[i].name.clone());
        if let Some(name) = locked {
            ui.label(egui::RichText::new(tf("agganciata a «{name}»", std::slice::from_ref(&name))).color(style::ACCENT));
            if ui.small_button(tr("Sgancia")).clicked() {
                app.release_aim();
            }
        }
    });
    ui.separator();

    // Parametri della camera di rendering attiva, oppure della vista se non ce n'è una.
    let active = app.scene.active_camera;
    ui.strong(match active {
        Some(i) => tf("Parametri: {}", &[app.scene.render_cameras[i].name.to_string()]),
        None => tr("Parametri della vista").to_string(),
    });
    let mut changed = false;
    let mut cam = app.scene.render_camera().clone();
    changed |= lens_controls(app, ui, &mut cam);
    ui.add_space(4.0);
    let r = app.scene.bounds().radius().max(0.01) as f64;
    changed |= vec3_edit(ui, tr("Posizione"), &mut cam.position, r * 0.005);
    changed |= vec3_edit(ui, tr("Bersaglio"), &mut cam.target, r * 0.005);
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
        app.render_camera_changed();
    }
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        let views = [
            (tr("Frontale"), 0.0, 0.0),
            (tr("Destra"), std::f32::consts::FRAC_PI_2, 0.0),
            (tr("Alto"), 0.0, 1.55),
            (tr("Isometrica"), std::f32::consts::FRAC_PI_4, crate::ui::navigator::ISO_PITCH),
        ];
        for (name, yaw, pitch) in views {
            if ui.button(name).clicked() {
                app.orbit.yaw = yaw;
                app.orbit.pitch = pitch;
                // L'isometrica è per definizione ortografica.
                if name == "Isometrica" {
                    app.orbit.lens.ortho = true;
                }
                app.frame_all();
            }
        }
    });
    ui.horizontal(|ui| {
        if ui.button(tr("Inquadra tutto")).clicked() {
            app.frame_all();
        }
        if ui.button(tr("Inquadra selezione")).clicked() {
            app.frame_selection();
        }
    });
    ui.label(
        egui::RichText::new(tr(
            "Viewport: trascina = orbita · Shift/destro/centrale = pan · rotella = zoom · doppio clic = inquadra",
        ))
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
    egui::ComboBox::from_label(tr("Dispositivo"))
        .selected_text(tr(app.settings.backend.label()))
        .show_ui(ui, |ui| {
            for b in [forge_render_core::BackendKind::Cpu, forge_render_core::BackendKind::Gpu] {
                if ui.selectable_value(&mut app.settings.backend, b, tr(b.label())).changed() {
                    app.simple_gpu_checked = true;
                }
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
                ui.colored_label(style::WARN, tf("GPU non disponibile: {e}", std::slice::from_ref(&e)));
            }
        }
    }
    let s = &mut app.settings;
    let max_t = app.system.logical_threads;
    ui.horizontal(|ui| {
        ui.label(tr("Thread CPU"));
        changed |= ui
            .add(egui::Slider::new(&mut s.threads, 0..=max_t))
            .on_hover_text(tr("0 = automatico (thread logici − 1, per lasciare reattiva l'interfaccia)"))
            .changed();
        if s.threads == 0 {
            ui.label(egui::RichText::new(tf("auto: {}", &[format!("{}", s.effective_threads())])).weak());
        }
    });

    ui.separator();
    ui.strong(tr("Risoluzione"));
    ui.horizontal_wrapped(|ui| {
        for (name, w, h) in [
            ("HD", 1280, 720),
            (tr("Full HD"), 1920, 1080),
            ("QHD", 2560, 1440),
            ("4K", 3840, 2160),
            (tr("Quadrato"), 1080, 1080),
            (tr("Anteprima"), 640, 360),
        ] {
            if ui.selectable_label(s.width == w && s.height == h, name).clicked() {
                s.width = w;
                s.height = h;
                changed = true;
            }
        }
    });
    ui.horizontal(|ui| {
        changed |= ui
            .add(egui::DragValue::new(&mut s.width).range(16..=16384).suffix(tr(" px")))
            .changed();
        ui.label(tr("×"));
        changed |= ui
            .add(egui::DragValue::new(&mut s.height).range(16..=16384).suffix(tr(" px")))
            .changed();
    });

    ui.separator();
    ui.strong(tr("Campioni per pixel"));
    ui.horizontal_wrapped(|ui| {
        for n in forge_render_core::SamplesPreset::FIXED {
            if ui.selectable_label(s.target_samples == n, n.to_string()).clicked() {
                s.target_samples = n;
                changed = true;
            }
        }
        let custom = s.target_samples != 0 && !forge_render_core::SamplesPreset::FIXED.contains(&s.target_samples);
        if ui.selectable_label(custom, tr("Personalizzato")).clicked() {
            s.target_samples = app.custom_samples;
            changed = true;
        }
        if ui
            .selectable_label(s.target_samples == 0, tr("Illimitato"))
            .on_hover_text(tr("Il rendering continua finché non lo fermi"))
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
                    .prefix(tr("campioni: ")),
            )
            .changed()
    {
        app.custom_samples = s.target_samples;
        changed = true;
    }
    ui.label(
        egui::RichText::new(tr("Si può cambiare durante il rendering: l'accumulo prosegue senza ripartire."))
            .weak()
            .small(),
    );
    // Campionamento adattivo: le zone già pulite smettono di ricevere campioni.
    ui.horizontal(|ui| {
        let mut on = s.noise_threshold > 0.0;
        if ui
            .checkbox(&mut on, tr("Arresto a soglia di rumore"))
            .on_hover_text(tr(
                "Campionamento adattivo: ogni zona dell'immagine si ferma quando il rumore stimato \
                 scende sotto la soglia; il tempo va alle zone difficili (vetro, ombre, nebbia). \
                 Il numero di campioni sopra resta il massimo.",
            ))
            .changed()
        {
            s.noise_threshold = if on { 0.03 } else { 0.0 };
            changed = true;
        }
    });
    if s.noise_threshold > 0.0 {
        egui::Grid::new("adaptive-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr("Soglia di rumore"));
            changed |= ui
                .add(
                    egui::Slider::new(&mut s.noise_threshold, 0.002..=0.1)
                        .logarithmic(true)
                        .fixed_decimals(3),
                )
                .on_hover_text(tr(
                    "Più bassa = più pulita e più lenta. Sulla scena di test: 0.08 ≈ qualità di 64 campioni, \
                     0.04 ≈ 256, 0.02 ≈ 1000. Conviene con sfondi e zone facili; nelle scene scure con \
                     nebbia e scintille può costare più del rendering uniforme (in sviluppo).",
                ))
                .changed();
            ui.end_row();
            ui.label(tr("Campioni minimi"));
            changed |= ui
                .add(egui::DragValue::new(&mut s.adaptive_min_samples).range(4..=4096))
                .on_hover_text(tr(
                    "Campioni per pixel prima di valutare il rumore (evita di fermare troppo presto zone con luce rara)",
                ))
                .changed();
            ui.end_row();
        });
    }

    ui.separator();
    ui.strong(tr("Path tracing"));
    egui::Grid::new("pt-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Rimbalzi massimi"));
        changed |= ui.add(egui::Slider::new(&mut s.max_bounces, 1..=64)).changed();
        ui.end_row();
        ui.label(tr("Russian roulette da"));
        changed |= ui.add(egui::Slider::new(&mut s.russian_roulette_start, 1..=16)).changed();
        ui.end_row();
        ui.label(tr("Clamp indiretto"));
        changed |= ui
            .add(egui::DragValue::new(&mut s.clamp_indirect).speed(0.1).range(0.0..=1000.0))
            .on_hover_text(tr(
                "0 = disattivato (fisicamente corretto). Valori > 0 riducono i firefly introducendo un bias.",
            ))
            .changed();
        ui.end_row();
        ui.label(tr("Seed"));
        changed |= ui.add(egui::DragValue::new(&mut s.seed)).changed();
        ui.end_row();
    });

    ui.separator();
    ui.strong(tr("Visualizzazione (non riavvia il render)"));
    egui::ComboBox::from_label(tr("Tone mapping"))
        .selected_text(tr(s.tone_mapping.label()))
        .show_ui(ui, |ui| {
            for t in [ToneMapping::AcesFilmic, ToneMapping::Standard] {
                changed |= ui.selectable_value(&mut s.tone_mapping, t, tr(t.label())).changed();
            }
        });
    changed |= ui
        .add(egui::Slider::new(&mut s.exposure_ev, -6.0..=6.0).text(tr("Esposizione (EV)")))
        .changed();

    ui.separator();
    changed |= denoiser(app, ui);
    if changed {
        app.unsaved = true;
    }
    if app.render_is_stale() && !app.auto_restart {
        ui.colored_label(style::WARN, tr("Le impostazioni sono cambiate: premi Riavvia per applicarle."));
    }
}

/// Regolazioni della riduzione del rumore (Open Image Denoise). Nessuna di
/// queste riavvia il render: la sessione ripulisce di nuovo l'immagine corrente.
fn denoiser(app: &mut ForgeApp, ui: &mut egui::Ui) -> bool {
    use forge_render_core::{DenoiseMode, DenoiseQuality};
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.strong(tr("Riduzione del rumore (denoiser)")).help("render.denoiser");
        help_button(ui, "render.denoiser");
    });
    let info = app.denoiser_info.clone();
    match &info {
        Ok(v) => {
            ui.label(egui::RichText::new(v).weak().small());
        }
        Err(e) => {
            ui.colored_label(
                style::WARN,
                tr("Open Image Denoise non trovato: il render resta senza riduzione del rumore."),
            )
            .on_hover_text(e);
            return false;
        }
    }
    let status = app.session.progress().denoiser;
    let d = &mut app.settings.denoise;
    egui::ComboBox::from_label(tr("Modalità"))
        .selected_text(tr(d.mode.label()))
        .show_ui(ui, |ui| {
            for m in [DenoiseMode::Final, DenoiseMode::Progressive, DenoiseMode::Off] {
                changed |= ui.selectable_value(&mut d.mode, m, tr(m.label())).changed();
            }
        })
        .response
        .on_hover_text(tr(
            "Solo immagine finale: ripulisce a fine render, in pausa o quando lo fermi.\n\
             Anteprima veloce: ripulisce anche durante il render a 8, 16, 32… campioni, così vedi subito il risultato.",
        ));
    if d.mode == DenoiseMode::Off {
        return changed;
    }
    changed |= ui
        .checkbox(&mut d.use_cuda, tr("Usa CUDA (GPU NVIDIA)"))
        .on_hover_text(tr(
            "Esegue il denoiser sulla GPU NVIDIA con i core tensor: molto più veloce, ideale per l'anteprima.\n\
             Richiede una GPU Volta o successiva (RTX 20/30/40/50, Quadro RTX) e il driver NVIDIA; nessun CUDA Toolkit da installare.\n\
             Se non è disponibile si usa automaticamente la CPU.",
        ))
        .changed();
    if d.use_cuda && status.contains("· CPU") {
        ui.colored_label(style::WARN, tr("CUDA non disponibile su questa GPU: il denoiser usa la CPU."));
    }
    egui::Grid::new("dn-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        let quality = |ui: &mut egui::Ui, id: &str, q: &mut DenoiseQuality| {
            let mut c = false;
            egui::ComboBox::from_id_salt(id).selected_text(tr(q.label())).show_ui(ui, |ui| {
                for v in [DenoiseQuality::High, DenoiseQuality::Balanced, DenoiseQuality::Fast] {
                    c |= ui.selectable_value(q, v, tr(v.label())).changed();
                }
            });
            c
        };
        ui.label(tr("Qualità finale"));
        changed |= quality(ui, "dn-q-final", &mut d.quality);
        ui.end_row();
        if d.mode == DenoiseMode::Progressive {
            ui.label(tr("Qualità anteprima"));
            changed |= quality(ui, "dn-q-preview", &mut d.preview_quality);
            ui.end_row();
            ui.label(tr("Prima anteprima a"));
            changed |= ui
                .add(
                    egui::DragValue::new(&mut d.first_preview_samples)
                        .range(1..=1024)
                        .suffix(tr(" campioni")),
                )
                .on_hover_text(tr("Poi a ogni raddoppio dei campioni (es. 8, 16, 32, 64…)"))
                .changed();
            ui.end_row();
        }
        ui.label(tr("Intensità"));
        let mut pct = d.strength * 100.0;
        if ui
            .add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %"))
            .on_hover_text(tr(
                "Miscela tra l'immagine grezza (0 %) e quella ripulita (100 %): utile se il filtro appiattisce dettagli fini",
            ))
            .changed()
        {
            d.strength = pct / 100.0;
            changed = true;
        }
        ui.end_row();
    });
    changed |= ui
        .checkbox(&mut d.use_guides, tr("Preserva i dettagli (albedo e normali)"))
        .on_hover_text(tr(
            "Guida il filtro con colore e orientamento delle superfici: spigoli, testi e texture restano nitidi",
        ))
        .changed();
    ui.horizontal(|ui| {
        ui.label(tr("Vista:"));
        for v in [RenderView::Final, RenderView::Raw, RenderView::Albedo, RenderView::Normals] {
            ui.selectable_value(&mut app.render_view, v, tr(v.label()));
        }
    });
    if !status.is_empty() {
        ui.label(egui::RichText::new(status).weak().small());
    }
    changed
}

/// Elenco delle camere di rendering salvate nel progetto.
fn render_cameras(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.strong(tr("Camere di rendering"));
    ui.label(
        egui::RichText::new(tr(
            "Il render usa la camera attiva: nella viewport ti muovi liberamente senza perdere l'inquadratura.",
        ))
        .weak()
        .small(),
    );
    let active = app.scene.active_camera;
    if ui
        .radio(active.is_none(), tr("Nessuna: il render usa la vista della viewport"))
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
                .on_hover_text(tr("Usa questa camera per il render"))
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
                .small_button(tr("Guarda"))
                .on_hover_text(tr("Guarda attraverso: porta la vista su questa inquadratura"))
                .clicked()
            {
                action = Some((i, "guarda"));
            }
            if ui
                .small_button(tr("Da vista"))
                .on_hover_text(tr(
                    "Imposta dalla vista: questa camera prende l'inquadratura attuale della viewport",
                ))
                .clicked()
            {
                action = Some((i, "imposta"));
            }
            if ui.small_button(tr("Elimina")).on_hover_text(tr("Elimina la camera")).clicked() {
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
            .button(tr("+ Nuova camera dalla vista"))
            .on_hover_text(tr("Crea una camera di rendering con l'inquadratura attuale"))
            .clicked()
        {
            app.add_render_camera(None);
        }
        if app.scene.active_camera.is_some() {
            ui.checkbox(&mut app.camera_follow, tr("Pilota")).on_hover_text(tr(
                "La camera attiva segue i movimenti della viewport (per regolare l'inquadratura muovendoti nella scena)",
            ));
        }
    });
    if app.scene.active_camera.is_some() && !app.viewing_through_active() && !app.camera_follow {
        ui.label(
            egui::RichText::new(tr(
                "Vista libera: il render resta sull'inquadratura della camera attiva («Guarda» per tornarci).",
            ))
            .weak()
            .small(),
        );
    }
}

/// Tendina per scegliere la camera usata dal render (vista o camere di rendering).
pub(crate) fn camera_picker(app: &mut ForgeApp, ui: &mut egui::Ui, id: &str) {
    let current = match app.scene.active_camera {
        Some(i) => app.scene.render_cameras[i].name.clone(),
        None => tr("Vista della viewport").to_string(),
    };
    let mut choice: Option<Option<usize>> = None;
    egui::ComboBox::from_id_salt(id)
        .selected_text(tf("Camera: {current}", std::slice::from_ref(&current)))
        .width(200.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(app.scene.active_camera.is_none(), tr("Vista della viewport"))
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
        .on_hover_text(tr("Camera usata dal render (le camere si creano nella scheda Camera)"));
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
        tf("{:.0} cm", &[format!("{:.0}", m * 100.0)])
    } else {
        tf("{m:.2} m", &[format!("{:.2}", m)])
    }
}

/// Obiettivo: focale, profondità di campo (diaframma, fuoco, lamelle) e decentramento.
fn lens_controls(app: &mut ForgeApp, ui: &mut egui::Ui, cam: &mut forge_scene::Camera) -> bool {
    let mut changed = false;
    egui::Grid::new("lens-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Proiezione"));
        ui.horizontal(|ui| {
            changed |= ui
                .selectable_value(&mut cam.lens.ortho, false, tr("Prospettica"))
                .on_hover_text(tr("Come una fotocamera: gli oggetti lontani sono più piccoli"))
                .changed();
            changed |= ui
                .selectable_value(&mut cam.lens.ortho, true, tr("Ortografica"))
                .on_hover_text(tr(
                    "Raggi paralleli: misure costanti con la distanza (vista tecnica, isometrica). Niente profondità di campo",
                ))
                .changed();
        });
        ui.end_row();
        if cam.lens.ortho {
            ui.label(tr("Altezza inquadratura"));
            ui.horizontal(|ui| {
                let mut auto = cam.lens.ortho_height <= 0.0;
                if ui
                    .checkbox(&mut auto, "automatica")
                    .on_hover_text(tr("Quella che la prospettiva avrebbe alla distanza del bersaglio (segue lo zoom)"))
                    .changed()
                {
                    cam.lens.ortho_height = if auto { 0.0 } else { 2.0 * cam.ortho_half_height() };
                    changed = true;
                }
                if !auto {
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut cam.lens.ortho_height)
                                .range(0.001..=10000.0)
                                .speed(0.01)
                                .suffix(tr(" m")),
                        )
                        .changed();
                }
            });
            ui.end_row();
        }
        ui.label(tr("Obiettivo"));
        ui.horizontal(|ui| {
            let mut mm = cam.focal_mm();
            if ui
                .add(
                    egui::DragValue::new(&mut mm)
                        .range(4.0..=2000.0)
                        .speed(0.5)
                        .suffix(tr(" mm"))
                        .max_decimals(0),
                )
                .on_hover_text(tr("Focale equivalente full frame (sensore 36×24 mm)"))
                .changed()
            {
                cam.set_focal_mm(mm);
                changed = true;
            }
            ui.label(
                egui::RichText::new(tf("{:.1}° vert.", &[format!("{:.1}", cam.vfov_deg)]))
                    .weak()
                    .small(),
            );
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
        ui.label(tr("Profondità di campo"));
        changed |= ui
            .checkbox(&mut cam.lens.dof, "attiva")
            .on_hover_text(tr(
                "Sfocato ottico reale (lente sottile): nitido sul piano di fuoco, sfocato davanti e dietro",
            ))
            .changed();
        ui.end_row();
        if cam.lens.dof {
            ui.label(tr("Diaframma"));
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
                    .on_hover_text(tr("Numero f: più basso = più sfocato (zona nitida stretta), più alto = più nitido"));
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
            ui.label(tr("Messa a fuoco"));
            ui.horizontal(|ui| {
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut cam.lens.focus_distance)
                            .range(0.01..=10000.0)
                            .speed(0.01)
                            .suffix(tr(" m"))
                            .max_decimals(3),
                    )
                    .changed();
                if ui
                    .selectable_label(app.focus_pick, tr("Scegli col clic"))
                    .on_hover_text(tr("Clicca nella viewport il punto da mettere a fuoco"))
                    .clicked()
                {
                    app.focus_pick = !app.focus_pick;
                    app.view = crate::app::View::Viewport;
                }
                if ui
                    .add_enabled(!app.selection.is_empty(), egui::Button::new(tr("Sulla selezione")))
                    .on_hover_text(tr("Mette a fuoco il centro dei componenti selezionati"))
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
            ui.label(tr("Zona nitida"));
            ui.label(
                egui::RichText::new(tf(
                    "da {} a {}{}",
                    &[
                        fmt_m(near).to_string(),
                        fmt_m(far).to_string(),
                        (if far.is_finite() {
                            tf("  (profondità {})", &[fmt_m(far - near).to_string()])
                        } else {
                            String::new()
                        })
                        .to_string(),
                    ],
                ))
                .strong(),
            )
            .on_hover_text(tr("Calcolata come su una fotocamera full frame (circolo di confusione 0,03 mm)"));
            ui.end_row();
            ui.label(tr("Lamelle"));
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
                            changed |= ui
                                .selectable_value(&mut cam.lens.blades, n, tf("{n} lamelle", &[format!("{}", n)]))
                                .changed();
                        }
                    })
                    .response
                    .on_hover_text(tr("Forma del bokeh: le luci sfocate diventano poligoni come negli obiettivi veri"));
                if cam.lens.blades >= 3 {
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut cam.lens.blade_rotation_deg)
                                .range(-180.0..=180.0)
                                .suffix("°"),
                        )
                        .on_hover_text(tr("Rotazione del diaframma"))
                        .changed();
                }
            });
            ui.end_row();
        }
        ui.label(tr("Decentramento"));
        ui.horizontal(|ui| {
            let mut x = cam.lens.shift_x * 100.0;
            let mut y = cam.lens.shift_y * 100.0;
            let rx = ui.add(
                egui::DragValue::new(&mut x)
                    .range(-100.0..=100.0)
                    .speed(0.2)
                    .prefix(tr("orizz. "))
                    .suffix(" %"),
            );
            let ry = ui.add(
                egui::DragValue::new(&mut y)
                    .range(-100.0..=100.0)
                    .speed(0.2)
                    .prefix(tr("vert. "))
                    .suffix(" %"),
            );
            if rx.changed() || ry.changed() {
                cam.lens.shift_x = x / 100.0;
                cam.lens.shift_y = y / 100.0;
                changed = true;
            }
            ry.on_hover_text(tr(
                "Shift (come un obiettivo decentrabile): sposta l'inquadratura senza inclinare la camera, \
                 così le verticali restano dritte (prodotti alti, architettura)",
            ));
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
