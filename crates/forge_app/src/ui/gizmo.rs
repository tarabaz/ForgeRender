//! Gizmo di trasformazione nella viewport: sposta (W), ruota (E), scala (R).
//!
//! Il gizmo agisce sulle trasformazioni utente non distruttive dei componenti
//! selezionati. Durante il trascinamento la trasformazione è sempre ricalcolata
//! a partire dallo stato iniziale (nessun accumulo di errori, snap esatto sul
//! valore totale, Esc ripristina tutto).
//!
//! - Sposta: frecce lungo gli assi, quadratini per muovere su un piano, cerchio
//!   centrale per muovere nel piano dello schermo.
//! - Ruota: anelli attorno agli assi (passanti per il punto di rotazione).
//! - Scala: cubetti sugli assi del componente, cerchio centrale = scala uniforme.
//! - Ctrl durante il trascinamento: snap ai passi impostati.

use crate::app::ForgeApp;
use crate::viewport::project;
use eframe::egui;
use forge_math::{Mat4, Quat, Vec3};
use forge_scene::{Scene, UserTransform};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GizmoMode {
    /// Solo selezione (nessun gizmo).
    Select,
    #[default]
    Move,
    Rotate,
    Scale,
}

impl GizmoMode {
    pub fn label(self) -> &'static str {
        match self {
            GizmoMode::Select => "Seleziona (Q)",
            GizmoMode::Move => "Sposta (W)",
            GizmoMode::Rotate => "Ruota (E)",
            GizmoMode::Scale => "Scala (R)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Handle {
    Axis(usize),
    /// Piano perpendicolare all'asse indicato.
    Plane(usize),
    Screen,
    Ring(usize),
    ScaleAxis(usize),
    ScaleUniform,
}

struct Drag {
    handle: Handle,
    origin: egui::Pos2,
    center: Vec3,
    axes: [Vec3; 3],
    start: Vec<(usize, UserTransform)>,
    /// Parametro iniziale (posizione sull'asse, punto sul piano, angolo o distanza a schermo).
    start_param: f32,
    start_hit: Vec3,
    /// Lunghezza dell'asse a schermo (per la scala).
    axis_px: f32,
}

pub struct Gizmo {
    pub mode: GizmoMode,
    /// Assi del componente invece di quelli del mondo.
    pub local: bool,
    pub snap_move_mm: f32,
    pub snap_rot_deg: f32,
    pub snap_scale: f32,
    hovered: Option<Handle>,
    drag: Option<Drag>,
    readout: Option<String>,
}

impl Default for Gizmo {
    fn default() -> Self {
        Self {
            mode: GizmoMode::Move,
            local: false,
            snap_move_mm: 10.0,
            snap_rot_deg: 15.0,
            snap_scale: 0.1,
            hovered: None,
            drag: None,
            readout: None,
        }
    }
}

impl Gizmo {
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }
}

const AXIS_COLORS: [egui::Color32; 3] = [
    egui::Color32::from_rgb(225, 75, 75),
    egui::Color32::from_rgb(110, 200, 90),
    egui::Color32::from_rgb(80, 135, 240),
];
const HOVER: egui::Color32 = egui::Color32::from_rgb(255, 210, 60);
const AXIS_NAMES: [&str; 3] = ["X", "Y", "Z"];
/// Lunghezza delle frecce a schermo (punti).
const GIZMO_PX: f32 = 95.0;

/// Componenti selezionati senza antenati anch'essi selezionati.
pub fn top_selected(app: &ForgeApp) -> Vec<usize> {
    app.selection
        .iter()
        .copied()
        .filter(|&i| {
            let mut p = app.scene.nodes[i].parent;
            while let Some(k) = p {
                if app.selection.contains(&k) {
                    return false;
                }
                p = app.scene.nodes[k].parent;
            }
            true
        })
        .collect()
}

