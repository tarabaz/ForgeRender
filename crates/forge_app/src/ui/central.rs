use crate::app::{ForgeApp, View};
use crate::i18n::{tf, tr, tr_dyn};
use crate::ui::style;
use crate::viewport::project;
use eframe::egui;
use forge_math::Vec3;
use forge_render_core::{Accumulator, RenderBackend, RenderScene, RenderSettings, SessionState};
use forge_scene::{primitives, Camera, ComponentId, Light, LightKind, Material, Scene};
use std::sync::mpsc::Receiver;

pub fn show(app: &mut ForgeApp, ui: &mut egui::Ui, frame: &eframe::Frame) {
    match app.view {
        View::Viewport => viewport(app, ui, frame),
        View::Render => render_view(app, ui),
    }
}

/// Gestione mouse comune: orbita, pan, zoom. Restituisce la risposta per il picking.
fn camera_controls(app: &mut ForgeApp, ui: &mut egui::Ui, rect: egui::Rect) -> egui::Response {
    let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    camera_input(app, ui, rect, &resp);
    resp
}

fn camera_input(app: &mut ForgeApp, ui: &egui::Ui, rect: egui::Rect, resp: &egui::Response) {
    let shift = ui.input(|i| i.modifiers.shift);
    let alt = ui.input(|i| i.modifiers.alt);
    let delta = resp.drag_delta();
    // Alt + trascina: sposta la selezione invece della camera.
    if alt && !app.selection.is_empty() && resp.dragged_by(egui::PointerButton::Primary) && delta != egui::Vec2::ZERO {
        if let Some(pos) = resp.interact_pointer_pos() {
            move_selection(app, rect, pos - delta, pos, shift);
        }
        return;
    }
    let mut moved = false;
    if delta != egui::Vec2::ZERO {
        let pan = resp.dragged_by(egui::PointerButton::Secondary)
            || resp.dragged_by(egui::PointerButton::Middle)
            || (shift && resp.dragged_by(egui::PointerButton::Primary));
        if pan {
            app.release_aim();
            app.orbit.pan(delta.x / rect.height(), delta.y / rect.height());
        } else if resp.dragged_by(egui::PointerButton::Primary) {
            app.orbit.orbit(delta.x * 0.008, delta.y * 0.008);
        }
        moved = true;
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            app.orbit.zoom((-scroll * 0.0015).exp());
            moved = true;
        }
    }
    if moved {
        app.camera_changed();
    }
}

/// Sposta i componenti selezionati seguendo il mouse: sul piano orizzontale
/// passante per il loro centro, oppure in verticale (`vertical`).
fn move_selection(app: &mut ForgeApp, rect: egui::Rect, from: egui::Pos2, to: egui::Pos2, vertical: bool) {
    let mut bounds = forge_math::Aabb::EMPTY;
    for &s in &app.selection {
        bounds.grow(&app.scene.bounds_of(s));
    }
    if bounds.is_empty() {
        return;
    }
    let center = bounds.center();
    let cam = app.scene.camera.clone();
    let aspect = rect.width() / rect.height().max(1.0);
    let ray = |p: egui::Pos2| {
        let nx = (p.x - rect.left()) / rect.width() * 2.0 - 1.0;
        let ny = 1.0 - (p.y - rect.top()) / rect.height() * 2.0;
        cam.ray(nx, ny, aspect)
    };
    // Piano: orizzontale, oppure verticale rivolto verso la camera.
    let normal = if vertical {
        let f = cam.forward();
        Vec3::new(f.x, 0.0, f.z).normalize_or(Vec3::Z)
    } else {
        Vec3::Y
    };
    let hit = |(o, d): (Vec3, Vec3)| {
        let denom = d.dot(normal);
        if denom.abs() < 1e-6 {
            return None;
        }
        let t = (center - o).dot(normal) / denom;
        (t > 0.0).then(|| o + d * t)
    };
    let (Some(a), Some(b)) = (hit(ray(from)), hit(ray(to))) else {
        return;
    };
    let mut d = b - a;
    if vertical {
        d = Vec3::new(0.0, d.y, 0.0);
    } else {
        d.y = 0.0;
    }
    // Se sono selezionati sia un assieme sia un suo figlio, si sposta solo l'assieme.
    let sel: Vec<usize> = app.selection.iter().copied().collect();
    for &i in &sel {
        let mut p = app.scene.nodes[i].parent;
        let mut covered = false;
        while let Some(k) = p {
            if app.selection.contains(&k) {
                covered = true;
                break;
            }
            p = app.scene.nodes[k].parent;
        }
        if !covered {
            app.scene.nodes[i].user.translation += d;
        }
    }
    app.geometry_changed();
}

