use crate::app::{ForgeApp, View};
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
        cam.ray_dir(nx, ny, aspect)
    };
    // Piano: orizzontale, oppure verticale rivolto verso la camera.
    let normal = if vertical {
        let f = cam.forward();
        Vec3::new(f.x, 0.0, f.z).normalize_or(Vec3::Z)
    } else {
        Vec3::Y
    };
    let hit = |d: Vec3| {
        let denom = d.dot(normal);
        if denom.abs() < 1e-6 {
            return None;
        }
        let t = (center - cam.position).dot(normal) / denom;
        (t > 0.0).then(|| cam.position + d * t)
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
                "Viewport 3D GPU non disponibile (modalità sicura).\nUsa la vista Render: il rendering CPU funziona normalmente.",
                egui::FontId::proportional(15.0),
                style::WARN,
            );
        }
    }
    draw_lights(app, &painter, rect);
    draw_cameras(app, &painter, rect);
    // Il gizmo ha la precedenza: se usa il mouse la camera non si muove e il clic non seleziona.
    let gizmo_used = crate::ui::gizmo::update(app, ui, rect, &resp, &painter);
    if !gizmo_used {
        camera_input(app, ui, rect, &resp);
    }
    crate::ui::gizmo::toolbar(app, ui, rect);

    // Picking con un clic (senza trascinamento).
    if resp.clicked() && !gizmo_used {
        if let Some(pos) = resp.interact_pointer_pos() {
            let ndc_x = (pos.x - rect.left()) / rect.width() * 2.0 - 1.0;
            let ndc_y = 1.0 - (pos.y - rect.top()) / rect.height() * 2.0;
            let cam = app.scene.camera.clone();
            let dir = cam.ray_dir(ndc_x, ndc_y, rect.width() / rect.height());
            let geo = app.geometry_now();
            let multi = ui.input(|i| i.modifiers.command);
            match forge_render_core::pick(&geo, cam.position, dir) {
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
            .unwrap_or("viewport disattivata"),
        if app.selection.is_empty() {
            "nessuna selezione".to_string()
        } else {
            format!("{} selezionati", app.selection.len())
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
                format!(
                    "Camera {} · {}×{}",
                    app.scene.render_cameras[i].name, app.settings.width, app.settings.height
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
        match l.kind {
            LightKind::Area { width, height } => {
                let n = l.direction();
                let hint = if n.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
                let u = hint.cross(n).normalize() * width * 0.5;
                let v = n.cross(u.normalize()) * height * 0.5;
                let corners = [l.position - u - v, l.position + u - v, l.position + u + v, l.position - u + v];
                let pts: Vec<_> = corners.iter().filter_map(|c| project(cam, rect, *c)).collect();
                if pts.len() == 4 {
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
                let scale = (width + height) * 0.25;
                if let Some(q) = project(cam, rect, l.position + n * scale) {
                    painter.arrow(p, q - p, stroke);
                }
            }
            LightKind::Point => {
                painter.circle_stroke(p, 6.0, stroke);
            }
            LightKind::Spot { .. } | LightKind::Directional => {
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

fn render_view(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let resp = camera_controls(app, ui, rect);
    let _ = resp;
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_gray(14));
    let ctx = ui.ctx().clone();
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
            "Premi «Avvia render» (F5) per avviare il path tracing",
            egui::FontId::proportional(16.0),
            egui::Color32::from_gray(160),
        );
    }
    let mut overlay = Vec::new();
    if preparing {
        overlay.push(("Preparazione della scena (costruzione BVH)…".to_string(), style::WARN));
    }
    if stale && !matches!(progress.state, SessionState::Idle) {
        overlay.push((
            "Immagine non aggiornata: la scena è cambiata (premi Riavvia)".to_string(),
            style::WARN,
        ));
    }
    overlay.push((
        format!("{} · {} campioni", progress.state.label(), progress.samples),
        egui::Color32::from_gray(200),
    ));
    if let Some(f) = app.session.latest_frame().filter(|f| f.denoised.is_some()) {
        if app.render_view == crate::app::RenderView::Final {
            overlay.push((
                format!("Ripulita a {} campioni (denoiser)", f.denoised_samples),
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
                let _ = tx.send(render_preview(&mat));
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
            ui.label(egui::RichText::new("Anteprima").strong());
            ui.label(egui::RichText::new("Path tracing CPU, 32 campioni").weak().small());
            if pv.pending.is_some() {
                ui.spinner();
            }
        });
    });
}

pub(crate) fn render_preview(mat: &Material) -> Vec<u8> {
    let mut s = Scene::default();
    let mut m = mat.clone();
    m.id = forge_scene::MaterialId::new("__preview");
    s.materials.upsert(m);
    let sphere = s.add_mesh(primitives::uv_sphere(1.0, 48, 24));
    let floor = s.add_mesh(primitives::plane(20.0, 20.0));
    let id = ComponentId("sfera".into());
    s.add_node(None, id.clone(), "sfera", forge_math::Mat4::from_translation(Vec3::Y), Some(sphere));
    s.add_node(None, ComponentId("piano".into()), "piano", forge_math::Mat4::IDENTITY, Some(floor));
    s.assignments.insert(id, forge_scene::MaterialId::new("__preview"));
    s.lights = vec![Light::area("key", Vec3::new(-2.5, 4.0, 3.0), Vec3::Y, 3.0, 3.0, 4.0)];
    s.camera = Camera {
        position: Vec3::new(0.0, 1.6, 4.2),
        target: Vec3::new(0.0, 0.9, 0.0),
        up: Vec3::Y,
        vfov_deg: 32.0,
    };
    let rs = RenderScene::compile(&s);
    let settings = RenderSettings {
        width: PREVIEW_SIZE,
        height: PREVIEW_SIZE,
        max_bounces: 8,
        ..Default::default()
    };
    let mut acc = Accumulator::new(PREVIEW_SIZE, PREVIEW_SIZE);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = std::sync::atomic::AtomicBool::new(false);
    if let Ok(mut be) = forge_render_cpu::CpuBackend::new(2) {
        for i in 0..32 {
            if let Ok(n @ 1..) = be.render_pass(&rs, &settings, i, 1, &mut pass, &cancel) {
                acc.add_samples(&pass, n);
            }
        }
    }
    forge_render_core::tonemap_to_rgba8(&acc.resolve(), settings.tone_mapping, 0.0)
}
