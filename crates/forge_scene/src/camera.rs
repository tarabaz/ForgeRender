use forge_math::{Aabb, Mat4, Vec3};
use serde::{Deserialize, Serialize};

/// Camera prospettica condivisa da viewport e renderer. Con [`Lens::dof`] è una
/// lente sottile (profondità di campo); senza è una camera pinhole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    /// Campo visivo verticale in gradi.
    pub vfov_deg: f32,
    #[serde(default)]
    pub lens: Lens,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(2.0, 1.5, 3.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            vfov_deg: 40.0,
            lens: Lens::default(),
        }
    }
}

/// Altezza del sensore di riferimento (full frame 36×24 mm): lega campo visivo e focale.
pub const SENSOR_HEIGHT_MM: f32 = 24.0;
/// Circolo di confusione per il full frame (limiti della zona nitida).
pub const CIRCLE_OF_CONFUSION_MM: f32 = 0.03;

/// Parametri dell'obiettivo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lens {
    /// Profondità di campo attiva.
    pub dof: bool,
    /// Apertura del diaframma (numero f/).
    pub f_stop: f32,
    /// Distanza di messa a fuoco in metri, lungo l'asse della camera.
    pub focus_distance: f32,
    /// Lamelle del diaframma (0 = apertura circolare, altrimenti 3..=12: bokeh poligonale).
    pub blades: u32,
    pub blade_rotation_deg: f32,
    /// Decentramento dell'obiettivo (shift) in frazioni della larghezza / altezza del fotogramma.
    pub shift_x: f32,
    pub shift_y: f32,
}

impl Default for Lens {
    fn default() -> Self {
        Self {
            dof: false,
            f_stop: 5.6,
            focus_distance: 2.0,
            blades: 0,
            blade_rotation_deg: 0.0,
            shift_x: 0.0,
            shift_y: 0.0,
        }
    }
}

impl Camera {
    /// Lunghezza focale equivalente full frame in millimetri.
    pub fn focal_mm(&self) -> f32 {
        SENSOR_HEIGHT_MM * 0.5 / (self.vfov_deg.to_radians() * 0.5).tan()
    }

    pub fn set_focal_mm(&mut self, mm: f32) {
        let mm = mm.clamp(4.0, 2000.0);
        self.vfov_deg = (2.0 * (SENSOR_HEIGHT_MM * 0.5 / mm).atan()).to_degrees();
    }

    /// Raggio della lente in metri (0 se la profondità di campo è spenta).
    pub fn lens_radius(&self) -> f32 {
        if !self.lens.dof || !self.lens.f_stop.is_finite() || self.lens.f_stop <= 0.0 {
            return 0.0;
        }
        self.focal_mm() * 1e-3 / self.lens.f_stop.max(0.5) * 0.5
    }

    /// Limiti della zona nitida (vicino, lontano) in metri dalla camera; il lontano può essere
    /// infinito (oltre la distanza iperfocale). Formule della profondità di campo fotografica.
    pub fn dof_limits(&self) -> (f32, f32) {
        let f = self.focal_mm() * 1e-3;
        let n = self.lens.f_stop.max(0.5);
        let c = CIRCLE_OF_CONFUSION_MM * 1e-3;
        let s = self.lens.focus_distance.max(f * 1.01);
        let h = f * f / (n * c) + f;
        let near = s * (h - f) / (h + s - 2.0 * f);
        let far = if s >= h { f32::INFINITY } else { s * (h - f) / (h - s) };
        (near, far)
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
    /// Obiettivo della camera (conservato mentre si orbita).
    #[serde(default)]
    pub lens: Lens,
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
            lens: cam.lens.clone(),
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
            lens: self.lens.clone(),
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
    #[test]
    fn focal_length_and_depth_of_field() {
        let mut c = Camera::default();
        c.set_focal_mm(50.0);
        assert!((c.focal_mm() - 50.0).abs() < 1e-3);
        assert!(
            (c.vfov_deg - 26.99).abs() < 0.05,
            "50 mm full frame ≈ 27° verticali: {}",
            c.vfov_deg
        );
        c.lens.dof = true;
        c.lens.focus_distance = 2.0;
        c.lens.f_stop = 2.0;
        let (n2, f2) = c.dof_limits();
        c.lens.f_stop = 11.0;
        let (n11, f11) = c.dof_limits();
        assert!(n2 < 2.0 && f2 > 2.0);
        assert!(n11 < n2 && f11 > f2, "chiudendo il diaframma la zona nitida si allarga");
        // 50 mm f/2 a 2 m: zona nitida ≈ 1.91–2.10 m (tabelle fotografiche).
        assert!((n2 - 1.91).abs() < 0.02 && (f2 - 2.10).abs() < 0.02, "{n2} {f2}");
        // Oltre l'iperfocale (50 mm f/11 ≈ 7.6 m) il limite lontano è infinito.
        c.lens.focus_distance = 10.0;
        assert!(c.dof_limits().1.is_infinite());
        // Raggio della lente: focale / (2 · f).
        assert!((c.lens_radius() - 0.050 / 11.0 / 2.0).abs() < 1e-6);
        c.lens.dof = false;
        assert_eq!(c.lens_radius(), 0.0);
    }

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