fn viewport(app: &mut ForgeApp, ui: &mut egui::Ui, frame: &eframe::Frame) {
    let rect = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    crate::ui::navigator::tick(app, ui.ctx());
    let ppp = ui.ctx().pixels_per_point();
    let size = [
        (rect.width() * ppp).round().max(1.0) as u32,
        (rect.height() * ppp).round().max(1.0) as u32,
    ];
    let painter = ui.painter_at(rect);
    match app.viewport_texture(frame, size) {
        Some(tex) => {
            painter.image(
                tex,
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        None => {
            painter.rect_filled(rect, 0.0, egui::Color32::from_gray(20));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("Viewport 3D GPU non disponibile (modalità sicura).\nUsa la vista Render: il rendering CPU funziona normalmente."),
                egui::FontId::proportional(15.0),
                style::WARN,
            );
        }
    }
    draw_lights(app, &painter, rect);
    draw_fogs(app, &painter, rect);
    draw_cameras(app, &painter, rect);
    draw_depth_of_field(app, &painter, rect);
    if app.aim_pick {
        painter.text(
            rect.center_top() + egui::vec2(0.0, 52.0),
            egui::Align2::CENTER_TOP,
            tr("Clic sull'oggetto da inquadrare (Esc annulla)"),
            egui::FontId::proportional(14.0),
            style::ACCENT,
        );
    }
    if app.focus_pick {
        painter.text(
            rect.center_top() + egui::vec2(0.0, 52.0),
            egui::Align2::CENTER_TOP,
            tr("Clic sul punto da mettere a fuoco (Esc annulla)"),
            egui::FontId::proportional(14.0),
            style::ACCENT,
        );
    }
    // Il gizmo ha la precedenza: se usa il mouse la camera non si muove e il clic non seleziona.
    // Maniglie del cono dello spot (precedenza sul gizmo quando le si usa).
    let cone_used = crate::ui::gizmo::cone_handles(app, ui, rect, &resp, &painter);
    let gizmo_used = cone_used || crate::ui::gizmo::update(app, ui, rect, &resp, &painter);
    let nav_used = crate::ui::navigator::show(app, ui, rect, &painter);
    let gizmo_used = gizmo_used || nav_used;
    if !gizmo_used {
        camera_input(app, ui, rect, &resp);
    }
    crate::ui::gizmo::toolbar(app, ui, rect);

    app.follow_aim();
    // Punta su oggetto: il clic sceglie l'oggetto a cui agganciare la camera.
    if app.aim_pick {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.aim_pick = false;
        }
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let ndc_x = (pos.x - rect.left()) / rect.width() * 2.0 - 1.0;
                let ndc_y = 1.0 - (pos.y - rect.top()) / rect.height() * 2.0;
                let view = app.scene.camera.clone();
                let (o, dir) = view.ray(ndc_x, ndc_y, rect.width() / rect.height());
                let geo = app.geometry_now();
                if let Some((node, _)) = forge_render_core::pick(&geo, o, dir) {
                    app.aim_at(node);
                }
            }
            app.aim_pick = false;
            return;
        }
    }
    // Messa a fuoco col clic: distanza lungo l'asse della camera che si sta regolando.
    if app.focus_pick {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.focus_pick = false;
        }
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let ndc_x = (pos.x - rect.left()) / rect.width() * 2.0 - 1.0;
                let ndc_y = 1.0 - (pos.y - rect.top()) / rect.height() * 2.0;
                let view = app.scene.camera.clone();
                let (o, dir) = view.ray(ndc_x, ndc_y, rect.width() / rect.height());
                let geo = app.geometry_now();
                if let Some((_, t)) = forge_render_core::pick(&geo, o, dir) {
                    let hit = o + dir * t;
                    app.set_focus_point(hit);
                }
            }
            app.focus_pick = false;
            return;
        }
    }
    // Picking con un clic (senza trascinamento).
    if resp.clicked() && !gizmo_used {
        if let Some(pos) = resp.interact_pointer_pos() {
            let ndc_x = (pos.x - rect.left()) / rect.width() * 2.0 - 1.0;
            let ndc_y = 1.0 - (pos.y - rect.top()) / rect.height() * 2.0;
            let cam = app.scene.camera.clone();
            let (origin, dir) = cam.ray(ndc_x, ndc_y, rect.width() / rect.height());
            // Luci ed emettitori di nebbia: si scelgono cliccando vicino all'icona.
            let near = |p: Vec3| project(&cam, rect, p).map(|q| q.distance(pos)).filter(|d| *d < 10.0);
            let mut best: Option<(f32, bool, usize)> = None;
            for (i, l) in app.scene.lights.iter().enumerate() {
                if let Some(d) = near(l.position) {
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, true, i));
                    }
                }
            }
            for (i, f) in app.scene.fogs.iter().enumerate() {
                if let Some(d) = near(f.emitter) {
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, false, i));
                    }
                }
            }
            if let Some((_, is_light, i)) = best {
                app.selection.clear();
                if is_light {
                    app.selected_light = Some(i);
                    app.selected_fog = None;
                    app.tab = crate::app::InspectorTab::Lights;
                } else {
                    app.selected_fog = Some(i);
                    app.selected_light = None;
                    app.tab = crate::app::InspectorTab::Effects;
                }
                return;
            }
            let geo = app.geometry_now();
            let multi = ui.input(|i| i.modifiers.command);
            app.selected_light = None;
            app.selected_fog = None;
            match forge_render_core::pick(&geo, origin, dir) {
                Some((node, _)) => {
                    crate::ui::tree::select(&mut app.selection, node, multi);
                    app.selected_material = app.scene.resolve_material(node).clone();
                    app.show_selection_tab();
                }
                None if !multi => app.selection.clear(),
                None => {}
            }
        }
    }
    if resp.double_clicked() {
        app.frame_selection();
    }
    let hint = format!(
        "{} | {}",
        app.viewport
            .as_ref()
            .map(|v| v.adapter_info.as_str())
            .unwrap_or(tr("viewport disattivata")),
        if app.selection.is_empty() {
            tr("nessuna selezione").to_string()
        } else {
            tf("{} selezionati", &[format!("{}", app.selection.len())])
        }
    );
    painter.text(
        rect.left_bottom() + egui::vec2(8.0, -8.0),
        egui::Align2::LEFT_BOTTOM,
        hint,
        egui::FontId::proportional(12.0),
        egui::Color32::from_gray(150),
    );
}

