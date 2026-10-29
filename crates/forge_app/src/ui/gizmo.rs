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
use forge_scene::{Light, LightKind, Scene, UserTransform};

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

/// Stato iniziale del bersaglio del trascinamento (per ricalcolare e per Esc).
enum DragStart {
    Nodes(Vec<(usize, UserTransform)>),
    Light(usize, Light),
    /// Emettitore e zona simulata della nebbia.
    /// Nebbia: emettitore, dimensioni e spostamento della zona all'inizio.
    Fog(usize, Vec3, Vec3, Vec3),
}

/// Su cosa agisce il gizmo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Nodes,
    Light(usize),
    Fog(usize),
}

/// Dati per lo snap a contatto, preparati all'inizio del trascinamento: gli ostacoli
/// non si muovono, quindi le loro BVH restano valide per tutto il trascinamento.
struct Contact {
    /// Vertici (in spazio mondo, all'inizio) dei componenti spostati.
    verts: Vec<Vec3>,
    /// Centro dei componenti spostati (per capire da che lato sta l'oggetto a contatto).
    moving_center: Vec3,
    /// Centro dell'ostacolo a cui appartiene ogni vertice di `obstacle_verts`.
    obstacle_centers: Vec<Vec3>,
    moving: forge_accel::TriangleAccel,
    moving_n: Vec<Vec3>,
    obstacles: forge_accel::TriangleAccel,
    obstacle_n: Vec<Vec3>,
    obstacle_verts: Vec<Vec3>,
}

/// Distanza di sicurezza lasciata tra le superfici a contatto (m).
const CONTACT_GAP: f32 = 2e-4;
/// Massimo di vertici usati per lato (oltre si campiona a passo regolare).
const CONTACT_MAX_VERTS: usize = 40_000;

