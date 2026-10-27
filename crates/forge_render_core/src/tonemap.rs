use crate::ToneMapping;
use forge_math::Vec3;
use forge_scene::material::linear_to_srgb;
use rayon::prelude::*;

#[inline]
fn aces(x: f32) -> f32 {
    const A: f32 = 2.51;
    const B: f32 = 0.03;
    const C: f32 = 2.43;
    const D: f32 = 0.59;
    const E: f32 = 0.14;
    ((x * (A * x + B)) / (x * (C * x + D) + E)).clamp(0.0, 1.0)
}

/// Converte un valore di radianza lineare in sRGB 8 bit.
#[inline]
pub fn tonemap_pixel(c: Vec3, mode: ToneMapping, exposure_mul: f32) -> [u8; 3] {
    let c = c * exposure_mul;
    let f = |v: f32| {
        let v = if v.is_finite() { v.max(0.0) } else { 0.0 };
        let t = match mode {
            ToneMapping::Standard => v.min(1.0),
            ToneMapping::AcesFilmic => aces(v),
        };
        (linear_to_srgb(t) * 255.0 + 0.5) as u8
    };
    [f(c.x), f(c.y), f(c.z)]
}

/// Converte un'immagine lineare in RGBA8 (in parallelo).
pub fn tonemap_to_rgba8(linear: &[[f32; 3]], mode: ToneMapping, exposure_ev: f32) -> Vec<u8> {
    let mul = 2f32.powf(exposure_ev);
    let mut out = vec![0u8; linear.len() * 4];
    out.par_chunks_mut(4).zip(linear.par_iter()).for_each(|(o, p)| {
        let [r, g, b] = tonemap_pixel(Vec3::from_array(*p), mode, mul);
        o.copy_from_slice(&[r, g, b, 255]);
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_is_srgb() {
        assert_eq!(tonemap_pixel(Vec3::ZERO, ToneMapping::Standard, 1.0), [0, 0, 0]);
        assert_eq!(tonemap_pixel(Vec3::ONE, ToneMapping::Standard, 1.0), [255, 255, 255]);
        assert_eq!(tonemap_pixel(Vec3::splat(0.2158), ToneMapping::Standard, 1.0)[0], 128);
        assert_eq!(tonemap_pixel(Vec3::splat(f32::NAN), ToneMapping::AcesFilmic, 1.0), [0, 0, 0]);
    }

    #[test]
    fn aces_is_monotonic() {
        let mut prev = 0.0;
        for i in 0..100 {
            let v = aces(i as f32 * 0.1);
            assert!(v >= prev);
            prev = v;
        }
    }
}
