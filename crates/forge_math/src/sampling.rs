//! Funzioni di campionamento con le rispettive PDF.
//! Le direzioni restituite sono in spazio locale (z = normale) salvo indicazione.

use glam::{Vec2, Vec3};
use std::f32::consts::{FRAC_1_PI, PI};

/// Emisfero pesato per coseno (mappatura concentrica di Shirley-Chiu).
/// PDF = cos(θ)/π.
#[inline]
pub fn cosine_hemisphere(u1: f32, u2: f32) -> Vec3 {
    let d = concentric_disk(u1, u2);
    let z = (1.0 - d.x * d.x - d.y * d.y).max(0.0).sqrt();
    Vec3::new(d.x, d.y, z)
}

#[inline]
pub fn cosine_hemisphere_pdf(cos_theta: f32) -> f32 {
    cos_theta.max(0.0) * FRAC_1_PI
}

/// Disco unitario con la mappatura concentrica (preserva la stratificazione).
#[inline]
pub fn concentric_disk(u1: f32, u2: f32) -> Vec2 {
    let ox = 2.0 * u1 - 1.0;
    let oy = 2.0 * u2 - 1.0;
    if ox == 0.0 && oy == 0.0 {
        return Vec2::ZERO;
    }
    let (r, theta) = if ox.abs() > oy.abs() {
        (ox, (PI / 4.0) * (oy / ox))
    } else {
        (oy, (PI / 2.0) - (PI / 4.0) * (ox / oy))
    };
    Vec2::new(r * theta.cos(), r * theta.sin())
}

/// Sfera uniforme, PDF = 1/(4π).
#[inline]
pub fn uniform_sphere(u1: f32, u2: f32) -> Vec3 {
    let z = 1.0 - 2.0 * u1;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let phi = 2.0 * PI * u2;
    Vec3::new(r * phi.cos(), r * phi.sin(), z)
}

pub const UNIFORM_SPHERE_PDF: f32 = 1.0 / (4.0 * PI);

/// Euristica della potenza (β = 2) di Veach per il Multiple Importance Sampling.
#[inline]
pub fn power_heuristic(pdf_a: f32, pdf_b: f32) -> f32 {
    let a = pdf_a * pdf_a;
    let b = pdf_b * pdf_b;
    if a + b == 0.0 {
        0.0
    } else {
        a / (a + b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pcg32;

    #[test]
    fn cosine_hemisphere_is_normalized_and_upper() {
        let mut rng = Pcg32::new(1, 1);
        // Stima Monte Carlo di ∫ pdf dω tramite campionamento uniforme della sfera: deve valere 1.
        let n = 200_000;
        let mut integral = 0.0f64;
        for _ in 0..n {
            let (a, b) = rng.next_2d();
            let d = uniform_sphere(a, b);
            integral += (cosine_hemisphere_pdf(d.z) / UNIFORM_SPHERE_PDF) as f64;
            let (a, b) = rng.next_2d();
            let c = cosine_hemisphere(a, b);
            assert!(c.z >= 0.0);
            assert!((c.length() - 1.0).abs() < 1e-4);
        }
        let integral = integral / n as f64;
        assert!((integral - 1.0).abs() < 0.01, "integrale {integral}");
    }
}