impl Contact {
    /// Prepara il contatto se uno dei componenti trascinati ha lo snap attivo.
    fn build(scene: &Scene, sel: &[usize]) -> Option<Contact> {
        if !sel.iter().any(|&i| scene.nodes[i].contact_snap) {
            return None;
        }
        // Componenti che si muovono: i selezionati con tutti i discendenti.
        let mut moving = vec![false; scene.nodes.len()];
        let mut stack: Vec<usize> = sel.to_vec();
        while let Some(i) = stack.pop() {
            moving[i] = true;
            stack.extend(scene.nodes[i].children.iter().copied());
        }
        let (mut mt, mut ot) = (Vec::new(), Vec::new());
        let (mut mv, mut ov, mut oc) = (Vec::new(), Vec::new(), Vec::new());
        for (i, n) in scene.nodes.iter().enumerate() {
            let Some(mi) = n.mesh else { continue };
            if !scene.is_visible(i) {
                continue;
            }
            let m = &scene.meshes[mi];
            let xf = scene.world_transform(i);
            let wp: Vec<Vec3> = m.positions.iter().map(|p| xf.transform_point3(*p)).collect();
            let tris = if moving[i] { &mut mt } else { &mut ot };
            for idx in &m.indices {
                tris.push(idx.map(|k| wp[k as usize]));
            }
            if moving[i] {
                mv.extend(wp);
            } else {
                let c = forge_math::Aabb::from_points(&wp).center();
                oc.extend(std::iter::repeat_n(c, wp.len()));
                ov.extend(wp);
            }
        }
        let step = |n: usize| n.div_ceil(CONTACT_MAX_VERTS).max(1);
        let (sm, so) = (step(mv.len()), step(ov.len()));
        let moving_center = forge_math::Aabb::from_points(&mv).center();
        let normals = |t: &[[Vec3; 3]]| t.iter().map(|t| (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero()).collect();
        Some(Contact {
            verts: mv.into_iter().step_by(sm).collect(),
            moving_center,
            moving_n: normals(&mt),
            moving: forge_accel::TriangleAccel::build(&mt),
            obstacle_n: normals(&ot),
            obstacles: forge_accel::TriangleAccel::build(&ot),
            obstacle_verts: ov.into_iter().step_by(so).collect(),
            obstacle_centers: oc.into_iter().step_by(so).collect(),
        })
    }

    /// Lunghezza ammessa dello spostamento `d` (≤ |d|) prima di toccare un ostacolo.
    /// Le superfici quasi parallele al movimento non fermano (si può scivolare su un piano).
    fn limit(&self, d: Vec3) -> f32 {
        use rayon::prelude::*;
        let len = d.length();
        if len < 1e-9 {
            return len;
        }
        let dir = d / len;
        // Raggi da ogni punto lungo `dir`; i contatti già in essere (t ≈ 0) fermano solo se
        // il movimento entra nell'ostacolo (dal lato del proprio centro verso l'altro).
        let cast =
            |accel: &forge_accel::TriangleAccel, normals: &[Vec3], from: &[Vec3], centers: &(dyn Fn(usize) -> Vec3 + Sync), dir: Vec3| {
                from.par_iter()
                    .enumerate()
                    .filter_map(|(k, v)| {
                        let center = centers(k);
                        accel
                            .intersect_filtered(*v, dir, -CONTACT_GAP, len + CONTACT_GAP, |prim, t, _, _| {
                                let n = normals[prim as usize];
                                let along = n.dot(dir);
                                if along.abs() < 0.05 {
                                    return false; // superficie parallela al movimento: si scivola
                                }
                                if t > CONTACT_GAP {
                                    return true;
                                }
                                let side = n.dot(center - *v);
                                side != 0.0 && along * side.signum() < 0.0
                            })
                            .map(|h| h.t)
                    })
                    .reduce(|| f32::INFINITY, f32::min)
            };
        // Vertici dell'oggetto contro gli ostacoli, e vertici degli ostacoli contro l'oggetto.
        let mc = self.moving_center;
        let t = cast(&self.obstacles, &self.obstacle_n, &self.verts, &|_| mc, dir).min(cast(
            &self.moving,
            &self.moving_n,
            &self.obstacle_verts,
            &|k| self.obstacle_centers[k],
            -dir,
        ));
        if t.is_finite() {
            (t - CONTACT_GAP).clamp(0.0, len)
        } else {
            len
        }
    }
}

struct Drag {
    contact: Option<Contact>,
    handle: Handle,
    origin: egui::Pos2,
    center: Vec3,
    axes: [Vec3; 3],
    start: DragStart,
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
    /// Maniglia del cono dello spot trascinata: (esterna?, tipo di luce iniziale per Esc).
    cone_drag: Option<(bool, LightKind)>,
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
            cone_drag: None,
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

/// Bersaglio del gizmo: i componenti selezionati, altrimenti la luce o la nebbia selezionata.
pub fn target(app: &ForgeApp) -> Option<Target> {
    if !app.selection.is_empty() {
        return Some(Target::Nodes);
    }
    if let Some(i) = app.selected_light.filter(|&i| i < app.scene.lights.len()) {
        return Some(Target::Light(i));
    }
    app.selected_fog.filter(|&i| i < app.scene.fogs.len()).map(Target::Fog)
}

/// Assi propri di una luce: (destra, su, direzione) attorno alla direzione di emissione.
fn light_axes(l: &Light) -> [Vec3; 3] {
    let n = l.direction();
    let hint = if n.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let u = hint.cross(n).normalize_or(Vec3::X);
    [u, n.cross(u).normalize_or(Vec3::Y), n]
}

fn frame_for(app: &ForgeApp, rect: egui::Rect, t: Target, sel: &[usize]) -> Option<Frame> {
    let (center, axes) = match t {
        Target::Nodes => return frame(app, rect, sel),
        Target::Light(i) => {
            let l = &app.scene.lights[i];
            let local = app.gizmo.local || app.gizmo.mode == GizmoMode::Scale;
            let axes = if local && !matches!(l.kind, LightKind::Point) {
                light_axes(l)
            } else {
                [Vec3::X, Vec3::Y, Vec3::Z]
            };
            (l.position, axes)
        }
        Target::Fog(i) => {
            let f = &app.scene.fogs[i];
            let c = if app.fog_gizmo_zone {
                let (lo, hi) = f.bounds();
                (lo + hi) * 0.5
            } else {
                f.emitter
            };
            (c, [Vec3::X, Vec3::Y, Vec3::Z])
        }
    };
    let cam = &app.scene.camera;
    let dist = (center - cam.position).length().max(1e-4);
    let len = dist * 2.0 * (cam.vfov_deg.to_radians() * 0.5).tan() * GIZMO_PX / rect.height().max(1.0);
    let c2 = project(cam, rect, center)?;
    Some(Frame { center, c2, axes, len })
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
    cam.ray(nx, ny, rect.width() / rect.height().max(1.0))
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
    let tgt = target(app);
    // Modalità senza senso per il bersaglio: ruotare una luce puntiforme o la nebbia,
    // scalare una luce che non sia ad area.
    let pointless = match tgt {
        Some(Target::Light(i)) => {
            let k = &app.scene.lights[i].kind;
            (app.gizmo.mode == GizmoMode::Rotate && matches!(k, LightKind::Point))
                || (app.gizmo.mode == GizmoMode::Scale && !matches!(k, LightKind::Area { .. } | LightKind::Portal { .. }))
        }
        Some(Target::Fog(_)) => app.gizmo.mode == GizmoMode::Rotate,
        _ => false,
    };
    if app.gizmo.mode == GizmoMode::Select || tgt.is_none() || pointless || ui.input(|i| i.modifiers.alt) {
        app.gizmo.drag = None;
        app.gizmo.hovered = None;
        return false;
    }
    let tgt = tgt.expect("bersaglio");
    let Some(f) = frame_for(app, rect, tgt, &sel) else { return false };

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
                let start = match tgt {
                    Target::Nodes => {
                        app.push_undo(&sel);
                        DragStart::Nodes(sel.iter().map(|&i| (i, app.scene.nodes[i].user)).collect())
                    }
                    Target::Light(i) => DragStart::Light(i, app.scene.lights[i].clone()),
                    Target::Fog(i) => {
                        let f = &app.scene.fogs[i];
                        DragStart::Fog(i, f.emitter, f.size, f.zone_offset)
                    }
                };
                let contact = match (tgt, h) {
                    (Target::Nodes, Handle::Axis(_) | Handle::Plane(_) | Handle::Screen) => Contact::build(&app.scene, &sel),
                    _ => None,
                };
                app.gizmo.drag = Some(Drag {
                    contact,
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
            match &drag.start {
                DragStart::Nodes(start) => {
                    for (i, u) in start {
                        app.scene.nodes[*i].user = *u;
                    }
                    app.discard_last_undo();
                    app.geometry_changed();
                }
                DragStart::Light(i, l) => {
                    app.scene.lights[*i] = l.clone();
                    app.shading_changed();
                }
                DragStart::Fog(i, e, size, off) => {
                    app.scene.fogs[*i].emitter = *e;
                    app.scene.fogs[*i].size = *size;
                    app.scene.fogs[*i].zone_offset = *off;
                    app.shading_changed();
                }
            }
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
    let (op, mut text) = match drag.handle {
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
            // Verso della rotazione rispetto al trascinamento lungo l'anello.
            let mut deg = ang.to_degrees();
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
    match &drag.start {
        DragStart::Nodes(start) => {
            // Snap a contatto: lo spostamento si accorcia fino al primo ostacolo.
            let mut op = op;
            if let (Some(c), Op::World(m)) = (&drag.contact, &op) {
                let t = m.w_axis.truncate();
                let allowed = c.limit(t);
                if allowed < t.length() - 1e-7 {
                    let t = t.normalize_or_zero() * allowed;
                    op = Op::World(Mat4::from_translation(t));
                    text = format!("{text}  · a contatto ({:.1} mm)", allowed * 1000.0);
                }
            }
            for (i, st) in start {
                let u = match &op {
                    Op::World(m) => apply_world(&app.scene, *i, st, *m),
                    Op::Scale(s) => UserTransform {
                        scale: st.scale * *s,
                        ..*st
                    },
                };
                app.scene.nodes[*i].user = u;
            }
            app.geometry_changed();
        }
        DragStart::Light(i, st) => {
            let l = &mut app.scene.lights[*i];
            match &op {
                // Spostamento: posizione e bersaglio insieme (la direzione resta);
                // rotazione attorno alla luce: ruota il bersaglio.
                Op::World(m) => {
                    l.position = m.transform_point3(st.position);
                    l.target = m.transform_point3(st.target);
                }
                Op::Scale(s) => match st.kind {
                    LightKind::Area { width, height } => {
                        l.kind = LightKind::Area {
                            width: (width * s.x.abs()).max(1e-3),
                            height: (height * s.y.abs()).max(1e-3),
                        };
                    }
                    LightKind::Portal { width, height } => {
                        l.kind = LightKind::Portal {
                            width: (width * s.x.abs()).max(1e-3),
                            height: (height * s.y.abs()).max(1e-3),
                        };
                    }
                    _ => {}
                },
            }
            app.shading_changed();
        }
        DragStart::Fog(i, e, size, off) => {
            let zone = app.fog_gizmo_zone;
            let f = &mut app.scene.fogs[*i];
            match &op {
                // Modalità zona: si sposta il riquadro, l'emettitore resta dov'è.
                Op::World(m) if zone => f.zone_offset = m.transform_point3(*e + *off) - *e,
                Op::World(m) => f.emitter = m.transform_point3(*e),
                Op::Scale(s) => f.size = (*size * s.abs()).max(Vec3::splat(0.05)),
            }
            app.shading_changed();
        }
    }
    app.gizmo.readout = Some(text);
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

/// Geometria del cono di uno spot: apice nella luce, base alla distanza del bersaglio.
pub struct SpotCone {
    apex: Vec3,
    axis: Vec3,
    u: Vec3,
    v: Vec3,
    len: f32,
}

impl SpotCone {
    pub fn of(l: &Light) -> SpotCone {
        let [u, v, axis] = light_axes(l);
        SpotCone {
            apex: l.position,
            axis,
            u,
            v,
            len: (l.target - l.position).length().max(0.1),
        }
    }

    fn end(&self) -> Vec3 {
        self.apex + self.axis * self.len
    }

    /// Punto del bordo per un'apertura (angolo pieno, in gradi) e un azimut.
    fn point(&self, angle_deg: f32, az: f32) -> Vec3 {
        let r = self.len * (angle_deg.to_radians() * 0.5).min(1.48).tan();
        self.end() + (self.u * az.cos() + self.v * az.sin()) * r
    }

    /// Azimut del lato del cono rivolto verso `eye` (lì le maniglie sono sempre visibili).
    fn front_az(&self, eye: Vec3) -> f32 {
        let d = eye - self.end();
        let (x, y) = (d.dot(self.u), d.dot(self.v));
        if x * x + y * y < 1e-12 {
            0.0
        } else {
            y.atan2(x)
        }
    }

    /// Cerchio del bordo (65 punti, chiuso).
    pub fn circle(&self, angle_deg: f32) -> Vec<Vec3> {
        (0..=64)
            .map(|k| self.point(angle_deg, k as f32 / 64.0 * std::f32::consts::TAU))
            .collect()
    }
}

/// Maniglie del cono dello spot selezionato: quadrato = apertura del cono, cerchio =
/// inizio della sfumatura del bordo. Restituisce vero se usano il mouse.
pub fn cone_handles(app: &mut ForgeApp, ui: &egui::Ui, rect: egui::Rect, resp: &egui::Response, painter: &egui::Painter) -> bool {
    let Some(Target::Light(i)) = target(app) else {
        app.gizmo.cone_drag = None;
        return false;
    };
    let l = &app.scene.lights[i];
    let LightKind::Spot {
        inner_angle_deg: inner,
        outer_angle_deg: outer,
    } = l.kind
    else {
        app.gizmo.cone_drag = None;
        return false;
    };
    let cone = SpotCone::of(l);
    let cam = &app.scene.camera;
    let az = cone.front_az(cam.position);
    let (Some(po), Some(pi)) = (project(cam, rect, cone.point(outer, az)), project(cam, rect, cone.point(inner, az))) else {
        return false;
    };
    if app.gizmo.cone_drag.is_none() && resp.drag_started_by(egui::PointerButton::Primary) {
        if let Some(origin) = ui.input(|i| i.pointer.press_origin()) {
            if origin.distance(po) < 9.0 {
                app.gizmo.cone_drag = Some((true, l.kind.clone()));
            } else if origin.distance(pi) < 9.0 {
                app.gizmo.cone_drag = Some((false, l.kind.clone()));
            }
        }
    }
    let hover = resp.hover_pos().filter(|p| p.distance(po) < 9.0 || p.distance(pi) < 9.0);
    if let Some((is_outer, start)) = app.gizmo.cone_drag.clone() {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.scene.lights[i].kind = start;
            app.gizmo.cone_drag = None;
            app.gizmo.readout = None;
            app.shading_changed();
            return true;
        }
        if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged()) {
            let (o, d) = mouse_ray(app, rect, pos);
            if let Some(hit) = plane_hit(o, d, cone.end(), cone.axis) {
                let rho = (hit - cone.end()).length();
                let ang = (2.0 * (rho / cone.len).atan().to_degrees()).clamp(1.0, 170.0);
                let (ni, no) = if is_outer {
                    // L'apertura cambia mantenendo la stessa proporzione di sfumatura.
                    (inner / outer.max(1e-3) * ang, ang)
                } else {
                    (ang.min(outer), outer)
                };
                app.scene.lights[i].kind = LightKind::Spot {
                    inner_angle_deg: ni,
                    outer_angle_deg: no,
                };
                app.gizmo.readout = Some(format!("Cono {no:.1}°  ·  sfumatura {:.0}%", (1.0 - ni / no.max(1e-3)) * 100.0));
                app.shading_changed();
            }
        }
        if resp.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
            app.gizmo.cone_drag = None;
            app.gizmo.readout = None;
        }
    }
    let active = app.gizmo.cone_drag.as_ref().map(|c| c.0);
    let hot = |is_outer: bool, p: egui::Pos2| active == Some(is_outer) || (active.is_none() && hover.is_some_and(|h| h.distance(p) < 9.0));
    let col = |h: bool| if h { HOVER } else { egui::Color32::from_rgb(240, 220, 120) };
    painter.rect_filled(egui::Rect::from_center_size(po, egui::vec2(9.0, 9.0)), 1.0, col(hot(true, po)));
    painter.circle_filled(pi, 5.0, col(hot(false, pi)));
    if let (Some(t), Some(p)) = (&app.gizmo.readout, painter.ctx().pointer_latest_pos()) {
        if active.is_some() {
            painter.text(
                p + egui::vec2(16.0, 18.0),
                egui::Align2::LEFT_TOP,
                t,
                egui::FontId::proportional(13.0),
                egui::Color32::WHITE,
            );
        }
    }
    active.is_some() || hover.is_some()
}

/// Barra degli strumenti nella viewport: modalità, assi globali/locali, passi di snap.
pub fn toolbar(app: &mut ForgeApp, ui: &mut egui::Ui, rect: egui::Rect) {
    // Dentro la viewport (stesso livello del pannello): le finestre restano sopra.
    let bar = egui::Rect::from_min_size(rect.left_top() + egui::vec2(8.0, 8.0), egui::vec2(rect.width() - 16.0, 40.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(bar)
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
    );
    {
        let ui = &mut child;
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
    }
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

    /// Pavimento 10×10 a y = 0, cubo A di 1 m a quota `y` in x = 0, cubo B di 1 m in x = 3 appoggiato.
    fn contact_scene(y: f32) -> (Scene, usize) {
        let mut s = Scene::default();
        let floor = s.add_mesh(forge_scene::primitives::plane(10.0, 10.0));
        let cube = s.add_mesh(forge_scene::primitives::cuboid(Vec3::ONE));
        s.add_node(None, ComponentId("pavimento".into()), "Pavimento", Mat4::IDENTITY, Some(floor));
        let a = s.add_node(
            None,
            ComponentId("a".into()),
            "A",
            Mat4::from_translation(Vec3::new(0.0, y + 0.5, 0.0)),
            Some(cube),
        );
        s.add_node(
            None,
            ComponentId("b".into()),
            "B",
            Mat4::from_translation(Vec3::new(3.0, 0.5, 0.0)),
            Some(cube),
        );
        s.refresh_pivots();
        s.nodes[a].contact_snap = true;
        (s, a)
    }

    #[test]
    fn contact_snap_stops_on_the_floor_and_against_objects() {
        // Cubo sospeso a 1 m: scendendo di 3 m si ferma dopo 1 m (tocca il pavimento).
        let (s, a) = contact_scene(1.0);
        let c = Contact::build(&s, &[a]).expect("snap attivo");
        let down = c.limit(Vec3::new(0.0, -3.0, 0.0));
        assert!((down - 1.0).abs() < 1e-3, "si ferma a {down}");
        // Verso l'alto e di lato nessun ostacolo.
        assert!((c.limit(Vec3::new(0.0, 2.0, 0.0)) - 2.0).abs() < 1e-6);
        // Appoggiato: scivola sul pavimento, ma si ferma contro il cubo B (facce a 1 m e 2.5 m).
        let (s, a) = contact_scene(0.0);
        let c = Contact::build(&s, &[a]).unwrap();
        let side = c.limit(Vec3::new(4.0, 0.0, 0.0));
        assert!((side - 2.0).abs() < 1e-3, "contro B a {side}");
        assert!(
            (c.limit(Vec3::new(0.0, 0.0, 2.0)) - 2.0).abs() < 1e-6,
            "deve scivolare sul pavimento"
        );
        // Non scende sotto il pavimento.
        assert!(c.limit(Vec3::new(0.0, -0.5, 0.0)) < 1e-3);
        // Spostamento in diagonale verso B: si ferma quando le facce si toccano lungo x.
        let diag = c.limit(Vec3::new(4.0, 0.0, 0.4));
        assert!((diag / Vec3::new(4.0, 0.0, 0.4).length() * 4.0 - 2.0).abs() < 1e-2, "{diag}");
        // Senza snap sull'oggetto non si prepara nulla.
        let (mut s, a) = contact_scene(1.0);
        s.nodes[a].contact_snap = false;
        assert!(Contact::build(&s, &[a]).is_none());
    }

    #[test]
    fn spot_cone_radius_matches_angle() {
        let mut l = Light::point("S", Vec3::new(0.0, 2.0, 0.0), 1.0);
        l.kind = LightKind::Spot {
            inner_angle_deg: 20.0,
            outer_angle_deg: 60.0,
        };
        l.target = Vec3::ZERO;
        let c = SpotCone::of(&l);
        for p in c.circle(60.0) {
            // Base a y = 0, raggio 2·tan(30°).
            assert!(p.y.abs() < 1e-5);
            assert!((Vec3::new(p.x, 0.0, p.z).length() - 2.0 * 30f32.to_radians().tan()).abs() < 1e-4);
        }
    }

    #[test]
    fn axis_param_finds_closest_point() {
        // Raggio verticale che attraversa l'asse X in x = 3.
        let s = axis_param(Vec3::new(3.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0), Vec3::ZERO, Vec3::X).unwrap();
        assert!((s - 3.0).abs() < 1e-5);
        assert!(axis_param(Vec3::ZERO, Vec3::X, Vec3::ZERO, Vec3::X).is_none());
    }
}
