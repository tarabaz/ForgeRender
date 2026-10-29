//! Navigatore degli assi nella viewport (come in Blender): palline X/Y/Z e
//! opposte; un clic porta la vista lungo quell'asse con una breve animazione,
//! trascinando sul navigatore si orbita.

use crate::app::ForgeApp;
use crate::i18n::tr;
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
            view: tr("Destra"),
            positive: true,
        },
        Axis {
            dir: Vec3::Y,
            color: g,
            label: "Y",
            view: tr("Alto"),
            positive: true,
        },
        Axis {
            dir: Vec3::Z,
            color: b,
            label: "Z",
            view: tr("Frontale"),
            positive: true,
        },
        Axis {
            dir: -Vec3::X,
            color: r,
            label: "-X",
            view: tr("Sinistra"),
            positive: false,
        },
        Axis {
            dir: -Vec3::Y,
            color: g,
            label: "-Y",
            view: tr("Basso"),
            positive: false,
        },
        Axis {
            dir: -Vec3::Z,
            color: b,
            label: "-Z",
            view: tr("Retro"),
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
    let buttons = projection_buttons(app, ui, center, painter);
    resp.hovered() || resp.dragged() || resp.clicked() || buttons
}

/// Inclinazione della vista isometrica: atan(1/√2) ≈ 35,26°.
pub const ISO_PITCH: f32 = 0.615_479_7;

/// Sotto il navigatore: cubo prospettico/ortografico (alterna la proiezione della vista) e
/// "Iso" (vista isometrica: ortografica, 45° di lato, 35,26° dall'alto).
fn projection_buttons(app: &mut ForgeApp, ui: &egui::Ui, center: egui::Pos2, painter: &egui::Painter) -> bool {
    let size = egui::vec2(30.0, 24.0);
    let y = center.y + RADIUS + BALL + 30.0;
    let proj_rect = egui::Rect::from_center_size(egui::pos2(center.x - 18.0, y), size);
    let iso_rect = egui::Rect::from_center_size(egui::pos2(center.x + 18.0, y), size);
    let ortho = app.orbit.lens.ortho;
    let proj = ui
        .interact(proj_rect, egui::Id::NULL.with("nav-proiezione"), egui::Sense::click())
        .on_hover_text(if ortho {
            tr("Vista ortografica (clic: torna prospettica)")
        } else {
            tr("Vista prospettica (clic: ortografica, misure senza prospettiva)")
        });
    let iso = ui
        .interact(iso_rect, egui::Id::NULL.with("nav-iso"), egui::Sense::click())
        .on_hover_text(tr("Vista isometrica (ortografica, 45° di lato)"));
    for (r, hot, on) in [(proj_rect, proj.hovered(), ortho), (iso_rect, iso.hovered(), false)] {
        let fill = if on {
            crate::ui::style::ACCENT.gamma_multiply(0.8)
        } else if hot {
            egui::Color32::from_white_alpha(60)
        } else {
            egui::Color32::from_black_alpha(120)
        };
        painter.rect_filled(r, 4.0, fill);
    }
    // Icona: cubo in fil di ferro, con la faccia posteriore più piccola se prospettico.
    let c = proj_rect.center();
    let stroke = egui::Stroke::new(1.3, egui::Color32::WHITE);
    let front = egui::Rect::from_center_size(c + egui::vec2(-2.5, 2.0), egui::vec2(12.0, 12.0));
    let back = if ortho {
        front.translate(egui::vec2(5.0, -4.0))
    } else {
        egui::Rect::from_center_size(c + egui::vec2(3.0, -2.5), egui::vec2(7.0, 7.0))
    };
    painter.rect_stroke(front, 0.0, stroke, egui::StrokeKind::Middle);
    painter.rect_stroke(back, 0.0, stroke, egui::StrokeKind::Middle);
    for (a, b) in [
        (front.left_top(), back.left_top()),
        (front.right_top(), back.right_top()),
        (front.left_bottom(), back.left_bottom()),
        (front.right_bottom(), back.right_bottom()),
    ] {
        painter.line_segment([a, b], stroke);
    }
    painter.text(
        iso_rect.center(),
        egui::Align2::CENTER_CENTER,
        tr("Iso"),
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );
    if proj.clicked() {
        app.toggle_ortho();
    }
    if iso.clicked() {
        app.orbit.lens.ortho = true;
        app.nav_anim = Some(NavAnim {
            from: (app.orbit.yaw, app.orbit.pitch),
            to: (std::f32::consts::FRAC_PI_4, ISO_PITCH),
            start: Instant::now(),
        });
        app.camera_changed();
        ui.ctx().request_repaint();
    }
    proj.hovered() || iso.hovered() || proj.clicked() || iso.clicked()
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
                lens: Default::default(),
            };
            let cam_dir = (o.camera().position - o.target).normalize();
            assert!(cam_dir.dot(d) > 0.999, "{d}: camera da {cam_dir}");
        }
    }
}