/// Prodotto delle trasformazioni utente degli antenati (dalla radice al genitore).
fn ancestor_user(scene: &Scene, idx: usize) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    let mut k = scene.nodes[idx].parent;
    while let Some(i) = k {
        let n = &scene.nodes[i];
        if !n.user.is_identity() {
            m = n.user.matrix(n.pivot) * m;
        }
        k = n.parent;
    }
    m
}

/// Nuova trasformazione utente tale che il componente subisca `world` (in spazio mondo)
/// rispetto allo stato `start`.
pub fn apply_world(scene: &Scene, idx: usize, start: &UserTransform, world: Mat4) -> UserTransform {
    let a = ancestor_user(scene, idx);
    let p = scene.nodes[idx].pivot;
    let m = a.inverse() * world * a * start.matrix(p);
    let (scale, rot, _) = m.to_scale_rotation_translation();
    let (x, y, z) = rot.to_euler(forge_math::glam::EulerRot::XYZ);
    UserTransform {
        translation: m.transform_point3(p) - p,
        rotation_deg: Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees()),
        scale,
    }
}

/// Punto di rotazione del componente in spazio mondo.
fn world_pivot(scene: &Scene, idx: usize) -> Vec3 {
    let n = &scene.nodes[idx];
    ancestor_user(scene, idx).transform_point3(n.pivot + n.user.translation)
}

/// Assi del componente: rotazione utente composta con quella degli antenati.
fn local_axes(scene: &Scene, idx: usize) -> [Vec3; 3] {
    let n = &scene.nodes[idx];
    let r = n.user.rotation_deg;
    let rot = Quat::from_euler(
        forge_math::glam::EulerRot::XYZ,
        r.x.to_radians(),
        r.y.to_radians(),
        r.z.to_radians(),
    );
    let a = ancestor_user(scene, idx);
    [Vec3::X, Vec3::Y, Vec3::Z].map(|v| a.transform_vector3(rot * v).normalize_or(v))
}

fn snap(v: f32, step: f32) -> f32 {
    if step > 0.0 {
        (v / step).round() * step
    } else {
        v
    }
}

fn dist_to_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

struct Frame {
    center: Vec3,
    c2: egui::Pos2,
    axes: [Vec3; 3],
    /// Lunghezza dell'asse in metri, scelta per avere ~GIZMO_PX a schermo.
    len: f32,
}

impl Frame {
    fn pt(&self, app: &ForgeApp, rect: egui::Rect, p: Vec3) -> Option<egui::Pos2> {
        project(&app.scene.camera, rect, p)
    }

    fn ring(&self, app: &ForgeApp, rect: egui::Rect, k: usize) -> Vec<egui::Pos2> {
        let (u, v) = (self.axes[(k + 1) % 3], self.axes[(k + 2) % 3]);
        (0..=64)
            .filter_map(|i| {
                let a = i as f32 / 64.0 * std::f32::consts::TAU;
                self.pt(app, rect, self.center + (u * a.cos() + v * a.sin()) * self.len * 0.85)
            })
            .collect()
    }

    fn plane_quad(&self, app: &ForgeApp, rect: egui::Rect, k: usize) -> Option<[egui::Pos2; 4]> {
        let (u, v) = (self.axes[(k + 1) % 3], self.axes[(k + 2) % 3]);
        let (a, b) = (0.22 * self.len, 0.42 * self.len);
        let c = self.center;
        Some([
            self.pt(app, rect, c + u * a + v * a)?,
            self.pt(app, rect, c + u * b + v * a)?,
            self.pt(app, rect, c + u * b + v * b)?,
            self.pt(app, rect, c + u * a + v * b)?,
        ])
    }
}

fn inside_quad(p: egui::Pos2, q: &[egui::Pos2; 4]) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        let c = (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        if c.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = c.signum();
        } else if c.signum() != sign {
            return false;
        }
    }
    true
}

