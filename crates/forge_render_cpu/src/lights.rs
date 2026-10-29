//! Campionamento delle sorgenti luminose (Next Event Estimation).

use forge_math::Vec3;
use forge_render_core::{RenderLight, RenderLightKind};

pub struct LightSample {
    /// Direzione dal punto ombreggiato verso la luce.
    pub wi: Vec3,
    /// Distanza dalla luce (∞ per le direzionali).
    pub dist: f32,
    /// Radianza (area) o intensità/irradianza già divisa per la distanza² (luci delta).
    pub li: Vec3,
    /// PDF in angolo solido (0 per luci delta).
    pub pdf: f32,
    pub delta: bool,
}

pub fn sample_light(light: &RenderLight, p: Vec3, u1: f32, u2: f32) -> Option<LightSample> {
    match light.kind {
        RenderLightKind::Point { position } => {
            let d = position - p;
            let d2 = d.length_squared();
            if d2 <= 0.0 {
                return None;
            }
            let dist = d2.sqrt();
            let window = light.range_window(dist);
            if window <= 0.0 {
                return None;
            }
            Some(LightSample {
                wi: d / dist,
                dist,
                li: light.emission * (window / d2),
                pdf: 0.0,
                delta: true,
            })
        }
        RenderLightKind::Spot {
            position,
            direction,
            cos_inner,
            cos_outer,
        } => {
            let d = position - p;
            let d2 = d.length_squared();
            if d2 <= 0.0 {
                return None;
            }
            let dist = d2.sqrt();
            let wi = d / dist;
            let cos = (-wi).dot(direction);
            let falloff = smoothstep(cos_outer, cos_inner.max(cos_outer + 1e-4), cos) * light.range_window(dist);
            if falloff <= 0.0 {
                return None;
            }
            Some(LightSample {
                wi,
                dist,
                li: light.emission * (falloff / d2),
                pdf: 0.0,
                delta: true,
            })
        }
        RenderLightKind::Directional { direction } => Some(LightSample {
            wi: -direction,
            dist: f32::INFINITY,
            li: light.emission,
            pdf: 0.0,
            delta: true,
        }),
        RenderLightKind::Area {
            center,
            normal,
            u,
            v,
            half_w,
            half_h,
            area,
        } => {
            let q = center + u * ((2.0 * u1 - 1.0) * half_w) + v * ((2.0 * u2 - 1.0) * half_h);
            let d = q - p;
            let d2 = d.length_squared();
            if d2 <= 0.0 {
                return None;
            }
            let dist = d2.sqrt();
            let wi = d / dist;
            let cos_l = (-wi).dot(normal);
            if cos_l <= 0.0 {
                return None; // emette solo dalla faccia frontale
            }
            Some(LightSample {
                wi,
                dist,
                li: light.emission,
                pdf: d2 / (cos_l * area),
                delta: false,
            })
        }
    }
}

/// Intersezione di un raggio con una luce ad area. Restituisce `(t, radianza)`
/// (radianza nulla se il raggio colpisce il retro) e la PDF in angolo solido
/// che avrebbe avuto il campionamento della luce in quel punto.
pub fn intersect_area(light: &RenderLight, o: Vec3, d: Vec3, t_max: f32) -> Option<(f32, Vec3, f32)> {
    let RenderLightKind::Area {
        center,
        normal,
        u,
        v,
        half_w,
        half_h,
        area,
    } = light.kind
    else {
        return None;
    };
    let denom = d.dot(normal);
    if denom.abs() < 1e-12 {
        return None;
    }
    let t = (center - o).dot(normal) / denom;
    if t <= 1e-6 || t >= t_max {
        return None;
    }
    let local = o + d * t - center;
    if local.dot(u).abs() > half_w || local.dot(v).abs() > half_h {
        return None;
    }
    let cos_l = -denom;
    if cos_l <= 0.0 {
        return Some((t, Vec3::ZERO, 0.0));
    }
    Some((t, light.emission, t * t / (cos_l * area)))
}

#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(range: f32, fade_start: f32) -> RenderLight {
        RenderLight {
            kind: RenderLightKind::Point { position: Vec3::ZERO },
            emission: Vec3::splat(10.0),
            cast_shadows: true,
            visible_to_camera: false,
            range,
            fade_start,
        }
    }

    #[test]
    fn range_cuts_and_fades_independently_of_intensity() {
        let li = |l: &RenderLight, d: f32| sample_light(l, Vec3::new(d, 0.0, 0.0), 0.5, 0.5).map_or(0.0, |s| s.li.x);
        // Illimitata: legge dell'inverso del quadrato, invariata.
        let free = point(0.0, 0.0);
        assert!((li(&free, 2.0) - 2.5).abs() < 1e-5);
        assert!((li(&free, 50.0) - 10.0 / 2500.0).abs() < 1e-7);
        // Raggio 4 m, sfumatura da 2 m: piena dentro, metà a 3 m, zero oltre.
        let l = point(4.0, 2.0);
        assert!((li(&l, 1.5) - li(&free, 1.5)).abs() < 1e-5);
        assert!((li(&l, 3.0) - 0.5 * li(&free, 3.0)).abs() < 1e-5);
        assert_eq!(li(&l, 4.0), 0.0);
        assert_eq!(li(&l, 9.0), 0.0);
        // Taglio netto (sfumatura 0): piena fino al bordo.
        let hard = point(4.0, 4.0);
        assert!((li(&hard, 3.99) - li(&free, 3.99)).abs() < 1e-6);
        // Indipendente dall'intensità: stesso rapporto con emissione 10× più forte.
        let mut strong = point(4.0, 2.0);
        strong.emission *= 10.0;
        assert!((li(&strong, 3.0) / li(&l, 3.0) - 10.0).abs() < 1e-3);
    }
}
