use crate::app::{ForgeApp, InspectorTab};
use crate::i18n::{tf, tr};
use eframe::egui;
use std::collections::BTreeSet;

/// Altezza massima dell'elenco di un gruppo sotto gli oggetti (luci, camere, effetti): oltre
/// scorre, e agli oggetti resta il resto.
const GROUP_MAX: f32 = 180.0;

/// Pannello di sinistra: un solo albero della scena a gruppi. Gli oggetti prendono lo spazio che
/// resta; sotto, i gruppi Luci, Camere ed Effetti si aprono e si chiudono, ognuno col suo
/// scorrimento. Un clic su qualunque riga mostra le sue proprietà nella scheda Selezione.
pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui) {
    crate::ui::style::scene_selection(ui);
    egui::Panel::bottom("tree-groups").resizable(false).show(ui, |ui| {
        ui.add_space(2.0);
        lights(app, ui);
        cameras(app, ui);
        effects(app, ui);
        ui.add_space(2.0);
    });
    egui::CentralPanel::default().show(ui, |ui| scene_tree(app, ui));
}

/// Testata di un gruppo: freccia, nome e numero di elementi, a destra il "+" per aggiungerne uno.
/// Restituisce se è aperto e se è stato premuto il "+".
fn group_header(ui: &mut egui::Ui, id: &str, label: &str, count: usize, default_open: bool, add_hint: Option<&str>) -> (bool, bool) {
    let id = ui.make_persistent_id(id);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
    let mut add = false;
    ui.horizontal(|ui| {
        state.show_toggle_button(ui, egui::collapsing_header::paint_default_icon);
        let r = ui.add(egui::Label::new(egui::RichText::new(format!("{label} ({count})")).strong()).sense(egui::Sense::click()));
        if r.clicked() {
            state.toggle(ui);
        }
        if let Some(hint) = add_hint {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                add = ui.small_button("+").on_hover_text(hint).clicked();
            });
        }
    });
    let open = state.is_open();
    state.store(ui.ctx());
    (open, add)
}

/// Mostra le proprietà di ciò che si è appena scelto nell'albero: toglie le altre selezioni.
fn select_only(app: &mut ForgeApp) {
    app.deselect_all();
    app.tab = InspectorTab::Object;
}

fn scene_tree(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading(tr("Scena"));
        ui.label(
            egui::RichText::new(tf(
                "{} componenti · {} triangoli",
                &[
                    format!("{}", app.scene.nodes.len()),
                    fmt_count(app.scene.triangle_count()).to_string(),
                ],
            ))
            .weak(),
        );
    });
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

    // Eliminati dal progetto: sopra l'albero, così l'albero prende tutto lo spazio che resta.
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

    let row_h = ui.spacing().interact_size.y;
    let tree_h = ui.available_height().max(row_h * 3.0);

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
}

/// Camere di rendering come componenti: selezionandone una il gizmo la sposta e la fa orbitare
/// attorno al bersaglio; il pallino dice quale usa il render.
fn cameras(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let (open, add) = group_header(
        ui,
        "albero-camere",
        tr("Camere"),
        app.scene.render_cameras.len(),
        false,
        Some(tr("Aggiungi una camera di rendering dalla vista attuale")),
    );
    if add {
        app.add_render_camera(None);
        select_only(app);
        app.selected_camera = Some(app.scene.render_cameras.len() - 1);
    }
    if !open {
        return;
    }
    let mut click = None;
    let mut activate = None;
    egui::ScrollArea::vertical()
        .id_salt("tree-cameras-list")
        .max_height(GROUP_MAX)
        .show(ui, |ui| {
            if app.scene.render_cameras.is_empty() {
                ui.label(egui::RichText::new(tr("Nessuna: il render usa la vista")).weak());
            }
            for (i, c) in app.scene.render_cameras.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    if ui
                        .radio(app.scene.active_camera == Some(i), "")
                        .on_hover_text(tr("Usa questa camera per il render"))
                        .clicked()
                    {
                        activate = Some(i);
                    }
                    let label = if c.aim.is_some() {
                        format!("{} {}", c.name, crate::ui::icons::FRAME)
                    } else {
                        c.name.clone()
                    };
                    if ui
                        .selectable_label(app.selected_camera == Some(i), label)
                        .on_hover_text(tr("Seleziona: il gizmo sposta la camera, la fa girare attorno al bersaglio \
                             (Ruota) o sposta il bersaglio (clic sul rombo nella viewport)"))
                        .clicked()
                    {
                        click = Some(i);
                    }
                });
            }
        });
    if let Some(i) = activate {
        app.set_active_camera(Some(i));
    }
    if let Some(i) = click {
        select_only(app);
        app.selected_camera = Some(i);
    }
}