fn frame(app: &ForgeApp, rect: egui::Rect, sel: &[usize]) -> Option<Frame> {
    let center = if sel.len() == 1 {
        world_pivot(&app.scene, sel[0])
    } else {
        let mut b = forge_math::Aabb::EMPTY;
        for &s in sel {
            b.grow(&app.scene.bounds_of(s));
        }
        if b.is_empty() {
            return None;
        }
        b.center()
    };
    let cam = &app.scene.camera;
    let local = sel.len() == 1 && (app.gizmo.local || app.gizmo.mode == GizmoMode::Scale);
    let axes = if local {
        local_axes(&app.scene, sel[0])
    } else {
        [Vec3::X, Vec3::Y, Vec3::Z]
    };
    let dist = (center - cam.position).length().max(1e-4);
    let len = dist * 2.0 * (cam.vfov_deg.to_radians() * 0.5).tan() * GIZMO_PX / rect.height().max(1.0);
    let c2 = project(cam, rect, center)?;
    Some(Frame { center, c2, axes, len })
}

fn hit_test(app: &ForgeApp, rect: egui::Rect, f: &Frame, p: egui::Pos2) -> Option<Handle> {
    let mut best: Option<(f32, Handle)> = None;
    let mut consider = |d: f32, h: Handle, tol: f32| {
        if d < tol && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, h));
        }
    };
    match app.gizmo.mode {
        GizmoMode::Select => {}
        GizmoMode::Move => {
            consider(f.c2.distance(p), Handle::Screen, 9.0);
            for k in 0..3 {
                if let Some(q) = f.plane_quad(app, rect, k) {
                    if inside_quad(p, &q) {
                        consider(4.0, Handle::Plane(k), 9.0);
                    }
                }
                if let (Some(a), Some(b)) = (
                    f.pt(app, rect, f.center + f.axes[k] * f.len * 0.2),
                    f.pt(app, rect, f.center + f.axes[k] * f.len),
                ) {
                    consider(dist_to_segment(p, a, b), Handle::Axis(k), 8.0);
                }
            }
        }
        GizmoMode::Rotate => {
            for k in 0..3 {
                let r = f.ring(app, rect, k);
                let d = r.windows(2).map(|w| dist_to_segment(p, w[0], w[1])).fold(f32::INFINITY, f32::min);
                consider(d, Handle::Ring(k), 7.0);
            }
        }
        GizmoMode::Scale => {
            consider(f.c2.distance(p), Handle::ScaleUniform, 11.0);
            for k in 0..3 {
                if let (Some(a), Some(b)) = (
                    f.pt(app, rect, f.center + f.axes[k] * f.len * 0.2),
                    f.pt(app, rect, f.center + f.axes[k] * f.len),
                ) {
                    consider(dist_to_segment(p, a, b), Handle::ScaleAxis(k), 8.0);
                }
            }
        }
    }
    best.map(|b| b.1)
}

fn mouse_ray(app: &ForgeApp, rect: egui::Rect, p: egui::Pos2) -> (Vec3, Vec3) {
    let cam = &app.scene.camera;
    let nx = (p.x - rect.left()) / rect.width() * 2.0 - 1.0;
    let ny = 1.0 - (p.y - rect.top()) / rect.height() * 2.0;
    (cam.position, cam.ray_dir(nx, ny, rect.width() / rect.height().max(1.0)))
}

/// Parametro del punto dell'asse (c + a·s) più vicino al raggio del mouse.
fn axis_param(o: Vec3, d: Vec3, c: Vec3, a: Vec3) -> Option<f32> {
    let w0 = c - o;
    let b = a.dot(d);
    let denom = 1.0 - b * b;
    if denom < 1e-4 {
        return None;
    }
    Some((b * d.dot(w0) - a.dot(w0)) / denom)
}

fn plane_hit(o: Vec3, d: Vec3, c: Vec3, n: Vec3) -> Option<Vec3> {
    let den = d.dot(n);
    if den.abs() < 1e-5 {
        return None;
    }
    let t = (c - o).dot(n) / den;
    (t > 0.0).then(|| o + d * t)
}

fn screen_angle(c: egui::Pos2, p: egui::Pos2) -> f32 {
    (p.y - c.y).atan2(p.x - c.x)
}

