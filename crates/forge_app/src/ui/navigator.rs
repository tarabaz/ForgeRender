//! Navigatore degli assi nella viewport (come in Blender): palline X/Y/Z e
//! opposte; un clic porta la vista lungo quell'asse con una breve animazione,
//! trascinando sul navigatore si orbita.

use crate::app::ForgeApp;
use eframe::egui;
use forge_math::Vec3;
use std::time::{Duration, Instant};

const RADIUS: f32 = 42.0;
const BALL: f32 = 9.0;
const ANIM: Duration = Duration::from_millis(220);

/// Animazione della vista verso una direzione (yaw/pitch dell'orbita).
pub struct NavAnim {
    from: (f32, f32),
    to: (f32, f32),
    start: Instant,
}

struct Axis {
    dir: Vec3,
    color: egui::Color32,
    label: &'static str,
    view: &'static str,
    positive: bool,
}

fn axes() -> [Axis; 6] {
    let (r, g, b) = (
        egui::Color32::from_rgb(225, 75, 75),
        egui::Color32::from_rgb(110, 200, 90),
        egui::Color32::from_rgb(80, 135, 240),
    );
    [
        Axis {
            dir: Vec3::X,
            color: r,
            label: "X",
            view: "Destra",
            positive: true,
        },
        Axis {
            dir: Vec3::Y,
            color: g,
            label: "Y",
            view: "Alto",
            positive: true,
        },
        Axis {
            dir: Vec3::Z,
            color: b,
            label: "Z",
            view: "Frontale",
            positive: true,
        },
        Axis {
            dir: -Vec3::X,
            color: r,
            label: "-X",
            view: "Sinistra",
            positive: false,
        },
        Axis {
            dir: -Vec3::Y,
            color: g,
            label: "-Y",
            view: "Basso",
            positive: false,
        },
        Axis {
            dir: -Vec3::Z,
            color: b,
            label: "-Z",
            view: "Retro",
            positive: false,
        },
    ]
}

/// Yaw/pitch dell'orbita per guardare il bersaglio dalla direzione `d`.
fn yaw_pitch(d: Vec3, current_yaw: f32) -> (f32, f32) {
    let lim = std::f32::consts::FRAC_PI_2 - 0.01;
    if d.y.abs() > 0.99 {
        // Dall'alto o dal basso: si mantiene l'orientamento orizzontale attuale.
        (current_yaw, if d.y > 0.0 { lim } else { -lim })
    } else {
        (d.x.atan2(d.z), d.y.clamp(-1.0, 1.0).asin().clamp(-lim, lim))
    }
}

/// Avanza l'animazione in corso (da chiamare a ogni frame della viewport).
pub fn tick(app: &mut ForgeApp, ctx: &egui::Context) {
    let Some(a) = &app.nav_anim else { return };
    let t = (a.start.elapsed().as_secs_f32() / ANIM.as_secs_f32()).min(1.0);
    let s = t * t * (3.0 - 2.0 * t); // smoothstep
                                     // Yaw lungo l'arco più breve.
    let mut dy = a.to.0 - a.from.0;
    dy = (dy + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    app.orbit.yaw = a.from.0 + dy * s;
    app.orbit.pitch = a.from.1 + (a.to.1 - a.from.1) * s;
    if t >= 1.0 {
        app.nav_anim = None;
    } else {
        ctx.request_repaint();
    }
    app.camera_changed();
}

/// Disegna il navigatore e gestisce clic e trascinamento. Restituisce vero se usa il mouse.
pub fn show(app: &mut ForgeApp, ui: &egui::Ui, rect: egui::Rect, painter: &egui::Painter) -> bool {
    let center = egui::pos2(rect.right() - RADIUS - 14.0, rect.top() + RADIUS + 14.0);
    let area = egui::Rect::from_center_size(center, egui::vec2(2.0 * (RADIUS + BALL), 2.0 * (RADIUS + BALL)));
    let resp = ui.interact(area, egui::Id::NULL.with("navigatore-assi"), egui::Sense::click_and_drag());
    let cam = app.scene.camera.clone();
    let (r, u, f) = cam.basis();

    // Posizioni a schermo; prima le palline più lontane (dietro).
    let mut items: Vec<(Axis, egui::Pos2, f32)> = axes()
        .into_iter()
        .map(|a| {
            let p = center + egui::vec2(a.dir.dot(r), -a.dir.dot(u)) * RADIUS;
            let depth = a.dir.dot(f);
            (a, p, depth)
        })
        .collect();
    items.sort_by(|a, b| b.2.total_cmp(&a.2));

    let hover = resp.hover_pos().and_then(|m| {
        items
            .iter()
            .rev()
            .find(|(_, p, _)| p.distance(m) <= BALL + 2.0)
            .map(|(a, ..)| (a.dir, a.view))
    });
    if resp.hovered() || resp.dragged() {
        painter.circle_filled(center, RADIUS + BALL + 2.0, egui::Color32::from_white_alpha(18));
    }
    for (a, p, _) in &items {
        let hot = hover.is_some_and(|(d, _)| d == a.dir);
        if a.positive {
            painter.line_segment([center, *p], egui::Stroke::new(2.0, a.color));
            painter.circle_filled(*p, BALL, if hot { egui::Color32::WHITE } else { a.color });
            painter.text(
                *p,
                egui::Align2::CENTER_CENTER,
                a.label,
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
        } else {
            let c = if hot { egui::Color32::WHITE } else { a.color.gamma_multiply(0.45) };
            painter.circle_filled(*p, BALL - 1.5, c);
            painter.circle_stroke(*p, BALL - 1.5, egui::Stroke::new(1.0, a.color));
        }
    }
    if let Some((_, view)) = hover {
        painter.text(
            center + egui::vec2(0.0, RADIUS + BALL + 12.0),
            egui::Align2::CENTER_CENTER,
            view,
            egui::FontId::proportional(11.0),
            egui::Color32::from_gray(220),
        );
    }

    if resp.clicked() {
        if let Some((dir, _)) = hover {
            let to = yaw_pitch(dir, app.orbit.yaw);
            app.nav_anim = Some(NavAnim {
                from: (app.orbit.yaw, app.orbit.pitch),
                to,
                start: Instant::now(),
            });
            ui.ctx().request_repaint();
        }
    }
    if resp.dragged() {
        let d = resp.drag_delta();
        app.nav_anim = None;
        app.orbit.orbit(d.x * 0.012, d.y * 0.012);
        app.camera_changed();
    }
    resp.hovered() || resp.dragged() || resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_scene::OrbitController;

    #[test]
    fn axis_views_look_along_the_axis() {
        for d in [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z, Vec3::Y, -Vec3::Y] {
            let (yaw, pitch) = yaw_pitch(d, 0.3);
            let o = OrbitController {
                target: Vec3::ZERO,
                distance: 5.0,
                yaw,
                pitch,
                vfov_deg: 40.0,
            };
            let cam_dir = (o.camera().position - o.target).normalize();
            assert!(cam_dir.dot(d) > 0.999, "{d}: camera da {cam_dir}");
        }
    }
}