/// Camere di rendering come piramidi; guardando attraverso la camera attiva, la
/// cornice con le proporzioni esatte dell'immagine renderizzata.
fn draw_cameras(app: &ForgeApp, painter: &egui::Painter, rect: egui::Rect) {
    let cam = &app.scene.camera;
    let aspect = app.settings.aspect();
    if app.viewing_through_active() {
        let h = rect.height();
        let w = h * aspect;
        let frame = egui::Rect::from_center_size(rect.center(), egui::vec2(w, h));
        let shade = egui::Color32::from_black_alpha(110);
        if frame.left() > rect.left() {
            painter.rect_filled(
                egui::Rect::from_min_max(rect.left_top(), egui::pos2(frame.left(), rect.bottom())),
                0.0,
                shade,
            );
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(frame.right(), rect.top()), rect.right_bottom()),
                0.0,
                shade,
            );
        }
        painter.rect_stroke(frame, 0.0, egui::Stroke::new(1.5, style::ACCENT), egui::StrokeKind::Inside);
        // Terzi: aiuto alla composizione.
        let thirds = egui::Stroke::new(0.5, style::ACCENT.gamma_multiply(0.4));
        for k in 1..3 {
            let x = frame.left() + frame.width() * k as f32 / 3.0;
            let y = frame.top() + frame.height() * k as f32 / 3.0;
            painter.line_segment([egui::pos2(x, frame.top()), egui::pos2(x, frame.bottom())], thirds);
            painter.line_segment([egui::pos2(frame.left(), y), egui::pos2(frame.right(), y)], thirds);
        }
        if let Some(i) = app.scene.active_camera {
            painter.text(
                frame.left_top().max(rect.left_top()) + egui::vec2(8.0, 40.0),
                egui::Align2::LEFT_TOP,
                tf(
                    "Camera {} · {}×{}",
                    &[
                        app.scene.render_cameras[i].name.to_string(),
                        format!("{}", app.settings.width),
                        format!("{}", app.settings.height),
                    ],
                ),
                egui::FontId::proportional(12.0),
                style::ACCENT,
            );
        }
    }
    let size = (app.scene.bounds().radius() * 0.12).max(0.05);
    for (i, rc) in app.scene.render_cameras.iter().enumerate() {
        if app.viewing_through_active() && app.scene.active_camera == Some(i) {
            continue;
        }
        let c = &rc.camera;
        let (r, u, f) = c.basis();
        let th = (c.vfov_deg.to_radians() * 0.5).tan() * size;
        let tw = th * aspect;
        let base = c.position + f * size;
        let corners = [
            base - r * tw - u * th,
            base + r * tw - u * th,
            base + r * tw + u * th,
            base - r * tw + u * th,
        ];
        let color = if app.scene.active_camera == Some(i) {
            style::ACCENT
        } else {
            egui::Color32::from_gray(170)
        };
        let stroke = egui::Stroke::new(if app.scene.active_camera == Some(i) { 2.0 } else { 1.2 }, color);
        let Some(apex) = project(cam, rect, c.position) else { continue };
        let pts: Vec<_> = corners.iter().filter_map(|p| project(cam, rect, *p)).collect();
        if pts.len() == 4 {
            for p in &pts {
                painter.line_segment([apex, *p], stroke);
            }
            painter.add(egui::Shape::closed_line(pts.clone(), stroke));
            // Triangolo sul lato superiore: indica il verso "alto" dell'inquadratura.
            if let Some(tip) = project(cam, rect, base + u * th * 1.5) {
                painter.add(egui::Shape::convex_polygon(
                    vec![pts[3], pts[2], tip],
                    color.gamma_multiply(0.5),
                    stroke,
                ));
            }
        }
        painter.text(
            apex + egui::vec2(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            &rc.name,
            egui::FontId::proportional(11.0),
            color,
        );
    }
}