/// Gestisce hover, trascinamento e disegno del gizmo. Restituisce vero se il
/// gizmo usa il mouse (la camera non deve orbitare e il clic non seleziona).
pub fn update(app: &mut ForgeApp, ui: &egui::Ui, rect: egui::Rect, resp: &egui::Response, painter: &egui::Painter) -> bool {
    let sel = top_selected(app);
    if app.gizmo.mode == GizmoMode::Select || sel.is_empty() || ui.input(|i| i.modifiers.alt) {
        app.gizmo.drag = None;
        app.gizmo.hovered = None;
        return false;
    }
    let Some(f) = frame(app, rect, &sel) else { return false };

    // Inizio del trascinamento: si usa il punto in cui è stato premuto il tasto.
    if app.gizmo.drag.is_none() && resp.drag_started_by(egui::PointerButton::Primary) {
        if let Some(origin) = ui.input(|i| i.pointer.press_origin()) {
            if let Some(h) = hit_test(app, rect, &f, origin) {
                let (o, d) = mouse_ray(app, rect, origin);
                let cam_fwd = app.scene.camera.forward();
                let (start_param, start_hit, axis_px) = match h {
                    Handle::Axis(k) => (axis_param(o, d, f.center, f.axes[k]).unwrap_or(0.0), Vec3::ZERO, 0.0),
                    Handle::Plane(k) => (0.0, plane_hit(o, d, f.center, f.axes[k]).unwrap_or(f.center), 0.0),
                    Handle::Screen => (0.0, plane_hit(o, d, f.center, cam_fwd).unwrap_or(f.center), 0.0),
                    Handle::Ring(_) => (screen_angle(f.c2, origin), Vec3::ZERO, 0.0),
                    Handle::ScaleAxis(k) => {
                        let e = f.pt(app, rect, f.center + f.axes[k] * f.len).unwrap_or(f.c2);
                        let dir = (e - f.c2).normalized();
                        ((origin - f.c2).dot(dir), Vec3::ZERO, (e - f.c2).length().max(1.0))
                    }
                    Handle::ScaleUniform => (f.c2.distance(origin).max(4.0), Vec3::ZERO, 0.0),
                };
                let start = sel.iter().map(|&i| (i, app.scene.nodes[i].user)).collect::<Vec<_>>();
                app.push_undo(&sel);
                app.gizmo.drag = Some(Drag {
                    handle: h,
                    origin,
                    center: f.center,
                    axes: f.axes,
                    start,
                    start_param,
                    start_hit,
                    axis_px,
                });
            }
        }
    }

    if let Some(drag) = app.gizmo.drag.take() {
        let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if cancel {
            for (i, u) in &drag.start {
                app.scene.nodes[*i].user = *u;
            }
            app.discard_last_undo();
            app.geometry_changed();
            app.gizmo.readout = None;
            return true;
        }
        if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged()) {
            let snapping = ui.input(|i| i.modifiers.command);
            apply_drag(app, rect, &drag, pos, snapping);
        }
        let done = resp.drag_stopped() || !ui.input(|i| i.pointer.primary_down());
        let handle = drag.handle;
        if !done {
            app.gizmo.drag = Some(drag);
        } else {
            app.gizmo.readout = None;
        }
        draw(app, rect, painter, &f, Some(handle));
        return true;
    }

    let hover = resp.hover_pos().and_then(|p| hit_test(app, rect, &f, p));
    app.gizmo.hovered = hover;
    draw(app, rect, painter, &f, hover);
    hover.is_some()
}

