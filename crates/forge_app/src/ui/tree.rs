use crate::app::{ForgeApp, InspectorTab};
use crate::i18n::{tf, tr};
use eframe::egui;
use std::collections::BTreeSet;

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.heading(tr("Scena"));
    ui.label(
        egui::RichText::new(tf(
            "{} componenti · {} triangoli",
            &[
                format!("{}", app.scene.nodes.len()),
                fmt_count(app.scene.triangle_count()).to_string(),
            ],
        ))
        .weak()
        .small(),
    );
    ui.separator();

    // Righe visibili (rispettando i nodi chiusi).
    let mut rows = Vec::new();
    let mut skip_depth: Option<usize> = None;
    for (idx, depth) in app.scene.walk() {
        if let Some(d) = skip_depth {
            if depth > d {
                continue;
            }
            skip_depth = None;
        }
        // Eliminati dal progetto: fuori dall'albero (con i figli), elencati in fondo.
        if app.scene.nodes[idx].removed {
            skip_depth = Some(depth);
            continue;
        }
        rows.push((idx, depth));
        if app.collapsed.contains(&idx) {
            skip_depth = Some(depth);
        }
    }

    let row_h = ui.spacing().interact_size.y;
    let avail = ui.available_height();
    let lights_h = (app.scene.lights.len() as f32 + 2.5) * row_h;
    let tree_h = (avail - lights_h - 10.0).max(row_h * 4.0);

    let mut toggle_vis: Option<usize> = None;
    let mut click: Option<(usize, bool)> = None;
    let mut toggle_collapse: Option<usize> = None;
    let mut frame_node: Option<usize> = None;
    let mut show_all = false;
    let mut duplicate: Option<usize> = None;
    let mut delete: Option<usize> = None;

    egui::ScrollArea::vertical()
        .id_salt("tree-scroll")
        .max_height(tree_h)
        .auto_shrink([false, false])
        .show_rows(ui, row_h, rows.len(), |ui, range| {
            for &(idx, depth) in &rows[range] {
                let node = &app.scene.nodes[idx];
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 14.0);
                    if !node.children.is_empty() {
                        let icon = if app.collapsed.contains(&idx) { "+" } else { "−" };
                        if ui.small_button(icon).on_hover_text(tr("Espandi/comprimi")).clicked() {
                            toggle_collapse = Some(idx);
                        }
                    } else {
                        ui.add_space(18.0);
                    }
                    let mut vis = node.visible;
                    if ui.checkbox(&mut vis, "").on_hover_text(tr("Mostra/nascondi")).changed() {
                        toggle_vis = Some(idx);
                    }
                    let selected = app.selection.contains(&idx);
                    let mut text = egui::RichText::new(&node.name);
                    if !app.scene.is_visible(idx) {
                        text = text.weak();
                    }
                    let r = ui.selectable_label(selected, text);
                    let mat = app.scene.resolve_material(idx).clone();
                    let own = app.scene.assignments.contains_key(&node.id);
                    let r = r.on_hover_text(tf(
                        "id: {}\nmateriale: {}{}",
                        &[
                            format!("{}", node.id),
                            app.scene.materials.get(&mat).map(|m| m.name.as_str()).unwrap_or("?").to_string(),
                            (if own { tr(" (assegnato)") } else { tr(" (ereditato)") }).to_string(),
                        ],
                    ));
                    if r.clicked() {
                        click = Some((idx, ui.input(|i| i.modifiers.command || i.modifiers.shift)));
                    }
                    if r.double_clicked() {
                        frame_node = Some(idx);
                    }
                    r.context_menu(|ui| {
                        if ui.button(tr("Inquadra")).clicked() {
                            frame_node = Some(idx);
                            ui.close();
                        }
                        if ui.button(if node.visible { tr("Nascondi") } else { tr("Mostra") }).clicked() {
                            toggle_vis = Some(idx);
                            ui.close();
                        }
                        if ui.button(tr("Mostra tutto")).clicked() {
                            show_all = true;
                            ui.close();
                        }
                        if ui.button(tr("Duplica (Ctrl+D)")).clicked() {
                            duplicate = Some(idx);
                            ui.close();
                        }
                        if ui
                            .button(tr("Elimina dal progetto (Canc)"))
                            .on_hover_text(tr(
                                "Il file importato non viene modificato; si ripristina con Ctrl+Z o da \"Eliminati\"",
                            ))
                            .clicked()
                        {
                            delete = Some(idx);
                            ui.close();
                        }
                    });
                });
            }
        });

    if let Some(i) = toggle_collapse {
        if !app.collapsed.remove(&i) {
            app.collapsed.insert(i);
        }
    }
    if let Some(i) = toggle_vis {
        app.push_undo(&[i]);
        app.scene.nodes[i].visible = !app.scene.nodes[i].visible;
        app.geometry_changed();
    }
    if let Some(i) = delete {
        app.selection = BTreeSet::from([i]);
        app.delete_selected_copies();
    }
    if let Some(i) = duplicate {
        app.selection.clear();
        app.selection.insert(i);
        app.duplicate_selection();
    }
    if show_all {
        app.show_all();
    }
    if let Some((i, multi)) = click {
        select(&mut app.selection, i, multi);
        app.show_selection_tab();
        if let Some(&s) = app.selection.iter().next() {
            app.selected_material = app.scene.resolve_material(s).clone();
        }
    }
    if let Some(i) = frame_node {
        app.selection = BTreeSet::from([i]);
        app.frame_selection();
    }

    let removed = app.removed_nodes();
    if !removed.is_empty() {
        ui.separator();
        let mut restore: Vec<usize> = Vec::new();
        egui::CollapsingHeader::new(tf("Eliminati dal progetto ({})", &[format!("{}", removed.len())]))
            .id_salt("tree-removed")
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(tr(
                        "Componenti dei file importati tolti dal progetto: il file sorgente non è cambiato.",
                    ))
                    .weak()
                    .small(),
                );
                if ui.small_button(tr("Ripristina tutti")).clicked() {
                    restore = removed.clone();
                }
                egui::ScrollArea::vertical()
                    .id_salt("tree-removed-list")
                    .max_height(120.0)
                    .show(ui, |ui| {
                        for &i in &removed {
                            ui.horizontal(|ui| {
                                if ui.small_button(tr("Ripristina")).clicked() {
                                    restore = vec![i];
                                }
                                ui.label(&app.scene.nodes[i].name).on_hover_text(app.scene.nodes[i].id.to_string());
                            });
                        }
                    });
            });
        app.restore_removed(&restore);
    }

    ui.separator();
    ui.horizontal(|ui| {
        ui.strong(tr("Luci"));
        if ui.small_button("+").on_hover_text(tr("Aggiungi una luce ad area")).clicked() {
            let (c, r) = app.anchor();
            app.scene.lights.push(forge_scene::Light::area(
                tr("Nuova luce"),
                c + forge_math::Vec3::new(1.0, 2.0, 1.0) * r,
                c,
                r,
                r,
                5.0,
            ));
            app.selected_light = Some(app.scene.lights.len() - 1);
            app.tab = InspectorTab::Lights;
            app.shading_changed();
        }
    });
    let mut light_click = None;
    let mut light_toggle = None;
    for (i, l) in app.scene.lights.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut on = l.enabled;
            if ui.checkbox(&mut on, "").on_hover_text(tr("Accendi/spegni")).changed() {
                light_toggle = Some(i);
            }
            if ui
                .selectable_label(app.selected_light == Some(i), format!("{} ({})", l.name, tr(l.kind.label())))
                .clicked()
            {
                light_click = Some(i);
            }
        });
    }
    if let Some(i) = light_toggle {
        app.scene.lights[i].enabled = !app.scene.lights[i].enabled;
        app.shading_changed();
    }
    if let Some(i) = light_click {
        app.selected_light = Some(i);
        app.tab = InspectorTab::Lights;
    }
}

pub fn select(sel: &mut BTreeSet<usize>, i: usize, multi: bool) {
    if multi {
        if !sel.remove(&i) {
            sel.insert(i);
        }
    } else {
        *sel = BTreeSet::from([i]);
    }
}

pub fn fmt_count(n: usize) -> String {
    if n >= 1_000_000 {
        tf("{:.2} M", &[format!("{:.2}", n as f64 / 1e6)])
    } else if n >= 10_000 {
        tf("{:.1} k", &[format!("{:.1}", n as f64 / 1e3)])
    } else {
        n.to_string()
    }
}