/// Profondità di campo della camera di rendering: linea dalla camera al punto di fuoco con
/// la distanza, rettangolo del piano di fuoco e i due limiti della zona nitida.
fn draw_depth_of_field(app: &ForgeApp, painter: &egui::Painter, rect: egui::Rect) {
    let Some(i) = app.scene.active_camera else { return };
    let rc = &app.scene.render_cameras[i].camera;
    if !rc.lens.dof {
        return;
    }
    let view = &app.scene.camera;
    let (r, u, f) = rc.basis();
    let aspect = app.settings.aspect();
    let th = (rc.vfov_deg.to_radians() * 0.5).tan();
    // Sezione dell'inquadratura (shift compreso) alla distanza d.
    let section = |d: f32| -> Option<Vec<egui::Pos2>> {
        let (hh, hw) = (th * d, th * d * aspect);
        let c = rc.position + f * d + r * (2.0 * rc.lens.shift_x * hw) + u * (2.0 * rc.lens.shift_y * hh);
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|(a, b)| project(view, rect, c + r * (a * hw) + u * (b * hh)))
            .collect()
    };
    let focus = rc.lens.focus_distance;
    let (near, far) = rc.dof_limits();
    let accent = style::ACCENT;
    let zone = egui::Color32::from_rgb(90, 200, 230);
    let through = app.viewing_through_active();
    if !through {
        // Linea di mira fino al punto di fuoco, con la distanza.
        let fp = rc.position + f * focus;
        if let (Some(a), Some(b)) = (project(view, rect, rc.position), project(view, rect, fp)) {
            painter.line_segment([a, b], egui::Stroke::new(1.5, accent));
            painter.circle_filled(b, 4.0, accent);
            painter.text(
                b + egui::vec2(8.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                tf("Fuoco {:.2} m", &[format!("{:.2}", focus)]),
                egui::FontId::proportional(12.0),
                accent,
            );
        }
        if let Some(p) = section(focus) {
            painter.add(egui::Shape::closed_line(p, egui::Stroke::new(1.5, accent)));
        }
        for (d, label) in [(near, tr("nitido da")), (far, tr("fino a"))] {
            if !d.is_finite() {
                continue;
            }
            if let Some(p) = section(d) {
                painter.add(egui::Shape::dashed_line(
                    &[p[0], p[1], p[2], p[3], p[0]],
                    egui::Stroke::new(1.2, zone),
                    6.0,
                    4.0,
                ));
                // Etichette su angoli opposti: non si sovrappongono quando i limiti sono vicini.
                let (anchor, align, off) = if label == "nitido da" {
                    (p[3], egui::Align2::LEFT_BOTTOM, egui::vec2(4.0, -4.0))
                } else {
                    (p[1], egui::Align2::RIGHT_TOP, egui::vec2(-4.0, 4.0))
                };
                painter.text(
                    anchor + off,
                    align,
                    tf("{label} {:.2} m", &[label.to_string(), format!("{:.2}", d)]),
                    egui::FontId::proportional(11.0),
                    zone,
                );
            }
        }
    }
    // Riepilogo in basso a destra (utile anche guardando attraverso la camera).
    let far_txt = if far.is_finite() {
        tf("{far:.2} m", &[format!("{:.2}", far)])
    } else {
        "∞".into()
    };
    painter.text(
        rect.right_bottom() + egui::vec2(-10.0, -10.0),
        egui::Align2::RIGHT_BOTTOM,
        tf(
            "f/{:.1} · fuoco {:.2} m · nitido {:.2} m – {far_txt}",
            &[
                format!("{:.1}", rc.lens.f_stop),
                format!("{:.2}", focus),
                format!("{:.2}", near),
                far_txt.to_string(),
            ],
        ),
        egui::FontId::proportional(12.0),
        zone,
    );
}