fn apply_drag(app: &mut ForgeApp, rect: egui::Rect, drag: &Drag, pos: egui::Pos2, snapping: bool) {
    let (o, d) = mouse_ray(app, rect, pos);
    let g = &app.gizmo;
    let step_m = if snapping { g.snap_move_mm / 1000.0 } else { 0.0 };
    let c2 = project(&app.scene.camera, rect, drag.center).unwrap_or(drag.origin);
    enum Op {
        World(Mat4),
        Scale(Vec3),
    }
    let (op, text) = match drag.handle {
        Handle::Axis(k) => {
            let Some(s) = axis_param(o, d, drag.center, drag.axes[k]) else {
                return;
            };
            let delta = snap(s - drag.start_param, step_m);
            (
                Op::World(Mat4::from_translation(drag.axes[k] * delta)),
                format!("{} {:+.1} mm", AXIS_NAMES[k], delta * 1000.0),
            )
        }
        Handle::Plane(_) | Handle::Screen => {
            let n = match drag.handle {
                Handle::Plane(k) => drag.axes[k],
                _ => app.scene.camera.forward(),
            };
            let Some(hit) = plane_hit(o, d, drag.center, n) else { return };
            let mut delta = hit - drag.start_hit;
            if step_m > 0.0 {
                // Snap per componente lungo gli assi del gizmo.
                delta = drag
                    .axes
                    .iter()
                    .map(|a| *a * snap(delta.dot(*a), step_m))
                    .fold(Vec3::ZERO, |s, v| s + v);
            }
            (
                Op::World(Mat4::from_translation(delta)),
                format!(
                    "Δ {:.1} mm  (X {:+.1}  Y {:+.1}  Z {:+.1})",
                    delta.length() * 1000.0,
                    delta.x * 1000.0,
                    delta.y * 1000.0,
                    delta.z * 1000.0
                ),
            )
        }
        Handle::Ring(k) => {
            let mut ang = screen_angle(c2, pos) - drag.start_param;
            ang = (ang + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            // L'angolo a schermo cresce in senso orario (y verso il basso): verso dell'asse rispetto alla camera.
            if drag.axes[k].dot(app.scene.camera.forward()) < 0.0 {
                ang = -ang;
            }
            let mut deg = -ang.to_degrees();
            if snapping {
                deg = snap(deg, g.snap_rot_deg);
            }
            let r = Quat::from_axis_angle(drag.axes[k], deg.to_radians());
            (
                Op::World(Mat4::from_translation(drag.center) * Mat4::from_quat(r) * Mat4::from_translation(-drag.center)),
                format!("{} {:+.1}°", AXIS_NAMES[k], deg),
            )
        }
        Handle::ScaleAxis(k) => {
            let e = project(&app.scene.camera, rect, drag.center + drag.axes[k] * 1.0).unwrap_or(c2);
            let dir = (e - c2).normalized();
            let mut factor = 1.0 + ((pos - c2).dot(dir) - drag.start_param) / drag.axis_px;
            if snapping {
                factor = 1.0 + snap(factor - 1.0, g.snap_scale);
            }
            let factor = if factor.abs() < 0.01 { 0.01f32.copysign(factor) } else { factor };
            let mut s = Vec3::ONE;
            s[k] = factor;
            (Op::Scale(s), format!("Scala {} ×{factor:.3}", AXIS_NAMES[k]))
        }
        Handle::ScaleUniform => {
            let mut factor = c2.distance(pos).max(1.0) / drag.start_param;
            if snapping {
                factor = 1.0 + snap(factor - 1.0, g.snap_scale);
            }
            let factor = factor.max(0.01);
            (Op::Scale(Vec3::splat(factor)), format!("Scala ×{factor:.3}"))
        }
    };
    for (i, start) in &drag.start {
        let u = match &op {
            Op::World(m) => apply_world(&app.scene, *i, start, *m),
            Op::Scale(s) => UserTransform {
                scale: start.scale * *s,
                ..*start
            },
        };
        app.scene.nodes[*i].user = u;
    }
    app.gizmo.readout = Some(text);
    app.geometry_changed();
}

#[allow(clippy::needless_range_loop)] // k indicizza assi, colori e nomi insieme
fn draw(app: &ForgeApp, rect: egui::Rect, painter: &egui::Painter, f: &Frame, active: Option<Handle>) {
    let color = |h: Handle, k: usize| if active == Some(h) { HOVER } else { AXIS_COLORS[k] };
    let shadow = egui::Stroke::new(5.0, egui::Color32::from_black_alpha(90));
    match app.gizmo.mode {
        GizmoMode::Select => {}
        GizmoMode::Move => {
            for k in 0..3 {
                if let Some(q) = f.plane_quad(app, rect, k) {
                    let c = color(Handle::Plane(k), k);
                    painter.add(egui::Shape::convex_polygon(
                        q.to_vec(),
                        c.gamma_multiply(0.35),
                        egui::Stroke::new(1.0, c),
                    ));
                }
            }
            for k in 0..3 {
                let (Some(a), Some(b)) = (
                    f.pt(app, rect, f.center + f.axes[k] * f.len * 0.2),
                    f.pt(app, rect, f.center + f.axes[k] * f.len),
                ) else {
                    continue;
                };
                let c = color(Handle::Axis(k), k);
                painter.line_segment([a, b], shadow);
                painter.arrow(a, b - a, egui::Stroke::new(3.0, c));
                painter.text(
                    b + (b - a).normalized() * 10.0,
                    egui::Align2::CENTER_CENTER,
                    AXIS_NAMES[k],
                    egui::FontId::proportional(12.0),
                    c,
                );
            }
            let c = if active == Some(Handle::Screen) {
                HOVER
            } else {
                egui::Color32::from_gray(230)
            };
            painter.circle_stroke(f.c2, 7.0, egui::Stroke::new(2.0, c));
        }
        GizmoMode::Rotate => {
            for k in 0..3 {
                let r = f.ring(app, rect, k);
                let c = color(Handle::Ring(k), k);
                painter.add(egui::Shape::line(r.clone(), shadow));
                painter.add(egui::Shape::line(
                    r,
                    egui::Stroke::new(if active == Some(Handle::Ring(k)) { 3.5 } else { 2.5 }, c),
                ));
            }
            painter.circle_filled(f.c2, 3.0, egui::Color32::from_gray(230));
        }
        GizmoMode::Scale => {
            for k in 0..3 {
                let (Some(a), Some(b)) = (
                    f.pt(app, rect, f.center + f.axes[k] * f.len * 0.2),
                    f.pt(app, rect, f.center + f.axes[k] * f.len),
                ) else {
                    continue;
                };
                let c = color(Handle::ScaleAxis(k), k);
                painter.line_segment([a, b], shadow);
                painter.line_segment([a, b], egui::Stroke::new(3.0, c));
                painter.rect_filled(egui::Rect::from_center_size(b, egui::vec2(10.0, 10.0)), 1.0, c);
            }
            let c = if active == Some(Handle::ScaleUniform) {
                HOVER
            } else {
                egui::Color32::from_gray(230)
            };
            painter.circle_stroke(f.c2, 10.0, egui::Stroke::new(2.5, c));
        }
    }
    if let (Some(t), Some(p)) = (&app.gizmo.readout, painter.ctx().pointer_latest_pos()) {
        let pos = p + egui::vec2(16.0, 18.0);
        let galley = painter.layout_no_wrap(t.clone(), egui::FontId::proportional(13.0), egui::Color32::WHITE);
        painter.rect_filled(
            egui::Rect::from_min_size(pos - egui::vec2(4.0, 2.0), galley.size() + egui::vec2(8.0, 4.0)),
            3.0,
            egui::Color32::from_black_alpha(170),
        );
        painter.galley(pos, galley, egui::Color32::WHITE);
    }
}

/// Barra degli strumenti nella viewport: modalità, assi globali/locali, passi di snap.
pub fn toolbar(app: &mut ForgeApp, ui: &mut egui::Ui, rect: egui::Rect) {
    let area = egui::Area::new(egui::Id::NULL.with("gizmo-toolbar"))
        .fixed_pos(rect.left_top() + egui::vec2(8.0, 8.0))
        .order(egui::Order::Foreground);
    area.show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).inner_margin(4.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                for m in [GizmoMode::Select, GizmoMode::Move, GizmoMode::Rotate, GizmoMode::Scale] {
                    ui.selectable_value(&mut app.gizmo.mode, m, m.label());
                }
                ui.separator();
                ui.selectable_value(&mut app.gizmo.local, false, "Globale")
                    .on_hover_text("Assi del mondo (X, Y verso l'alto, Z)");
                ui.selectable_value(&mut app.gizmo.local, true, "Locale")
                    .on_hover_text("Assi ruotati con il componente (la scala usa sempre gli assi del componente)");
                ui.separator();
                ui.menu_button("Snap (Ctrl)", |ui| {
                    ui.label("Tieni premuto Ctrl durante il trascinamento");
                    ui.add(
                        egui::DragValue::new(&mut app.gizmo.snap_move_mm)
                            .range(0.01..=10000.0)
                            .prefix("Spostamento: ")
                            .suffix(" mm"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut app.gizmo.snap_rot_deg)
                            .range(0.1..=180.0)
                            .prefix("Rotazione: ")
                            .suffix("°"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut app.gizmo.snap_scale)
                            .range(0.001..=10.0)
                            .speed(0.01)
                            .prefix("Scala: ×"),
                    );
                });
                let (can_undo, can_redo) = app.can_undo_redo();
                if ui
                    .add_enabled(can_undo, egui::Button::new("Annulla"))
                    .on_hover_text("Annulla (Ctrl+Z)")
                    .clicked()
                {
                    app.undo();
                }
                if ui
                    .add_enabled(can_redo, egui::Button::new("Ripeti"))
                    .on_hover_text("Ripeti (Ctrl+Y)")
                    .clicked()
                {
                    app.redo();
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_scene::ComponentId;

    fn scene_with_child() -> (Scene, usize, usize) {
        let mut s = Scene::default();
        let m = s.add_mesh(forge_scene::primitives::cuboid(Vec3::ONE));
        let parent = s.add_node(
            None,
            ComponentId("a".into()),
            "A",
            Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0)),
            None,
        );
        let child = s.add_node(Some(parent), ComponentId("a/b".into()), "B", Mat4::IDENTITY, Some(m));
        s.refresh_pivots();
        (s, parent, child)
    }

    #[test]
    fn world_translation_maps_through_rotated_parent() {
        let (mut s, parent, child) = scene_with_child();
        s.nodes[parent].user.rotation_deg = Vec3::new(0.0, 90.0, 0.0);
        let before = s.bounds_of(child).center();
        let u = apply_world(
            &s,
            child,
            &s.nodes[child].user.clone(),
            Mat4::from_translation(Vec3::new(0.0, 0.0, 0.5)),
        );
        s.nodes[child].user = u;
        let after = s.bounds_of(child).center();
        assert!((after - before - Vec3::new(0.0, 0.0, 0.5)).length() < 1e-4, "{before} → {after}");
    }

    #[test]
    fn rotation_about_pivot_keeps_center_and_composes() {
        let (mut s, _, child) = scene_with_child();
        let c = world_pivot(&s, child);
        let r =
            |deg: f32| Mat4::from_translation(c) * Mat4::from_quat(Quat::from_rotation_y(deg.to_radians())) * Mat4::from_translation(-c);
        let u1 = apply_world(&s, child, &s.nodes[child].user.clone(), r(30.0));
        s.nodes[child].user = u1;
        let u2 = apply_world(&s, child, &u1, r(60.0));
        assert!(
            (u2.rotation_deg - Vec3::new(0.0, 90.0, 0.0)).length() < 1e-3,
            "{:?}",
            u2.rotation_deg
        );
        assert!(u2.translation.length() < 1e-4);
        s.nodes[child].user = u2;
        assert!((world_pivot(&s, child) - c).length() < 1e-4);
    }

    #[test]
    fn axis_param_finds_closest_point() {
        // Raggio verticale che attraversa l'asse X in x = 3.
        let s = axis_param(Vec3::new(3.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0), Vec3::ZERO, Vec3::X).unwrap();
        assert!((s - 3.0).abs() < 1e-5);
        assert!(axis_param(Vec3::ZERO, Vec3::X, Vec3::ZERO, Vec3::X).is_none());
    }
}
