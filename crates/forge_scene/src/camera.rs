use forge_math::{Aabb, Mat4, Vec3};
use serde::{Deserialize, Serialize};

/// Camera prospettica pinhole condivisa da viewport e renderer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    /// Campo visivo verticale in gradi.
    pub vfov_deg: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(2.0, 1.5, 3.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            vfov_deg: 40.0,
        }
    }
}

/// Camera di rendering con nome, salvata nel progetto.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderCamera {
    pub name: String,
    pub camera: Camera,
}

impl Camera {
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize_or(-Vec3::Z)
    }

    /// Base (destra, alto, avanti) della camera.
    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let f = self.forward();
        let mut r = f.cross(self.up);
        if r.length_squared() < 1e-12 {
            r = f.cross(Vec3::Z);
        }
        let r = r.normalize();
        let u = r.cross(f);
        (r, u, f)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.basis().1)
    }

    /// Proiezione per la viewport (profondità wgpu 0..1).
    pub fn projection_matrix(&self, aspect: f32, near: f32, far: f32) -> Mat4 {
        Mat4::perspective_rh(self.vfov_deg.to_radians(), aspect.max(1e-4), near, far)
    }

    /// Direzione del raggio primario per coordinate NDC (-1..1, y verso l'alto).
    #[inline]
    pub fn ray_dir(&self, ndc_x: f32, ndc_y: f32, aspect: f32) -> Vec3 {
        let (r, u, f) = self.basis();
        let h = (self.vfov_deg.to_radians() * 0.5).tan();
        (f + r * (ndc_x * h * aspect) + u * (ndc_y * h)).normalize()
    }
}

/// Controller orbitale (orbita, pan, zoom) che produce una [`Camera`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrbitController {
    pub target: Vec3,
    pub distance: f32,
    /// Rotazione attorno all'asse Y (radianti).
    pub yaw: f32,
    /// Elevazione (radianti), limitata per evitare il gimbal flip.
    pub pitch: f32,
    pub vfov_deg: f32,
}

impl Default for OrbitController {
    fn default() -> Self {
        Self::from_camera(&Camera::default())
    }
}

impl OrbitController {
    pub fn from_camera(cam: &Camera) -> Self {
        let off = cam.position - cam.target;
        let distance = off.length().max(1e-4);
        Self {
            target: cam.target,
            distance,
            yaw: off.x.atan2(off.z),
            pitch: (off.y / distance).clamp(-1.0, 1.0).asin(),
            vfov_deg: cam.vfov_deg,
        }
    }

    pub fn camera(&self) -> Camera {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let off = Vec3::new(sy * cp, sp, cy * cp) * self.distance;
        Camera {
            position: self.target + off,
            target: self.target,
            up: Vec3::Y,
            vfov_deg: self.vfov_deg,
        }
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx;
        let lim = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = (self.pitch + dy).clamp(-lim, lim);
    }

    /// Pan in unità di schermo normalizzate (frazione dell'altezza della vista).
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let cam = self.camera();
        let (r, u, _) = cam.basis();
        let h = 2.0 * self.distance * (self.vfov_deg.to_radians() * 0.5).tan();
        self.target += (-r * dx + u * dy) * h;
    }

    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(1e-4, 1e7);
    }

    /// Inquadra una bounding box.
    pub fn frame(&mut self, bounds: &Aabb) {
        if bounds.is_empty() {
            return;
        }
        self.target = bounds.center();
        let radius = bounds.radius().max(1e-4);
        let half = (self.vfov_deg.to_radians() * 0.5).min(1.5);
        self.distance = radius / half.sin() * 1.1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_roundtrip() {
        let cam = Camera::default();
        let o = OrbitController::from_camera(&cam);
        let c2 = o.camera();
        assert!((c2.position - cam.position).length() < 1e-4);
    }

    #[test]
    fn center_ray_is_forward() {
        let cam = Camera::default();
        assert!((cam.ray_dir(0.0, 0.0, 1.5) - cam.forward()).length() < 1e-5);
    }

    #[test]
    fn frame_contains_box() {
        let mut o = OrbitController::default();
        let b = Aabb::new(Vec3::splat(-10.0), Vec3::splat(10.0));
        o.frame(&b);
        assert!(o.distance > b.radius());
    }
}