/// Nebbie: riquadro della zona simulata (tratteggiato) ed emettitore con la caduta.
fn draw_fogs(app: &ForgeApp, painter: &egui::Painter, rect: egui::Rect) {
    let cam = &app.scene.camera;
    let color = egui::Color32::from_rgb(140, 210, 255);
    let geo = app.compiled_geometry();
    for f in &app.scene.fogs {
        let c = if f.enabled { color } else { egui::Color32::from_gray(90) };
        let f = &f.sanitized();
        let ground = geo.as_ref().and_then(|g| forge_render_core::fog::ground_under(f, g));
        let (lo, hi) = f.bounds_over(if f.collisions { ground } else { None });
        let corner = |i: usize| Vec3::new([lo.x, hi.x][i & 1], [lo.y, hi.y][(i >> 1) & 1], [lo.z, hi.z][(i >> 2) & 1]);
        for (a, b) in [
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
            (0, 2),
            (1, 3),
            (4, 6),
            (5, 7),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ] {
            if let (Some(p), Some(q)) = (project(cam, rect, corner(a)), project(cam, rect, corner(b))) {
                painter.add(egui::Shape::dashed_line(
                    &[p, q],
                    egui::Stroke::new(1.0, c.gamma_multiply(0.6)),
                    6.0,
                    4.0,
                ));
            }
        }
        let Some(p) = project(cam, rect, f.emitter) else { continue };
        // Bocca della sorgente: cerchio orizzontale del raggio impostato.
        let ring: Vec<_> = (0..=32)
            .filter_map(|k| {
                let a = k as f32 / 32.0 * std::f32::consts::TAU;
                project(cam, rect, f.emitter + Vec3::new(a.cos(), 0.0, a.sin()) * f.source_radius)
            })
            .collect();
        if ring.len() > 2 {
            painter.add(egui::Shape::line(ring, egui::Stroke::new(1.5, c)));
        }
        painter.circle_filled(p, 4.0, c);
        if let Some(q) = project(cam, rect, Vec3::new(f.emitter.x, lo.y, f.emitter.z)) {
            painter.add(egui::Shape::dashed_line(
                &[p, q],
                egui::Stroke::new(1.0, c.gamma_multiply(0.5)),
                4.0,
                4.0,
            ));
        }
        painter.text(
            p + egui::vec2(8.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            &f.name,
            egui::FontId::proportional(11.0),
            c,
        );
    }
}

fn draw_lights(app: &ForgeApp, painter: &egui::Painter, rect: egui::Rect) {
    let cam = &app.scene.camera;
    for (i, l) in app.scene.lights.iter().enumerate() {
        let selected = app.selected_light == Some(i);
        let color = if !l.enabled {
            egui::Color32::from_gray(90)
        } else if selected {
            style::ACCENT
        } else {
            egui::Color32::from_rgb(240, 220, 120)
        };
        let stroke = egui::Stroke::new(if selected { 2.0 } else { 1.2 }, color);
        let Some(p) = project(cam, rect, l.position) else { continue };
        if l.has_range() && l.range > 0.0 {
            draw_range_bubble(cam, painter, rect, l, color, selected);
        }
        match l.kind {
            LightKind::Area { width, height } | LightKind::Portal { width, height } => {
                // Portale: azzurro tratteggiato, la freccia indica l'interno della stanza.
                let portal = matches!(l.kind, LightKind::Portal { .. });
                let stroke = if portal && l.enabled && !selected {
                    egui::Stroke::new(1.2, egui::Color32::from_rgb(120, 200, 255))
                } else {
                    stroke
                };
                let n = l.direction();
                let hint = if n.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
                let u = hint.cross(n).normalize() * width * 0.5;
                let v = n.cross(u.normalize()) * height * 0.5;
                let corners = [l.position - u - v, l.position + u - v, l.position + u + v, l.position - u + v];
                let pts: Vec<_> = corners.iter().filter_map(|c| project(cam, rect, *c)).collect();
                if pts.len() == 4 {
                    if portal {
                        let mut ring = pts.clone();
                        ring.push(pts[0]);
                        painter.add(egui::Shape::dashed_line(&ring, stroke, 6.0, 4.0));
                        painter.line_segment([pts[0], pts[2]], egui::Stroke::new(0.8, stroke.color.gamma_multiply(0.6)));
                        painter.line_segment([pts[1], pts[3]], egui::Stroke::new(0.8, stroke.color.gamma_multiply(0.6)));
                    } else {
                        painter.add(egui::Shape::closed_line(pts, stroke));
                    }
                }
                let scale = (width + height) * 0.25;
                if let Some(q) = project(cam, rect, l.position + n * scale) {
                    painter.arrow(p, q - p, stroke);
                }
            }
            LightKind::Point => {
                painter.circle_stroke(p, 6.0, stroke);
            }
            LightKind::Spot {
                inner_angle_deg,
                outer_angle_deg,
            } => {
                painter.circle_stroke(p, 5.0, stroke);
                if let Some(q) = project(cam, rect, l.target) {
                    painter.line_segment([p, q], egui::Stroke::new(1.0, color.gamma_multiply(0.5)));
                }
                // Cono: bordo esterno pieno, inizio della sfumatura tratteggiato, alla distanza del bersaglio.
                let c = crate::ui::gizmo::SpotCone::of(l);
                let outer: Vec<_> = c.circle(outer_angle_deg).iter().filter_map(|v| project(cam, rect, *v)).collect();
                let inner: Vec<_> = c.circle(inner_angle_deg).iter().filter_map(|v| project(cam, rect, *v)).collect();
                if outer.len() > 2 {
                    for k in [0, 16, 32, 48] {
                        if let Some(e) = outer.get(k) {
                            painter.line_segment([p, *e], egui::Stroke::new(1.0, color.gamma_multiply(0.7)));
                        }
                    }
                    painter.add(egui::Shape::line(outer, stroke));
                }
                if inner.len() > 2 {
                    painter.add(egui::Shape::dashed_line(
                        &inner,
                        egui::Stroke::new(1.0, color.gamma_multiply(0.8)),
                        5.0,
                        4.0,
                    ));
                }
            }
            LightKind::Directional => {
                painter.circle_stroke(p, 5.0, stroke);
                if let Some(q) = project(cam, rect, l.target) {
                    painter.line_segment([p, q], egui::Stroke::new(1.0, color.gamma_multiply(0.5)));
                }
            }
        }
        painter.text(
            p + egui::vec2(8.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            &l.name,
            egui::FontId::proportional(11.0),
            color,
        );
    }
}

/// Bolla del raggio d'azione di una luce Point/Spot: contorno pieno dove la luce finisce,
/// tratteggiato dove comincia la sfumatura, più l'equatore per dare la profondità.
fn draw_range_bubble(
    cam: &forge_scene::Camera,
    painter: &egui::Painter,
    rect: egui::Rect,
    l: &forge_scene::Light,
    color: egui::Color32,
    selected: bool,
) {
    // Contorno della sfera visto dalla camera: cerchio perpendicolare alla linea di vista.
    let view = if cam.lens.ortho {
        cam.target - cam.position
    } else {
        l.position - cam.position
    }
    .normalize_or(Vec3::Z);
    let hint = if view.y.abs() > 0.99 { Vec3::X } else { Vec3::Y };
    let a = hint.cross(view).normalize();
    let b = view.cross(a);
    let ring = |r: f32, u: Vec3, v: Vec3| -> Vec<egui::Pos2> {
        (0..=64)
            .filter_map(|k| {
                let t = k as f32 / 64.0 * std::f32::consts::TAU;
                project(cam, rect, l.position + (u * t.cos() + v * t.sin()) * r)
            })
            .collect()
    };
    let alpha = if selected { 0.9 } else { 0.45 };
    let outer = ring(l.range, a, b);
    if outer.len() > 2 {
        painter.add(egui::Shape::line(outer, egui::Stroke::new(1.2, color.gamma_multiply(alpha))));
    }
    let fade = l.range_fade_start();
    if fade > 1e-4 && fade < l.range - 1e-4 {
        let inner = ring(fade, a, b);
        if inner.len() > 2 {
            painter.add(egui::Shape::dashed_line(
                &inner,
                egui::Stroke::new(1.0, color.gamma_multiply(alpha * 0.8)),
                5.0,
                4.0,
            ));
        }
    }
    if selected {
        let eq = ring(l.range, Vec3::X, Vec3::Z);
        if eq.len() > 2 {
            painter.add(egui::Shape::line(eq, egui::Stroke::new(0.8, color.gamma_multiply(0.35))));
        }
    }
}

fn render_view(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let resp = camera_controls(app, ui, rect);
    let _ = resp;
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_gray(14));
    let ctx = ui.ctx().clone();
    // Scelta della camera di rendering (se ce n'è più d'una), in alto a destra.
    if !app.scene.render_cameras.is_empty() {
        let bar = egui::Rect::from_min_max(
            egui::pos2(rect.right() - 230.0, rect.top() + 8.0),
            egui::pos2(rect.right() - 8.0, rect.top() + 40.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bar)
                .layout(egui::Layout::right_to_left(egui::Align::Min)),
        );
        crate::ui::inspector::camera_picker(app, &mut child, "render-view-camera");
    }
    let stale = app.render_is_stale();
    let preparing = app.is_preparing();
    let progress = app.session.progress();
    if let Some(tex) = app.render_texture(&ctx) {
        let [w, h] = tex.size();
        let scale = (rect.width() / w as f32).min(rect.height() / h as f32);
        let size = egui::vec2(w as f32, h as f32) * scale;
        let img_rect = egui::Rect::from_center_size(rect.center(), size);
        painter.image(
            tex.id(),
            img_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        painter.rect_stroke(
            img_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            egui::StrokeKind::Outside,
        );
    } else if !preparing {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            tr("Premi «Avvia render» (F5) per avviare il path tracing"),
            egui::FontId::proportional(16.0),
            egui::Color32::from_gray(160),
        );
    }
    let mut overlay = Vec::new();
    if preparing {
        overlay.push((tr("Preparazione della scena (costruzione BVH)…").to_string(), style::WARN));
    }
    if stale && !matches!(progress.state, SessionState::Idle) {
        overlay.push((
            tr("Immagine non aggiornata: la scena è cambiata (premi Riavvia)").to_string(),
            style::WARN,
        ));
    }
    overlay.push((
        tf(
            "{} · {} campioni",
            &[tr_dyn(&progress.state.label()).to_string(), format!("{}", progress.samples)],
        ),
        egui::Color32::from_gray(200),
    ));
    if let Some(f) = app.session.latest_frame().filter(|f| f.denoised.is_some()) {
        if app.render_view == crate::app::RenderView::Final {
            overlay.push((
                tf("Ripulita a {} campioni (denoiser)", &[format!("{}", f.denoised_samples)]),
                egui::Color32::from_rgb(140, 200, 140),
            ));
        }
    }
    for (k, (t, c)) in overlay.iter().enumerate() {
        painter.text(
            rect.left_top() + egui::vec2(10.0, 10.0 + k as f32 * 18.0),
            egui::Align2::LEFT_TOP,
            t,
            egui::FontId::proportional(13.0),
            *c,
        );
    }
}

// ----------------------------------------------------------------------
// Anteprima del materiale (sfera renderizzata con il path tracer CPU)
// ----------------------------------------------------------------------

#[derive(Default)]
pub struct MaterialPreview {
    shown_key: Option<String>,
    pending: Option<(String, Receiver<Vec<u8>>)>,
    texture: Option<egui::TextureHandle>,
}

const PREVIEW_SIZE: u32 = 128;

pub fn material_preview(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let Some(mat) = app.scene.materials.get(&app.selected_material).cloned() else {
        return;
    };
    let key = format!("{mat:?}");
    let pv = &mut app.preview;
    if let Some((k, rx)) = &pv.pending {
        if let Ok(rgba) = rx.try_recv() {
            let img = egui::ColorImage::from_rgba_unmultiplied([PREVIEW_SIZE as usize; 2], &rgba);
            match &mut pv.texture {
                Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                None => pv.texture = Some(ui.ctx().load_texture("material-preview", img, egui::TextureOptions::LINEAR)),
            }
            pv.shown_key = Some(k.clone());
            pv.pending = None;
        }
    }
    if pv.pending.is_none() && pv.shown_key.as_ref() != Some(&key) {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("forge-preview".into())
            .spawn(move || {
                let _ = tx.send(render_preview(&mat, PREVIEW_SIZE));
            })
            .ok();
        pv.pending = Some((key.clone(), rx));
    }
    if pv.pending.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if let Some(t) = &pv.texture {
            ui.image((t.id(), egui::vec2(PREVIEW_SIZE as f32, PREVIEW_SIZE as f32)));
        }
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(tr("Anteprima")).strong());
            ui.label(egui::RichText::new(tr("Path tracing CPU, 32 campioni")).weak().small());
            if pv.pending.is_some() {
                ui.spinner();
            }
        });
    });
}