/// Luci della scena (scorrono se non ci stanno): casella per accenderle, clic per sceglierle.
fn lights(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let (open, add) = group_header(
        ui,
        "albero-luci",
        tr("Luci"),
        app.scene.lights.len(),
        true,
        Some(tr("Aggiungi una luce ad area")),
    );
    if add {
        let (c, r) = app.anchor();
        app.scene.lights.push(forge_scene::Light::area(
            tr("Nuova luce"),
            c + forge_math::Vec3::new(1.0, 2.0, 1.0) * r,
            c,
            r,
            r,
            5.0,
        ));
        select_only(app);
        app.selected_light = Some(app.scene.lights.len() - 1);
        app.shading_changed();
    }
    if !open {
        return;
    }
    let mut light_click = None;
    let mut light_toggle = None;
    egui::ScrollArea::vertical()
        .id_salt("tree-lights-list")
        .max_height(GROUP_MAX)
        .show(ui, |ui| {
            for (i, l) in app.scene.lights.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    let mut on = l.enabled;
                    if ui.checkbox(&mut on, "").on_hover_text(tr("Accendi/spegni")).changed() {
                        light_toggle = Some(i);
                    }
                    if ui
                        .selectable_label(app.selected_light == Some(i), &l.name)
                        .on_hover_text(tr(l.kind.label()))
                        .clicked()
                    {
                        light_click = Some(i);
                    }
                });
            }
        });
    if let Some(i) = light_toggle {
        app.scene.lights[i].enabled = !app.scene.lights[i].enabled;
        app.shading_changed();
    }
    if let Some(i) = light_click {
        select_only(app);
        app.selected_light = Some(i);
    }
}

/// Un effetto dell'elenco: quelli col riquadro nella viewport si selezionano, gli altri aprono la
/// scheda Effetti della scena.
#[derive(Clone, Copy, PartialEq)]
enum EffectRow {
    Fog(usize),
    Smoke(usize),
    Dust,
    Haze,
    Fireflies,
    Decal(usize),
}

/// Effetti presenti nella scena: nebbie, fumi, polvere, foschia, lucciole e adesivi.
fn effects(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let s = &app.scene;
    let mut rows: Vec<(EffectRow, String, bool)> = Vec::new();
    rows.extend(
        s.fogs
            .iter()
            .enumerate()
            .map(|(i, f)| (EffectRow::Fog(i), f.name.clone(), f.enabled)),
    );
    rows.extend(
        s.smokes
            .iter()
            .enumerate()
            .map(|(i, f)| (EffectRow::Smoke(i), f.name.clone(), f.enabled)),
    );
    if s.haze_dust.enabled {
        rows.push((EffectRow::Dust, tr("Polvere in sospensione").to_string(), true));
    }
    if s.haze.enabled {
        rows.push((EffectRow::Haze, tr("Foschia").to_string(), true));
    }
    if s.fireflies.enabled {
        rows.push((EffectRow::Fireflies, tr("Lucciole").to_string(), true));
    }
    rows.extend(
        s.decals
            .iter()
            .enumerate()
            .map(|(i, d)| (EffectRow::Decal(i), d.name.clone(), d.enabled)),
    );
    let (open, add) = group_header(
        ui,
        "albero-effetti",
        tr("Effetti"),
        rows.len(),
        false,
        Some(tr("Aggiungi un effetto")),
    );
    if add {
        app.tab = InspectorTab::Effects;
    }
    if !open {
        return;
    }
    let selected = |r: EffectRow| match r {
        EffectRow::Fog(i) => app.selected_fog == Some(i),
        EffectRow::Smoke(i) => app.selected_smoke == Some(i),
        EffectRow::Dust => app.selected_dust,
        _ => false,
    };
    let mut click = None;
    egui::ScrollArea::vertical()
        .id_salt("tree-effects-list")
        .max_height(GROUP_MAX)
        .show(ui, |ui| {
            if rows.is_empty() {
                ui.label(egui::RichText::new(tr("Nessun effetto: aggiungine uno con il +.")).weak());
            }
            for (r, name, on) in &rows {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    let mut text = egui::RichText::new(name);
                    if !on {
                        text = text.weak();
                    }
                    if ui.selectable_label(selected(*r), text).clicked() {
                        click = Some(*r);
                    }
                });
            }
        });
    match click {
        Some(EffectRow::Fog(i)) => {
            select_only(app);
            app.selected_fog = Some(i);
        }
        Some(EffectRow::Smoke(i)) => {
            select_only(app);
            app.selected_smoke = Some(i);
        }
        Some(EffectRow::Dust) => {
            select_only(app);
            app.selected_dust = true;
        }
        Some(_) => app.tab = InspectorTab::Effects,
        None => {}
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