/// Anteprima del materiale su una sfera, `size` × `size` pixel (RGBA8). I campioni crescono
/// con il lato (32 a 128 px, fino a 128 a 512 px) perché le anteprime grandi non siano rumorose.
pub(crate) fn render_preview(mat: &Material, size: u32) -> Vec<u8> {
    let size = size.clamp(32, 1024);
    let samples = (32 * size / PREVIEW_SIZE).clamp(32, 128);
    // Le anteprime piccole usano 2 thread (se ne fanno tante insieme), le grandi metà dei core.
    let threads = if size <= PREVIEW_SIZE {
        2
    } else {
        std::thread::available_parallelism().map(|n| n.get() / 2).unwrap_or(2).max(2)
    };
    let mut s = Scene::default();
    let mut m = mat.clone();
    m.id = forge_scene::MaterialId::new("__preview");
    // Scena alla scala del materiale: con la proiezione box la sfera è grande quanto una
    // tessera della texture (come le miniature di ambientCG/Poly Haven), così rilievo e
    // displacement, che sono in millimetri reali, si vedono nella proporzione giusta.
    let k = if m.has_textures() && m.mapping.projection == forge_scene::Projection::Box {
        (m.mapping.scale * 0.5).clamp(0.01, 10.0)
    } else {
        1.0
    };
    s.materials.upsert(m);
    let sphere = s.add_mesh(primitives::uv_sphere(k, 64, 32));
    let floor = s.add_mesh(primitives::plane(20.0 * k, 20.0 * k));
    let id = ComponentId("sfera".into());
    s.add_node(
        None,
        id.clone(),
        "sfera",
        forge_math::Mat4::from_translation(Vec3::Y * k),
        Some(sphere),
    );
    s.add_node(None, ComponentId("piano".into()), "piano", forge_math::Mat4::IDENTITY, Some(floor));
    s.assignments.insert(id, forge_scene::MaterialId::new("__preview"));
    // Luce ad area scalata con la scena: stessa radianza e stesso angolo solido, quindi
    // stessa illuminazione a qualsiasi scala.
    s.lights = vec![Light::area(
        "key",
        Vec3::new(-2.5, 4.0, 3.0) * k,
        Vec3::Y * k,
        3.0 * k,
        3.0 * k,
        4.0,
    )];
    s.camera = Camera {
        position: Vec3::new(0.0, 1.6, 4.2) * k,
        target: Vec3::new(0.0, 0.9, 0.0) * k,
        up: Vec3::Y,
        vfov_deg: 32.0,
        ..Default::default()
    };
    let rs = RenderScene::compile(&s);
    let settings = RenderSettings {
        width: size,
        height: size,
        max_bounces: 8,
        ..Default::default()
    };
    let mut acc = Accumulator::new(size, size);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = std::sync::atomic::AtomicBool::new(false);
    if let Ok(mut be) = forge_render_cpu::CpuBackend::new(threads) {
        for i in 0..samples {
            if let Ok(n @ 1..) = be.render_pass(&rs, &settings, i, 1, &mut pass, &cancel) {
                acc.add_samples(&pass, n);
            }
        }
    }
    forge_render_core::tonemap_to_rgba8(&acc.resolve(), settings.tone_mapping, 0.0)
}
